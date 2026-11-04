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
use crate::metrics::{Metrics, yymmdd_today};
use crate::mode::{self, PermMode, Verdict};
use crate::osenv::resolve_bin;
use crate::proc::{self, ClaudeProc, ProcMsg, Shape, SpawnOpts};
use crate::resident::{
    CODER_REPLY_FORMAT, CODER_TIMEOUT, PROMPTER_RULE, PROMPTER_TIMEOUT, Prompter, PrompterOutcome,
    StagePhase, TUI_TIMEOUT, agent_body, summarize_reply, tui_prompt,
};
use crate::rotate::{
    HAIKU_THRESHOLD, HANDOFF_PROMPT, PREAMBLE, extract_reco, should_rotate, threshold_for,
    validate_handoff, with_reco,
};

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
    // A background agent resumed by SendMessage runs again under its original tool id until its AgentEnd.
    AgentResumed {
        id: String,
        description: String,
        started_at: u64,
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
    // Progress of a resident stage (prompter summary now, later tui and coder slots). slot names the resident role.
    Stage {
        slot: String,
        phase: crate::resident::StagePhase,
    },
    // The tui resident's Korean answer for the turn just ended (Go viaTUI). It replaces the main reply shown.
    TuiAnswer {
        text: String,
    },
    // The coder resident's summary for a task (Go runResidentCoder). role is "coder" or "coder2".."coder9".
    CoderReply {
        role: String,
        text: String,
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

// Recent turns kept for the handoff snapshot (Go --tail-turns) and their size limits (Go --snapshot-chars).
const TAIL_TURNS: usize = 4;
// Go coderSlot tools and the extra coder cap (settingsMaxExtraCoders).
const CODER_TOOLS: &str = "Read,Edit,Write,Bash,Grep,Glob";
const MAX_EXTRA_CODERS: usize = 8;
const TURN_CHARS: usize = 1_500;
const SNAPSHOT_CHARS: usize = 24_000;

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
    // Go --transcript: JSONL trace of unparsable child lines ({"kind":"unparsed"}). None = off (the CLI passes StateDir/transcript.jsonl).
    pub transcript: Option<PathBuf>,
    // Go context rotation: a turn that ends at or above this many context tokens starts a handoff and a fresh
    // child (Opus uses its own threshold, see rotate::threshold_for). Zero = rotation off.
    pub rotate_threshold: u64,
    // Go --rotate-defer-timeout: how long a due rotation waits for background agents before it runs anyway.
    // Zero = rotate at once.
    pub rotate_defer_timeout: Duration,
    // Resident slots are running (Go len(resident) > 0): the engine emits UiEvent::Stage rows for the main turn
    // and each agent call. Off = no stage rows.
    pub residents: bool,
    // Go extra_coders (.chaude/settings.json): coders coder2..coder(1+n) beyond the always-on coder, 0-8.
    // Extra coders start on demand and expire after 270 s idle (N14 idle sweep).
    pub extra_coders: usize,
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
            transcript: None,
            rotate_threshold: HAIKU_THRESHOLD,
            rotate_defer_timeout: Duration::from_secs(600),
            residents: true,
            extra_coders: 4,
        }
    }
}

enum Cmd {
    Submit(String),
    // Go runResidentCoder: one task for an idle coder. The reply comes back as UiEvent::CoderReply.
    Coder(String),
    Interrupt(InterruptSource),
    // Mode name as typed ("next" cycles); parsed by the actor so unknown names are reported as UiEvent::Error.
    SetMode(String),
    Permission { id: String, allow: bool },
    Usage,
    Shutdown,
}

enum Input {
    Cmd(Cmd),
    // Child message tagged with the child's generation; messages from a replaced child are dropped.
    Proc(u64, ProcMsg),
    // Prompter child line, tagged with the prompter's generation like Proc.
    Prompter(u64, ProcMsg),
    // Tui child line, tagged with the tui's generation like Proc.
    Tui(u64, ProcMsg),
    // Coder child line: slot index into State::coders, tagged with that slot's generation.
    Coder(usize, u64, ProcMsg),
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

    // Go runResidentCoder: sends a task to an idle coder (coder, then coder2...). Answers CoderReply or a Failed stage.
    pub fn coder(&self, task: &str) {
        let _ = self.tx.send(Input::Cmd(Cmd::Coder(task.to_string())));
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
    // A rotation that was due while background agents ran: when it was first deferred (Go rotate_defer).
    rotate_deferred: Option<Instant>,
    // Delegated agents by tool_use id. A background agent (launch result said "async") keeps running past the
    // turn, so an abandoned turn does not end it, and it is listed in the handoff snapshot.
    agent_info: HashMap<String, AgentInfo>,
    // agentId from an async launch result -> tool_use id, so a SendMessage resume finds the agent.
    agent_ids: HashMap<String, String>,
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
    self_tx: Sender<Input>,
    opts: SpawnOpts,
    // Bumped on every fresh child so the old child's late lines and EOF are ignored.
    epoch: u64,
    // Files written by Write/Edit/NotebookEdit in this session (Go r.files), for the handoff snapshot.
    files: Vec<String>,
    // Last recommendation line of a normal turn (Go lastReco), carried verbatim into the snapshot.
    last_reco: String,
    // Last normal turns as (prompt, reply), newest last (Go tr.turns).
    recent: VecDeque<(String, String)>,
    // Prompt and assistant text of the running turn.
    turn_prompt: String,
    turn_reply: String,
    // Snapshot for the next prompt sent to a fresh child; empty when none is pending.
    pending_snapshot: String,
    // Set while a hidden handoff turn runs; its text is not shown.
    handoff: Option<Handoff>,
    // Prompter resident (Go prompter slot): summarizes each prompt before the main child gets it.
    prompter: Prompter,
    prompter_proc: Option<ClaudeProc>,
    prompter_epoch: u64,
    // When the running prompter wait gives up (Go prompterTimeout).
    prompter_deadline: Option<Instant>,
    // Tui resident (Go tui slot): translates the main reply to the Korean answer. Kept alive across turns.
    tui_proc: Option<ClaudeProc>,
    tui_epoch: u64,
    // When the running tui wait gives up (Go tuiTimeout).
    tui_deadline: Option<Instant>,
    // Coder residents (Go coder, coder2..): slot 0 is "coder", slot i is "coder{i+1}". Grown on demand.
    coders: Vec<Coder>,
    coder_epoch: u64,
}

// One coder resident. A running deadline means a task is in flight (Go busyRec).
#[derive(Default)]
struct Coder {
    proc: Option<ClaudeProc>,
    // Generation of the child in this slot; a replaced child's lines are dropped by epoch.
    epoch: u64,
    deadline: Option<Instant>,
}

// A handoff turn in progress: the rotation reason and how many handoff prompts were sent (Go makeSnapshotN).
struct Handoff {
    reason: String,
    attempts: u32,
}

// What the handoff snapshot needs about a delegated agent (Go agentRec: Task, ResultPath, Background).
struct AgentInfo {
    name: String,
    result_path: String,
    background: bool,
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
        shape: Shape::default(),
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
        rotate_deferred: None,
        agent_info: HashMap::new(),
        agent_ids: HashMap::new(),
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
        self_tx,
        opts,
        epoch: 0,
        files: Vec::new(),
        last_reco: String::new(),
        recent: VecDeque::new(),
        turn_prompt: String::new(),
        turn_reply: String::new(),
        pending_snapshot: String::new(),
        handoff: None,
        prompter: Prompter::new(),
        prompter_proc: None,
        prompter_epoch: 0,
        prompter_deadline: None,
        tui_proc: None,
        tui_epoch: 0,
        tui_deadline: None,
        coders: Vec::new(),
        coder_epoch: 0,
    };
    st.start_child();
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
            Input::Proc(epoch, msg) if epoch == st.epoch => match msg {
                ProcMsg::Line(line) => st.on_line(&line),
                ProcMsg::Eof => st.on_eof(),
            },
            Input::Proc(_, _) => {}
            Input::Prompter(epoch, msg) if epoch == st.prompter_epoch => match msg {
                ProcMsg::Line(line) => st.on_prompter_line(&line),
                ProcMsg::Eof => st.on_prompter_eof(),
            },
            Input::Prompter(_, _) => {}
            Input::Tui(epoch, msg) if epoch == st.tui_epoch => match msg {
                ProcMsg::Line(line) => st.on_tui_line(&line),
                ProcMsg::Eof => st.on_tui_eof(),
            },
            Input::Tui(_, _) => {}
            Input::Cmd(Cmd::Coder(task)) => st.start_coder(task),
            Input::Coder(i, epoch, msg) if st.coder_matches(i, epoch) => match msg {
                ProcMsg::Line(line) => st.on_coder_line(i, &line),
                ProcMsg::Eof => st.on_coder_eof(i),
            },
            Input::Coder(_, _, _) => {}
        }
    }
    if let Some(mut p) = st.proc.take() {
        p.kill();
    }
    if let Some(mut p) = st.prompter_proc.take() {
        p.kill();
    }
    if let Some(mut p) = st.tui_proc.take() {
        p.kill();
    }
    for c in &mut st.coders {
        if let Some(mut p) = c.proc.take() {
            p.kill();
        }
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
        if self.busy || self.prompter.waiting() {
            self.queue.push_back(text);
            self.emit(UiEvent::Queued(self.queue.len()));
            return;
        }
        self.begin_turn(text);
    }

    // Go viaPrompter: with residents on, the prompt goes through the prompter first.
    fn begin_turn(&mut self, text: String) {
        if self.cfg.residents {
            self.start_prompter(text);
        } else {
            self.dispatch(&text);
        }
    }

    // Sends text to the main child as a turn; the engine is busy only when it went out.
    fn dispatch(&mut self, text: &str) {
        if self.send_turn(text) {
            self.busy = true;
            self.emit(UiEvent::Busy(true));
        }
    }

    // Sends text to the prompter. A spawn, start or write failure falls back to the raw prompt (Go).
    fn start_prompter(&mut self, text: String) {
        if self.prompter_proc.is_none()
            && let Err(e) = self.spawn_prompter()
        {
            self.stage(
                "prompter",
                StagePhase::Failed(format!("prompter spawn failed: {e}")),
            );
            self.dispatch(&text);
            return;
        }
        let phase = match self.prompter.start(&text) {
            Ok(phase) => phase,
            Err(e) => {
                self.stage("prompter", StagePhase::Failed(e));
                self.dispatch(&text);
                return;
            }
        };
        self.stage("prompter", phase);
        self.prompter_deadline = Some(Instant::now() + PROMPTER_TIMEOUT);
        let wrote = match self.prompter_proc.as_mut() {
            Some(p) => p.write_user(&text).map_err(|e| e.to_string()),
            None => Err("prompter not running".to_string()),
        };
        if let Err(e) = wrote {
            self.prompter_proc = None;
            if let Some(o) = self.prompter.fail(&format!("prompter write failed: {e}")) {
                self.finish_prompter(o);
            }
        }
    }

    // Go residentSlots prompter: haiku with the minimal shape and the prompter rule. Kept alive across prompts.
    fn spawn_prompter(&mut self) -> Result<(), String> {
        let opts = SpawnOpts {
            model: "haiku".to_string(),
            resume: None,
            max_budget_usd: 0.0,
            shape: Shape {
                minimal: true,
                rule: PROMPTER_RULE.to_string(),
                ..Shape::default()
            },
            ..self.opts.clone()
        };
        self.prompter_epoch += 1;
        let epoch = self.prompter_epoch;
        let p = proc::spawn(&opts, self.self_tx.clone(), move |m| {
            Input::Prompter(epoch, m)
        })
        .map_err(|e| e.to_string())?;
        self.prompter_proc = Some(p);
        Ok(())
    }

    // The prompter's result line is its reply.
    fn on_prompter_line(&mut self, line: &str) {
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        if let Some(o) = self.prompter.reply(&r.result.unwrap_or_default()) {
            self.finish_prompter(o);
        }
    }

    fn on_prompter_eof(&mut self) {
        self.prompter_proc = None;
        if let Some(o) = self.prompter.fail("prompter exited") {
            self.finish_prompter(o);
        }
    }

    // The prompter gave up waiting. Its child is replaced, so late lines are dropped by epoch.
    fn prompter_timeout(&mut self) {
        if let Some(mut p) = self.prompter_proc.take() {
            p.kill();
        }
        self.prompter_epoch += 1;
        if let Some(o) = self.prompter.timeout() {
            self.finish_prompter(o);
        }
    }

    // Ends the prompter wait: its stage row, then the outcome goes to the main child as a turn.
    fn finish_prompter(&mut self, o: PrompterOutcome) {
        self.prompter_deadline = None;
        if let Some(phase) = o.stage {
            self.stage("prompter", phase);
        }
        self.dispatch(&o.text);
    }

    // Go viaTUI: the main reply goes to the tui child. Failures keep the main reply (the stage row says why).
    fn start_tui(&mut self, reply: String) {
        if self.tui_proc.is_none()
            && let Err(e) = self.spawn_tui()
        {
            self.stage("tui", StagePhase::Failed(format!("tui spawn failed: {e}")));
            self.after_turn();
            return;
        }
        self.stage("tui", StagePhase::Working);
        self.tui_deadline = Some(Instant::now() + TUI_TIMEOUT);
        let wrote = match self.tui_proc.as_mut() {
            Some(p) => p.write_user(&tui_prompt(&reply)).map_err(|e| e.to_string()),
            None => Err("tui not running".to_string()),
        };
        if let Err(e) = wrote {
            self.tui_proc = None;
            self.finish_tui(Err(format!("tui write failed: {e}")));
        }
    }

    // Go residentSlots tui: haiku with the minimal shape and the haiku-tui agent body. Kept alive across turns.
    fn spawn_tui(&mut self) -> Result<(), String> {
        let cwd = self.opts.cwd.clone().unwrap_or_else(|| ".".into());
        let rule = agent_body(&cwd, "haiku-tui")?;
        let opts = SpawnOpts {
            model: "haiku".to_string(),
            resume: None,
            max_budget_usd: 0.0,
            shape: Shape {
                minimal: true,
                rule,
                ..Shape::default()
            },
            ..self.opts.clone()
        };
        self.tui_epoch += 1;
        let epoch = self.tui_epoch;
        let p = proc::spawn(&opts, self.self_tx.clone(), move |m| Input::Tui(epoch, m))
            .map_err(|e| e.to_string())?;
        self.tui_proc = Some(p);
        Ok(())
    }

    // Go runResidentCoder: the first idle coder takes the task (spawned if not running). Busy coders are skipped.
    fn start_coder(&mut self, task: String) {
        let task = task.trim().to_string();
        if task.is_empty() {
            self.emit(UiEvent::Error("task is required".to_string()));
            return;
        }
        let extra = self.cfg.extra_coders.min(MAX_EXTRA_CODERS);
        let idle = (0..=extra).find(|&i| self.coders.get(i).is_none_or(|c| c.deadline.is_none()));
        let Some(i) = idle else {
            self.stage(
                "coder",
                StagePhase::Failed("all resident coders are busy".into()),
            );
            return;
        };
        let role = coder_role(i);
        if self.coders.len() <= i {
            self.coders.resize_with(i + 1, Coder::default);
        }
        if self.coders[i].proc.is_none()
            && let Err(e) = self.spawn_coder(i)
        {
            self.stage(&role, StagePhase::Failed(format!("coder spawn failed: {e}")));
            return;
        }
        self.stage(&role, StagePhase::Working);
        self.coders[i].deadline = Some(Instant::now() + CODER_TIMEOUT);
        let text = format!("{task}{CODER_REPLY_FORMAT}");
        let wrote = match self.coders[i].proc.as_mut() {
            Some(p) => p.write_user(&text).map_err(|e| e.to_string()),
            None => Err("coder not running".to_string()),
        };
        if let Err(e) = wrote {
            self.drop_coder(i);
            self.finish_coder(i, Err(format!("coder write failed: {e}")));
        }
    }

    // Go residentSlots coder: haiku with the haiku-coder agent body, the coder tools, strict MCP and no slash commands.
    fn spawn_coder(&mut self, i: usize) -> Result<(), String> {
        let cwd = self.opts.cwd.clone().unwrap_or_else(|| ".".into());
        let rule = agent_body(&cwd, "haiku-coder")?;
        let opts = SpawnOpts {
            model: "haiku".to_string(),
            resume: None,
            max_budget_usd: 0.0,
            shape: Shape {
                minimal: false,
                tools: CODER_TOOLS.to_string(),
                strict_mcp: true,
                no_slash: true,
                rule,
            },
            ..self.opts.clone()
        };
        self.coder_epoch += 1;
        let epoch = self.coder_epoch;
        self.coders[i].epoch = epoch;
        let p = proc::spawn(&opts, self.self_tx.clone(), move |m| {
            Input::Coder(i, epoch, m)
        })
        .map_err(|e| e.to_string())?;
        self.coders[i].proc = Some(p);
        Ok(())
    }

    // A message is current only while its slot still has the child that sent it.
    fn coder_matches(&self, i: usize, epoch: u64) -> bool {
        self.coders.get(i).is_some_and(|c| c.epoch == epoch)
    }

    // Kills the slot's child and bumps its generation, so its late lines are dropped.
    fn drop_coder(&mut self, i: usize) {
        if let Some(mut p) = self.coders[i].proc.take() {
            p.kill();
        }
        self.coder_epoch += 1;
        self.coders[i].epoch = self.coder_epoch;
    }

    // The coder's result line is its reply. An empty reply is a failure, as for the tui.
    fn on_coder_line(&mut self, i: usize, line: &str) {
        if self.coders[i].deadline.is_none() {
            return;
        }
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        let text = summarize_reply(&r.result.unwrap_or_default());
        if text.is_empty() {
            self.finish_coder(i, Err("coder empty reply".to_string()));
        } else {
            self.finish_coder(i, Ok(text));
        }
    }

    fn on_coder_eof(&mut self, i: usize) {
        self.coders[i].proc = None;
        if self.coders[i].deadline.is_some() {
            self.finish_coder(i, Err("coder exited".to_string()));
        }
    }

    // The coder gave up waiting (Go residentCoderTimeout). The child is replaced.
    fn coder_timeout(&mut self, i: usize) {
        self.drop_coder(i);
        self.finish_coder(i, Err("coder timed out".to_string()));
    }

    // Ends a coder task: its stage row, then the reply (or the failure reason) goes to the UI.
    fn finish_coder(&mut self, i: usize, out: Result<String, String>) {
        self.coders[i].deadline = None;
        let role = coder_role(i);
        match out {
            Ok(text) => {
                self.stage(&role, StagePhase::Done);
                self.emit(UiEvent::CoderReply { role, text });
            }
            Err(reason) => self.stage(&role, StagePhase::Failed(reason)),
        }
    }

    // The tui's result line is its answer. An empty answer is a failure (Go empty reply).
    fn on_tui_line(&mut self, line: &str) {
        if self.tui_deadline.is_none() {
            return;
        }
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        let text = r.result.unwrap_or_default().trim().to_string();
        if text.is_empty() {
            self.finish_tui(Err("tui empty reply".to_string()));
        } else {
            self.finish_tui(Ok(text));
        }
    }

    fn on_tui_eof(&mut self) {
        self.tui_proc = None;
        if self.tui_deadline.is_some() {
            self.finish_tui(Err("tui exited".to_string()));
        }
    }

    // The tui gave up waiting. Its child is replaced, so late lines are dropped by epoch.
    fn tui_timeout(&mut self) {
        if let Some(mut p) = self.tui_proc.take() {
            p.kill();
        }
        self.tui_epoch += 1;
        self.finish_tui(Err("tui timed out".to_string()));
    }

    // Ends the tui wait: the answer (or its failure row), then the turn is over.
    fn finish_tui(&mut self, out: Result<String, String>) {
        self.tui_deadline = None;
        match out {
            Ok(text) => {
                self.stage("tui", StagePhase::Done);
                self.emit(UiEvent::TuiAnswer { text });
            }
            Err(reason) => self.stage("tui", StagePhase::Failed(reason)),
        }
        self.after_turn();
    }

    // Time left in the running tui wait. None when no wait is running.
    fn tui_left(&self) -> Option<Duration> {
        self.tui_deadline
            .map(|t| t.saturating_duration_since(Instant::now()))
    }

    // Time left in the running prompter wait. None when no wait is running.
    fn prompter_left(&self) -> Option<Duration> {
        self.prompter_deadline
            .map(|t| t.saturating_duration_since(Instant::now()))
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
        // Go runTurn: the pending snapshot of a rotated session goes in front of the first prompt sent to it.
        let frame = if self.pending_snapshot.is_empty() {
            text.to_string()
        } else {
            format!(
                "{}\n\n## Current user message\n{}",
                self.pending_snapshot, text
            )
        };
        match p.write_user(&frame) {
            Ok(()) => {
                self.pending_snapshot.clear();
                self.turn_prompt = text.to_string();
                self.turn_reply.clear();
                self.turn_started = Some(Instant::now());
                self.stage("main", StagePhase::Working);
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
        // While the tui translates, the main child is idle: there is nothing to interrupt.
        if busy
            && self.tui_deadline.is_none()
            && let Some(p) = self.proc.as_mut()
        {
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
        self.handoff = None;
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
        match parse_line(line) {
            Ok(ev) => self.on_event(ev),
            Err(_) => self.log_unparsed(line),
        }
    }

    // Go repl.go _unparsed -> Transcript.add("unparsed"): trace only, the turn is not affected. Write errors are ignored.
    fn log_unparsed(&self, line: &str) {
        let Some(path) = &self.cfg.transcript else {
            return;
        };
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let row = json!({"t": t, "kind": "unparsed", "raw": line});
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(format!("{row}\n").as_bytes()));
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
                        if matches!(role, Role::Main) {
                            self.turn_reply.push_str(&text);
                        }
                        if self.handoff.is_none() {
                            self.emit(UiEvent::TextDelta { role, text });
                        }
                    }
                    StreamInner::MessageStop if self.handoff.is_none() => {
                        self.emit(UiEvent::MessageDone { role })
                    }
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
                if let Some(Content::Text(text)) = &env.message.content {
                    self.on_task_notification(text);
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
        // Go runTurn: files written by Write/Edit/NotebookEdit are listed in the handoff snapshot.
        if let Some(fp) = input.get("file_path").and_then(Value::as_str)
            && matches!(name.as_str(), "Write" | "Edit" | "NotebookEdit")
            && !fp.is_empty()
            && !self.files.iter().any(|f| f == fp)
        {
            self.files.push(fp.to_string());
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
            self.agent_info.insert(
                id.clone(),
                AgentInfo {
                    name: description.clone(),
                    result_path: format!("notes/runs/{}-agent-{}.md", yymmdd_today(), id),
                    background: false,
                },
            );
            self.stage(&agent_slot(&description), StagePhase::Working);
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
        // SendMessage to a background agent resumes it: it runs again under its original tool id.
        // The SendMessage result is JSON: {"success":true,"message":"Resuming agent <short>","resumedAgentId":"<full>"}.
        let resume = serde_json::from_str::<Value>(&text)
            .ok()
            .filter(|v| v["success"] != false)
            .and_then(|v| v["resumedAgentId"].as_str().map(str::to_string));
        let known = resume.and_then(|aid| self.agent_ids.get(&aid).cloned());
        if let Some((tool, description)) = known.and_then(|t| {
            let name = self.agent_info.get(&t)?.name.clone();
            Some((t, name))
        }) {
            self.agents.insert(tool.clone(), Instant::now());
            if let Some(info) = self.agent_info.get_mut(&tool) {
                info.background = true;
            }
            self.emit(UiEvent::AgentResumed {
                id: tool,
                description,
                started_at: now_secs(),
            });
            return;
        }
        // A background agent keeps running after its launch result; it ends later, which E1 does not track.
        if text.contains(ASYNC_AGENT_MARK) {
            if let Some(aid) = after_marker(&text, "agentId: ") {
                self.agent_ids.insert(aid, id.clone());
            }
            if let Some(info) = self.agent_info.get_mut(&id) {
                info.background = true;
            }
            self.emit(UiEvent::AgentBackground { id });
            return;
        }
        self.finish_agent(id);
    }

    // Ends a delegated agent: AgentEnd, and the deferred rotation runs if this was the last agent.
    // Go repl.go stage rows ("· main 처리중", "· agent X 호출"): sent only while residents run.
    fn stage(&mut self, slot: &str, phase: StagePhase) {
        if self.cfg.residents {
            self.emit(UiEvent::Stage {
                slot: slot.to_string(),
                phase,
            });
        }
    }

    fn finish_agent(&mut self, id: String) {
        let info = self.agent_info.remove(&id);
        if let Some(t0) = self.agents.remove(&id) {
            if let Some(i) = info {
                self.stage(&agent_slot(&i.name), StagePhase::Done);
            }
            self.emit(UiEvent::AgentEnd {
                id,
                elapsed_ms: t0.elapsed().as_millis() as u64,
            });
        }
        // The last background agent finished while the child is idle: the deferred rotation runs now.
        if self.agents.is_empty()
            && !self.busy
            && self.rotate_deferred.is_some()
            && self.rotation_wanted()
        {
            self.run_deferred_rotation();
        }
    }

    // A <task-notification> names the background agents that finished (Go drain.go onNotification).
    fn on_task_notification(&mut self, text: &str) {
        if !text.contains("<task-notification>") {
            return;
        }
        for part in text.split("<tool-use-id>").skip(1) {
            let id: String = part
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if self.agents.contains_key(&id) {
                self.finish_agent(id);
            }
        }
    }

    // How long the actor may block before the inactivity timer must be checked. None means block until input.
    // A coder wait can run during a busy main turn, so its deadline is only added to the main timers.
    fn wait_budget(&self) -> Option<Duration> {
        let own = self
            .coders
            .iter()
            .filter_map(|c| c.deadline)
            .min()
            .map(|t| {
                t.saturating_duration_since(Instant::now())
                    .max(Duration::from_millis(1))
            });
        match (self.main_wait_budget(), own) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    // Idle, or any tool_use without its result, pauses the timer.
    fn main_wait_budget(&self) -> Option<Duration> {
        // A prompter wait only starts while idle, so it is the only timer that matters until it ends.
        if let Some(left) = self.prompter_left() {
            return Some(left.max(Duration::from_millis(1)));
        }
        // A tui wait runs on a busy turn with no open tool: its own deadline is the only timer.
        if let Some(left) = self.tui_left() {
            return Some(left.max(Duration::from_millis(1)));
        }
        if !self.busy {
            // Idle with a deferred rotation: wake when its defer timeout ends.
            return self
                .defer_left()
                .map(|left| left.max(Duration::from_millis(1)));
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
        if self.prompter_left().is_some_and(|d| d.is_zero()) {
            self.prompter_timeout();
        }
        if self.tui_left().is_some_and(|d| d.is_zero()) {
            self.tui_timeout();
        }
        for i in 0..self.coders.len() {
            if self.coders[i].deadline.is_some_and(|t| t <= Instant::now()) {
                self.coder_timeout(i);
            }
        }
        // A running tui wait is not an inactivity case: the main child is idle.
        if self.tui_deadline.is_some() {
            return;
        }
        if !self.busy {
            // A deferred rotation waits out a prompter wait: it would send its handoff while the prompter runs.
            if !self.prompter.waiting() && self.defer_left().is_some_and(|d| d.is_zero()) {
                self.run_deferred_rotation();
            }
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

    // Interrupted turn gave no result within the grace period (Go: finishForeground). Foreground agents still
    // open are ended here; background agents keep running. Then the turn is ended as if it had finished.
    fn abandon_turn(&mut self) {
        let ids: Vec<String> = self
            .agents
            .keys()
            .filter(|id| !self.agent_info.get(*id).is_some_and(|i| i.background))
            .cloned()
            .collect();
        for id in ids {
            if let Some(t0) = self.agents.remove(&id) {
                if let Some(i) = self.agent_info.get(&id) {
                    self.stage(&agent_slot(&i.name), StagePhase::Done);
                }
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
        if self.handoff.is_some() {
            self.finish_handoff();
            return;
        }
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
        self.stage("main", StagePhase::Done);
        self.emit(UiEvent::TurnEnd { duration_ms });
        self.remember_turn();
        if self.cfg.residents && !self.turn_reply.trim().is_empty() {
            // The turn stays busy until the tui answers or fails; after_turn runs then.
            self.start_tui(self.turn_reply.clone());
            return;
        }
        self.after_turn();
    }

    // Rotation and the queued turn, once the reply is final (Go runs these after viaTUI).
    fn after_turn(&mut self) {
        if self.rotation_wanted() {
            if self.rotation_due() {
                self.rotate_deferred = None;
                if self.start_handoff() {
                    return;
                }
            } else {
                self.rotate_deferred.get_or_insert_with(Instant::now);
            }
        } else {
            self.rotate_deferred = None;
        }
        self.next_turn();
    }

    // Go doneMsg / mergePrompts: everything queued meanwhile goes out as ONE turn, joined by a blank line.
    fn next_turn(&mut self) {
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

    fn remember_turn(&mut self) {
        let rc = extract_reco(&self.turn_reply);
        if !rc.is_empty() {
            self.last_reco = rc;
        }
        self.recent
            .push_back((self.turn_prompt.clone(), self.turn_reply.clone()));
        while self.recent.len() > TAIL_TURNS {
            self.recent.pop_front();
        }
    }

    // Go shouldRotate at turn end: the context is at or past the threshold.
    fn rotation_wanted(&self) -> bool {
        let thr = threshold_for(&self.model, self.cfg.rotate_threshold);
        self.cfg.rotate_threshold > 0 && self.proc.is_some() && should_rotate(self.ctx_tokens, thr)
    }

    // Time left before a deferred rotation runs anyway. None when nothing is deferred.
    fn defer_left(&self) -> Option<Duration> {
        self.rotate_deferred
            .map(|t0| self.cfg.rotate_defer_timeout.saturating_sub(t0.elapsed()))
    }

    // A wanted rotation may start now: no background agents, the defer timeout passed, or zero timeout.
    fn rotation_due(&self) -> bool {
        self.rotation_wanted()
            && (self.agents.is_empty()
                || self.cfg.rotate_defer_timeout.is_zero()
                || self.defer_left().is_some_and(|d| d.is_zero()))
    }

    // Go runDeferredRotation: the held handoff runs while the child is idle, so the turn is marked busy.
    fn run_deferred_rotation(&mut self) {
        self.rotate_deferred = None;
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        if !self.start_handoff() {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Sends the hidden handoff prompt on the current child. Returns false when it could not be sent.
    fn start_handoff(&mut self) -> bool {
        let thr = threshold_for(&self.model, self.cfg.rotate_threshold);
        let reason = format!("ctx {} >= {}", self.ctx_tokens, thr);
        self.handoff = Some(Handoff {
            reason,
            attempts: 1,
        });
        if self.send_turn(HANDOFF_PROMPT) {
            return true;
        }
        self.handoff = None;
        false
    }

    // Ends the hidden handoff turn (Go makeSnapshotN): retries once when invalid, then rotates or holds.
    fn finish_handoff(&mut self) {
        self.turn_started = None;
        self.interrupted_at = None;
        self.open_tools.clear();
        let Some(h) = self.handoff.take() else {
            return;
        };
        let note = self.turn_reply.trim().to_string();
        let problems = validate_handoff(&note, &self.files);
        let valid = problems.is_empty();
        if !valid && h.attempts < 2 {
            self.handoff = Some(Handoff {
                reason: h.reason.clone(),
                attempts: h.attempts + 1,
            });
            if self.send_turn(HANDOFF_PROMPT) {
                return;
            }
            self.handoff = None;
        }
        if !valid && note.is_empty() && self.recent.is_empty() {
            // Go: nothing to carry, so the rotation is held and the session keeps running.
            self.emit(UiEvent::Error(
                "context rotation held: handoff failed".to_string(),
            ));
            self.next_turn();
            return;
        }
        let body = if valid {
            note
        } else {
            format!(
                "(handoff note failed validation: {}; recent turns follow)",
                problems.join("; ")
            )
        };
        let snapshot = with_reco(&self.last_reco, &self.build_snapshot(&body));
        self.rotate_child(snapshot, h.reason, valid, h.attempts);
        self.next_turn();
    }

    // Go buildSnapshot: preamble, handoff note, changed files, and the last turns, capped at SNAPSHOT_CHARS.
    fn build_snapshot(&self, note: &str) -> String {
        let note = if note.is_empty() { "(empty)" } else { note };
        let mut parts = vec![
            PREAMBLE.to_string(),
            format!("## Handoff note (written by previous session)\n{note}"),
        ];
        // Go bgHandoffLines: background agents still running, oldest first, with their result file.
        let mut bg: Vec<(Instant, String)> = self
            .agents
            .iter()
            .filter_map(|(id, t0)| {
                let info = self.agent_info.get(id).filter(|i| i.background)?;
                let line = format!(
                    "- {} ({}): running, result file: {}",
                    info.name, id, info.result_path
                );
                Some((*t0, line))
            })
            .collect();
        bg.sort();
        if !bg.is_empty() {
            let lines: Vec<String> = bg.into_iter().map(|(_, l)| l).collect();
            parts.push(format!(
                "## Background agents (results in notes/runs/)\n{}",
                lines.join("\n")
            ));
        }
        if !self.files.is_empty() {
            let list: Vec<String> = self.files.iter().map(|f| format!("- {f}")).collect();
            parts.push(format!("## Files changed\n{}", list.join("\n")));
        }
        if !self.recent.is_empty() {
            let mut turns = vec!["## Last turns (verbatim, truncated)".to_string()];
            for (user, assistant) in &self.recent {
                turns.push(format!(
                    "User: {}\nAssistant: {}",
                    clip(user, TURN_CHARS),
                    clip(assistant, TURN_CHARS)
                ));
            }
            parts.push(turns.join("\n\n"));
        }
        clip(&parts.join("\n\n"), SNAPSHOT_CHARS)
    }

    // Closes the current child and starts a fresh one (no --resume) that gets the snapshot with its next prompt.
    fn rotate_child(&mut self, snapshot: String, reason: String, valid: bool, attempts: u32) {
        let old = self.sid.clone();
        if let Some(mut p) = self.proc.take() {
            p.kill();
        }
        self.epoch += 1;
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
        self.pending_perm.clear();
        self.pending_snapshot = snapshot;
        self.sid.clear();
        self.ctx_tokens = 0;
        self.opts.resume = None;
        self.start_child();
        let _ = self.metrics.log(json!({
            "stage": "rotate",
            "reason": reason,
            "old_session": old,
            "handoff_valid": valid,
            "attempts": attempts,
            "outcome": "closed",
        }));
        self.emit(UiEvent::Rotated { reason });
        self.emit_status(self.cost_usd);
    }

    // Spawns the child for the current generation; its lines are tagged so a replaced child is ignored.
    fn start_child(&mut self) {
        let epoch = self.epoch;
        match proc::spawn(&self.opts, self.self_tx.clone(), move |m| {
            Input::Proc(epoch, m)
        }) {
            Ok(p) => self.proc = Some(p),
            Err(e) => self.emit(UiEvent::Error(format!("claude spawn failed: {e}"))),
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

// Go coderRole: slot 0 is "coder", slot i is "coder{i+1}" (coder2 for the first extra coder).
fn coder_role(i: usize) -> String {
    if i == 0 {
        "coder".to_string()
    } else {
        format!("coder{}", i + 1)
    }
}

// First short string field of a tool input, for the one-line ToolStart summary.
// Go tui.go resultText: a plain string, or the "text" of each block joined by newlines.
// The id that follows `marker` in `text`, e.g. the agentId in "agentId: a1b2".
fn agent_slot(description: &str) -> String {
    format!("agent:{description}")
}

fn after_marker(text: &str, marker: &str) -> Option<String> {
    let rest = &text[text.find(marker)? + marker.len()..];
    let id: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    (!id.is_empty()).then_some(id)
}

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
