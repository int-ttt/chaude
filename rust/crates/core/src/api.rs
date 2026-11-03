// Engine: one main claude child behind an actor thread. The UI sends commands and reads UiEvent values.
// Commands and child stdout lines share one mpsc channel, so a single loop owns all state.
// Owned types only: no lifetimes. Subagent tool events are not surfaced in E1 (pipeline is E2).

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::event::{Block, Content, Delta, Event, StreamInner, parse_line};
use crate::osenv::resolve_bin;
use crate::proc::{self, ClaudeProc, ProcMsg, SpawnOpts};

// Text tool results of background agents say this; the agent keeps running after the tool result.
const ASYNC_AGENT_MARK: &str = "Async agent launched";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Main,
    // Text produced inside the subagent started by the Agent/Task tool_use with this id.
    Agent(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptSource {
    EscKey,
    CtrlC,
    Signal,
    Api,
}

impl InterruptSource {
    fn as_str(self) -> &'static str {
        match self {
            InterruptSource::EscKey => "esc",
            InterruptSource::CtrlC => "ctrl_c",
            InterruptSource::Signal => "signal",
            InterruptSource::Api => "api",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    TextDelta {
        role: Role,
        text: String,
    },
    MessageDone {
        role: Role,
    },
    ToolStart {
        id: String,
        name: String,
        summary: String,
    },
    ToolEnd {
        id: String,
        ok: bool,
    },
    AgentStart {
        id: String,
        description: String,
        started_at: u64,
    },
    AgentEnd {
        id: String,
        elapsed_ms: u64,
    },
    // tokens = tokens in the context window (last assistant usage); context_pct = tokens * 100 / context_window.
    Status {
        model: String,
        tokens: u64,
        cost_usd: f64,
        context_pct: u32,
    },
    Busy(bool),
    Queued(usize),
    TurnEnd {
        duration_ms: u64,
    },
    Rotated {
        reason: String,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Config {
    // Name or path of the claude binary; resolved on PATH like osenv::resolve_bin. Tests point this at a fake.
    pub claude_bin: String,
    pub model: String,
    pub resume: Option<String>,
    pub cwd: Option<PathBuf>,
    pub cache_ttl: String,
    pub compact_window: u32,
    pub context_window: u64,
    // Interrupt log sink: one text line per interrupt, with its InterruptSource. None disables logging.
    pub log_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            claude_bin: resolve_bin("claude"),
            model: "opus".to_string(),
            resume: None,
            cwd: None,
            cache_ttl: String::new(),
            compact_window: 0,
            context_window: 200_000,
            log_path: None,
        }
    }
}

enum Cmd {
    Submit(String),
    Interrupt(InterruptSource),
    Shutdown,
}

enum Input {
    Cmd(Cmd),
    Proc(ProcMsg),
}

pub struct EngineHandle {
    tx: Sender<Input>,
    join: Option<JoinHandle<()>>,
}

impl EngineHandle {
    // Starts a turn, or queues it behind the running turn (Queued(n)). Never interrupts.
    pub fn submit(&self, text: &str) {
        let _ = self.tx.send(Input::Cmd(Cmd::Submit(text.to_string())));
    }

    // Submits "/command" through the same queue as a normal prompt.
    pub fn slash(&self, command: &str) {
        self.submit(&format!("/{}", command.trim_start_matches('/')));
    }

    pub fn interrupt(&self, source: InterruptSource) {
        let _ = self.tx.send(Input::Cmd(Cmd::Interrupt(source)));
    }

    // Stops the child and joins the actor thread.
    pub fn shutdown(mut self) {
        let _ = self.tx.send(Input::Cmd(Cmd::Shutdown));
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        let _ = self.tx.send(Input::Cmd(Cmd::Shutdown));
    }
}

pub struct Engine;

impl Engine {
    // Spawn failures are reported as UiEvent::Error; the returned handle then rejects work with Error too.
    pub fn start(cfg: Config) -> (EngineHandle, Receiver<UiEvent>) {
        let (in_tx, in_rx) = mpsc::channel::<Input>();
        let (ui_tx, ui_rx) = mpsc::channel::<UiEvent>();
        let self_tx = in_tx.clone();
        let join = thread::spawn(move || actor(cfg, self_tx, in_rx, ui_tx));
        (
            EngineHandle {
                tx: in_tx,
                join: Some(join),
            },
            ui_rx,
        )
    }
}

struct State {
    cfg: Config,
    proc: Option<ClaudeProc>,
    ui: Sender<UiEvent>,
    busy: bool,
    turn_started: Option<Instant>,
    queue: VecDeque<String>,
    model: String,
    ctx_tokens: u64,
    cost_usd: f64,
    tools: HashSet<String>,
    // Agent tool_use id -> start time, for AgentEnd elapsed_ms.
    agents: HashMap<String, Instant>,
}

fn actor(cfg: Config, self_tx: Sender<Input>, rx: Receiver<Input>, ui: Sender<UiEvent>) {
    let opts = SpawnOpts {
        claude_bin: resolve_bin(&cfg.claude_bin),
        model: cfg.model.clone(),
        resume: cfg.resume.clone(),
        cwd: cfg.cwd.clone(),
        cache_ttl: cfg.cache_ttl.clone(),
        compact_window: cfg.compact_window,
    };
    let model = cfg.model.clone();
    let mut st = State {
        cfg,
        proc: None,
        ui,
        busy: false,
        turn_started: None,
        queue: VecDeque::new(),
        model,
        ctx_tokens: 0,
        cost_usd: 0.0,
        tools: HashSet::new(),
        agents: HashMap::new(),
    };
    match proc::spawn(&opts, self_tx, Input::Proc) {
        Ok(p) => st.proc = Some(p),
        Err(e) => st.emit(UiEvent::Error(format!("claude spawn failed: {e}"))),
    }
    while let Ok(input) = rx.recv() {
        match input {
            Input::Cmd(Cmd::Shutdown) => break,
            Input::Cmd(Cmd::Submit(text)) => st.submit(text),
            Input::Cmd(Cmd::Interrupt(src)) => st.interrupt(src),
            Input::Proc(ProcMsg::Line(line)) => st.on_line(&line),
            Input::Proc(ProcMsg::Eof) => st.on_eof(),
        }
    }
    if let Some(mut p) = st.proc.take() {
        p.kill();
    }
}

impl State {
    fn emit(&self, ev: UiEvent) {
        let _ = self.ui.send(ev);
    }

    fn submit(&mut self, text: String) {
        if self.proc.is_none() {
            self.emit(UiEvent::Error("claude is not running".to_string()));
            return;
        }
        if self.busy {
            self.queue.push_back(text);
            self.emit(UiEvent::Queued(self.queue.len()));
            return;
        }
        if self.send_turn(&text) {
            self.busy = true;
            self.emit(UiEvent::Busy(true));
        }
    }

    // Writes one user frame and starts the turn clock. Returns false if the write failed.
    fn send_turn(&mut self, text: &str) -> bool {
        let Some(p) = self.proc.as_mut() else {
            return false;
        };
        match p.write_user(text) {
            Ok(()) => {
                self.turn_started = Some(Instant::now());
                true
            }
            Err(e) => {
                self.emit(UiEvent::Error(format!("write to claude failed: {e}")));
                false
            }
        }
    }

    fn interrupt(&mut self, src: InterruptSource) {
        let busy = self.busy;
        let mut sent = false;
        if busy && let Some(p) = self.proc.as_mut() {
            match p.write_interrupt() {
                Ok(_) => sent = true,
                Err(e) => self.emit(UiEvent::Error(format!("interrupt write failed: {e}"))),
            }
        }
        self.log_interrupt(src, busy, sent);
    }

    fn log_interrupt(&self, src: InterruptSource, busy: bool, sent: bool) {
        let Some(path) = &self.cfg.log_path else {
            return;
        };
        let line = format!(
            "{} interrupt source={} busy={busy} sent={sent}\n",
            now_secs(),
            src.as_str()
        );
        let res = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(line.as_bytes()));
        if let Err(e) = res {
            self.emit(UiEvent::Error(format!("interrupt log failed: {e}")));
        }
    }

    fn on_eof(&mut self) {
        self.emit(UiEvent::Error("claude exited".to_string()));
        if let Some(mut p) = self.proc.take() {
            p.kill();
        }
        self.queue.clear();
        if self.busy {
            self.busy = false;
            self.turn_started = None;
            self.emit(UiEvent::Busy(false));
        }
    }

    fn on_line(&mut self, line: &str) {
        match parse_line(line) {
            Ok(ev) => self.on_event(ev),
            Err(e) => self.emit(UiEvent::Error(format!("bad stream line: {e}"))),
        }
    }

    fn on_event(&mut self, ev: Event) {
        match ev {
            Event::System(s) => {
                if let Some(m) = s.model {
                    self.model = m;
                    self.emit_status(self.cost_usd);
                }
            }
            Event::StreamEvent(w) => {
                let role = match w.parent_tool_use_id {
                    Some(id) => Role::Agent(id),
                    None => Role::Main,
                };
                match w.event {
                    StreamInner::ContentBlockDelta {
                        delta: Delta::TextDelta { text },
                    } => {
                        self.emit(UiEvent::TextDelta { role, text });
                    }
                    StreamInner::MessageStop => self.emit(UiEvent::MessageDone { role }),
                    _ => {}
                }
            }
            Event::Assistant(env) => {
                // Subagent messages are not surfaced beyond their text in E1.
                if env.parent_tool_use_id.is_some() {
                    return;
                }
                if let Some(u) = &env.message.usage {
                    self.ctx_tokens = u.context_tokens();
                    self.emit_status(self.cost_usd);
                }
                if let Some(Content::Blocks(blocks)) = env.message.content {
                    for b in blocks {
                        if let Block::ToolUse { id, name, input } = b {
                            self.on_tool_use(id, name, input.unwrap_or(Value::Null));
                        }
                    }
                }
            }
            Event::User(env) => {
                if env.parent_tool_use_id.is_some() {
                    return;
                }
                if let Some(Content::Blocks(blocks)) = env.message.content {
                    for b in blocks {
                        if let Block::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } = b
                        {
                            self.on_tool_result(tool_use_id, content, is_error == Some(true));
                        }
                    }
                }
            }
            Event::Result(r) => {
                if !self.busy {
                    return;
                }
                if let Some(c) = r.total_cost_usd {
                    self.cost_usd = c;
                }
                if r.is_error == Some(true) {
                    let msg = r
                        .result
                        .or(r.subtype)
                        .unwrap_or_else(|| "turn failed".to_string());
                    self.emit(UiEvent::Error(msg));
                }
                let duration_ms = r
                    .duration_ms
                    .unwrap_or_else(|| elapsed_ms(self.turn_started));
                self.emit_status(self.cost_usd);
                self.end_turn(duration_ms);
            }
            Event::ControlRequest(c) => {
                if c.request.subtype != "can_use_tool" {
                    return;
                }
                let tool = c.request.tool_name.unwrap_or_default();
                let msg = "chaude E1 engine has no permission prompt";
                let res = match self.proc.as_mut() {
                    Some(p) => p.write_permission(&c.request_id, false, Value::Null, msg),
                    None => Ok(()),
                };
                if let Err(e) = res {
                    self.emit(UiEvent::Error(format!("permission reply failed: {e}")));
                }
                self.emit(UiEvent::Error(format!(
                    "permission denied for {tool}: no prompt handler"
                )));
            }
            Event::RateLimitEvent(_) | Event::Unknown => {}
        }
    }

    fn on_tool_use(&mut self, id: String, name: String, input: Value) {
        if !self.tools.insert(id.clone()) {
            return;
        }
        let summary = tool_summary(&input);
        self.emit(UiEvent::ToolStart {
            id: id.clone(),
            name: name.clone(),
            summary,
        });
        if name == "Agent" || name == "Task" {
            let description = ["description", "subagent_type", "prompt"]
                .iter()
                .find_map(|k| {
                    input
                        .get(*k)
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                })
                .map(|s| clip(s, 80))
                .unwrap_or_default();
            self.agents.insert(id.clone(), Instant::now());
            self.emit(UiEvent::AgentStart {
                id,
                description,
                started_at: now_secs(),
            });
        }
    }

    fn on_tool_result(&mut self, id: String, content: Option<Value>, is_error: bool) {
        self.emit(UiEvent::ToolEnd {
            id: id.clone(),
            ok: !is_error,
        });
        let text = content.map(|v| v.to_string()).unwrap_or_default();
        // A background agent keeps running after its launch result; it ends later, which E1 does not track.
        if text.contains(ASYNC_AGENT_MARK) {
            return;
        }
        if let Some(t0) = self.agents.remove(&id) {
            self.emit(UiEvent::AgentEnd {
                id,
                elapsed_ms: t0.elapsed().as_millis() as u64,
            });
        }
    }

    // Finishes the running turn and starts the next queued one, if any.
    fn end_turn(&mut self, duration_ms: u64) {
        self.turn_started = None;
        self.emit(UiEvent::TurnEnd { duration_ms });
        match self.queue.pop_front() {
            Some(next) => {
                self.emit(UiEvent::Queued(self.queue.len()));
                if !self.send_turn(&next) {
                    self.busy = false;
                    self.emit(UiEvent::Busy(false));
                }
            }
            None => {
                self.busy = false;
                self.emit(UiEvent::Busy(false));
            }
        }
    }

    fn emit_status(&self, cost_usd: f64) {
        let window = self.cfg.context_window.max(1);
        let pct = u32::try_from(self.ctx_tokens.saturating_mul(100) / window).unwrap_or(u32::MAX);
        self.emit(UiEvent::Status {
            model: self.model.clone(),
            tokens: self.ctx_tokens,
            cost_usd,
            context_pct: pct,
        });
    }
}

// First short string field of a tool input, for the one-line ToolStart summary.
fn tool_summary(input: &Value) -> String {
    [
        "command",
        "file_path",
        "pattern",
        "description",
        "url",
        "prompt",
    ]
    .iter()
    .find_map(|k| input.get(*k).and_then(Value::as_str))
    .map(|s| clip(s, 80))
    .unwrap_or_default()
}

fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn elapsed_ms(since: Option<Instant>) -> u64 {
    since.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
