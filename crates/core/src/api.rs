// Engine: one main claude child behind an actor thread. The UI sends commands and reads UiEvent values.
// Commands and child stdout lines share one mpsc channel, so a single loop owns all state.
// Owned types only: no lifetimes. Subagent tool events are not surfaced in E1 (pipeline is E2).

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::budget::{self, Budget};
use crate::event::{Block, Content, Delta, Event, StreamInner, Usage, parse_line};
use crate::metrics::Metrics;
use crate::mode::{self, PermMode, Verdict};
use crate::osenv::resolve_bin;
use crate::proc::{self, ClaudeProc, ProcMsg, SpawnOpts};

// Text tool results of background agents say this; the agent keeps running after the tool result.
const ASYNC_AGENT_MARK: &str = "Async agent launched";
// The engine runs one child, so the cumulative cost total is kept under one baseline key (Go keys by Session).
const CHILD_PROC: u64 = 0;

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
#[non_exhaustive]
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
    // Text of a tool_result (Go tui.go resultText: a string, or the text blocks joined by newlines).
    ToolResult {
        id: String,
        text: String,
    },
    // The Agent launched in the background; it keeps running after this result, so no AgentEnd follows.
    AgentBackground {
        id: String,
    },
    // The running turn ended after an interrupt (before its TurnEnd).
    Interrupted,
    // Raw rate_limit_info object from a rate_limit_event (Go livechild.go setRate: rateLimitType, unifiedWindows, ...).
    RateLimit {
        info: Value,
    },
    // Permission prompt for a can_use_tool request the mode did not decide (Go ui.ask).
    // Answer with EngineHandle::answer_permission(id, allow).
    PermissionRequest {
        id: String,
        tool: String,
        summary: String,
    },
    // Harness permission mode changed (Go modeLine). The start state is not announced.
    ModeChanged {
        mode: String,
    },
    // Per-turn measurement, emitted just before TurnEnd (Go record_turn.go measureLine keys).
    // cost_turn_usd = this turn's share of the session cost; cost_usd = cumulative session cost;
    // ctx = context tokens from the last assistant usage; delegations = Agent/Task calls in the turn.
    TurnStats {
        duration_ms: u64,
        cost_turn_usd: f64,
        cost_usd: f64,
        ctx: u64,
        delegations: u32,
    },
    // Budget ledger after a turn's charge (Go budget.Spent vs cap). Only sent when a cap is set.
    BudgetStatus {
        spent_usd: f64,
        cap_usd: f64,
    },
    // The cap was reached (Go budgetStop): the running turn is interrupted, or the next prompt is refused.
    BudgetStop {
        spent_usd: f64,
        cap_usd: f64,
    },
    // Result of /usage (Go usageRaw): a one-shot claude run, not the session. Answered while busy, not charged.
    Usage {
        text: String,
    },
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
    // Quiet time with no claude output before a "still running" notice. Paused while any tool_use lacks its
    // result (a subagent may be silent for minutes). The notice never interrupts; late results are normal events.
    pub inactivity_timeout: Duration,
    // After an interrupt, how long to wait for the turn's result before force-ending the turn (Go: 20 s).
    pub interrupt_grace: Duration,
    // Go --auto-allow (testing): every ask is allowed without a UI prompt. Mode verdicts still apply first.
    pub auto_allow: bool,
    // Go --task-budget-usd: USD cap per prompt (the ledger resets on each prompt). Zero = off.
    pub task_budget_usd: f64,
    // Go --metrics-dir: per-turn JSONL log directory. None = off (the CLI passes <record>/metrics).
    pub metrics_dir: Option<PathBuf>,
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
            inactivity_timeout: Duration::from_secs(300),
            interrupt_grace: Duration::from_secs(20),
            auto_allow: false,
            task_budget_usd: 0.0,
            metrics_dir: None,
        }
    }
}

enum Cmd {
    Submit(String),
    Interrupt(InterruptSource),
    // Mode name as typed ("next" cycles); parsed by the actor so unknown names are reported as UiEvent::Error.
    SetMode(String),
    Permission { id: String, allow: bool },
    Usage,
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

    // Go /mode: "next" cycles, otherwise default | acceptEdits | plan | auto (case-insensitive).
    pub fn set_mode(&self, name: &str) {
        let _ = self.tx.send(Input::Cmd(Cmd::SetMode(name.to_string())));
    }

    // Answers a UiEvent::PermissionRequest. allow=false denies with the Go empty-message reply.
    pub fn answer_permission(&self, id: &str, allow: bool) {
        let _ = self.tx.send(Input::Cmd(Cmd::Permission {
            id: id.to_string(),
            allow,
        }));
    }

    // Go /usage: answers UiEvent::Usage or Error from a one-shot run. Works while a turn is busy.
    pub fn usage(&self) {
        let _ = self.tx.send(Input::Cmd(Cmd::Usage));
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
    // tool_use ids still waiting for their tool_result; the inactivity timer is paused while non-empty.
    open_tools: HashSet<String>,
    // Last claude line or turn start; the inactivity timer measures from here.
    last_activity: Instant,
    // Set by an interrupt on a busy turn; the turn is force-ended when the grace period passes without a result.
    interrupted_at: Option<Instant>,
    // Results still owed by abandoned turns (Go core.go Session.skip). Each one is discarded so it cannot end a later turn.
    stray_results: u32,
    // Session cost when the running turn started, and Agent/Task calls made in it (for TurnStats).
    turn_cost_base: f64,
    delegations: u32,
    // Harness permission mode (Go permMode); decides can_use_tool before any prompt.
    mode: PermMode,
    // can_use_tool requests waiting for a UI answer, keyed by request_id, with the tool input to echo back.
    pending_perm: HashMap<String, Value>,
    // Per-prompt USD ledger (Go budget); None when --task-budget-usd is off.
    budget: Option<Budget>,
    // Last main assistant API response, for the warm-cache test of the next-call estimate.
    last_api: Option<Instant>,
    // Set when a main usage event's in-flight estimate reached the cap; the turn is interrupted once.
    budget_hit: bool,
    // Per-turn metrics JSONL (Go metrics); disabled when cfg.metrics_dir is None.
    metrics: Metrics,
    // Session id from the child's system event (Go sess.SessionID); "" until the child reports one.
    sid: String,
}

fn actor(cfg: Config, self_tx: Sender<Input>, rx: Receiver<Input>, ui: Sender<UiEvent>) {
    let opts = SpawnOpts {
        claude_bin: resolve_bin(&cfg.claude_bin),
        model: cfg.model.clone(),
        resume: cfg.resume.clone(),
        cwd: cfg.cwd.clone(),
        cache_ttl: cfg.cache_ttl.clone(),
        compact_window: cfg.compact_window,
        max_budget_usd: cfg.task_budget_usd,
    };
    let model = cfg.model.clone();
    let budget = (cfg.task_budget_usd > 0.0).then(|| Budget::new(cfg.task_budget_usd));
    let metrics = Metrics::new(
        cfg.metrics_dir.clone(),
        json!({"compact_window": cfg.compact_window, "context_window": cfg.context_window, "cache_ttl": cfg.cache_ttl}),
    );
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
        open_tools: HashSet::new(),
        last_activity: Instant::now(),
        interrupted_at: None,
        stray_results: 0,
        turn_cost_base: 0.0,
        delegations: 0,
        mode: PermMode::Default,
        pending_perm: HashMap::new(),
        budget,
        last_api: None,
        budget_hit: false,
        metrics,
        sid: String::new(),
    };
    match proc::spawn(&opts, self_tx, Input::Proc) {
        Ok(p) => st.proc = Some(p),
        Err(e) => st.emit(UiEvent::Error(format!("claude spawn failed: {e}"))),
    }
    loop {
        let input = match st.wait_budget() {
            Some(d) => match rx.recv_timeout(d) {
                Ok(i) => i,
                Err(RecvTimeoutError::Timeout) => {
                    st.on_quiet();
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(i) => i,
                Err(_) => break,
            },
        };
        match input {
            Input::Cmd(Cmd::Shutdown) => break,
            Input::Cmd(Cmd::Submit(text)) => st.submit(text),
            Input::Cmd(Cmd::Interrupt(src)) => st.interrupt(src),
            Input::Cmd(Cmd::SetMode(name)) => st.set_mode(&name),
            Input::Cmd(Cmd::Permission { id, allow }) => st.answer_permission(&id, allow),
            Input::Cmd(Cmd::Usage) => st.usage(),
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

    // Runs the one-shot on its own thread so the actor keeps streaming a busy turn.
    fn usage(&self) {
        let bin = resolve_bin(&self.cfg.claude_bin);
        let ui = self.ui.clone();
        thread::spawn(move || {
            let ev = match proc::one_shot_usage(&bin) {
                Ok(text) => UiEvent::Usage { text },
                Err(e) => UiEvent::Error(e),
            };
            let _ = ui.send(ev);
        });
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
        // Go handle/runTurn: each prompt starts a fresh ledger, and is refused when its next call would exceed the cap.
        if let Some(b) = self.budget.as_mut() {
            b.reset();
        }
        self.budget_hit = false;
        let est = self.estimate_next();
        if self.budget.as_ref().is_some_and(|b| !b.allow(est)) {
            self.emit_budget_stop();
            return false;
        }
        let Some(p) = self.proc.as_mut() else {
            return false;
        };
        match p.write_user(text) {
            Ok(()) => {
                self.turn_started = Some(Instant::now());
                self.last_activity = Instant::now();
                self.turn_cost_base = self.cost_usd;
                self.delegations = 0;
                true
            }
            Err(e) => {
                self.emit(UiEvent::Error(format!("write to claude failed: {e}")));
                false
            }
        }
    }

    // Go estNext: planned cost of the next call. Warm while the last API response is inside the cache TTL
    // (Go idleDeadline, capped at 290 s for the 5m cache and 1 h for the 1h cache; no idle-rotate setting here).
    fn estimate_next(&self) -> f64 {
        let ttl = self.cfg.cache_ttl.as_str();
        let window = if ttl == "1h" {
            Duration::from_secs(3600)
        } else {
            Duration::from_secs(290)
        };
        let warm = self.last_api.is_some_and(|t| t.elapsed() < window);
        budget::estimate(&self.model, self.ctx_tokens, warm, ttl)
    }

    fn emit_budget_stop(&self) {
        if let Some(b) = &self.budget {
            self.emit(UiEvent::BudgetStop {
                spent_usd: b.spent(),
                cap_usd: b.cap(),
            });
        }
    }

    // Go runTurn assistant case: a main usage event whose in-flight estimate reaches the cap interrupts the turn once.
    fn budget_mid_turn(&mut self, u: &Usage) {
        if self.budget_hit {
            return;
        }
        let est = budget::mid_turn_estimate(&self.model, u);
        let Some(b) = self.budget.as_mut() else {
            return;
        };
        if !b.over(est) {
            return;
        }
        self.budget_hit = true;
        if let Some(p) = self.proc.as_mut()
            && let Err(e) = p.write_interrupt()
        {
            self.emit(UiEvent::Error(format!("interrupt write failed: {e}")));
        }
        self.interrupted_at.get_or_insert_with(Instant::now);
    }

    fn interrupt(&mut self, src: InterruptSource) {
        let busy = self.busy;
        let mut sent = false;
        if busy && let Some(p) = self.proc.as_mut() {
            match p.write_interrupt() {
                Ok(_) => sent = true,
                Err(e) => self.emit(UiEvent::Error(format!("interrupt write failed: {e}"))),
            }
            self.interrupted_at.get_or_insert_with(Instant::now);
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
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
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
        self.last_activity = Instant::now();
        if let Ok(ev) = parse_line(line) {
            self.on_event(ev);
        }
    }

    fn on_event(&mut self, ev: Event) {
        match ev {
            Event::System(s) => {
                if let Some(id) = s.session_id {
                    self.sid = id;
                }
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
                    self.last_api = Some(Instant::now());
                    self.budget_mid_turn(u);
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
                if self.stray_results > 0 {
                    self.stray_results -= 1;
                    return;
                }
                if !self.busy {
                    return;
                }
                if let Some(c) = r.total_cost_usd {
                    self.cost_usd = c;
                    if let Some(b) = self.budget.as_mut() {
                        b.charge(CHILD_PROC, c);
                    }
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
                if let Some(b) = &self.budget {
                    self.emit(UiEvent::BudgetStatus {
                        spent_usd: b.spent(),
                        cap_usd: b.cap(),
                    });
                }
                self.end_turn(duration_ms);
            }
            Event::ControlRequest(c) => {
                if c.request.subtype != "can_use_tool" {
                    return;
                }
                let tool = c.request.tool_name.unwrap_or_default();
                let input = c.request.input.unwrap_or(Value::Null);
                match mode::decide(self.mode, &tool) {
                    Verdict::Allow => self.respond_permission(&c.request_id, true, input, ""),
                    Verdict::Deny(msg) => {
                        self.respond_permission(&c.request_id, false, Value::Null, msg)
                    }
                    // Go askPerm: with a UI the ask goes to the UI whether or not a turn runs. The engine
                    // always has a UI channel, so Go's idle "no interactive prompt" deny is not reachable.
                    Verdict::Ask if self.cfg.auto_allow => {
                        self.respond_permission(&c.request_id, true, input, "")
                    }
                    Verdict::Ask => {
                        let summary = tool_summary(&input);
                        self.pending_perm.insert(c.request_id.clone(), input);
                        self.emit(UiEvent::PermissionRequest {
                            id: c.request_id,
                            tool,
                            summary,
                        });
                    }
                }
            }
            Event::RateLimitEvent(ev) => {
                if let Some(info) = ev.rate_limit_info {
                    self.emit(UiEvent::RateLimit { info });
                }
            }
            Event::Unknown => {}
        }
    }

    // Go /mode. Empty name reports the current mode (Go modeLine with no args). Unknown names are reported
    // with the Go message and the mode is unchanged.
    fn set_mode(&mut self, name: &str) {
        let next = if name.trim().is_empty() {
            Some(self.mode)
        } else if name.trim() == "next" {
            Some(self.mode.next())
        } else {
            PermMode::parse(name)
        };
        match next {
            Some(m) => {
                self.mode = m;
                self.emit(UiEvent::ModeChanged {
                    mode: m.as_str().to_string(),
                });
            }
            None => self.emit(UiEvent::Error(format!(
                "unknown mode {name} (default | acceptEdits | plan | auto)"
            ))),
        }
    }

    // Go RespondPermission for a pending UI prompt. An id that is not pending is reported, not sent.
    fn answer_permission(&mut self, id: &str, allow: bool) {
        match self.pending_perm.remove(id) {
            Some(input) => self.respond_permission(id, allow, input, ""),
            None => self.emit(UiEvent::Error(format!(
                "no pending permission request {id}"
            ))),
        }
    }

    fn respond_permission(&mut self, id: &str, allow: bool, input: Value, msg: &str) {
        let res = match self.proc.as_mut() {
            Some(p) => p.write_permission(id, allow, input, msg),
            None => Ok(()),
        };
        if let Err(e) = res {
            self.emit(UiEvent::Error(format!("permission reply failed: {e}")));
        }
    }

    fn on_tool_use(&mut self, id: String, name: String, input: Value) {
        if !self.tools.insert(id.clone()) {
            return;
        }
        self.open_tools.insert(id.clone());
        let summary = tool_summary(&input);
        self.emit(UiEvent::ToolStart {
            id: id.clone(),
            name: name.clone(),
            summary,
        });
        if name == "Agent" || name == "Task" {
            self.delegations += 1;
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
        self.open_tools.remove(&id);
        self.emit(UiEvent::ToolEnd {
            id: id.clone(),
            ok: !is_error,
        });
        let text = result_text(content.as_ref());
        self.emit(UiEvent::ToolResult {
            id: id.clone(),
            text: text.clone(),
        });
        // A background agent keeps running after its launch result; it ends later, which E1 does not track.
        if text.contains(ASYNC_AGENT_MARK) {
            self.emit(UiEvent::AgentBackground { id });
            return;
        }
        if let Some(t0) = self.agents.remove(&id) {
            self.emit(UiEvent::AgentEnd {
                id,
                elapsed_ms: t0.elapsed().as_millis() as u64,
            });
        }
    }

    // How long the actor may block before the inactivity timer must be checked. None means block until input.
    // Idle, or any tool_use without its result, pauses the timer.
    fn wait_budget(&self) -> Option<Duration> {
        if !self.busy {
            return None;
        }
        if let Some(t0) = self.interrupted_at {
            let left = self.cfg.interrupt_grace.saturating_sub(t0.elapsed());
            return Some(left.max(Duration::from_millis(1)));
        }
        if !self.open_tools.is_empty() {
            return None;
        }
        let left = self
            .cfg
            .inactivity_timeout
            .saturating_sub(self.last_activity.elapsed());
        Some(left.max(Duration::from_millis(1)))
    }

    // Inactivity notice. Informational only: the turn is never interrupted, and the clock restarts so the
    // notice repeats once per quiet period rather than on every wakeup.
    fn on_quiet(&mut self) {
        if !self.busy {
            return;
        }
        if let Some(t0) = self.interrupted_at {
            if t0.elapsed() >= self.cfg.interrupt_grace {
                self.abandon_turn();
            }
            return;
        }
        if !self.open_tools.is_empty() {
            return;
        }
        let quiet = self.last_activity.elapsed();
        if quiet < self.cfg.inactivity_timeout {
            return;
        }
        self.last_activity = Instant::now();
        self.emit(UiEvent::Error(format!(
            "still running: no output for {:.1}s (not interrupted)",
            quiet.as_secs_f64()
        )));
    }

    // Interrupted turn gave no result within the grace period (Go: finishForeground). Agents still open are
    // ended here, then the turn is ended as if it had finished.
    fn abandon_turn(&mut self) {
        let ids: Vec<String> = self.agents.keys().cloned().collect();
        for id in ids {
            if let Some(t0) = self.agents.remove(&id) {
                self.emit(UiEvent::AgentEnd {
                    id,
                    elapsed_ms: t0.elapsed().as_millis() as u64,
                });
            }
        }
        let duration_ms = elapsed_ms(self.turn_started);
        // The abandoned turn's prompts can no longer be answered by the child; drop them.
        self.pending_perm.clear();
        self.stray_results += 1;
        self.end_turn(duration_ms);
    }

    // Go repl.go turn record (stage "turn"). Keys the Rust engine does not produce yet (draining, rotations,
    // result_* usage) are left out until their groups land. Write errors are ignored, as in Go.
    fn log_turn(&self, duration_ms: u64) {
        let end_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let cost_turn = (self.cost_usd - self.turn_cost_base).max(0.0);
        let _ = self.metrics.log(json!({
            "stage": "turn",
            "start_ms": end_ms.saturating_sub(duration_ms),
            "end_ms": end_ms,
            "model": self.model,
            "ctx": self.ctx_tokens,
            "session": self.sid,
            "cost_usd": self.cost_usd,
            "cost_turn_usd": cost_turn,
            "duration_ms": duration_ms,
            "delegations": self.delegations,
        }));
    }

    // Finishes the running turn and starts the next queued one, if any.
    fn end_turn(&mut self, duration_ms: u64) {
        if self.interrupted_at.is_some() {
            self.emit(UiEvent::Interrupted);
        }
        if self.budget_hit {
            self.emit_budget_stop();
        }
        self.emit(UiEvent::TurnStats {
            duration_ms,
            cost_turn_usd: (self.cost_usd - self.turn_cost_base).max(0.0),
            cost_usd: self.cost_usd,
            ctx: self.ctx_tokens,
            delegations: self.delegations,
        });
        self.log_turn(duration_ms);
        self.turn_started = None;
        self.interrupted_at = None;
        self.open_tools.clear();
        self.emit(UiEvent::TurnEnd { duration_ms });
        // Go doneMsg / mergePrompts: everything queued meanwhile goes out as ONE turn, joined by a blank line.
        if self.queue.is_empty() {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
            return;
        }
        let next = self.queue.drain(..).collect::<Vec<_>>().join("\n\n");
        self.emit(UiEvent::Queued(0));
        if !self.send_turn(&next) {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
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
// Go tui.go resultText: a plain string, or the "text" of each block joined by newlines.
fn result_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .map(|b| b.get("text").and_then(Value::as_str).unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

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
