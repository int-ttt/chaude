// Engine: one main claude child behind an actor thread. The UI sends commands and reads UiEvent values.
// Commands and child stdout lines share one mpsc channel, so a single loop owns all state.
// Owned types only: no lifetimes. Subagent tool events are not surfaced in E1 (pipeline is E2).

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::agent_store::{
    AgentStore, MAX_AGENT_LOG, STO_DONE, STO_FAILED, STO_INTERRUPTED, STO_RUNNING, SavedAgent,
    SavedLine, last_summary, sub_event_entries,
};
use crate::budget::{self, Budget};
use crate::cache_ttl::observed_ttl;
use crate::category::{CATEGORY_RULE, MarkerFilter, category_rotation};
use crate::child_set::{ChildSet, Close, Parked, child_key};
use crate::clients::{ClientRec, ClientRegistry, LIVE, PARKED, now_stamp};
use crate::event::{
    Block, Content, Delta, Event, StreamInner, SystemEvent, ToolStats, Usage, parse_line,
    tool_stats,
};
use crate::forward::forward_rule_for;
use crate::haiku_pool::WorkerPool;
use crate::handoff::{self, ExitHandoff};
use crate::i18n::{Msg, t, tf};
use crate::import_claude::{ClaudeImport, import_note};
use crate::mcp;
use crate::metrics::{Metrics, RateLog, usage_fields};
use crate::mode::{self, PermMode, Verdict};
use crate::orch;
use crate::osenv::resolve_bin;
use crate::pick_child::{self, ChildAction, RegRow};
use crate::proc::{self, ClaudeProc, ProcMsg, Shape, SpawnOpts};
use crate::record::{self, Stamp};
use crate::record_turn::{self, RecordState};
use crate::resident::{
    CODER_REPLY_FORMAT, CODER_RULE, CODER_TIMEOUT, PROMPTER_RULE, Prompter, PrompterOutcome,
    StagePhase, agent_body, prompter_notice, resident_slots, summarize_reply, tui_notice,
    tui_prompt,
};
use crate::resume_group;
use crate::resume_note;
use crate::rotate::{
    HAIKU_THRESHOLD, HANDOFF_PROMPT, PREAMBLE, extract_reco, should_rotate, threshold_for,
    validate_handoff, with_reco,
};
use crate::rotate_defer::{self, Hold};
use crate::turn_spend::TurnSpend;
use crate::usage_poll::{USAGE_POLL_EVERY_SECS, log_usage};

mod retry;
mod turn;

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

// One live resident coder for the coder line: its 1-based slot (coder = 1, coder2 = 2, ...), its context tokens
// and its rotation limit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CoderContext {
    pub slot: usize,
    pub tokens: u64,
    pub limit: u64,
}

// Settled state of a main Agent/Task call the reply waits for (UiEvent::TaskResolved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Done,
    Failed,
    Timeout,
}

// A main Agent/Task call the reply waits for. Running entries form the turn's pending set.
struct QueuedTask {
    id: String,
    desc: String,
    status: TaskStatus,
    since: Instant,
    // The agent's totals from its tool result (ToolStats); None until the result carries them.
    tokens: Option<u64>,
    duration_ms: Option<u64>,
}

// Pure acknowledgements that skip the prompter summary (S26). Only the whole input counts, no other words.
const ACK_WORDS: &[&str] = &[
    "ㅇㅇ",
    "ㅇㅋ",
    "응",
    "네",
    "넵",
    "좋아",
    "계속",
    "ok",
    "okay",
    "yes",
    "thanks",
    "감사합니다",
];

// True when the input is one pure ack word, ignoring case, surrounding spaces and trailing "." or "!".
fn is_pure_ack(text: &str) -> bool {
    let t = text
        .trim()
        .trim_end_matches(['.', '!'])
        .trim()
        .to_lowercase();
    ACK_WORDS.contains(&t.as_str())
}

// Sums per-message usage into (input, output, cache). None when all are zero.
fn sum_turn_counts<'a>(usages: impl Iterator<Item = &'a Usage>) -> Option<(u64, u64, u64)> {
    let (mut input, mut output, mut cache) = (0u64, 0u64, 0u64);
    for u in usages {
        input += u.input_tokens.unwrap_or(0);
        output += u.output_tokens.unwrap_or(0);
        cache +=
            u.cache_read_input_tokens.unwrap_or(0) + u.cache_creation_input_tokens.unwrap_or(0);
    }
    (input + output + cache > 0).then_some((input, output, cache))
}

// Saved task row fields tokens / duration_ms (missing in older files). None when neither is saved.
fn saved_stats(r: &Value) -> Option<ToolStats> {
    let tokens = r["tokens"].as_u64();
    let duration_ms = r["duration_ms"].as_u64();
    (tokens.is_some() || duration_ms.is_some()).then_some(ToolStats {
        tokens,
        duration_ms,
    })
}

fn task_status_name(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Running => "running",
        TaskStatus::Done => "done",
        TaskStatus::Failed => "failed",
        TaskStatus::Timeout => "timeout",
    }
}

fn task_status_from_name(s: &str) -> TaskStatus {
    match s {
        "done" => TaskStatus::Done,
        "failed" => TaskStatus::Failed,
        "timeout" => TaskStatus::Timeout,
        _ => TaskStatus::Running,
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
        // The Agent call's `model` input (Go agentRec.Model, shown by doneLine); empty = inherits the session model.
        model: String,
        // Go agentRec.Name for doneLine: the `name` input, else `subagent_type`.
        name: String,
        // Go doneLine what: the agent's last plain text line (Summary, else lastSummary(Log)), else its Task
        // (the `description` input).
        summary: String,
    },
    // A background agent resumed by SendMessage runs again under its original tool id until its AgentEnd.
    // send_id is the SendMessage call (queued or resumed); its row stays open until that agent ends.
    AgentResumed {
        id: String,
        description: String,
        started_at: u64,
        send_id: String,
    },
    // tokens = tokens in the context window (last assistant usage); context_pct = tokens * 100 / context_window.
    Status {
        model: String,
        tokens: u64,
        cost_usd: f64,
        context_pct: u32,
    },
    Busy(bool),
    // True while a running Agent is on the opus model (Go opusRunning: name prefix "opus-" or model "opus").
    OpusBusy(bool),
    // A resident role (coder, prompter, tui) has a command in flight (Go residentBusy). While busy, prompt is the
    // first line of the command and started_at is Unix seconds (Go agentRec Prompt, Started); both are empty/0 when idle.
    ResidentBusy {
        role: String,
        busy: bool,
        // The role's slot model (Go residentSlots: haiku for every resident but main). Empty when idle.
        model: String,
        prompt: String,
        started_at: u64,
    },
    Queued(usize),
    TurnEnd {
        duration_ms: u64,
    },
    // A main Agent/Task call is queued: the reply waits for it until TaskResolved settles it.
    TaskQueued {
        id: String,
        desc: String,
    },
    // A queued task settles. Running only acknowledges an async launch: the task stays pending until it ends.
    // cancel_wait settles the still-running tasks as Timeout.
    TaskResolved {
        id: String,
        status: TaskStatus,
    },
    // A task's agent totals from its tool result (either may be None). Sent once the result carries them.
    TaskStats {
        id: String,
        tokens: Option<u64>,
        duration_ms: Option<u64>,
    },
    // Main's token counts of the finished turn: input, output, and cache (read + creation). Sent before TurnStats.
    TurnTokens {
        input: u64,
        output: u64,
        cache: u64,
    },
    // Main's turn ended with pending tasks: the final text is held and no TurnEnd is sent yet.
    ReplyHeld {
        pending: usize,
    },
    // The held reply is released: every task settled, and one continuation input went to main.
    ReplyReleased,
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
    // Every delegated agent in start order (Go agentTracker, drain.go agentRec), sent after each agent change.
    // The panel filters them as Go rowsA does (tui.go:1448-1451): foreground = !background, running = !done.
    AgentRows {
        rows: Vec<AgentRow>,
    },
    // Lines one delegated agent's stream event adds to its log (Go onSubEvent); the TUI keeps the newest 400.
    // tool_ids is index-aligned with lines: the tool-use id of a call or result line, None for text (may be empty).
    AgentLog {
        id: String,
        lines: Vec<String>,
        tool_ids: Vec<Option<String>>,
    },
    // Log lines of a stored agent, each with the unix second it first arrived (restored agent views keep it).
    AgentLogAt {
        id: String,
        lines: Vec<SavedLine>,
    },
    // Agents an earlier harness left unfinished, shown below the running rows (Go restored rows, tui.go:1468).
    // Rows have done = true. Sent empty once the next user line arrives (Go dismissRestored, repl.go:1081).
    AgentsRestored {
        rows: Vec<AgentRow>,
    },
    // Prompt texts waiting behind the running turn, oldest first (Go m.queue; Queued(n) stays the count).
    QueueTexts(Vec<String>),
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
    // A failed prompter or tui pass, as the Go notice line (Go notice): text is the whole note.
    Notice {
        text: String,
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
    // Tokens in a resident role's context window from its last result usage (Go per-role ctx).
    RoleContext {
        role: String,
        tokens: u64,
    },
    // Every live resident coder in spawn order, with its context tokens and rotation limit (the coder line).
    // Empty when no coder child is running, which hides the line.
    CoderContexts {
        coders: Vec<CoderContext>,
    },
    // This run's session ids (Go /view): the main child, every resident that has answered, and all owned ids.
    SessionIds {
        main: String,
        residents: BTreeMap<String, String>,
        owned: Vec<String>,
    },
    // The main child's session changed without a rotation (a user /clear): the old id is the one just replaced.
    NewSession {
        old_session: String,
    },
    // Emitted just before Rotated: the session id the rotation replaced (empty when none).
    RotatedFrom {
        old_session: String,
    },
    // EngineHandle::resume_session relaunched the main child with --resume session_id (old_session may be empty).
    SessionResumed {
        old_session: String,
        session_id: String,
    },
    // Tool input JSON of a tool call, emitted right after its ToolStart (Go tool_summary / Agent model).
    ToolInput {
        id: String,
        input: Value,
    },
    // A claude child a dead harness left running; sent at start and never killed (Go "[orphan]" notice).
    // role is the handoff role (MAIN_ROLE or a resident role).
    // model is the child's model alias (haiku for a resident); empty when the handoff file had none.
    OrphanNotice {
        role: String,
        pid: u32,
        model: String,
    },
}

// Go coderSlot tools and the extra coder cap (settingsMaxExtraCoders).
const CODER_TOOLS: &str = "Read,Edit,Write,Bash,Grep,Glob";
const MAX_EXTRA_CODERS: usize = 8;
const TURN_CHARS: usize = 1_500;
// Go --max-workers default and resident.go residentMaxWorkers (the floor when neither flag nor env is set).
const DEFAULT_MAX_WORKERS: usize = 3;
const RESIDENT_MAX_WORKERS: usize = 8;
// A held rotation is checked this often while idle. Deferral keeps the hold start private, so the wait is
// polled rather than timed to the exact deadline.
const DEFER_POLL: Duration = Duration::from_secs(1);
// Go switchchild.go neverCold: the warm deadline when the idle rotate is disabled.
const NEVER_COLD: Duration = Duration::from_secs(24 * 365 * 3600);
// How long an idle resident child gets to exit after its stdin closes, before ClaudeProc::finish kills it.
const CODER_EXIT_WAIT: Duration = Duration::from_secs(5);
// S21: a coder turn that fails with a transient API error (429, overloaded) or a dead child is sent again,
// at most RETRY_MAX times with the backoff below, and not past RETRY_TOTAL after the first failure.
const RETRY_MAX: u32 = 3;
const RETRY_BACKOFF: [Duration; 3] = [
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
];
const RETRY_TOTAL: Duration = Duration::from_secs(60);

// Idle limit of an extra coder and of the base coder (Go extraCoderTTL). The settings screen shows it.
pub const CODER_IDLE_TTL_SECS: u64 = 3570;

#[derive(Debug, Clone)]
pub struct Config {
    // Name or path of the claude binary; resolved on PATH like osenv::resolve_bin. Tests point this at a fake.
    pub claude_bin: String,
    pub model: String,
    pub resume: Option<String>,
    pub cwd: Option<PathBuf>,
    // Home dir for the agent rule fallback (~/.claude/agents). Tests inject a temp dir instead of setting HOME.
    pub home: Option<PathBuf>,
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
    // Go r.ui == nil (plain line mode): stdin belongs to the prompt loop, so an ask with no turn running is denied
    // with a notice instead of waiting for an answer that never comes.
    pub no_ui: bool,
    // Go --task-budget-usd: USD cap per prompt (the ledger resets on each prompt). Zero = off.
    pub task_budget_usd: f64,
    // Go --metrics-dir: per-turn JSONL log directory. None = off (the CLI passes <record>/metrics).
    pub metrics_dir: Option<PathBuf>,
    // Go --transcript: JSONL trace of unparsable child lines ({"kind":"unparsed"}). None = off (the CLI passes StateDir/transcript.jsonl).
    pub transcript: Option<PathBuf>,
    // Go recordDir override (HARNESS_RECORD_DIR in the CLI): turn records and the /clear mark go here. None = record::dir_for(cwd).
    pub record_dir: Option<PathBuf>,
    // Go context rotation: a turn that ends at or above this many context tokens starts a handoff and a fresh
    // child (Opus uses its own threshold, see rotate::threshold_for). Zero = rotation off.
    pub rotate_threshold: u64,
    // Go --rotate-defer-timeout: how long a due rotation waits for background agents before it runs anyway.
    // Zero = rotate at once.
    pub rotate_defer_timeout: Duration,
    // Go --idle-rotate: a handoff and fresh child once the main child has no API response for this long (3570 s by
    // default, not shortened by an observed 5m cache). Zero = off.
    pub idle_rotate: Duration,
    // Harness permission mode the engine starts in. The TUI status bar starts on the same value (Config is its source).
    pub perm_mode: PermMode,
    // Resident slots are running (Go len(resident) > 0): the engine emits UiEvent::Stage rows for the main turn
    // and each agent call. Off = no stage rows.
    pub residents: bool,
    // Go extra_coders (.chaude/settings.json): coders coder2..coder(1+n) beyond the always-on coder, 0-8.
    // Extra coders start on demand and expire after 3570 s idle (N14 idle sweep).
    pub extra_coders: usize,
    // Go extraCoderTTL: an idle extra coder (coder2 and up) is ended after this long (3570 s). The base coder stays.
    pub coder_idle_ttl: Duration,
    // Idle time of the base coder before it exits on its own (3570 s), counted from its spawn. Zero = never exit.
    pub base_idle_exit_secs: u64,
    // Go tuiTimeout: how long the resident tui may take to translate the main reply. On timeout the main reply stays.
    pub tui_timeout: Duration,
    // Go prompterTimeout: how long the resident prompter may take to summarize. On timeout the input goes unchanged.
    pub prompter_timeout: Duration,
    // Go exit_handoff.go: JSON of this run's session ids, written on shutdown or disconnect. None = off.
    pub handoff_path: Option<PathBuf>,
    // Go --max-workers (env CHAUDE_MAX_WORKERS): cap on concurrent live worker sessions. Resolve with
    // resolve_max_workers. No engine effect yet: the Rust engine runs no worker pool.
    pub max_workers: usize,
    // Go --max-auto-rotations: automatic rotations allowed per engine run. Negative = unlimited, 0 = none.
    pub max_auto_rotations: i64,
    // Go --tail-turns: recent turns kept for the handoff snapshot and the imported-session snapshot.
    pub tail_turns: usize,
    // Go --snapshot-chars: cap on the handoff snapshot length, in chars.
    pub snapshot_chars: usize,
    // Go --show-rotation: print a one-line rotation marker. No engine effect yet: the TUI always shows Rotated.
    pub show_rotation: bool,
    // Go --no-handoff-validate: accept the handoff note without the section and file checks (no retry).
    pub no_handoff_validate: bool,
    // Go --max-drain: concurrently draining old sessions (0 = never drain). A rotation with agents running keeps the old child.
    pub max_drain: usize,
    // Go --drain-timeout: how long a draining child is awaited before it is closed.
    pub drain_timeout: Duration,
    // How long an agent whose completion arrived while its owned background task still runs stays open. The
    // agent ends at once when the owned task finishes, or at this grace when it never reports.
    pub agent_end_grace: Duration,
    // How long a queued main Agent/Task call may stay pending before it settles as Timeout (its reply then releases).
    pub agent_wait_timeout: Duration,
    // Chaude setting agent_wait (.chaude/settings.json): when false, a main turn ends at once even while its
    // Agent/Task calls run (no ReplyHeld). True holds the reply until every queued task settles.
    pub agent_wait: bool,
    // Go --rotate-cooldown: after a held rotation, no automatic context rotation for this long.
    pub rotate_cooldown: Duration,
    // Go --no-category-rotate: disable category-based session rotation. No engine effect yet.
    pub no_category_rotate: bool,
    // Go --no-delegation-rule: skip the haiku delegation rule in the child's system prompt. No engine effect yet.
    pub no_delegation_rule: bool,
    // Go --agents-file (default ~/.chaude/agents.json): persisted sub-agent state. None = off. No engine effect yet.
    pub agents_file: Option<PathBuf>,
    // Go --clients-file (default ~/.chaude/clients.json): per-(cwd,model) client registry. None = off. No engine effect yet.
    pub clients_file: Option<PathBuf>,
}

// Resolves the worker cap (Go resolveMaxWorkers): the flag when set, else the CHAUDE_MAX_WORKERS value when
// non-empty, else DEFAULT_MAX_WORKERS (3). Values below 1 are errors. The resident floor (Go repl.go) is a
// separate step: the caller applies `worker_floor` only when neither the flag nor the env is set.
// `extra_coders` is kept for the existing CLI call shape; the floor is applied by `worker_floor`, not here.
pub fn resolve_max_workers(
    flag: Option<usize>,
    env: &str,
    _extra_coders: usize,
) -> Result<usize, String> {
    if let Some(n) = flag {
        if n < 1 {
            return Err(format!("max-workers must be >= 1 (got {n})"));
        }
        return Ok(n);
    }
    let s = env.trim();
    if !s.is_empty() {
        return match s.parse::<usize>() {
            Ok(n) if n >= 1 => Ok(n),
            _ => Err(format!(
                "CHAUDE_MAX_WORKERS must be an integer >= 1 (got {env:?})"
            )),
        };
    }
    Ok(DEFAULT_MAX_WORKERS)
}

// Go repl.go floor, applied only when neither --max-workers nor CHAUDE_MAX_WORKERS is set: parked resident
// children plus extra coders need at least this many slots. Returns the larger of `current` and the floor.
pub fn worker_floor(current: usize, extra_coders: usize) -> usize {
    current.max(RESIDENT_MAX_WORKERS + extra_coders)
}

// Go wireClients: the client registry exists only when cfg.clients_file is set; None otherwise (Go nil registry).
pub fn wire_clients(cfg: &Config) -> Option<ClientRegistry> {
    cfg.clients_file.clone().map(ClientRegistry::new)
}

impl Default for Config {
    fn default() -> Self {
        let state = crate::osenv::home_dir().join(".chaude");
        Config {
            claude_bin: resolve_bin("claude"),
            model: "opus".to_string(),
            resume: None,
            cwd: None,
            home: std::env::var_os("HOME")
                .filter(|h| !h.is_empty())
                .map(PathBuf::from),
            cache_ttl: String::new(),
            compact_window: 0,
            context_window: 200_000,
            log_path: None,
            inactivity_timeout: Duration::from_secs(300),
            interrupt_grace: Duration::from_secs(20),
            auto_allow: false,
            no_ui: false,
            task_budget_usd: 0.0,
            metrics_dir: None,
            transcript: None,
            record_dir: None,
            rotate_threshold: HAIKU_THRESHOLD,
            rotate_defer_timeout: Duration::from_secs(600),
            idle_rotate: Duration::from_secs(3570),
            perm_mode: PermMode::Auto,
            residents: true,
            extra_coders: 4,
            coder_idle_ttl: Duration::from_secs(CODER_IDLE_TTL_SECS),
            base_idle_exit_secs: CODER_IDLE_TTL_SECS,
            tui_timeout: Duration::from_secs(120),
            prompter_timeout: Duration::from_secs(60),
            handoff_path: None,
            max_workers: DEFAULT_MAX_WORKERS,
            max_auto_rotations: -1,
            tail_turns: 4,
            snapshot_chars: 24_000,
            show_rotation: false,
            no_handoff_validate: false,
            max_drain: 3,
            drain_timeout: Duration::from_secs(60 * 60),
            agent_end_grace: Duration::from_secs(60),
            agent_wait_timeout: Duration::from_secs(1800),
            agent_wait: true,
            rotate_cooldown: Duration::from_secs(2 * 60),
            no_category_rotate: false,
            no_delegation_rule: false,
            agents_file: Some(state.join("agents.json")),
            clients_file: Some(state.join("clients.json")),
        }
    }
}

enum Cmd {
    Submit(String),
    // Abandons the queued-task wait of a held reply (cancel_wait).
    CancelWait,
    // Go runResidentCoder: one task for an idle coder. The reply comes back as UiEvent::CoderReply.
    Coder(String),
    // Go residentMCPConfig: the MCP tool call waits here for the coder's summary (Err = reason).
    CoderCall(String, Sender<Result<String, String>>),
    Interrupt(InterruptSource),
    // Mode name as typed ("next" cycles); parsed by the actor so unknown names are reported as UiEvent::Error.
    SetMode(String),
    // Go /resume (mid-run): relaunches the main child with --resume on this session id. Rejected while busy.
    ResumeSession(String),
    // Go applyImport: a transcript from import_claude::load_import_arg, installed for the next prompt.
    Import(Box<ClaudeImport>),
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
    agent_rows: Arc<Mutex<Vec<AgentRow>>>,
}

impl EngineHandle {
    // Go agentTracker read by the TUI tick: the actor rewrites this list on every agent change (emit_agent_rows).
    pub fn agent_rows(&self) -> Arc<Mutex<Vec<AgentRow>>> {
        Arc::clone(&self.agent_rows)
    }

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

    // Handle for the MCP server threads: each call blocks until the coder answers (Go runResidentCoder).
    pub fn coder_caller(&self) -> CoderCaller {
        CoderCaller {
            tx: self.tx.clone(),
        }
    }

    pub fn interrupt(&self, source: InterruptSource) {
        let _ = self.tx.send(Input::Cmd(Cmd::Interrupt(source)));
    }

    // Go /mode: "next" cycles, otherwise default | acceptEdits | plan | auto (case-insensitive).
    pub fn set_mode(&self, name: &str) {
        let _ = self.tx.send(Input::Cmd(Cmd::SetMode(name.to_string())));
    }

    // Relaunches the main child on an existing Claude session (claude --resume <session_id>), mid-run.
    // Only when idle; a busy run answers UiEvent::Error and keeps its child. Answers SessionResumed.
    pub fn resume_session(&self, session_id: &str) {
        let _ = self
            .tx
            .send(Input::Cmd(Cmd::ResumeSession(session_id.to_string())));
    }

    // Go applyImport: makes imp the session's history and queues its snapshot in front of the next prompt.
    // Refused while a turn runs (answers UiEvent::Error). Load imp with import_claude::load_import_arg.
    pub fn install_import(&self, imp: ClaudeImport) {
        let _ = self.tx.send(Input::Cmd(Cmd::Import(Box::new(imp))));
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

    // Abandons the wait of a held reply: the held text goes through the usual end of turn now.
    pub fn cancel_wait(&self) {
        let _ = self.tx.send(Input::Cmd(Cmd::CancelWait));
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
        let (handle, rx, _boot) = Self::spawn(cfg);
        (handle, rx)
    }

    // Go repl.go runTUI: waits for the first claude spawn. A failure returns its error for the caller to print.
    pub fn try_start(cfg: Config) -> Result<(EngineHandle, Receiver<UiEvent>), String> {
        let (handle, rx, boot) = Self::spawn(cfg);
        match boot.recv() {
            Ok(Err(e)) => Err(e),
            _ => Ok((handle, rx)),
        }
    }

    fn spawn(
        cfg: Config,
    ) -> (
        EngineHandle,
        Receiver<UiEvent>,
        Receiver<Result<(), String>>,
    ) {
        let (in_tx, in_rx) = mpsc::channel::<Input>();
        let (ui_tx, ui_rx) = mpsc::channel::<UiEvent>();
        let (boot_tx, boot_rx) = mpsc::channel();
        let self_tx = in_tx.clone();
        let agent_rows = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&agent_rows);
        let join = thread::spawn(move || actor(cfg, self_tx, in_rx, ui_tx, boot_tx, shared));
        (
            EngineHandle {
                tx: in_tx,
                join: Some(join),
                agent_rows,
            },
            ui_rx,
            boot_rx,
        )
    }
}

struct State {
    cfg: Config,
    // Shared with EngineHandle::agent_rows; written by emit_agent_rows.
    agent_rows: Arc<Mutex<Vec<AgentRow>>>,
    proc: Option<ClaudeProc>,
    ui: Sender<UiEvent>,
    busy: bool,
    // Set while an idle <task-notification> completes agents: its child-initiated reply answers them, so no notice turn.
    hold_notice_turn: bool,
    turn_started: Option<Instant>,
    queue: VecDeque<String>,
    model: String,
    // Go orch mode f: workspace routing of each turn (inert in other modes).
    router: orch::Router,
    ctx_tokens: u64,
    cost_usd: f64,
    tools: HashSet<String>,
    // Agent tool_use id -> start time, for AgentEnd elapsed_ms.
    agents: HashMap<String, Instant>,
    // Running Agent tool ids on the opus model; OpusBusy follows whether this set is empty.
    opus_agents: HashSet<String>,
    // A rotation that was due while background agents ran, and when its hold started (Go rotate_defer).
    rotate_deferred: rotate_defer::Deferral,
    // Delegated agents by tool_use id. A background agent (launch result said "async") keeps running past the
    // turn, so an abandoned turn does not end it, and it is listed in the handoff snapshot.
    agent_info: HashMap<String, AgentInfo>,
    // Owned background task id -> the agent (tool_use id) it belongs to. A live entry keeps that agent open.
    owned_tasks: HashMap<String, String>,
    // Agent task id -> agent tool_use id (from the agent's task_started). An owned task names its parent by task id.
    agent_task_ids: HashMap<String, String>,
    // Agent tool ids whose completion arrived while an owned task still ran, and when (ends at agent_end_grace).
    pending_end: HashMap<String, Instant>,
    // Main Agent/Task calls of the current task list (the pending ones are Running).
    queued_tasks: Vec<QueuedTask>,
    // A held turn: its final text and duration. Set by hold_reply; cleared by release_held or cancel_wait.
    held: Option<(String, u64)>,
    // A child-initiated turn (a background hand-back) runs while a reply is held: its text is not a reply, and the
    // release waits for its Result, since a continuation written mid-turn would start a second reply.
    held_turn: bool,
    // The next send_turn is a release continuation: it keeps the task list.
    continuation: bool,
    // Set once a system task_started arrives: the reply-ends-oldest-agent fallback (idle_result) is then off.
    seen_task_started: bool,
    // Restored agents still shown (UiEvent::AgentsRestored); the next user line clears them.
    restored_rows: Vec<AgentRow>,
    // The pick_child session orch_child resumed: one panel row, running while a turn runs, done when idle.
    picked_row: Option<AgentRow>,
    // Go agentStore: this session's delegated agents are written here on every change (None = off).
    agent_store: Option<AgentStore>,
    // agentId from an async launch result -> tool_use id, so a SendMessage resume finds the agent.
    agent_ids: HashMap<String, String>,
    // SendMessage tool_use id of a resume -> the resumed agent's tool id. Its task-notification names the SendMessage id.
    resume_ids: HashMap<String, String>,
    // Start label by tool_use id. Kept after the agent ends, so a later SendMessage resume still shows it.
    agent_labels: HashMap<String, String>,
    // Go subagentLabel by tool_use id (subagent_type, else "agent"): the stage row's agent name.
    agent_types: HashMap<String, String>,
    // tool_use ids still waiting for their tool_result; the inactivity timer is paused while non-empty.
    open_tools: HashSet<String>,
    // Last claude line or turn start; the inactivity timer measures from here.
    last_activity: Instant,
    // Set by an interrupt on a busy turn; the turn is force-ended when the grace period passes without a result.
    interrupted_at: Option<Instant>,
    // Results still owed by abandoned turns (Go core.go Session.skip). Each one is discarded so it cannot end a later turn.
    stray_results: u32,
    // Go repl.go idleTurn: a child-initiated turn that runs while no user turn does (reply to a background notice).
    idle_turn: IdleTurn,
    // Session cost when the running turn started, and Agent/Task calls made in it (for TurnStats).
    turn_cost_base: f64,
    // Go r.spend: subagent usage and spawn counts of the running turn, merged into its turn row.
    spend: TurnSpend,
    delegations: u32,
    // Go result usage and modelUsage of the turn that just ended (repl.go result_* keys); taken by log_turn.
    result_usage: Option<Usage>,
    result_model_usage: Option<Value>,
    // Main assistant usage of the running turn keyed by message id (a streamed reply repeats its id);
    // the fallback for TurnTokens when the result carries no usage.
    turn_msg_usage: HashMap<String, Usage>,
    // Harness permission mode (Go permMode); decides can_use_tool before any prompt.
    mode: PermMode,
    // can_use_tool requests waiting for a UI answer, keyed by request_id, with the tool input to echo back.
    pending_perm: HashMap<String, Value>,
    // Per-prompt USD ledger (Go budget); None when --task-budget-usd is off.
    budget: Option<Budget>,
    // Last main assistant API response, for the warm-cache test of the next-call estimate.
    last_api: Option<Instant>,
    // The last_api value an idle rotation already fired for: one idle rotation per API response.
    idle_fired_for: Option<Instant>,
    // Set when a main usage event's in-flight estimate reached the cap; the turn is interrupted once.
    budget_hit: bool,
    // Per-turn metrics JSONL (Go metrics); disabled when cfg.metrics_dir is None.
    metrics: Arc<Metrics>,
    // Go usagePollAt/usagePolling: last background /usage poll start, and whether one is in flight.
    usage_poll_at: Option<Instant>,
    usage_polling: Arc<AtomicBool>,
    // Go setRate metrics state: last five-hour utilization logged, and whether rate_raw was written.
    rate_log: RateLog,
    // Go r.reg: client rows in clients.json; None when cfg.clients_file is unset.
    clients: Option<ClientRegistry>,
    // Session id from the child's system event (Go sess.SessionID); "" until the child reports one.
    sid: String,
    // Every main and resident session id this run started (Go r.owned), kept for the handoff.
    owned: Vec<String>,
    // Last session id each resident role reported on a result line (Go resident_sessions). Live roles only.
    resident_sids: BTreeMap<String, String>,
    // Go residentSlots: the slots this run starts (main, prompter, tui, coder); empty with residents off.
    resident_slots: Vec<(&'static str, String)>,
    // Session a resident role starts with (`--resume`), decided at start by handoff::resume_mode; taken on spawn.
    resume_pending: BTreeMap<String, String>,
    // Last context tokens each resident role reported (Go per-role ctx), kept for the live handoff.
    role_ctx: BTreeMap<String, u64>,
    self_tx: Sender<Input>,
    opts: SpawnOpts,
    // Bumped on every fresh child so the old child's late lines and EOF are ignored.
    epoch: u64,
    // Source of new epochs. A restored parked child takes its old epoch back, so new ones never reuse a value.
    epoch_seq: u64,
    // Parked-live main children by (cwd, model) (Go childSet).
    children: ChildSet<ParkedProc>,
    // Haiku sessions that edited files (Go r.pool), and the prompt those edits belong to (Go curTask).
    pool: WorkerPool,
    cur_task: String,
    // Files written by Write/Edit/NotebookEdit in this session (Go r.files), for the handoff snapshot.
    files: Vec<String>,
    // Last recommendation line of a normal turn (Go lastReco), carried verbatim into the snapshot.
    last_reco: String,
    // Last normal turns as (prompt, reply), newest last (Go tr.turns).
    recent: VecDeque<(String, String)>,
    // Normal turns of this session as (turn number, prompt, reply), newest last; at most TURN_LOG_MAX. Only the
    // user prompt and the assistant text are kept (tool results never reach turn_reply).
    turn_log: VecDeque<(u64, String, String)>,
    // Normal turns finished this session; a launch records turn_seq + 1 (its turn's number).
    turn_seq: u64,
    // Automatic rotations done this run (Go rotations, checked against max_auto_rotations).
    rotations: u64,
    // When the last held rotation failed (Go lastRotFail), for rotate_cooldown.
    last_rot_fail: Option<Instant>,
    // Prompt and assistant text of the running turn.
    turn_prompt: String,
    turn_reply: String,
    // Filtered main text held back while residents route the reply to the tui; shown only when the tui fails.
    routed_text: String,
    // Hides [cat: X] markers from the streamed text of the main child (Go markerFilter).
    mark: MarkerFilter,
    // Current task category from the last normal turn (Go r.cat).
    cat: String,
    // Cache TTL seen in an assistant usage.cache_creation (Go ttlSeen); "" until one is seen.
    ttl_seen: String,
    // Snapshot for the next prompt sent to a fresh child; empty when none is pending.
    pending_snapshot: String,
    // Go repl.go notices: "[agent done]" lines of drained agents, sent in front of the next user message.
    notices: Vec<String>,
    // Agents whose "[agent done]" notice is already queued or sent. Cleared when the agent is resumed.
    announced: HashSet<String>,
    // Set while a hidden handoff turn runs; its text is not shown.
    handoff: Option<Handoff>,
    // Model switch waiting for its handoff turn (Go switchChild): target model and the held prompt.
    switch_to: Option<(String, String)>,
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
    // Old main children kept alive while their agents run (Go repl.go draining, drain.go); oldest first.
    drains: Vec<Drain>,
    // Coder residents (Go coder, coder2..): slot 0 is "coder", slot i is "coder{i+1}". Grown on demand.
    coders: Vec<Coder>,
    coder_epoch: u64,
    // Go residentMCPConfig, created on first use when residents are on. Its guard removes the temp dir on drop.
    mcp_cfg: Option<mcp::McpConfig>,
    // Go recordSave state: the reply and measurement of the turn that just ended, the topic, the /clear split.
    record: RecordState,
}

// Go repl.go idleTurn: what a child-initiated turn showed. active: a main reply was seen; notif: a
// task-notification was seen, so the completion is already known.
#[derive(Default)]
struct IdleTurn {
    active: bool,
    notif: bool,
    // When the idle reply started (Go idleTurn.t0).
    t0: Option<Instant>,
}

// One coder resident. A running deadline means a task is in flight (Go busyRec).
#[derive(Default)]
struct Coder {
    proc: Option<ClaudeProc>,
    // Generation of the child in this slot; a replaced child's lines are dropped by epoch.
    epoch: u64,
    deadline: Option<Instant>,
    // When the last task ended; drives the idle expiry of extra coders.
    idle_since: Option<Instant>,
    // Answer for an MCP call in flight; taken by finish_coder.
    waiter: Option<Sender<Result<String, String>>>,
    // S21: the full task text, sent again on a retry; cleared by finish_coder.
    task: Option<String>,
    retries: u32,
    // When the first failure of this task happened; the RETRY_TOTAL budget runs from it.
    retry_started: Option<Instant>,
    // When the next send of `task` is due.
    retry_at: Option<Instant>,
}

// A handoff turn in progress: the rotation reason and how many handoff prompts were sent (Go makeSnapshotN).
struct Handoff {
    reason: String,
    attempts: u32,
    // Go clearWithHandoff: the snapshot goes to a fresh session after /clear instead of a rotation.
    clear: bool,
}

// One panel/background row (Go agentRec fields the TUI shows). Field order follows agentRec in drain.go:17-38.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentRow {
    pub id: String,
    // Go Task: the Agent call's `description` input.
    pub task: String,
    // Go Prompt: first line of the Agent call's `prompt` input.
    pub prompt: String,
    // Go Name: the `name` input, else `subagent_type`. Empty when neither is set.
    pub name: String,
    // Go Model: the `model` input; empty = inherits the session model.
    pub model: String,
    pub background: bool,
    pub done: bool,
    // Finished with an error (a restored failed agent).
    pub failed: bool,
    // Unfinished when its child went away (a restored interrupted or running agent).
    pub interrupted: bool,
    // Go AgentID: handle from the launch result ("agentId: <id>"); empty until a background launch reports it.
    pub agent_id: String,
    pub result_path: String,
    // Unix seconds, as AgentStart.started_at.
    pub started_at: u64,
    // Unix seconds of the agent's last output or tool activity (its start until then). 0: unknown.
    pub last_active: u64,
}

// The pick_child session row: its id is the session id, its name and model the child's model.
fn picked_agent_row(session_id: &str, model: &str, started_at: u64) -> AgentRow {
    AgentRow {
        id: format!("pick:{session_id}"),
        task: "pick_child".to_string(),
        prompt: String::new(),
        name: model.to_string(),
        model: model.to_string(),
        background: false,
        done: false,
        failed: false,
        interrupted: false,
        agent_id: session_id.to_string(),
        result_path: String::new(),
        started_at,
        last_active: started_at,
    }
}

// Panel rows with the picked child's row appended last: running (done = false) while busy, done when idle.
fn with_picked(mut rows: Vec<AgentRow>, picked: Option<&AgentRow>, busy: bool) -> Vec<AgentRow> {
    if let Some(p) = picked {
        rows.push(AgentRow {
            done: !busy,
            ..p.clone()
        });
    }
    rows
}

// The status an agent is saved with: a finished agent whose tool result was an error is saved as failed, so a
// restart shows 실패 too. Other statuses are kept as given.
fn saved_status(status: &str, failed: bool) -> &str {
    if status == STO_DONE && failed {
        STO_FAILED
    } else {
        status
    }
}

// The panel rows of stored agents and the events that show them: their log lines (with arrival times), the saved
// summary for agents stored before log lines were kept, and the row list itself.
fn restored_events(saved: &[SavedAgent]) -> (Vec<AgentRow>, Vec<UiEvent>) {
    let rows: Vec<AgentRow> = saved.iter().map(restored_row).collect();
    let mut events = Vec::new();
    // A stored agent's log lines return with their own arrival times.
    for s in saved.iter().filter(|s| !s.log.is_empty()) {
        events.push(UiEvent::AgentLogAt {
            id: s.id.clone(),
            lines: s.log.clone(),
        });
    }
    // Go restore (agentview): a stored agent's saved summary is its agent-view body, marked "(restored)".
    // Agents stored before log lines were kept have no log, so only they take the summary.
    for s in saved
        .iter()
        .filter(|s| s.log.is_empty() && !s.summary.is_empty())
    {
        events.push(UiEvent::AgentLog {
            id: s.id.clone(),
            lines: vec![format!("(restored) {}", s.summary)],
            tool_ids: Vec::new(),
        });
    }
    events.push(UiEvent::AgentsRestored { rows: rows.clone() });
    (rows, events)
}

// A stored agent as a panel row. Restored agents are finished (done); only a stored interrupted or running one
// (its child is gone) is marked interrupted. A done agent keeps its done status, a failed one its failure.
fn restored_row(s: &SavedAgent) -> AgentRow {
    AgentRow {
        id: s.id.clone(),
        task: s.task.clone(),
        prompt: s.prompt.clone(),
        name: s.name.clone(),
        model: s.model.clone(),
        background: s.background,
        done: true,
        failed: s.status == STO_FAILED,
        interrupted: s.status == STO_INTERRUPTED || s.status == STO_RUNNING,
        agent_id: s.agent_id.clone(),
        result_path: s.out_path.clone(),
        started_at: s
            .started
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        // The last log line's arrival, else the stored last change (the finish, or the interrupt for an agent cut
        // short). Its duration runs from start to here.
        last_active: match s.log.last() {
            Some(l) => l.at,
            None => s
                .updated
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        },
    }
}

// Go drain: an old main child that keeps running its agents after a rotation. Its lines are tagged with its epoch.
struct Drain {
    proc: ClaudeProc,
    epoch: u64,
    session: String,
    since: Instant,
    // Every agent the drain waited for, sorted (its end notices).
    started: Vec<String>,
    // Set when the drained child starts a turn on its own (a native task notification): it was interrupted, and its
    // output up to its result is discarded. Cleared by that result.
    interrupting: bool,
}

// Go rotate: an old child is drained only when agents are running and --max-drain allows it.
fn drain_wanted(max_drain: usize, running: usize) -> bool {
    max_drain > 0 && running > 0
}

// Go drain.finished: done once the shared tracker has no running agent, timed out at drain_timeout, else still
// waiting (None).
fn drain_finished(running: usize, elapsed: Duration, timeout: Duration) -> Option<bool> {
    if running == 0 {
        Some(false)
    } else if elapsed >= timeout {
        Some(true)
    } else {
        None
    }
}

// Go pollDrains over drain.finished (drain.go:305-313): the drains that finish now, as (epoch, timed_out).
// running is the shared tracker's running count; each drain is (epoch, time since its start).
fn finished_drains(
    drains: &[(u64, Duration)],
    running: usize,
    timeout: Duration,
) -> Vec<(u64, bool)> {
    drains
        .iter()
        .filter_map(|&(epoch, elapsed)| {
            drain_finished(running, elapsed, timeout).map(|to| (epoch, to))
        })
        .collect()
}

// Longest report body a "[agent done]" notice carries; a longer report is cut and marked.
const NOTICE_REPORT_CHARS: usize = 4000;
// The brief line of a done notice for an agent the previous main launched (one line, short).
const NOTICE_BRIEF_CHARS: usize = 300;
// The turns carried into a rotated main with its live agents (newest kept).
const CARRIED_TURN_CHARS: usize = 12_000;
// The brief of each live agent in the carried block (head kept, cut marked).
const CARRIED_BRIEF_CHARS: usize = 1_500;
// Normal turns kept for the carried block; a launch turn older than this is carried from the oldest kept turn.
const TURN_LOG_MAX: usize = 200;

// Go pollDrains: the notice of one agent a drain waited for. done is false when the drain timed out or the agent is still
// running. A non-empty brief (the previous main's agent) adds a one-line "brief:" after the header. The report body (the
// agent's final report text) follows; empty report: header only.
fn drain_done_notice(
    id: &str,
    task: &str,
    out: &str,
    done: bool,
    brief: &str,
    report: &str,
) -> String {
    let st = if done {
        t(Msg::StateDone)
    } else {
        t(Msg::StateTimeout)
    };
    let mut head = format!("[agent done] {id} · {task} · {out} · {st}");
    let brief: String = brief.split_whitespace().collect::<Vec<_>>().join(" ");
    if !brief.is_empty() {
        head.push_str(&format!("\nbrief: {}", clip(&brief, NOTICE_BRIEF_CHARS)));
    }
    if report.is_empty() {
        return head;
    }
    if report.chars().count() <= NOTICE_REPORT_CHARS {
        return format!("{head}\n{report}");
    }
    let cut: String = report.chars().take(NOTICE_REPORT_CHARS).collect();
    format!("{head}\n{cut}\n[cut at {NOTICE_REPORT_CHARS} chars]")
}

// input.message of a sub-agent's SubagentHandback tool_use (its final report); None for other lines.
fn handback_message(raw: &Value) -> Option<String> {
    raw.get("message")?
        .get("content")?
        .as_array()?
        .iter()
        .find(|b| {
            b.get("type").and_then(Value::as_str) == Some("tool_use")
                && b.get("name").and_then(Value::as_str) == Some("SubagentHandback")
        })?
        .get("input")?
        .get("message")?
        .as_str()
        .map(str::to_string)
}

// Go repl.go runTurn: queued notices, one per line, lead the user text (empty list: text unchanged).
fn with_notices(notices: &[String], text: &str) -> String {
    if notices.is_empty() {
        return text.to_string();
    }
    format!("{}\n\n{}", notices.join("\n"), text)
}

// Tool-use ids named by one <task-notification> text (Go drain.go onNotification); empty for other text.
fn task_notification_ids(text: &str) -> Vec<String> {
    if !text.contains("<task-notification>") {
        return Vec::new();
    }
    text.split("<tool-use-id>")
        .skip(1)
        .map(|part| {
            part.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect()
        })
        .collect()
}

// Go drain.consume: the deny message for a permission prompt of a draining child.
const DRAIN_DENY: &str = "denied: session is draining";

// request_id of a can_use_tool control_request line (Go drain.consume denies it); None for other lines.
// True when the agent's "[agent done]" notice is not queued yet for this run of the agent (once per agent).
fn announce_once(announced: &mut HashSet<String>, id: &str) -> bool {
    announced.insert(id.to_string())
}

fn drain_permission_id(line: &str) -> Option<String> {
    let raw = serde_json::from_str::<Value>(line).ok()?;
    if raw["type"] != "control_request" || raw["request"]["subtype"] != "can_use_tool" {
        return None;
    }
    raw["request_id"].as_str().map(str::to_string)
}

// What the handoff snapshot needs about a delegated agent (Go agentRec: Name, ResultPath, Background, Started,
// Done, ResultDone). The entry stays after the agent ends, marked done, so the snapshot can list it.
struct AgentInfo {
    name: String,
    result_path: String,
    background: bool,
    started: Instant,
    done: bool,
    // The result file was written. Not set yet: Rust has no agentresult port (Go agentresult.go).
    result_done: bool,
    // Go agentRec OutPath: the first notes/runs/*.md path named in the prompt (drain notice).
    out_path: String,
    // The Agent call's `model` input; empty when the agent inherits the session model.
    model: String,
    // Go agentRec Task, Prompt, Name (session name), AgentID, Started (unix secs) for AgentRow.
    task: String,
    prompt: String,
    // The full Agent input prompt (the brief): carried to a rotated main and into its done notice.
    brief: String,
    // The epoch of the main child that launched the agent, and the session turn number of its launch turn.
    launch_epoch: u64,
    launch_seq: u64,
    session_name: String,
    agent_id: String,
    started_at: u64,
    // Unix seconds of the agent's last output or tool activity (AgentRow.last_active).
    last_active: u64,
    // The agent's log lines with their arrival seconds, capped like the view log, for the stored record.
    log: Vec<SavedLine>,
    // Go lastSummary over the agent's log: its last plain text line, kept as the done notice's what.
    last_text: String,
    // The message of the agent's SubagentHandback call: its final report, preferred over last_text.
    handback: String,
    // handback_report.text of the agent's latest task_notification: the body of its "[agent done]" notice.
    report: String,
    // The agent's tool result was an error (a failed run or a hook block): it ends as failed, saved as failed.
    failed: bool,
}

impl AgentInfo {
    // Go doneLine what: the final report (SubagentHandback), else the last text line, else the Task.
    fn done_what(&self) -> String {
        if !self.handback.is_empty() {
            self.handback.clone()
        } else if self.last_text.is_empty() {
            self.task.clone()
        } else {
            self.last_text.clone()
        }
    }
}

fn actor(
    cfg: Config,
    self_tx: Sender<Input>,
    rx: Receiver<Input>,
    ui: Sender<UiEvent>,
    boot: Sender<Result<(), String>>,
    agent_rows: Arc<Mutex<Vec<AgentRow>>>,
) {
    // Session loads: a killed or below-threshold previous session resumes (handoff::resume_mode). An explicit
    // cfg.resume wins for the main child.
    let mut main_resume = cfg.resume.clone();
    let mut resume_pending = BTreeMap::new();
    let mut orphans = Vec::new();
    let prev_file = cfg
        .handoff_path
        .as_deref()
        .map(|p| handoff::read_path(p, cfg.resume.as_deref()));
    if let Some((prev, killed)) = prev_file.as_deref().and_then(handoff::previous_run) {
        if killed {
            orphans = handoff::orphans(&prev, handoff::pid_alive)
                .into_iter()
                .map(|(role, pid)| {
                    let model = prev.child_models.get(&role).cloned().unwrap_or_default();
                    (role, pid, model)
                })
                .collect();
        }
        let main_thr = threshold_for(&cfg.model, cfg.rotate_threshold);
        if let handoff::Load::Resume(sid) =
            handoff::resume_mode(&prev, handoff::MAIN_ROLE, killed, main_thr)
            && cfg.resume.is_none()
        {
            main_resume = Some(sid);
        }
        let res_thr = threshold_for("haiku", cfg.rotate_threshold);
        for role in prev.resident_sessions.keys() {
            if let handoff::Load::Resume(sid) = handoff::resume_mode(&prev, role, killed, res_thr) {
                resume_pending.insert(role.clone(), sid);
            }
        }
    }
    let rule = main_rule(&cfg);
    let restore_sid = main_resume.clone();
    let opts = SpawnOpts {
        claude_bin: resolve_bin(&cfg.claude_bin),
        model: cfg.model.clone(),
        resume: main_resume,
        cwd: cfg.cwd.clone(),
        cache_ttl: cfg.cache_ttl.clone(),
        compact_window: cfg.compact_window,
        max_budget_usd: cfg.task_budget_usd,
        shape: Shape {
            rule,
            ..Shape::default()
        },
    };
    let model = cfg.model.clone();
    let budget = (cfg.task_budget_usd > 0.0).then(|| Budget::new(cfg.task_budget_usd));
    let perm_mode = cfg.perm_mode;
    // Go repl.go start(): agents a crashed harness left running are marked interrupted (before the reconcile).
    let agent_store = cfg.agents_file.as_deref().map(AgentStore::new);
    if let Some(store) = &agent_store {
        store.interrupt_running(false);
    }
    // Go repl.go:1354 residentSlots(a.Model): the table the resident spawns read. Empty when residents are off.
    let slots = if cfg.residents {
        resident_slots(&cfg.model)
    } else {
        Vec::new()
    };
    let clients = wire_clients(&cfg);
    let metrics = Arc::new(Metrics::new(
        cfg.metrics_dir.clone(),
        // Go repl.go:1375-1377 newMetrics flags, same keys; the first three are Rust-only extras.
        json!({
            "compact_window": cfg.compact_window,
            "context_window": cfg.context_window,
            "cache_ttl": cfg.cache_ttl,
            "threshold": cfg.rotate_threshold,
            "handoff_validate": !cfg.no_handoff_validate,
            "max_drain": cfg.max_drain,
            "drain_timeout_s": cfg.drain_timeout.as_secs(),
            "delegation_rule": !cfg.no_delegation_rule,
            "category_rotate": !cfg.no_category_rotate,
        }),
    ));
    let mut st = State {
        cfg,
        agent_rows,
        proc: None,
        ui,
        busy: false,
        hold_notice_turn: false,
        turn_started: None,
        queue: VecDeque::new(),
        model,
        router: orch::Router::new(&std::env::var(orch::ENV_KEY).unwrap_or_default()),
        ctx_tokens: 0,
        cost_usd: 0.0,
        tools: HashSet::new(),
        agents: HashMap::new(),
        opus_agents: HashSet::new(),
        rotate_deferred: rotate_defer::Deferral::default(),
        agent_info: HashMap::new(),
        owned_tasks: HashMap::new(),
        agent_task_ids: HashMap::new(),
        pending_end: HashMap::new(),
        queued_tasks: Vec::new(),
        held: None,
        held_turn: false,
        continuation: false,
        seen_task_started: false,
        restored_rows: Vec::new(),
        picked_row: None,
        agent_store,
        agent_ids: HashMap::new(),
        resume_ids: HashMap::new(),
        agent_labels: HashMap::new(),
        agent_types: HashMap::new(),
        open_tools: HashSet::new(),
        last_activity: Instant::now(),
        interrupted_at: None,
        stray_results: 0,
        idle_turn: IdleTurn::default(),
        turn_cost_base: 0.0,
        spend: TurnSpend::new(),
        delegations: 0,
        result_usage: None,
        result_model_usage: None,
        turn_msg_usage: HashMap::new(),
        // Go repl.go start(): the harness starts in Config.perm_mode (default auto, same as claude --permission-mode auto).
        mode: perm_mode,
        pending_perm: HashMap::new(),
        budget,
        last_api: None,
        idle_fired_for: None,
        budget_hit: false,
        metrics,
        sid: String::new(),
        owned: Vec::new(),
        resident_sids: BTreeMap::new(),
        resident_slots: slots,
        resume_pending,
        role_ctx: BTreeMap::new(),
        mcp_cfg: None,
        record: RecordState::default(),
        self_tx,
        opts,
        epoch: 0,
        epoch_seq: 0,
        children: ChildSet::new(),
        pool: WorkerPool::new(),
        cur_task: String::new(),
        files: Vec::new(),
        last_reco: String::new(),
        recent: VecDeque::new(),
        turn_log: VecDeque::new(),
        turn_seq: 0,
        rotations: 0,
        last_rot_fail: None,
        turn_prompt: String::new(),
        turn_reply: String::new(),
        routed_text: String::new(),
        mark: MarkerFilter::default(),
        cat: String::new(),
        ttl_seen: String::new(),
        pending_snapshot: String::new(),
        notices: Vec::new(),
        announced: HashSet::new(),
        handoff: None,
        switch_to: None,
        prompter: Prompter::new(),
        prompter_proc: None,
        prompter_epoch: 0,
        prompter_deadline: None,
        tui_proc: None,
        tui_epoch: 0,
        tui_deadline: None,
        drains: Vec::new(),
        coders: Vec::new(),
        coder_epoch: 0,
        usage_poll_at: None,
        usage_polling: Arc::new(AtomicBool::new(false)),
        rate_log: RateLog::default(),
        clients,
    };
    for (role, pid, model) in orphans {
        st.emit(UiEvent::OrphanNotice { role, pid, model });
    }
    // Resumed main session: show the task list it saved, same as /resume.
    if let Some(sid) = &restore_sid {
        st.load_tasks(sid);
    }
    // Go repl.go reconcileStart: repair rows a killed harness left open; show, never kill, orphaned children.
    if let Some(reg) = &st.clients {
        // Startup repair parks a killed session only while inside its idle limit (3570 s), not the cache window.
        let idle = st.cfg.idle_rotate;
        let deadline = if idle.is_zero() { NEVER_COLD } else { idle };
        for row in reg.reconcile(now_stamp(), deadline) {
            st.emit(UiEvent::OrphanNotice {
                role: String::new(),
                pid: row.child_pid,
                model: row.model,
            });
        }
    }
    st.persist_live();
    let started = st.start_child();
    let main_up = started.is_ok();
    let _ = boot.send(started);
    if main_up && st.cfg.residents {
        st.spawn_residents();
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
            Input::Cmd(Cmd::ResumeSession(sid)) => st.resume_session(&sid),
            Input::Cmd(Cmd::Import(imp)) => st.install_import(*imp),
            Input::Cmd(Cmd::Permission { id, allow }) => st.answer_permission(&id, allow),
            Input::Cmd(Cmd::Usage) => st.usage(),
            Input::Cmd(Cmd::CancelWait) => st.cancel_wait(),
            Input::Proc(epoch, msg) if epoch == st.epoch => match msg {
                ProcMsg::Line(line) => st.on_line(&line),
                ProcMsg::Eof => st.on_eof(),
            },
            Input::Proc(epoch, msg) if st.drains.iter().any(|d| d.epoch == epoch) => {
                st.on_drain_msg(epoch, msg)
            }
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
            Input::Cmd(Cmd::Coder(task)) => st.start_coder(task, None),
            Input::Cmd(Cmd::CoderCall(task, waiter)) => st.start_coder(task, Some(waiter)),
            Input::Coder(i, epoch, msg) if st.coder_matches(i, epoch) => match msg {
                ProcMsg::Line(line) => st.on_coder_line(i, &line),
                ProcMsg::Eof => st.on_coder_eof(i),
            },
            Input::Coder(_, _, _) => {}
        }
    }
    if let Some(path) = st.cfg.handoff_path.clone() {
        let exit = handoff::exit_write_path(&path, &st.sid);
        let _ = handoff::write_exit_handoff(&exit, &st.exit_handoff());
        handoff::remove_live_handoff(&exit);
    }
    // Go markInterrupted on exit: this process's running agents are stored as interrupted.
    if let Some(store) = &st.agent_store {
        store.interrupt_running(true);
    }
    // Quit path: every child is killed at once. Session files are already persisted per turn, so nothing
    // waits for a transcript flush (the old stdin-EOF grace cost up to 3 s per child).
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
    // Go closeDrains on exit: draining children are closed.
    for mut d in std::mem::take(&mut st.drains) {
        d.proc.kill();
    }
}

// Go usagePollEvery gate: a poll starts when none is in flight and the last start is a full interval ago.
fn usage_poll_due(last: Option<Instant>, now: Instant, polling: bool) -> bool {
    let every = Duration::from_secs(USAGE_POLL_EVERY_SECS);
    !polling && last.is_none_or(|t| now.duration_since(t) >= every)
}
impl State {
    fn emit(&self, ev: UiEvent) {
        // The picked child's row follows the busy state, so the list updates on every Busy event.
        let busy = match &ev {
            UiEvent::Busy(b) if self.picked_row.is_some() => Some(*b),
            _ => None,
        };
        let _ = self.ui.send(ev);
        if let Some(b) = busy {
            self.emit_agent_rows_at(b);
        }
    }

    // Runs the one-shot on its own thread so the actor keeps streaming a busy turn.
    fn usage(&self) {
        let bin = resolve_bin(&self.cfg.claude_bin);
        let cwd = self.engine_cwd();
        let ui = self.ui.clone();
        thread::spawn(move || {
            let ev = match proc::one_shot_usage(&bin, &cwd) {
                Ok(text) => UiEvent::Usage { text },
                Err(e) => UiEvent::Error(e),
            };
            let _ = ui.send(ev);
        });
    }

    // Go pollUsage: after a turn ends, at most one background /usage poll per interval. Off when
    // HARNESS_USAGE_POLL=0 or metrics are off (Go returns before the throttle, so no claude runs).
    fn poll_usage(&mut self) {
        if self.cfg.metrics_dir.is_none()
            || std::env::var("HARNESS_USAGE_POLL").as_deref() == Ok("0")
        {
            return;
        }
        let now = Instant::now();
        if !usage_poll_due(
            self.usage_poll_at,
            now,
            self.usage_polling.load(Ordering::SeqCst),
        ) {
            return;
        }
        self.usage_poll_at = Some(now);
        self.usage_polling.store(true, Ordering::SeqCst);
        let bin = resolve_bin(&self.cfg.claude_bin);
        let cwd = self.engine_cwd();
        let metrics = Arc::clone(&self.metrics);
        let polling = Arc::clone(&self.usage_polling);
        thread::spawn(move || {
            let text = proc::one_shot_usage(&bin, &cwd);
            let _ = log_usage(&metrics, text.as_deref().map_err(|e| e.as_str()));
            polling.store(false, Ordering::SeqCst);
        });
    }

    fn submit(&mut self, text: String) {
        // Go handle (repl.go:1080): pollDrains runs first, so finished drains queue their notices for this turn.
        self.reap_drains();
        // Go handle (repl.go:1081): the next user line hides the restored agents.
        if !self.restored_rows.is_empty() {
            self.restored_rows.clear();
            self.emit(UiEvent::AgentsRestored { rows: Vec::new() });
        }
        if self.proc.is_none() {
            self.emit(UiEvent::Error("claude is not running".to_string()));
            return;
        }
        // Go /clear and /rotate are session commands, not turns: they run here and never reach the child as text.
        let cmd = text.trim();
        if cmd == "/clear" || cmd == "/rotate" {
            if self.busy || self.prompter.waiting() {
                self.emit(UiEvent::Error(format!("{cmd} needs an idle session")));
                return;
            }
            if cmd == "/clear" {
                self.clear_session();
            } else {
                self.manual_rotate();
            }
            return;
        }
        // Go /model NAME (tui.go runTUI): a session command too, handled as a rotation to the new model.
        if let Some(model) = cmd.strip_prefix("/model ").map(str::trim)
            && !model.is_empty()
        {
            if self.busy || self.prompter.waiting() {
                self.emit(UiEvent::Error("/model needs an idle session".to_string()));
                return;
            }
            self.model_switch(model.to_string());
            return;
        }
        // /lang CODE (TUI and plain /lang, after the setting is saved and the process language set): a handoff so the
        // fresh main child starts with the new --settings language. Busy: the saved language applies at the next rotation.
        if let Some(code) = cmd.strip_prefix("/lang ").map(str::trim)
            && !code.is_empty()
        {
            if self.busy || self.prompter.waiting() {
                self.emit(UiEvent::Error(
                    "language saved; the running session switches on the next /rotate".to_string(),
                ));
                return;
            }
            self.busy = true;
            self.emit(UiEvent::Busy(true));
            if !self.start_handoff(format!("lang {code}")) {
                self.busy = false;
                self.emit(UiEvent::Busy(false));
            }
            return;
        }
        if self.busy || self.prompter.waiting() {
            self.queue.push_back(text);
            self.emit(UiEvent::Queued(self.queue.len()));
            self.emit_queue();
            return;
        }
        self.begin_turn(text);
    }

    // Sends text to the main child as a turn; the engine is busy only when it went out.
    fn dispatch(&mut self, text: &str) {
        // Go routeOrch: mode f sends a workspace-tagged line to its haiku session and an untagged line to the main model.
        let routed = self.router.route(text, &self.cfg.model, Instant::now());
        match routed.target {
            orch::Target::Current => {}
            orch::Target::Main => {
                if let Some(m) = self.router.main_model().map(str::to_string)
                    && m != self.opts.model
                {
                    if self.start_switch_handoff(&m, &routed.text) {
                        return;
                    }
                    self.orch_child(m, None);
                }
            }
            orch::Target::Resume { session_id, .. } => {
                self.orch_child("haiku".to_string(), Some(session_id));
            }
            orch::Target::Fresh { .. } => self.orch_child("haiku".to_string(), None),
        }
        self.cur_task = routed.text.clone();
        if self.send_turn(&routed.text) {
            self.busy = true;
            self.emit(UiEvent::Busy(true));
        }
    }

    // Go switchChild: the active main child is parked, then the child for the new model is reused live,
    // resumed from its registry row, or started new with the handoff snapshot in front of its first prompt.
    // resume keeps a live workspace session, None starts a fresh one.
    fn orch_child(&mut self, model: String, resume: Option<String>) {
        let switched = model != self.model;
        if switched {
            self.park_active();
        }
        self.model = model.clone();
        self.opts.model = model.clone();
        // The previous pick_child row belongs to the child that is now parked or replaced.
        if self.picked_row.take().is_some() {
            self.emit_agent_rows();
        }
        if let Some(sid) = resume {
            self.resume_session(&sid);
            return;
        }
        let cwd = self.client_cwd();
        let key = child_key(&cwd, &model, "");
        let parked = match self.pick_for(&model) {
            ChildAction::UseLive => self.children.take(&key),
            ChildAction::Resume => {
                if let Some(r) = self.child_row(&cwd, &model) {
                    self.resume_session(&r.session_id);
                    self.picked_row = Some(picked_agent_row(&r.session_id, &model, now_secs()));
                    self.emit_agent_rows();
                }
                return;
            }
            ChildAction::StartNew => None,
        };
        match parked {
            Some(p) => self.restore_child(p),
            // A pending snapshot (import or kept rotation) stays for the new child (Go switchChild).
            None => {
                self.restart_child();
            }
        }
    }

    // Registry row for (cwd, model), as pickChild reads it.
    fn child_row(&self, cwd: &str, model: &str) -> Option<RegRow> {
        self.clients
            .as_ref()
            .and_then(|reg| reg.get(cwd, model))
            .map(|r| RegRow {
                parked: r.state == PARKED,
                session_id: r.session_id,
                owner_pid: r.owner_pid,
                last_api: stamp_time(r.last_api),
                last_ctx: u64::try_from(r.last_ctx).unwrap_or(0),
            })
    }

    // Go pickChild for the (cwd, model) child: reuse live, resume the registry row, or start new.
    fn pick_for(&self, model: &str) -> ChildAction {
        let cwd = self.client_cwd();
        let live = self.children.warm_info(&child_key(&cwd, model, ""));
        let row = self.child_row(&cwd, model);
        // Go pickChild compares against thresholdFor(target): Opus gets its own 200k limit.
        let limit = threshold_for(model, self.cfg.rotate_threshold);
        pick_child::pick_child(
            live,
            row.as_ref(),
            limit,
            SystemTime::now(),
            pick_warm_window(&self.cfg),
        )
    }

    // Go switchChild snapshot: a switch to a fresh child first runs the handoff turn on the running child. The
    // prompt is held in switch_to and sent to the new child when the handoff ends (finish_handoff).
    fn start_switch_handoff(&mut self, model: &str, text: &str) -> bool {
        let pick = self.pick_for(model);
        let due = switch_needs_handoff(
            &self.pending_snapshot,
            self.recent.len(),
            self.proc.is_some(),
            pick,
            self.handoff.is_some(),
        );
        if !due {
            return false;
        }
        self.switch_to = Some((model.to_string(), text.to_string()));
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        if self.start_handoff(format!("route {model}")) {
            return true;
        }
        self.switch_to = None;
        self.busy = false;
        self.emit(UiEvent::Busy(false));
        false
    }

    // Go switchChild after the snapshot: the old child is parked under its own model, the new child for the
    // target starts with the snapshot in front of the held prompt.
    fn complete_switch(&mut self, model: String, text: String, snapshot: String) {
        self.park_active();
        self.model = model.clone();
        self.opts.model = model;
        self.restart_child();
        self.pending_snapshot = snapshot;
        self.cur_task = text.clone();
        if !self.send_turn(&text) {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Go parkOld: the active main child stays alive under its (cwd, model) key and its registry row is parked.
    fn park_active(&mut self) {
        let Some(proc) = self.proc.take() else {
            return;
        };
        let now = SystemTime::now();
        let last_api = self
            .last_api
            .and_then(|t| now.checked_sub(t.elapsed()))
            .unwrap_or(UNIX_EPOCH);
        let cwd = self.client_cwd();
        let key = child_key(&cwd, &self.model, "");
        let sid = self.sid.clone();
        let ctx = self.ctx_tokens;
        if let Some(reg) = &self.clients {
            reg.upsert(ClientRec {
                cwd: cwd.clone(),
                model: self.model.clone(),
                session_id: sid.clone(),
                state: PARKED.to_string(),
                last_api: now_stamp(),
                last_ctx: i64::try_from(ctx).unwrap_or(i64::MAX),
                owner_pid: std::process::id(),
                child_pid: proc.pid(),
                ..Default::default()
            });
        }
        let epoch = self.epoch;
        self.children.park(
            &key,
            Parked {
                sess: ParkedProc { proc, epoch, sid },
                model: self.model.clone(),
                last_api,
                last_ctx: ctx,
                busy: !self.agents.is_empty(),
            },
        );
    }

    // Go endAllParked("clear"): every parked child is ended and its registry row marked, so a /clear cannot bring
    // them back (their pre-clear session ids included).
    fn end_all_parked(&mut self) {
        let cwd = self.client_cwd();
        for (_, mut p) in self.children.take_all() {
            p.sess.close();
            if let Some(reg) = &self.clients {
                reg.mark_ended(&cwd, &p.model, "clear", now_stamp());
            }
        }
    }

    // Go switchChild with actUseLive: the parked child becomes the active one again under its own epoch.
    fn restore_child(&mut self, p: Parked<ParkedProc>) {
        let ParkedProc { proc, epoch, sid } = p.sess;
        let pid = proc.pid();
        self.proc = Some(proc);
        self.epoch = epoch;
        self.sid = sid.clone();
        self.ctx_tokens = p.last_ctx;
        self.last_api = SystemTime::now()
            .duration_since(p.last_api)
            .ok()
            .and_then(|age| Instant::now().checked_sub(age));
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
        self.pending_perm.clear();
        self.opts.resume = None;
        self.upsert_live(pid, sid);
        // The restored child's id is now the main id (/status and /view read it from SessionIds).
        self.emit_session_ids();
        self.emit_status(self.cost_usd);
    }

    // Adds or removes an opus Agent and emits OpusBusy only when the running state changes.
    fn opus_set(&mut self, id: &str, on: bool) {
        let before = !self.opus_agents.is_empty();
        if on {
            self.opus_agents.insert(id.to_string());
        } else {
            self.opus_agents.remove(id);
        }
        let after = !self.opus_agents.is_empty();
        if before != after {
            self.emit(UiEvent::OpusBusy(after));
        }
    }

    fn resident_busy(&mut self, role: &str, busy: bool, prompt: &str) {
        let (prompt, started_at) = if busy {
            (
                prompt.lines().next().unwrap_or_default().to_string(),
                now_secs(),
            )
        } else {
            (String::new(), 0)
        };
        let model = if busy {
            self.resident_model(role)
        } else {
            String::new()
        };
        self.emit(UiEvent::ResidentBusy {
            role: role.to_string(),
            busy,
            model,
            prompt,
            started_at,
        });
    }

    // Sends text to the prompter. A spawn, start or write failure falls back to the raw prompt (Go).
    fn start_prompter(&mut self, text: String) {
        if self.prompter_proc.is_none()
            && let Err(e) = self.spawn_prompter()
        {
            self.notice(prompter_notice(&tf(Msg::Error, &[&e])));
            self.dispatch(&text);
            return;
        }
        let phase = match self.prompter.start(&text) {
            Ok(phase) => phase,
            Err(e) => {
                self.notice(prompter_notice(&tf(Msg::Error, &[&e])));
                self.dispatch(&text);
                return;
            }
        };
        self.stage("prompter", phase);
        self.prompter_deadline = Some(Instant::now() + self.cfg.prompter_timeout);
        self.resident_busy("prompter", true, &text);
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
        self.resident_sids.remove("prompter");
        let opts = SpawnOpts {
            model: self.resident_model("prompter"),
            resume: self.resume_pending.remove("prompter"),
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
        if let Ok(Event::ControlRequest(c)) = parse_line(line)
            && c.request.subtype == "can_use_tool"
        {
            deny_parked(self.prompter_proc.as_mut(), &c.request_id);
            return;
        }
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        self.note_resident("prompter", r.session_id.clone());
        self.role_context("prompter", r.usage.as_ref());
        if let Some(o) = self.prompter.reply(&r.result.unwrap_or_default()) {
            self.finish_prompter(o);
        }
    }

    fn on_prompter_eof(&mut self) {
        self.prompter_proc = None;
        if let Some(o) = self.prompter.fail("exited") {
            self.finish_prompter(o);
        }
    }

    // Ends the prompter wait: its stage row, then the outcome goes to the main child as a turn.
    fn finish_prompter(&mut self, o: PrompterOutcome) {
        if self.prompter_deadline.take().is_some() {
            self.resident_busy("prompter", false, "");
        }
        match o.stage {
            Some(StagePhase::Failed(text)) => self.notice(text),
            Some(phase) => self.stage("prompter", phase),
            None => {}
        }
        self.dispatch(&o.text);
    }

    // Go viaTUI: the main reply goes to the tui child. Failures keep the main reply (the stage row says why).
    fn start_tui(&mut self, reply: String) {
        if self.tui_proc.is_none()
            && let Err(e) = self.spawn_tui()
        {
            self.finish_tui(Err(tf(Msg::Error, &[&e])));
            return;
        }
        self.stage("tui", StagePhase::Working);
        self.tui_deadline = Some(Instant::now() + self.cfg.tui_timeout);
        self.resident_busy("tui", true, &reply);
        let wrote = match self.tui_proc.as_mut() {
            Some(p) => p
                .write_user(&tui_prompt(&reply, crate::i18n::current()))
                .map_err(|e| e.to_string()),
            None => Err("tui not running".to_string()),
        };
        if let Err(e) = wrote {
            self.tui_proc = None;
            self.finish_tui(Err(tf(Msg::Error, &[&format!("write failed: {e}")])));
        }
    }

    // Go residentSlots tui: haiku with the minimal shape and the haiku-output agent body. Kept alive across turns.
    fn spawn_tui(&mut self) -> Result<(), String> {
        self.resident_sids.remove("tui");
        let cwd = self.opts.cwd.clone().unwrap_or_else(|| ".".into());
        let rule = self.agent_rule(&cwd, "haiku-output");
        let opts = SpawnOpts {
            model: self.resident_model("tui"),
            resume: self.resume_pending.remove("tui"),
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
    // waiter (MCP calls only) gets the reply or the failure reason.
    fn start_coder(&mut self, task: String, waiter: Option<Sender<Result<String, String>>>) {
        let task = task.trim().to_string();
        if task.is_empty() {
            self.emit(UiEvent::Error("task is required".to_string()));
            answer(waiter, Err("task is required".to_string()));
            return;
        }
        let extra = self.cfg.extra_coders.min(MAX_EXTRA_CODERS);
        let idle = (0..=extra).find(|&i| self.coders.get(i).is_none_or(|c| c.deadline.is_none()));
        let Some(i) = idle else {
            self.stage(
                "coder",
                StagePhase::Failed("all resident coders are busy".into()),
            );
            answer(waiter, Err("all resident coders are busy".to_string()));
            return;
        };
        let role = coder_role(i);
        if self.coders.len() <= i {
            self.coders.resize_with(i + 1, Coder::default);
        }
        if self.coders[i].proc.is_none()
            && let Err(e) = self.spawn_coder(i)
        {
            self.stage(
                &role,
                StagePhase::Failed(format!("coder spawn failed: {e}")),
            );
            answer(waiter, Err(format!("coder spawn failed: {e}")));
            return;
        }
        self.stage(&role, StagePhase::Working);
        self.coders[i].deadline = Some(Instant::now() + CODER_TIMEOUT);
        self.resident_busy(&role, true, &task);
        self.coders[i].idle_since = None;
        self.coders[i].waiter = waiter;
        let text = format!("{task}{CODER_REPLY_FORMAT}");
        self.coders[i].task = Some(text.clone());
        let wrote = match self.coders[i].proc.as_mut() {
            Some(p) => p.write_user(&text).map_err(|e| e.to_string()),
            None => Err("coder not running".to_string()),
        };
        if let Err(e) = wrote {
            self.drop_coder(i);
            self.finish_coder(i, Err(format!("coder write failed: {e}")));
        }
    }

    // Go spawnResident: after the main child starts, the prompter, tui and base coder start and stay parked,
    // in slot-table order (main is already up). A failed spawn is left to its first use, which reports it.
    fn spawn_residents(&mut self) {
        if self.coders.is_empty() {
            self.coders.resize_with(1, Coder::default);
        }
        let roles: Vec<&'static str> = self.resident_slots.iter().map(|(r, _)| *r).collect();
        for role in roles {
            let _ = match role {
                "prompter" => self.spawn_prompter(),
                "tui" => self.spawn_tui(),
                "coder" => self.spawn_coder(0),
                _ => Ok(()),
            };
        }
    }

    // Go residentSlots model of a role. A role outside the table (coder2..) is haiku, as Go coderSlot.
    fn resident_model(&self, role: &str) -> String {
        self.resident_slots
            .iter()
            .find(|(r, _)| *r == role)
            .map_or_else(|| "haiku".to_string(), |(_, m)| m.clone())
    }

    // Go residentSlots coder: haiku with the haiku-coder agent body, the coder tools, strict MCP and no slash commands.
    fn spawn_coder(&mut self, i: usize) -> Result<(), String> {
        self.resident_sids.remove(&coder_role(i));
        let cwd = self.opts.cwd.clone().unwrap_or_else(|| ".".into());
        let rule = self.agent_rule(&cwd, "haiku-coder");
        let opts = SpawnOpts {
            model: self.resident_model(&coder_role(i)),
            resume: self.resume_pending.remove(&coder_role(i)),
            max_budget_usd: 0.0,
            shape: Shape {
                minimal: false,
                tools: CODER_TOOLS.to_string(),
                strict_mcp: true,
                no_slash: true,
                rule,
                mcp_config: None,
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
        // The idle timers run from spawn: the base coder starts at boot, before any task.
        self.coders[i].idle_since = Some(Instant::now());
        self.emit_coder_contexts();
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
        self.coders[i].idle_since = None;
        self.emit_coder_contexts();
    }

    // Records a session id this run started (Go noteOwn), once.
    fn note_own(&mut self, id: &str) {
        if !self.owned.iter().any(|o| o == id) {
            self.owned.push(id.to_string());
        }
    }

    // Keeps the session id a resident result line carries, for its role; owned too.
    fn note_resident(&mut self, role: &str, session_id: Option<String>) {
        if let Some(id) = session_id.filter(|id| !id.is_empty()) {
            self.note_own(&id);
            if self.resident_sids.get(role) != Some(&id) {
                self.record_session(role, &id);
            }
            self.resident_sids.insert(role.to_string(), id);
            self.emit_session_ids();
        }
    }

    // Appends a changed session id to the record folder's session log (/resume lists it). Only with a handoff path.
    fn record_session(&self, role: &str, session_id: &str) {
        if let Some(path) = &self.cfg.handoff_path {
            let _ = handoff::append_session(path, role, session_id);
        }
    }

    // Go /view: this run's session ids, sent whenever the main or a resident id changes.
    fn emit_session_ids(&self) {
        self.emit(UiEvent::SessionIds {
            main: self.sid.clone(),
            residents: self.resident_sids.clone(),
            owned: self.owned.clone(),
        });
        self.persist_live();
    }

    // Rewrites the live marker with this run's ids and ctx, so a force kill leaves the latest state behind.
    // Only with a handoff path (the TUI sets one).
    fn persist_live(&self) {
        if let Some(path) = &self.cfg.handoff_path {
            // No session id yet (startup, after /clear): write_live_marker writes nothing, never the shared name.
            let _ = handoff::write_live_marker(path, &self.sid, &self.exit_handoff());
        }
    }

    // Tokens in the role's context window from its result usage (Go per-role ctx). Nothing without usage.
    fn role_context(&mut self, role: &str, usage: Option<&Usage>) {
        if let Some(u) = usage {
            let tokens = u.context_tokens();
            self.role_ctx.insert(role.to_string(), tokens);
            self.emit(UiEvent::RoleContext {
                role: role.to_string(),
                tokens,
            });
            if role.starts_with("coder") {
                self.emit_coder_contexts();
            }
            self.persist_live();
        }
    }

    // The coder line: every live coder child in spawn order (slot index order), with its context tokens (0 until
    // its first result) and its rotation limit (the threshold its model rotates at, as the Go per-role limit).
    fn emit_coder_contexts(&self) {
        let coders = self
            .coders
            .iter()
            .enumerate()
            .filter(|(_, c)| c.proc.is_some())
            .map(|(i, _)| {
                let role = coder_role(i);
                CoderContext {
                    slot: i + 1,
                    tokens: self.role_ctx.get(&role).copied().unwrap_or(0),
                    limit: threshold_for(&self.resident_model(&role), self.cfg.rotate_threshold),
                }
            })
            .collect();
        self.emit(UiEvent::CoderContexts { coders });
    }

    // This run's session ids for the handoff (Go handoffSnapshot). A resident counts only while its child runs.
    fn exit_handoff(&self) -> ExitHandoff {
        let mut roles = vec![
            ("prompter".to_string(), self.prompter_proc.is_some()),
            ("tui".to_string(), self.tui_proc.is_some()),
        ];
        roles.extend(
            self.coders
                .iter()
                .enumerate()
                .map(|(i, c)| (coder_role(i), c.proc.is_some())),
        );
        let resident_sessions: BTreeMap<String, String> = roles
            .into_iter()
            .filter(|(_, alive)| *alive)
            .filter_map(|(role, _)| self.resident_sids.get(&role).map(|id| (role, id.clone())))
            .collect();
        let mut ctx_tokens: BTreeMap<String, u64> = self
            .role_ctx
            .iter()
            .filter(|(role, _)| resident_sessions.contains_key(*role))
            .map(|(role, tokens)| (role.clone(), *tokens))
            .collect();
        if !self.sid.is_empty() {
            ctx_tokens.insert(handoff::MAIN_ROLE.to_string(), self.ctx_tokens);
        }
        // Live children by role (Go child_pid), so a dead harness's leftovers can be found at the next start.
        // Go residentSlots: every resident runs haiku; the main child runs the session model.
        let child_models: BTreeMap<String, String> = self
            .proc
            .iter()
            .map(|_| (handoff::MAIN_ROLE.to_string(), self.model.clone()))
            .chain(
                self.prompter_proc
                    .iter()
                    .map(|_| ("prompter".to_string(), "haiku".to_string())),
            )
            .chain(
                self.tui_proc
                    .iter()
                    .map(|_| ("tui".to_string(), "haiku".to_string())),
            )
            .chain(self.coders.iter().enumerate().filter_map(|(i, c)| {
                c.proc
                    .as_ref()
                    .map(|_| (coder_role(i), "haiku".to_string()))
            }))
            .collect();
        let child_pids: BTreeMap<String, u32> = self
            .proc
            .iter()
            .map(|p| (handoff::MAIN_ROLE.to_string(), p.pid()))
            .chain(
                self.prompter_proc
                    .iter()
                    .map(|p| ("prompter".to_string(), p.pid())),
            )
            .chain(self.tui_proc.iter().map(|p| ("tui".to_string(), p.pid())))
            .chain(
                self.coders
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.proc.as_ref().map(|p| (coder_role(i), p.pid()))),
            )
            .collect();
        ExitHandoff {
            cwd: self
                .opts
                .cwd
                .as_deref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            main_session: self.sid.clone(),
            resident_sessions,
            owned_sessions: self.owned.clone(),
            ctx_tokens,
            owner_pid: std::process::id(),
            child_pids,
            child_models,
            ..Default::default()
        }
    }

    // Ends an idle resident child with ClaudeProc::finish. Its session id stays in resume_pending, so the next
    // task respawns it with --resume on the same session.
    fn exit_idle_coder(&mut self, i: usize) {
        if let Some(mut p) = self.coders[i].proc.take() {
            p.finish(CODER_EXIT_WAIT);
        }
        let role = coder_role(i);
        if let Some(sid) = self.resident_sids.get(&role).cloned() {
            self.resume_pending.insert(role, sid);
        }
        self.coder_epoch += 1;
        self.coders[i].epoch = self.coder_epoch;
        self.coders[i].idle_since = None;
        self.emit_coder_contexts();
    }

    // The coder's result line is its reply. An empty reply is a failure, as for the tui.
    fn on_coder_line(&mut self, i: usize, line: &str) {
        if let Ok(Event::ControlRequest(c)) = parse_line(line)
            && c.request.subtype == "can_use_tool"
        {
            self.coder_permission(i, c);
            return;
        }
        if self.coders[i].deadline.is_none() {
            return;
        }
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        self.note_resident(&coder_role(i), r.session_id.clone());
        self.role_context(&coder_role(i), r.usage.as_ref());
        let raw = r.result.unwrap_or_default();
        if r.is_error == Some(true) && retryable_api_error(&raw) {
            if !self.coder_retry(i) {
                self.finish_coder(i, Err(raw));
            }
            return;
        }
        let text = summarize_reply(&raw);
        if text.is_empty() {
            self.finish_coder(i, Err("coder empty reply".to_string()));
        } else {
            self.finish_coder(i, Ok(text));
        }
    }

    // Go parkedIdleEvent: a resident coder's can_use_tool is approved only while it is busy (coder_may_use);
    // every other request is denied with the Go parked note.
    fn coder_permission(&mut self, i: usize, c: crate::event::ControlRequest) {
        let tool = c.request.tool_name.unwrap_or_default();
        let input = c.request.input.unwrap_or(Value::Null);
        let busy = self.coders[i].deadline.is_some();
        let allow = coder_may_use(busy, self.opts.cwd.as_deref(), &tool, &input);
        let res = match self.coders[i].proc.as_mut() {
            Some(p) if allow => p.write_permission(&c.request_id, true, input, ""),
            Some(p) => p.write_permission(
                &c.request_id,
                false,
                Value::Null,
                "denied: session parked (another model is active)",
            ),
            None => Ok(()),
        };
        if let Err(e) = res {
            self.emit(UiEvent::Error(format!("permission reply failed: {e}")));
        }
    }

    fn on_coder_eof(&mut self, i: usize) {
        self.coders[i].proc = None;
        self.emit_coder_contexts();
        if self.coders[i].deadline.is_some() && !self.coder_retry(i) {
            self.finish_coder(i, Err("coder exited".to_string()));
        }
    }

    // Ends a coder task: its stage row, then the reply (or the failure reason) goes to the UI.
    fn finish_coder(&mut self, i: usize, out: Result<String, String>) {
        self.coders[i].task = None;
        self.coders[i].retries = 0;
        self.coders[i].retry_started = None;
        self.coders[i].retry_at = None;
        if self.coders[i].deadline.take().is_some() {
            self.resident_busy(&coder_role(i), false, "");
        }
        self.coders[i].idle_since = Some(Instant::now());
        let waiter = self.coders[i].waiter.take();
        let role = coder_role(i);
        answer(waiter, out.clone());
        // Every turn end is a handback: a failed task still sends its reason as the reply, so main never sees silence.
        match out {
            Ok(text) => {
                self.stage(&role, StagePhase::Done);
                self.emit(UiEvent::CoderReply { role, text });
            }
            Err(reason) => {
                self.emit(UiEvent::CoderReply {
                    role: role.clone(),
                    text: tf(Msg::Error, &[&reason]),
                });
                self.stage(&role, StagePhase::Failed(reason));
            }
        }
    }

    // The tui's result line is its answer. An empty answer is a failure (Go empty reply).
    fn on_tui_line(&mut self, line: &str) {
        if let Ok(Event::ControlRequest(c)) = parse_line(line)
            && c.request.subtype == "can_use_tool"
        {
            deny_parked(self.tui_proc.as_mut(), &c.request_id);
            return;
        }
        if self.tui_deadline.is_none() {
            return;
        }
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        self.note_resident("tui", r.session_id.clone());
        self.role_context("tui", r.usage.as_ref());
        let text = r.result.unwrap_or_default().trim().to_string();
        if text.is_empty() {
            self.finish_tui(Err(t(Msg::EmptyResponse).to_string()));
        } else {
            self.finish_tui(Ok(text));
        }
    }

    fn on_tui_eof(&mut self) {
        self.tui_proc = None;
        if self.tui_deadline.is_some() {
            self.finish_tui(Err(tf(Msg::Error, &[&"exited"])));
        }
    }

    // With residents the main text waits for the tui answer. Text written before a tool call is the work plan: it is
    // shown when that tool call arrives, so it stays above the tools and the final reply still comes from the tui.
    fn release_routed_text(&mut self) {
        if self.cfg.residents && self.busy && self.handoff.is_none() {
            let text = std::mem::take(&mut self.routed_text);
            if !text.is_empty() {
                self.emit(UiEvent::TextDelta {
                    role: Role::Main,
                    text,
                });
                self.emit(UiEvent::MessageDone { role: Role::Main });
            }
        }
    }

    // Ends the tui wait: the answer (or its failure row), then the turn is over.
    fn finish_tui(&mut self, out: Result<String, String>) {
        if self.tui_deadline.take().is_some() {
            self.resident_busy("tui", false, "");
        }
        let raw = std::mem::take(&mut self.routed_text);
        match out {
            Ok(text) => {
                self.stage("tui", StagePhase::Done);
                self.emit(UiEvent::TuiAnswer { text });
            }
            Err(detail) => {
                // Go viaTUI: on failure the raw main reply is shown, then the notice.
                if !raw.is_empty() {
                    self.emit(UiEvent::TextDelta {
                        role: Role::Main,
                        text: raw,
                    });
                    self.emit(UiEvent::MessageDone { role: Role::Main });
                }
                self.notice(tui_notice(&detail));
            }
        }
        self.after_turn();
    }

    // Writes one user frame and starts the turn clock. Returns false if the write failed.
    fn send_turn(&mut self, text: &str) -> bool {
        // A release continuation keeps the task list; any other prompt starts a fresh one.
        if !std::mem::take(&mut self.continuation) && !self.queued_tasks.is_empty() {
            self.queued_tasks.clear();
            self.persist_tasks();
        }
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
        // Go runTurn: queued notices go in front of the prompt; the pending snapshot of a rotated session goes in
        // front of the first prompt sent to it.
        let body = with_notices(&std::mem::take(&mut self.notices), text);
        let frame = if self.pending_snapshot.is_empty() {
            body
        } else {
            format!(
                "{}\n\n## Current user message\n{}",
                self.pending_snapshot, body
            )
        };
        match p.write_user(&frame) {
            Ok(()) => {
                self.pending_snapshot.clear();
                self.turn_prompt = text.to_string();
                self.turn_reply.clear();
                self.routed_text.clear();
                self.mark = MarkerFilter::default();
                self.turn_started = Some(Instant::now());
                self.stage("main", StagePhase::Working);
                self.last_activity = Instant::now();
                self.turn_cost_base = self.cost_usd;
                self.delegations = 0;
                self.spend = TurnSpend::new();
                self.turn_msg_usage.clear();
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
        let ttl = effective_ttl(&self.ttl_seen, &self.cfg.cache_ttl);
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
        // A held reply has no child turn to interrupt: the interrupt ends the wait and the held turn ends.
        if self.held.is_some() {
            self.cancel_wait();
            return;
        }
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
        self.emit_queue();
        if self.busy {
            self.busy = false;
            self.turn_started = None;
            self.emit(UiEvent::Busy(false));
        }
    }

    fn on_line(&mut self, line: &str) {
        self.last_activity = Instant::now();
        if let Some(t) = observe_ttl(line, &self.ttl_seen) {
            self.ttl_seen = t;
        }
        if let Ok(raw) = serde_json::from_str::<Value>(line) {
            self.spend.on_event(&raw);
            self.on_sub_agent_line(&raw);
        }
        match parse_line(line) {
            Ok(ev) => self.on_event(ev),
            Err(_) => self.log_unparsed(line),
        }
    }

    // Go repl.go _unparsed -> Transcript.add("unparsed"): trace only, the turn is not affected. Write errors are ignored.
    fn log_unparsed(&self, line: &str) {
        self.log_transcript("unparsed", json!({"raw": line}));
    }

    // Go Transcript.add: one JSONL row {"t", "kind", ...fields} appended to the transcript. Write errors are ignored.
    fn log_transcript(&self, kind: &str, fields: Value) {
        let Some(path) = &self.cfg.transcript else {
            return;
        };
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let mut row = json!({"t": t, "kind": kind});
        if let (Some(m), Value::Object(f)) = (row.as_object_mut(), fields) {
            m.extend(f);
        }
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(format!("{row}\n").as_bytes()));
    }

    fn on_event(&mut self, ev: Event) {
        match ev {
            Event::System(s) => {
                self.on_system_task(&s);
                if let Some(id) = s.session_id {
                    self.note_own(&id);
                    let prev = std::mem::replace(&mut self.sid, id.clone());
                    if prev != id {
                        self.record_session(handoff::MAIN_ROLE, &id);
                        // A rotation clears sid first, so a non-empty previous id here means a /clear.
                        if !prev.is_empty() {
                            self.emit(UiEvent::NewSession { old_session: prev });
                        }
                        self.emit_session_ids();
                    }
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
                if matches!(role, Role::Main) {
                    self.note_held_turn();
                }
                match w.event {
                    StreamInner::ContentBlockDelta {
                        delta: Delta::TextDelta { text },
                    } => {
                        if matches!(role, Role::Main) && !self.busy {
                            self.idle_started();
                        }
                        // The raw reply keeps the [cat: X] marker for category_rotation; the screen gets it filtered.
                        let shown = if matches!(role, Role::Main) {
                            self.turn_reply.push_str(&text);
                            self.mark.feed(&text)
                        } else {
                            text
                        };
                        // With residents the pane shows only the tui answer (Go runTurn routed): hold main text back.
                        // An idle reply (agent handback) has no tui wait to drain it, so it shows at once (Go idleText).
                        if self.cfg.residents && self.busy && matches!(role, Role::Main) {
                            self.routed_text.push_str(&shown);
                        } else if self.handoff.is_none() && !shown.is_empty() {
                            self.emit(UiEvent::TextDelta { role, text: shown });
                        }
                    }
                    StreamInner::MessageStop if self.handoff.is_none() => {
                        if matches!(role, Role::Main) {
                            let rest = self.mark.flush();
                            if self.cfg.residents && self.busy {
                                self.routed_text.push_str(&rest);
                            } else if !rest.is_empty() {
                                self.emit(UiEvent::TextDelta {
                                    role: role.clone(),
                                    text: rest,
                                });
                            }
                        }
                        self.emit(UiEvent::MessageDone { role })
                    }
                    _ => {}
                }
            }
            Event::Assistant(env) => {
                // Subagent messages are not surfaced beyond their text in E1; they only refresh their agent's activity.
                if let Some(pid) = &env.parent_tool_use_id {
                    self.note_agent_activity(pid);
                    return;
                }
                self.note_held_turn();
                if !self.busy {
                    self.idle_started();
                }
                // The server-reported model of the reply: the label follows it when it differs from the init model.
                if let Some(m) = env
                    .message
                    .model
                    .clone()
                    .filter(|m| !m.is_empty() && *m != self.model)
                {
                    self.model = m;
                    self.emit_status(self.cost_usd);
                }
                if let (Some(id), Some(u)) = (&env.message.id, &env.message.usage) {
                    self.turn_msg_usage
                        .entry(id.clone())
                        .or_default()
                        .max_with(u);
                }
                if let Some(u) = &env.message.usage {
                    self.ctx_tokens = u.context_tokens();
                    self.persist_live();
                    self.last_api = Some(Instant::now());
                    self.budget_mid_turn(u);
                    self.emit_status(self.cost_usd);
                }
                if let Some(Content::Blocks(blocks)) = env.message.content {
                    if blocks.iter().any(|b| matches!(b, Block::ToolUse { .. })) {
                        self.release_routed_text();
                    }
                    for b in blocks {
                        if let Block::ToolUse { id, name, input } = b {
                            self.on_tool_use(id, name, input.unwrap_or(Value::Null));
                        }
                    }
                }
            }
            Event::User(env) => {
                if let Some(pid) = &env.parent_tool_use_id {
                    self.note_agent_activity(pid);
                    return;
                }
                self.note_held_turn();
                if let Some(Content::Text(text)) = &env.message.content {
                    self.on_user_notice(text);
                }
                if let Some(Content::Blocks(blocks)) = env.message.content {
                    // The top-level tool_use_result belongs to the single tool result of its message.
                    let single = blocks
                        .iter()
                        .filter(|b| matches!(b, Block::ToolResult { .. }))
                        .count()
                        == 1;
                    let stats = env.tool_use_result.as_ref().and_then(tool_stats);
                    for b in blocks {
                        match b {
                            Block::Text { text } => self.on_user_notice(&text),
                            Block::ToolResult {
                                tool_use_id,
                                content,
                                is_error,
                            } => {
                                if let (true, Some(s)) = (single, stats) {
                                    self.record_task_stats(&tool_use_id, s);
                                }
                                self.on_tool_result(tool_use_id, content, is_error == Some(true))
                            }
                            _ => {}
                        }
                    }
                }
            }
            Event::Result(r) => {
                if self.stray_results > 0 {
                    self.stray_results -= 1;
                    return;
                }
                self.result_usage = r.usage.clone();
                self.result_model_usage = r.model_usage.clone();
                if !self.busy {
                    self.idle_result();
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
                self.poll_usage();
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
                    // Go askPerm: with a UI the ask goes to the UI whether or not a turn runs. Without one
                    // (plain mode) an idle ask is denied with the Go note.
                    Verdict::Ask if self.cfg.auto_allow => {
                        self.respond_permission(&c.request_id, true, input, "")
                    }
                    Verdict::Ask if self.cfg.no_ui && !self.busy => {
                        let note = format!(
                            "[permission denied while idle] {tool} {} (use /mode auto or acceptEdits)",
                            crate::import_view::trunc(&input.to_string(), 120)
                        );
                        self.emit(UiEvent::Notice { text: note });
                        self.respond_permission(
                            &c.request_id,
                            false,
                            Value::Null,
                            "denied: no interactive prompt available between turns",
                        );
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
                    self.rate_log.set_rate(&info, &self.metrics);
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
        // Go repl.go:743: an edit by a Haiku session that has a session id joins the worker pool.
        if self.model == "haiku"
            && !self.sid.is_empty()
            && matches!(name.as_str(), "Write" | "Edit" | "NotebookEdit")
            && let Some(fp) = input.get("file_path").and_then(Value::as_str)
        {
            self.pool.note_edit(&self.sid, &self.cur_task, fp);
        }
        self.open_tools.insert(id.clone());
        // Go repl.go:721: an Agent call's "· agent <label> 호출" row comes before its tool line.
        if name == "Agent" || name == "Task" {
            let label = subagent_label(&input);
            self.stage(&agent_slot(&label), StagePhase::Working);
            self.agent_types.insert(id.clone(), label);
        }
        let summary = tool_summary(&input);
        self.emit(UiEvent::ToolStart {
            id: id.clone(),
            name: name.clone(),
            summary,
        });
        self.emit(UiEvent::ToolInput {
            id: id.clone(),
            input: input.clone(),
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
            // Go drain.go opusRunning: Name is `name`, else `subagent_type`.
            let agent_name = ["name", "subagent_type"].iter().find_map(|k| {
                input
                    .get(*k)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
            });
            if input.get("model").and_then(Value::as_str) == Some("opus")
                || agent_name.is_some_and(|n| n.starts_with("opus-"))
            {
                self.opus_set(&id, true);
            }
            self.agents.insert(id.clone(), Instant::now());
            self.agent_labels.insert(id.clone(), description.clone());
            self.queue_task(id.clone(), description.clone());
            let started_at = now_secs();
            self.agent_info.insert(
                id.clone(),
                AgentInfo {
                    name: description.clone(),
                    // No result file is written for an agent, so none is advertised.
                    result_path: String::new(),
                    background: input
                        .get("run_in_background")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    started: Instant::now(),
                    done: false,
                    result_done: false,
                    out_path: input
                        .get("prompt")
                        .and_then(Value::as_str)
                        .map(out_path_in)
                        .unwrap_or_default(),
                    model: input
                        .get("model")
                        .and_then(Value::as_str)
                        .filter(|m| !m.is_empty())
                        .map(str::to_string)
                        .or_else(|| agent_type_model(agent_name.unwrap_or_default()))
                        .unwrap_or_default(),
                    task: input
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    prompt: input
                        .get("prompt")
                        .and_then(Value::as_str)
                        .and_then(|p| p.lines().next())
                        .unwrap_or_default()
                        .to_string(),
                    brief: input
                        .get("prompt")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    launch_epoch: self.epoch,
                    launch_seq: self.turn_seq + 1,
                    session_name: ["name", "subagent_type"]
                        .iter()
                        .find_map(|k| {
                            input
                                .get(*k)
                                .and_then(Value::as_str)
                                .filter(|s| !s.is_empty())
                        })
                        .unwrap_or_default()
                        .to_string(),
                    agent_id: String::new(),
                    started_at,
                    last_active: started_at,
                    log: Vec::new(),
                    last_text: String::new(),
                    handback: String::new(),
                    report: String::new(),
                    failed: false,
                },
            );
            self.emit(UiEvent::AgentStart {
                id: id.clone(),
                description,
                started_at,
            });
            self.persist_agent(&id, STO_RUNNING);
            self.emit_agent_rows();
        }
    }

    fn on_tool_result(&mut self, id: String, content: Option<Value>, is_error: bool) {
        self.open_tools.remove(&id);
        let text = result_text(content.as_ref());
        // SendMessage to a background agent resumes it: it runs again under its original tool id.
        // The SendMessage result is JSON: {"success":true,"message":"Resuming agent <short>","resumedAgentId":"<full>"}.
        // A queued message ("Message queued for delivery to <id>", no resumedAgentId) opens the same row:
        // the agent is still running and its next notice closes it.
        let resume = serde_json::from_str::<Value>(&text)
            .ok()
            .filter(|v| v["success"] != false)
            .and_then(|v| {
                v["resumedAgentId"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| {
                        v["message"]
                            .as_str()
                            .filter(|m| m.starts_with("Message queued for delivery to"))
                            .and_then(|_| v["pin"]["id"].as_str().map(str::to_string))
                    })
            });
        // agent_info stays after the agent ends, so a resume uses its start label, then the agent id.
        // An agent launched outside this process (earlier run or a sub-agent) has no launch id here: its resumed id keys the row.
        let known = resume.map(|aid| {
            let tool = self
                .agent_ids
                .get(&aid)
                .cloned()
                .unwrap_or_else(|| aid.clone());
            (tool, aid)
        });
        if let Some((tool, aid)) = known {
            let description = self
                .agent_info
                .get(&tool)
                .map(|i| i.name.clone())
                .or_else(|| self.agent_labels.get(&tool).cloned())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("agent {}", aid.chars().take(8).collect::<String>()));
            // A queued message to a still-running agent keeps its running start time.
            self.agents.entry(tool.clone()).or_insert_with(Instant::now);
            self.resume_ids.insert(id.clone(), tool.clone());
            // The resumed agent runs in the current child now. An older drain must not settle it when it closes
            // (that wrote a 타임아웃 notice and ended the live agent while it kept working).
            for d in self.drains.iter_mut() {
                d.started.retain(|s| s != &tool);
            }
            // A resumed agent's next finish is announced again.
            self.announced.remove(&tool);
            if let Some(info) = self.agent_info.get_mut(&tool) {
                info.background = true;
                info.done = false;
            }
            self.persist_agent(&tool, STO_RUNNING);
            self.emit_tool_result(id.clone(), text);
            // The SendMessage call's row stays open as the resumed agent's row: no ToolEnd for it here.
            self.emit(UiEvent::AgentResumed {
                id: tool,
                description,
                started_at: now_secs(),
                send_id: id,
            });
            self.emit_agent_rows();
            return;
        }
        self.emit(UiEvent::ToolEnd {
            id: id.clone(),
            ok: !is_error,
        });
        if is_error && let Some(info) = self.agent_info.get_mut(&id) {
            info.failed = true;
        }
        // A handback launch result has "agentId: <id>" without the async mark: map it too, so a later SendMessage resolves to this row.
        let launched_aid =
            after_marker(&text, "agentId: ").filter(|_| self.agent_info.contains_key(&id));
        if let Some(aid) = launched_aid {
            self.agent_ids.insert(aid.clone(), id.clone());
            if let Some(info) = self.agent_info.get_mut(&id) {
                info.agent_id = aid;
            }
        }
        // A background agent keeps running after its launch result; it ends later, which E1 does not track.
        if text.contains(ASYNC_AGENT_MARK) {
            if let Some(info) = self.agent_info.get_mut(&id) {
                info.background = true;
            }
            self.settle_task(&id, TaskStatus::Running);
            self.emit_tool_result(id.clone(), text);
            self.emit(UiEvent::AgentBackground { id });
            self.emit_agent_rows();
            return;
        }
        if self.agents.contains_key(&id) {
            self.finish_agent(id.clone());
            self.emit_tool_result(id, text);
        } else {
            self.emit_tool_result(id.clone(), text);
            self.finish_agent(id);
        }
    }

    fn emit_tool_result(&self, id: String, text: String) {
        self.emit(UiEvent::ToolResult { id, text });
    }

    // Ends a delegated agent: AgentEnd, and the deferred rotation runs if this was the last agent.
    // Go agentTracker rows: every delegated agent in start order, for the panel and background lines.
    // Output or tool activity from a delegated agent's own messages refreshes its last_active in place. The panel
    // re-reads the shared rows on its tick, so no event is sent per message.
    fn note_agent_activity(&mut self, id: &str) {
        let now = now_secs();
        let Some(info) = self.agent_info.get_mut(id) else {
            return;
        };
        info.last_active = now;
        let mut rows = self.agent_rows.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
            row.last_active = now;
        }
    }

    /// A new run for an agent that already ended (resumed) shows its row as running again.
    fn reopen_agent(&mut self, tool: String) {
        self.agents.entry(tool.clone()).or_insert_with(Instant::now);
        for d in self.drains.iter_mut() {
            d.started.retain(|s| s != &tool);
        }
        self.announced.remove(&tool);
        if let Some(info) = self.agent_info.get_mut(&tool) {
            info.background = true;
            info.done = false;
        }
        self.persist_agent(&tool, STO_RUNNING);
        self.emit_agent_rows();
    }

    fn emit_agent_rows(&self) {
        self.emit_agent_rows_at(self.busy);
    }

    // busy: the state the picked child's row shows (running while busy, done otherwise).
    fn emit_agent_rows_at(&self, busy: bool) {
        let mut rows: Vec<(Instant, AgentRow)> = self
            .agent_info
            .iter()
            .map(|(id, i)| {
                (
                    i.started,
                    AgentRow {
                        id: id.clone(),
                        task: i.task.clone(),
                        prompt: i.prompt.clone(),
                        name: i.session_name.clone(),
                        model: i.model.clone(),
                        background: i.background,
                        done: i.done,
                        failed: i.failed,
                        interrupted: false,
                        agent_id: i.agent_id.clone(),
                        result_path: i.result_path.clone(),
                        started_at: i.started_at,
                        last_active: i.last_active,
                    },
                )
            })
            .collect();
        rows.sort_by_key(|(started, _)| *started);
        let rows: Vec<AgentRow> = rows.into_iter().map(|(_, r)| r).collect();
        let rows = with_picked(rows, self.picked_row.as_ref(), busy);
        *self.agent_rows.lock().unwrap_or_else(|e| e.into_inner()) = rows.clone();
        self.emit(UiEvent::AgentRows { rows });
    }

    // Go restoreAgents (agentstore.go:221) for /resume: the stored agents of the imported session are listed
    // below the running rows until the next user line. A fresh start restores none; the records stay on disk.
    fn restore_agents_for(&mut self, sid: &str) {
        let saved = self
            .agent_store
            .as_ref()
            .map(|s| s.load(sid))
            .unwrap_or_default();
        let (rows, events) = restored_events(&saved);
        self.restored_rows = rows;
        for ev in events {
            self.emit(ev);
        }
    }

    // The agent's SubagentHandback message (its final report) is kept for the done line. Both the current child and a
    // drained child can carry it (a rotation may leave the agent on the drained child).
    fn record_handback(&mut self, raw: &Value) {
        let Some(msg) = handback_message(raw) else {
            return;
        };
        let Some(p) = raw
            .get("parent_tool_use_id")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
        else {
            return;
        };
        if let Some(info) = self.agent_info.get_mut(p) {
            info.handback = msg;
        }
    }

    // Go onSubEvent (repl.go:426, 686): a delegated agent's text and tool lines go to its agent-view log.
    fn on_sub_agent_line(&mut self, raw: &Value) {
        self.record_handback(raw);
        let Some(p) = raw
            .get("parent_tool_use_id")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
        else {
            return;
        };
        let Some(info) = self.agent_info.get_mut(p) else {
            return;
        };
        // The agent's own model is the one its stream reports; it overrides the input or type-prefix guess.
        // A change re-emits the panel rows, so the running row shows the model its done row will.
        let model = stream_model(raw);
        let model_changed = model.as_ref().is_some_and(|m| *m != info.model);
        if let Some(m) = model {
            info.model = m;
        }
        let (lines, tool_ids): (Vec<String>, Vec<Option<String>>) =
            sub_event_entries(raw).into_iter().unzip();
        if lines.is_empty() {
            if model_changed {
                self.emit_agent_rows();
            }
            return;
        }
        // Go lastSummary: the newest plain text line of this batch, if any, is the agent's last text.
        let last = last_summary(&lines);
        if !last.is_empty() {
            info.last_text = last;
        }
        // Kept with this arrival second, so a restored agent view shows when each line came.
        let at = now_secs();
        info.log.extend(lines.iter().map(|text| SavedLine {
            at,
            text: text.clone(),
        }));
        let excess = info.log.len().saturating_sub(MAX_AGENT_LOG);
        info.log.drain(..excess);
        self.emit(UiEvent::AgentLog {
            id: p.to_string(),
            lines,
            tool_ids,
        });
        if model_changed {
            self.emit_agent_rows();
        }
    }

    // Go agentTracker persist: writes one delegated agent to the agents file (no-op without one).
    fn persist_agent(&self, id: &str, status: &str) {
        let (Some(store), Some(info)) = (&self.agent_store, self.agent_info.get(id)) else {
            return;
        };
        let status = saved_status(status, info.failed);
        let mut s = SavedAgent::new(
            id,
            status,
            UNIX_EPOCH + Duration::from_secs(info.started_at),
        );
        // The agent's own model (its stream, else its input or type prefix); never the main session model.
        s.model = info.model.clone();
        s.name = info.session_name.clone();
        s.task = info.task.clone();
        s.prompt = info.prompt.clone();
        // Go persistLocked: the summary the restored agent view shows is the final report, else the last text line.
        s.summary = if info.handback.is_empty() {
            info.last_text.clone()
        } else {
            info.handback.clone()
        };
        s.background = info.background;
        s.agent_id = info.agent_id.clone();
        s.log = info.log.clone();
        // The session the agent runs in: /resume of that session lists it (AgentStore::load matches parent).
        s.parent = self.sid.clone();
        store.put(&[s]);
    }

    // Queued prompt texts, oldest first (Go m.queue), sent with each queue change.
    fn emit_queue(&self) {
        self.emit(UiEvent::QueueTexts(self.queue.iter().cloned().collect()));
    }

    // Go resident.go notice(): a failed prompter or tui pass, sent only while residents run.
    fn notice(&mut self, text: String) {
        if self.cfg.residents {
            self.emit(UiEvent::Notice { text });
        }
    }

    // Go spawnSlot: a missing agent file shows the notice and the slot starts a default session (no rule).
    fn agent_rule(&mut self, cwd: &Path, name: &str) -> String {
        match agent_body(cwd, name, self.cfg.home.as_deref()) {
            Ok(body) => body,
            Err(e) => {
                self.notice(tf(Msg::DefaultSession, &[&e]));
                String::new()
            }
        }
    }

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
        let failed = self.agent_info.get(&id).is_some_and(|i| i.failed);
        self.settle_task(
            &id,
            if failed {
                TaskStatus::Failed
            } else {
                TaskStatus::Done
            },
        );
        self.pending_end.remove(&id);
        self.owned_tasks.retain(|_, parent| *parent != id);
        self.agent_task_ids.retain(|_, tool| *tool != id);
        self.opus_set(&id, false);
        if let Some(t0) = self.agents.remove(&id) {
            let label = self
                .agent_types
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "agent".to_string());
            self.stage(&agent_slot(&label), StagePhase::Done);
            let (name, summary, model) = self
                .agent_info
                .get_mut(&id)
                .map(|i| {
                    i.done = true;
                    (i.session_name.clone(), i.done_what(), i.model.clone())
                })
                .unwrap_or_default();
            self.persist_agent(&id, STO_DONE);
            self.emit(UiEvent::AgentEnd {
                id,
                elapsed_ms: t0.elapsed().as_millis() as u64,
                model,
                name,
                summary,
            });
            self.emit_agent_rows();
        }
        // The last background agent finished while the child is idle: the deferred rotation runs now.
        if !self.busy && self.deferred_due() {
            self.run_deferred_rotation();
        }
    }

    // System task events (stream-json). An owned task (owned_by_subagent) keeps its agent open: its completion
    // ends the agent once no owned task of it runs. An agent-level completion (tool_use_id of a tracked agent)
    // goes through complete_agent. Shared by the main stream and the drained children.
    fn on_system_task(&mut self, s: &SystemEvent) {
        match s.subtype.as_deref() {
            Some("task_started") => {
                if !s.owned_by_subagent {
                    self.seen_task_started = true;
                    if let (Some(tid), Some(tool), Some("local_agent")) =
                        (&s.task_id, &s.tool_use_id, s.task_type.as_deref())
                    {
                        self.agent_task_ids.insert(tid.clone(), tool.clone());
                        // A resumed run replaces the previous run's handback, text, and report, and cancels a pending end.
                        if let Some(info) = self.agent_info.get_mut(tool) {
                            info.handback.clear();
                            info.last_text.clear();
                            info.report.clear();
                            self.pending_end.remove(tool);
                        }
                        // A run that starts for an agent already ended (resumed) reopens its row.
                        if !self.agents.contains_key(tool) && self.agent_info.contains_key(tool) {
                            self.reopen_agent(tool.clone());
                        }
                    }
                } else if let (Some(tid), Some(parent)) = (&s.task_id, &s.parent_task_id) {
                    // Live claude names the parent by the agent's task id; the engine keys agents by tool_use id.
                    let parent = self
                        .agent_task_ids
                        .get(parent)
                        .cloned()
                        .unwrap_or(parent.clone());
                    self.owned_tasks.insert(tid.clone(), parent);
                }
            }
            Some(sub @ ("task_updated" | "task_notification")) => {
                let done = if sub == "task_updated" {
                    s.patch
                        .as_ref()
                        .and_then(|p| p["status"].as_str())
                        .is_some_and(|st| st == "completed" || st == "killed")
                } else {
                    matches!(s.status.as_deref(), Some("completed" | "stopped"))
                };
                if !done {
                    return;
                }
                if let Some(parent) = s.task_id.as_ref().and_then(|t| self.owned_tasks.remove(t)) {
                    // The last owned task gone restarts a pending agent's grace timer; it does not finish the agent.
                    // The agent ends on its own completion, or on grace expiry when no resumed run cancels the pending end.
                    if self.pending_end.contains_key(&parent) && !self.owned_live(&parent) {
                        self.pending_end.insert(parent, Instant::now());
                    }
                } else if sub == "task_notification"
                    && let Some(id) = s.tool_use_id.clone()
                {
                    let id = self.resume_ids.get(&id).cloned().unwrap_or(id);
                    // The agent's own report becomes the body of its "[agent done]" notice.
                    if let Some(text) = s.handback_report.as_ref().and_then(|r| r["text"].as_str())
                        && let Some(info) = self.agent_info.get_mut(&id)
                    {
                        info.report = text.to_string();
                    }
                    self.complete_agent(id);
                }
            }
            _ => {}
        }
    }

    // An agent's completion. It ends now, unless one of its owned tasks still runs: then it is pending until that
    // task completes or agent_end_grace passes (expire_pending_ends). A drained agent settles through its drain.
    fn complete_agent(&mut self, id: String) {
        let id = self.resume_ids.get(&id).cloned().unwrap_or(id);
        if !self.agents.contains_key(&id) {
            self.pending_end.remove(&id);
            return;
        }
        if self.owned_tasks.values().any(|p| *p == id) {
            self.pending_end.entry(id).or_insert_with(Instant::now);
            return;
        }
        if self.drains.iter().any(|d| d.started.contains(&id)) {
            self.settle_drained(&id, true);
        } else {
            // A report of a task that already timed out is not a new reply: its notice waits for the next prompt.
            let late = self
                .queued_tasks
                .iter()
                .any(|t| t.id == id && t.status == TaskStatus::Timeout);
            self.queue_main_notice(&id);
            self.finish_agent(id);
            if !late {
                self.send_notice_turn();
            }
        }
    }

    // A background agent that finished on the main child: its "[agent done]" notice with the final text is queued
    // once per run, unless the native task_notification already carried that text to main. The native body counts
    // as delivered only when this run's SubagentHandback equals the notification's handback_report.
    fn queue_main_notice(&mut self, id: &str) {
        let Some(info) = self.agent_info.get(id) else {
            return;
        };
        if !info.background || (!info.handback.is_empty() && info.report == info.handback) {
            return;
        }
        if !announce_once(&mut self.announced, id) {
            return;
        }
        let body = if info.handback.is_empty() {
            info.last_text.clone()
        } else {
            info.handback.clone()
        };
        self.notices.push(drain_done_notice(
            id,
            &info.task,
            &info.out_path,
            true,
            &self.old_brief(info),
            &body,
        ));
    }

    // The brief of an agent the previous main launched (a rotation came after its launch); empty otherwise.
    fn old_brief(&self, info: &AgentInfo) -> String {
        if info.launch_epoch == self.epoch {
            String::new()
        } else {
            info.brief.clone()
        }
    }

    // Queued notices go out as a turn of their own while the main child is idle. A child-initiated turn in flight
    // answers its own notification, so the notices wait for the next user message instead (no second reply).
    fn send_notice_turn(&mut self) {
        if self.notices.is_empty()
            || self.busy
            || self.hold_notice_turn
            || self.idle_turn.active
            || self.prompter.waiting()
            || self.proc.is_none()
        {
            return;
        }
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        if !self.send_turn("") {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Whether an owned task of this agent still runs.
    fn owned_live(&self, id: &str) -> bool {
        self.owned_tasks.values().any(|p| p == id)
    }

    // A user text (string content or text block) that carries a notice (Go onUserEvent). While no turn runs,
    // a seen notice is recorded for the idle turn (Go idleEvent sawNotif).
    fn on_user_notice(&mut self, text: &str) {
        let idle = !self.busy;
        self.hold_notice_turn = idle;
        let found = self.on_task_notification(text);
        self.hold_notice_turn = false;
        if found && idle {
            self.idle_turn.notif = true;
        }
    }

    // A <task-notification> names the background agents that finished (Go drain.go onNotification).
    // Returns whether it names a tracked agent, finished or not.
    fn on_task_notification(&mut self, text: &str) -> bool {
        let mut found = false;
        for id in task_notification_ids(text) {
            let id = self.resume_ids.get(&id).cloned().unwrap_or(id);
            if self.agent_info.contains_key(&id) {
                found = true;
            }
            self.complete_agent(id);
        }
        found
    }

    // Go repl.go idleFinish: a child-initiated turn ends. Its reply with no visible completion notice means
    // the oldest background agent finished (Go drain.go finishOldestBackground).
    fn idle_started(&mut self) {
        self.idle_turn.active = true;
        self.idle_turn.t0.get_or_insert_with(Instant::now);
    }

    // Go idleFinish: a child-initiated reply ends the oldest background agent unless a notice already named it,
    // and is logged as a background turn (metrics stage=turn, source=background; repl.go:518-575).
    fn idle_result(&mut self) {
        let idle = std::mem::take(&mut self.idle_turn);
        // With a system task_started seen, the agents end on their own events: the reply-ends-oldest fallback is off.
        if idle.active && !idle.notif && !self.seen_task_started {
            self.finish_oldest_background();
        }
        if idle.active {
            let duration_ms = idle.t0.map_or(0, |t| t.elapsed().as_millis() as u64);
            self.log_turn(duration_ms, Some("background"));
        }
    }

    // Go drain.go finishOldestBackground: the longest-running background agent ends.
    fn finish_oldest_background(&mut self) {
        let oldest = self
            .agents
            .iter()
            .filter(|(id, _)| self.agent_info.get(*id).is_some_and(|i| i.background))
            .min_by_key(|(_, t0)| **t0)
            .map(|(id, _)| id.clone());
        if let Some(id) = oldest {
            self.finish_agent(id);
        }
    }

    // Go repl.go turn record (stage "turn"). Keys the Rust engine does not produce yet (draining, rotations)
    // are left out until their groups land. Write errors are ignored, as in Go.
    // Go recordSave: the turn that just ended goes to its project folder under the record root (slash
    // commands and empty replies are skipped by RecordState). A failed write is reported as an Error.
    // Go clearMark: a /clear splits /resume groups at this time, in the record dir of cwd.
    fn clear_mark(&self) {
        let cwd = self.engine_cwd();
        let root = self.record_root(&cwd);
        let cwd_text = cwd.to_string_lossy();
        if let Err(e) = resume_group::clear_mark_now(&root, &cwd_text) {
            self.emit(UiEvent::Error(format!("clear mark failed: {e}")));
        }
    }

    // Go recordProjectName: the cwd is absolute (os.Getwd when unset). A "." fallback would name the
    // record folder "misc" and key /clear marks differently from the absolute cwd /resume reads.
    // Pending main Agent/Task calls: the reply is held while any runs.
    fn running_tasks(&self) -> usize {
        self.queued_tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .count()
    }

    // Registers a main Agent/Task call (on_tool_use). Its settle comes from its result, hand-back, notice or timeout.
    fn queue_task(&mut self, id: String, desc: String) {
        if self.queued_tasks.iter().any(|t| t.id == id) {
            return;
        }
        self.queued_tasks.push(QueuedTask {
            id: id.clone(),
            desc: desc.clone(),
            status: TaskStatus::Running,
            since: Instant::now(),
            tokens: None,
            duration_ms: None,
        });
        self.emit(UiEvent::TaskQueued { id, desc });
        self.persist_tasks();
    }

    // Stores a task's agent totals from its tool result and tells the TUI. Unknown ids are ignored.
    fn record_task_stats(&mut self, id: &str, s: ToolStats) {
        let Some(t) = self.queued_tasks.iter_mut().find(|t| t.id == id) else {
            return;
        };
        t.tokens = s.tokens;
        t.duration_ms = s.duration_ms;
        self.emit(UiEvent::TaskStats {
            id: id.to_string(),
            tokens: s.tokens,
            duration_ms: s.duration_ms,
        });
        self.persist_tasks();
    }

    // Settles a queued task. Running only acknowledges an async launch and keeps the task pending.
    fn settle_task(&mut self, id: &str, status: TaskStatus) {
        let Some(t) = self.queued_tasks.iter_mut().find(|t| t.id == id) else {
            return;
        };
        if t.status != TaskStatus::Running {
            return;
        }
        if status != TaskStatus::Running {
            t.status = status;
            self.persist_tasks();
        }
        self.emit(UiEvent::TaskResolved {
            id: id.to_string(),
            status,
        });
        self.release_held();
    }

    // Main output while a reply is held means a child-initiated turn started (its own hand-back notice).
    fn note_held_turn(&mut self) {
        if self.held.is_some() {
            self.held_turn = true;
        }
    }

    // Ends the hold once no queued task runs: one continuation input carries the results to main.
    // A child turn still running defers the release to its end (end_turn), so no input goes in mid-turn.
    fn release_held(&mut self) {
        if self.held.is_none() || self.running_tasks() > 0 || self.held_turn {
            return;
        }
        self.held = None;
        self.emit(UiEvent::ReplyReleased);
        let rows: Vec<String> = self
            .queued_tasks
            .iter()
            .map(|t| format!("- {} ({}): {}", t.id, t.desc, task_status_name(t.status)))
            .collect();
        let text = format!(
            "The background agents you started have settled:\n{}\nContinue with the reply to the user.",
            rows.join("\n")
        );
        self.continuation = true;
        if !self.send_turn(&text) {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Main's turn ended with queued tasks still running: its final text is held (ReplyHeld), no TurnEnd yet.
    fn hold_reply(&mut self, duration_ms: u64) -> bool {
        let pending = self.running_tasks();
        if !self.cfg.agent_wait
            || pending == 0
            || self.interrupted_at.is_some()
            || self.budget_hit
            || self.handoff.is_some()
        {
            return false;
        }
        self.held = Some((self.turn_reply.clone(), duration_ms));
        self.emit(UiEvent::ReplyHeld { pending });
        true
    }

    // Abandons the wait of a held reply: the running tasks settle as Timeout, then the held text ends the turn.
    fn cancel_wait(&mut self) {
        let Some((reply, duration_ms)) = self.held.take() else {
            return;
        };
        // The held text is the reply; text of a child turn that ran during the hold is not.
        self.turn_reply = reply;
        self.held_turn = false;
        let ids: Vec<String> = self
            .queued_tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            self.settle_task(&id, TaskStatus::Timeout);
        }
        self.finish_turn(duration_ms);
    }

    // The task list of this session: notes/tmp/long/tasks-<session>.json, read back on /resume.
    fn persist_tasks(&self) {
        if self.sid.is_empty() {
            return;
        }
        let dir = crate::tmp_layout::long_dir(&self.engine_cwd());
        let rows: Vec<Value> = self
            .queued_tasks
            .iter()
            .map(|t| {
                json!({"id": t.id, "desc": t.desc, "status": task_status_name(t.status),
                    "tokens": t.tokens, "duration_ms": t.duration_ms})
            })
            .collect();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(
            dir.join(format!("tasks-{}.json", self.sid)),
            Value::Array(rows).to_string(),
        );
    }

    // /resume: the saved task list is shown again. Restored rows are display only, never pending.
    fn load_tasks(&mut self, sid: &str) {
        if sid.is_empty()
            || sid
                .chars()
                .any(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        {
            return;
        }
        let path =
            crate::tmp_layout::long_dir(&self.engine_cwd()).join(format!("tasks-{sid}.json"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&text) else {
            return;
        };
        for r in rows {
            let id = r["id"].as_str().unwrap_or_default().to_string();
            let desc = r["desc"].as_str().unwrap_or_default().to_string();
            let status = task_status_from_name(r["status"].as_str().unwrap_or_default());
            self.emit(UiEvent::TaskQueued {
                id: id.clone(),
                desc,
            });
            self.emit(UiEvent::TaskResolved {
                id: id.clone(),
                status,
            });
            if let Some(s) = saved_stats(&r) {
                self.emit(UiEvent::TaskStats {
                    id,
                    tokens: s.tokens,
                    duration_ms: s.duration_ms,
                });
            }
        }
    }

    fn engine_cwd(&self) -> PathBuf {
        resolve_cwd(self.opts.cwd.clone())
    }

    // Record root: Config.record_dir when set (tests and the CLI override), else Go recordDir for cwd.
    fn record_root(&self, cwd: &Path) -> PathBuf {
        self.cfg
            .record_dir
            .clone()
            .unwrap_or_else(|| record::dir_for(cwd))
    }

    fn record_turn_save(&mut self, duration_ms: u64) {
        let cost_turn = (self.cost_usd - self.turn_cost_base).max(0.0);
        let mut fields = serde_json::Map::new();
        fields.insert("duration_ms".into(), json!(duration_ms));
        fields.insert("cost_turn_usd".into(), json!(cost_turn));
        fields.insert("cost_usd".into(), json!(self.cost_usd));
        fields.insert("ctx".into(), json!(self.ctx_tokens));
        fields.insert("delegations".into(), json!(self.delegations));
        self.record.last_measure = record_turn::measure_line(&fields);
        self.record.last_reply = self.turn_reply.clone();
        let Some(stamp) = local_stamp() else {
            return;
        };
        let cwd = self.engine_cwd();
        let root = self.record_root(&cwd);
        if let Err(e) = self.record.save(&root, &cwd, &self.turn_prompt, &stamp) {
            self.emit(UiEvent::Error(format!("record save failed: {e}")));
        }
    }

    // (input, output, cache) of the running turn: the result usage, else the sum of main message usage. None when zero.
    fn turn_token_counts(&self) -> Option<(u64, u64, u64)> {
        if let Some(c) = self.result_usage.as_ref().and_then(Usage::turn_counts) {
            return Some(c);
        }
        sum_turn_counts(self.turn_msg_usage.values())
    }

    fn log_turn(&mut self, duration_ms: u64, source: Option<&str>) {
        let end_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let cost_turn = (self.cost_usd - self.turn_cost_base).max(0.0);
        let mut row = json!({
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
        });
        if let Some(source) = source {
            row["source"] = json!(source);
        }
        // Go repl.go: result usage is summed over the turn's API calls; keys are written even when absent (zeros).
        let ru = serde_json::to_value(self.result_usage.take().unwrap_or_default())
            .unwrap_or(Value::Null);
        let usage = usage_fields(&ru);
        if let Some(fields) = usage.as_object() {
            for (k, v) in fields {
                row[format!("result_{k}")] = v.clone();
            }
        }
        if let Some(mu) = self.result_model_usage.take()
            && mu.as_object().is_some_and(|o| !o.is_empty())
        {
            row["result_model_usage"] = mu;
        }
        let _ = self.metrics.log(with_spend(row, &self.spend));
    }

    // Rotation with live agents (the previous main launched them): each live agent's full brief, and the previous
    // main's turns from the first live launch on, newest kept within CARRIED_TURN_CHARS. Empty when none is live.
    // Called before the old child settles its agents, so a killed child's agents are still listed.
    fn carried_block(&self) -> String {
        let mut live: Vec<(&String, &AgentInfo)> = self
            .agents
            .keys()
            .filter_map(|id| self.agent_info.get(id).map(|i| (id, i)))
            .filter(|(_, i)| i.launch_epoch == self.epoch)
            .collect();
        if live.is_empty() {
            return String::new();
        }
        live.sort_by_key(|(_, i)| i.launch_seq);
        let mut block =
            String::from("## Agents still running from the previous session (launched by it)\n");
        for (id, i) in &live {
            let task = if i.task.is_empty() { &i.name } else { &i.task };
            block.push_str(&format!(
                "- {id} · {task} · launched at {} (unix secs)\n  brief:\n{}\n",
                i.started_at,
                cap_brief(&i.brief)
            ));
        }
        let turns = self.carried_turns(live[0].1.launch_seq);
        if !turns.is_empty() {
            block.push_str(
                "\n## Previous session turns (from the first live agent's launch, newest kept)\n",
            );
            block.push_str(&turns);
        }
        block
    }

    // The logged turns from turn number `from` on, as "User:" and "Assistant:" text, newest kept within
    // CARRIED_TURN_CHARS. The oldest included turn keeps its tail. Tool results are not in the log.
    fn carried_turns(&self, from: u64) -> String {
        let mut left = CARRIED_TURN_CHARS;
        let mut parts: Vec<String> = Vec::new();
        for (seq, prompt, reply) in self.turn_log.iter().rev() {
            if *seq < from || left == 0 {
                break;
            }
            let text = tail(&format!("User: {prompt}\nAssistant: {reply}"), left);
            left -= text.chars().count();
            parts.push(text);
        }
        parts.reverse();
        parts.join("\n\n")
    }

    // Go shouldRotate at turn end: the context is at or past the threshold.
    fn rotation_wanted(&self) -> bool {
        let thr = threshold_for(&self.model, self.cfg.rotate_threshold);
        self.cfg.rotate_threshold > 0
            && self.proc.is_some()
            && should_rotate(self.ctx_tokens, thr)
            && self.auto_rotate_allowed()
    }

    // Go maxAutoRotations and rotate-cooldown: a negative cap is unlimited, and a held rotation blocks automatic
    // context rotation until the cooldown has passed.
    fn auto_rotate_allowed(&self) -> bool {
        let capped = self.cfg.max_auto_rotations >= 0
            && self.rotations >= self.cfg.max_auto_rotations as u64;
        let cooling = self
            .last_rot_fail
            .is_some_and(|t| t.elapsed() < self.cfg.rotate_cooldown);
        !capped && !cooling
    }

    // A held rotation may run now: still wanted, and no background agent runs or the defer timeout passed.
    // A held reply defers it too: the rotation waits for ReplyReleased.
    fn deferred_due(&self) -> bool {
        self.held.is_none()
            && self.rotate_deferred.pending()
            && self.rotation_wanted()
            && self.rotate_deferred.is_due(
                self.agents.len(),
                Instant::now(),
                self.cfg.rotate_defer_timeout,
            )
    }

    // Drops a held rotation. hold(0) is due at once, so it ends the hold and its start (Deferral has no clear).
    fn clear_deferral(&mut self) {
        let _ = self
            .rotate_deferred
            .hold(0, "", Instant::now(), Duration::ZERO);
    }

    // Go runDeferredRotation: the held handoff runs while the child is idle, so the turn is marked busy.
    fn run_deferred_rotation(&mut self) {
        self.clear_deferral();
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        if !self.start_handoff(self.ctx_reason()) {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Go idle rotation: the child has no API response for idle_deadline, so the handoff runs now.
    fn run_idle_rotation(&mut self) {
        self.idle_fired_for = self.last_api;
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        let reason = format!("idle {}s", self.idle_deadline().as_secs());
        if !self.start_handoff(reason) {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Reason shown for a context rotation: the context tokens against the threshold.
    fn ctx_reason(&self) -> String {
        let thr = threshold_for(&self.model, self.cfg.rotate_threshold);
        format!("ctx {} >= {}", self.ctx_tokens, thr)
    }

    // Sends the hidden handoff prompt on the current child. Returns false when it could not be sent, or when a
    // pending snapshot was kept instead (the rotation then runs without a prompt; the caller goes on as after a failure).
    fn start_handoff(&mut self, reason: String) -> bool {
        if !self.pending_snapshot.is_empty() {
            self.rotate_keep_snapshot(reason);
            return false;
        }
        self.start_handoff_as(reason, false)
    }

    // The handoff turn itself; the caller has already ruled out a pending snapshot.
    fn start_handoff_as(&mut self, reason: String, clear: bool) -> bool {
        self.handoff = Some(Handoff {
            reason,
            attempts: 1,
            clear,
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
        let problems = if self.cfg.no_handoff_validate {
            Vec::new()
        } else {
            validate_handoff(&note, &self.files)
        };
        let valid = problems.is_empty();
        if !valid && h.attempts < 2 {
            self.handoff = Some(Handoff {
                reason: h.reason.clone(),
                attempts: h.attempts + 1,
                clear: h.clear,
            });
            if self.send_turn(HANDOFF_PROMPT) {
                return;
            }
            self.handoff = None;
        }
        if !valid && note.is_empty() && self.recent.is_empty() {
            // Go: nothing to carry, so the rotation is held and the session keeps running.
            self.switch_to = None;
            self.last_rot_fail = Some(Instant::now());
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
        if h.clear {
            self.finish_clear(snapshot);
            self.next_turn();
            return;
        }
        if let Some((model, text)) = self.switch_to.take() {
            self.complete_switch(model, text, snapshot);
            return;
        }
        self.rotate_child(snapshot, h.reason, valid, h.attempts);
        self.next_turn();
    }

    // Go buildSnapshot: preamble, handoff note, changed files, and the last turns, capped at snapshot_chars.
    fn build_snapshot(&self, note: &str) -> String {
        let note = if note.is_empty() { "(empty)" } else { note };
        // Each header and its body are separate parts, joined with a blank line (Go buildSnapshot).
        let mut parts = vec![
            PREAMBLE.to_string(),
            "## Handoff note (written by previous session)".to_string(),
            note.to_string(),
        ];
        // Go bgHandoffLines: background agents still running or with a written result, oldest first.
        let bg: Vec<rotate_defer::BgAgent> = self
            .agent_info
            .iter()
            .map(|(id, i)| rotate_defer::BgAgent {
                id: id.as_str(),
                name: &i.name,
                task: "",
                started: i.started,
                background: i.background,
                done: i.done,
                result_done: i.result_done,
                // result_path is never set at launch; out_path is the file the prompt asked for.
                result_path: if i.result_path.is_empty() {
                    &i.out_path
                } else {
                    &i.result_path
                },
            })
            .collect();
        let lines = rotate_defer::bg_handoff_lines(&bg);
        if !lines.is_empty() {
            parts.push("## Background agents (running)".to_string());
            parts.push(lines.join("\n"));
        }
        if !self.files.is_empty() {
            let list: Vec<String> = self.files.iter().map(|f| format!("- {f}")).collect();
            parts.push("## Files changed".to_string());
            parts.push(list.join("\n"));
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
        clip(&parts.join("\n\n"), self.cfg.snapshot_chars)
    }

    // Closes the current child and starts a fresh one (no --resume). Returns the closed session id.
    fn restart_child(&mut self) -> String {
        self.restart_child_with(false).0
    }

    // Returns the old session id and whether the old child was kept draining (Go rotate: drained).
    fn restart_child_with(&mut self, drain: bool) -> (String, bool) {
        let old = self.sid.clone();
        let old_epoch = self.epoch;
        let mut drained = false;
        if let Some(mut p) = self.proc.take() {
            if drain {
                // Go drain.consume: prompts still pending on the old child are denied as it drains.
                for id in self.pending_perm.keys() {
                    let _ = p.write_permission(id, false, Value::Null, DRAIN_DENY);
                }
                self.start_drain(p, old_epoch, old.clone());
                drained = true;
            } else {
                p.kill();
                // The killed child took its running agents with it: they settle now (타임아웃), so the notice
                // queues for the new child and the tracker does not keep them forever.
                let ids: Vec<String> = self
                    .agents
                    .keys()
                    .filter(|id| !self.drains.iter().any(|d| d.started.contains(id)))
                    .cloned()
                    .collect();
                for id in ids {
                    self.settle_drained(&id, false);
                }
            }
        }
        self.next_epoch();
        self.rotations += 1;
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
        self.pending_perm.clear();
        self.sid.clear();
        self.ctx_tokens = 0;
        self.last_api = None;
        self.opts.resume = None;
        let _ = self.start_child();
        (old, drained)
    }

    // Go drain: the old child keeps its running agents. Its agents are the ones still in self.agents. The oldest
    // drains beyond max_drain are closed.
    fn start_drain(&mut self, proc: ClaudeProc, epoch: u64, session: String) {
        let mut started: Vec<String> = self.agents.keys().cloned().collect();
        started.sort();
        self.drains.push(Drain {
            proc,
            epoch,
            session,
            since: Instant::now(),
            started,
            interrupting: false,
        });
        while self.drains.len() > self.cfg.max_drain {
            let oldest = self.drains.remove(0);
            // Evicted before it finished: its agents are closed as timed out, same as a drain that ran out.
            self.close_drain(oldest, true);
        }
    }

    // Ends one drain: its child is killed, the entry dropped, and one "[agent done]" notice queued per agent
    // (Go pollDrains). timed_out marks every agent 타임아웃, as Go does.
    fn end_drain(&mut self, epoch: u64, timed_out: bool) {
        if let Some(i) = self.drains.iter().position(|d| d.epoch == epoch) {
            let d = self.drains.remove(i);
            self.close_drain(d, timed_out);
        }
    }

    // The one finish path for a drain that is closed (end or eviction): its child is killed and each agent it
    // started settles once through settle_drained.
    fn close_drain(&mut self, mut d: Drain, timed_out: bool) {
        d.proc.kill();
        for id in &d.started {
            let done = !timed_out && !self.agents.contains_key(id);
            self.settle_drained(id, done);
        }
    }

    // Settles one drained or killed agent exactly once: it leaves every drain's started list, its "[agent done]"
    // notice is queued (done or 타임아웃), and it leaves the tracker so its status is terminal.
    fn settle_drained(&mut self, id: &str, done: bool) {
        let owners: Vec<u64> = self
            .drains
            .iter()
            .filter(|d| d.started.iter().any(|s| s == id))
            .map(|d| d.epoch)
            .collect();
        for d in self.drains.iter_mut() {
            d.started.retain(|s| s != id);
        }
        let (task, out, brief, report) = self
            .agent_info
            .get(id)
            .map(|i| {
                (
                    i.task.clone(),
                    i.out_path.clone(),
                    self.old_brief(i),
                    i.report.clone(),
                )
            })
            .unwrap_or_default();
        if announce_once(&mut self.announced, id) {
            self.notices
                .push(drain_done_notice(id, &task, &out, done, &brief, &report));
        }
        if self.agents.contains_key(id) {
            self.finish_agent(id.to_string());
        }
        // A drain whose last owned child just completed has nothing left to wait for: the old main is ended now
        // (killed and reaped), so a turn it would start on its own never reaches the TUI.
        if done {
            for epoch in owners {
                if self
                    .drains
                    .iter()
                    .any(|d| d.epoch == epoch && d.started.is_empty())
                {
                    self.end_drain(epoch, false);
                }
            }
        }
        // The drained child consumed the completion: an idle current main gets the notice as its own turn now.
        // A timed-out drain keeps the notice for the next user message (Go).
        if done
            && !self.notices.is_empty()
            && !self.busy
            && !self.prompter.waiting()
            && self.proc.is_some()
        {
            self.busy = true;
            self.emit(UiEvent::Busy(true));
            if !self.send_turn("") {
                self.busy = false;
                self.emit(UiEvent::Busy(false));
            }
        }
    }

    // Go drain.consume: a drained child's permission prompts are denied and every line is logged as drain_deny or
    // drain_event (transcript rows, Go r.tr.add). They never finish the shared tracker's agents. Eof only returns,
    // as Go consume does on stdout close; the drain ends in reap_drains (Go pollDrains).
    fn on_drain_msg(&mut self, epoch: u64, msg: ProcMsg) {
        let ProcMsg::Line(line) = msg else {
            return;
        };
        let Some(i) = self.drains.iter().position(|d| d.epoch == epoch) else {
            return;
        };
        let kind = match drain_permission_id(&line) {
            Some(id) => {
                let _ = self.drains[i]
                    .proc
                    .write_permission(&id, false, Value::Null, DRAIN_DENY);
                "drain_deny"
            }
            None => "drain_event",
        };
        let raw = serde_json::from_str::<Value>(&line).ok();
        // A turn the drained child starts on its own (a native task notification) is not caused by chaude: it is
        // interrupted once, and its output is discarded up to its result. The agent-done handling still runs below.
        if raw.as_ref().is_some_and(Self::drain_turn_started) && !self.drains[i].interrupting {
            self.drains[i].interrupting = true;
            let _ = self.drains[i].proc.write_interrupt();
        }
        let discard = self.drains[i].interrupting;
        if raw.as_ref().is_some_and(|v| v["type"] == "result") {
            self.drains[i].interrupting = false;
        }
        if !discard {
            let ty = raw
                .as_ref()
                .and_then(|v| v["type"].as_str().map(str::to_string))
                .unwrap_or_default();
            let session = self.drains[i].session.clone();
            self.log_transcript(kind, json!({"type": ty, "session": session}));
            if let Some(v) = &raw {
                self.record_handback(v);
            }
        }
        self.drain_completed(epoch, &line);
    }

    // A turn start on a drained child: an assistant message or message_start of the main thread (no parent tool use).
    // Subagent lines carry parent_tool_use_id and do not count.
    fn drain_turn_started(v: &Value) -> bool {
        let top = v["parent_tool_use_id"]
            .as_str()
            .is_none_or(|p| p.is_empty());
        top && (v["type"] == "assistant"
            || (v["type"] == "stream_event" && v["event"]["type"] == "message_start"))
    }

    // A drained agent's <task-notification> ends it at once: the agent leaves the tracker (terminal, AgentEnd) and its
    // "[agent done]" notice is queued now, once. end_drain then skips it. The notice goes out with the next user prompt
    // to the current main, so a rotation in between does not lose it.
    fn drain_completed(&mut self, epoch: u64, line: &str) {
        if !self.drains.iter().any(|d| d.epoch == epoch) {
            return;
        }
        match parse_line(line) {
            Ok(Event::System(s)) => self.on_system_task(&s),
            _ => {
                self.on_task_notification(line);
            }
        }
    }

    // Go pollDrains: finished drains are closed (done when the shared tracker is empty, timed out at drain_timeout).
    // Runs at the start of each user submit and on quiet, as Go polls at handle().
    // self.agents is the engine's tracker of running background agents (Go d.tr.running()).
    fn reap_drains(&mut self) {
        let ages: Vec<(u64, Duration)> = self
            .drains
            .iter()
            .map(|d| (d.epoch, d.since.elapsed()))
            .collect();
        let finished = finished_drains(&ages, self.agents.len(), self.cfg.drain_timeout);
        for (epoch, timed_out) in finished {
            self.end_drain(epoch, timed_out);
        }
    }

    // Takes the next epoch from the monotonic counter, so a restored parked child keeps a unique tag.
    fn next_epoch(&mut self) {
        self.epoch_seq += 1;
        self.epoch = self.epoch_seq;
    }

    // Go rotate with a pending snapshot (snapshot_kept): the child has not received it yet, so a handoff from
    // it would be empty and would replace the snapshot. The child restarts and the snapshot stays for its next prompt.
    fn rotate_keep_snapshot(&mut self, reason: String) {
        let old = self.restart_child();
        let _ = self.metrics.log(json!({
            "stage": "rotate",
            "reason": reason,
            "old_session": old,
            "snapshot_kept": true,
            "outcome": "snapshot_kept",
        }));
        self.emit(UiEvent::RotatedFrom { old_session: old });
        self.emit(UiEvent::Rotated { reason });
        self.emit_status(self.cost_usd);
    }

    // Go /rotate (repl.go rotate("manual")): the same handoff as a context rotation, reason "manual".
    fn manual_rotate(&mut self) {
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        if !self.start_handoff("manual".to_string()) {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Go /model NAME (repl.go runTUI): the parked child for NAME is ended, the session model changes, and a rotation
    // runs with reason "model NAME". The handoff runs on the current child; the fresh child starts on NAME.
    fn model_switch(&mut self, model: String) {
        self.end_parked(&model);
        self.model = model.clone();
        self.opts.model = model.clone();
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        if !self.start_handoff(format!("model {model}")) {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Go endParked(key, reason): the parked child under (cwd, model) is closed and its registry row marked ended.
    fn end_parked(&mut self, model: &str) {
        let cwd = self.client_cwd();
        if let Some(mut p) = self.children.take(&child_key(&cwd, model, "")) {
            p.sess.close();
            if let Some(reg) = &self.clients {
                reg.mark_ended(&cwd, model, "model-switch", now_stamp());
            }
        }
    }

    // Go /clear in the TUI (clearWithHandoff): the snapshot is taken first when there is a conversation to carry.
    // A pending snapshot is dropped, as in Go (clear() resets pendingSnapshot before the new one is set).
    fn clear_session(&mut self) {
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        if self.recent.is_empty() || !self.pending_snapshot.is_empty() {
            self.finish_clear(String::new());
            self.next_turn();
            return;
        }
        if !self.start_handoff_as("clear".to_string(), true) {
            self.finish_clear(String::new());
            self.next_turn();
        }
    }

    // Go clear(): the old session ends, the record splits at this time, and the fresh child gets the snapshot
    // (empty when none) with its next prompt.
    fn finish_clear(&mut self, snapshot: String) {
        self.clear_mark();
        self.clear_client_row();
        self.end_all_parked();
        self.record.split = true;
        self.restart_child();
        self.recent.clear();
        self.files.clear();
        self.pending_snapshot = snapshot;
    }

    // Closes the current child and starts a fresh one that gets the snapshot with its next prompt.
    fn rotate_child(&mut self, snapshot: String, reason: String, valid: bool, attempts: u32) {
        // Built before restart_child_with: a killed child's agents leave self.agents when they settle.
        let carried = self.carried_block();
        let drain = drain_wanted(self.cfg.max_drain, self.agents.len());
        let (old, drained) = self.restart_child_with(drain);
        self.pending_snapshot = if carried.is_empty() {
            snapshot
        } else {
            format!("{snapshot}\n\n{carried}")
        };
        let _ = self.metrics.log(json!({
            "stage": "rotate",
            "reason": reason,
            "old_session": old,
            "handoff_valid": valid,
            "attempts": attempts,
            "drained": drained,
            "outcome": if drained { "drained" } else { "closed" },
        }));
        self.emit(UiEvent::RotatedFrom { old_session: old });
        self.emit(UiEvent::Rotated { reason });
        self.emit_status(self.cost_usd);
    }

    // Relaunches the main child on an existing session with --resume (EngineHandle::resume_session).
    // The caller names the session, so resume_mode does not apply here. Idle only, like a fresh start.
    // Go applyImport: the imported turns, files and recommendation become this session's history, and the
    // snapshot built from them goes in front of the next prompt (send_turn). Turns beyond tail_turns are dropped.
    fn install_import(&mut self, imp: ClaudeImport) {
        if self.busy || self.prompter.waiting() {
            self.emit(UiEvent::Error(
                "cannot import while a turn is running".to_string(),
            ));
            return;
        }
        let stem = imp
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let skip = imp.turns.len().saturating_sub(self.cfg.tail_turns);
        self.recent = imp.turns.into_iter().skip(skip).collect();
        self.files = imp.files;
        self.last_reco = imp.reco;
        self.restore_agents_for(&stem);
        let mut note = import_note(&stem);
        if let Some(p) = &imp.note_path {
            note = format!("{note}\n\n{}", resume_note::hint(p));
        }
        self.pending_snapshot = with_reco(&self.last_reco, &self.build_snapshot(&note));
    }

    fn resume_session(&mut self, session_id: &str) {
        if session_id.is_empty() {
            self.emit(UiEvent::Error("resume needs a session id".to_string()));
            return;
        }
        if self.busy || self.prompter.waiting() {
            self.emit(UiEvent::Error(
                "cannot resume while a turn is running".to_string(),
            ));
            return;
        }
        let old = self.sid.clone();
        if let Some(mut p) = self.proc.take() {
            p.kill();
        }
        self.next_epoch();
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
        self.pending_perm.clear();
        self.queued_tasks.clear();
        self.sid.clear();
        self.ctx_tokens = 0;
        self.last_api = None;
        self.opts.resume = Some(session_id.to_string());
        self.load_tasks(session_id);
        let _ = self.start_child();
        self.emit(UiEvent::SessionResumed {
            old_session: old,
            session_id: session_id.to_string(),
        });
        self.emit_status(self.cost_usd);
    }

    // Spawns the child for the current generation; its lines are tagged so a replaced child is ignored.
    // Go residentMCPConfig: started once, only with residents on. None leaves the main child without the coder tool.
    fn mcp_config(&mut self) -> Option<PathBuf> {
        if !self.cfg.residents {
            return None;
        }
        if self.mcp_cfg.is_none() {
            let caller = CoderCaller {
                tx: self.self_tx.clone(),
            };
            let cwd = self.engine_cwd();
            self.mcp_cfg =
                mcp::start_owned(&cwd, Arc::new(move |task: &str| caller.call(task))).ok();
        }
        self.mcp_cfg.as_ref().map(|c| c.path().to_path_buf())
    }

    // Returns the spawn error text; the UiEvent::Error is sent either way when the spawn fails.
    fn start_child(&mut self) -> Result<(), String> {
        self.seen_task_started = false;
        let mut opts = self.opts.clone();
        if let Some(path) = self.mcp_config() {
            opts.shape = Shape {
                strict_mcp: true,
                mcp_config: Some(path),
                rule: join_rules(&[&main_rule(&self.cfg), CODER_RULE]),
                ..Shape::default()
            };
        }
        let epoch = self.epoch;
        match proc::spawn(&opts, self.self_tx.clone(), move |m| Input::Proc(epoch, m)) {
            Ok(p) => {
                self.upsert_live(p.pid(), opts.resume.clone().unwrap_or_default());
                self.proc = Some(p);
                Ok(())
            }
            Err(e) => {
                self.emit(UiEvent::Error(format!("claude spawn failed: {e}")));
                Err(e.to_string())
            }
        }
    }

    // Go switchchild.go:263: the live row for this harness's main child, owned by this process.
    fn upsert_live(&self, child_pid: u32, session_id: String) {
        let Some(reg) = &self.clients else {
            return;
        };
        reg.upsert(ClientRec {
            cwd: self.client_cwd(),
            model: self.model.clone(),
            session_id,
            state: LIVE.to_string(),
            last_api: now_stamp(),
            owner_pid: std::process::id(),
            child_pid,
            ..Default::default()
        });
    }

    // Go repl.go clear(): the old row ends with "clear", then the new session gets a fresh live row.
    fn clear_client_row(&self) {
        let Some(reg) = &self.clients else {
            return;
        };
        reg.mark_ended(&self.client_cwd(), &self.model, "clear", now_stamp());
        let child_pid = self.proc.as_ref().map_or(0, |p| p.pid());
        self.upsert_live(child_pid, String::new());
    }

    // Go r.a.Cwd as text: the absolute engine cwd (current dir when the engine has no cwd option).
    fn client_cwd(&self) -> String {
        self.engine_cwd().to_string_lossy().into_owned()
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

// A parked main child: its process, the epoch its reader threads are tagged with, and its session id.
pub struct ParkedProc {
    proc: ClaudeProc,
    epoch: u64,
    sid: String,
}

impl Close for ParkedProc {
    fn close(&mut self) {
        self.proc.kill();
    }
}

// Registry Go time as a SystemTime. Go's zero time (negative Unix seconds) reads as the epoch, which is never warm.
fn stamp_time(s: crate::clients::Stamp) -> SystemTime {
    match u64::try_from(s.secs) {
        Ok(secs) => UNIX_EPOCH + Duration::new(secs, s.nanos),
        Err(_) => UNIX_EPOCH,
    }
}

// Sends one MCP task into the engine and waits for its answer. Cloneable across server threads.
pub struct CoderCaller {
    tx: Sender<Input>,
}

impl CoderCaller {
    pub fn call(&self, task: &str) -> Result<String, String> {
        let (tx, rx) = mpsc::channel();
        if self
            .tx
            .send(Input::Cmd(Cmd::CoderCall(task.to_string(), tx)))
            .is_err()
        {
            return Err("engine stopped".to_string());
        }
        rx.recv()
            .unwrap_or_else(|_| Err("engine stopped".to_string()))
    }
}

// Delivers an MCP answer when one is waiting. TUI and handoff tasks have no waiter.
fn answer(waiter: Option<Sender<Result<String, String>>>, out: Result<String, String>) {
    if let Some(tx) = waiter {
        let _ = tx.send(out);
    }
}

// Go coderRole: slot 0 is "coder", slot i is "coder{i+1}" (coder2 for the first extra coder).
// Go start() rule for the main child: the forward rule for its model (none for an Opus main), joined with the
// category rule when category rotation is on. The resident coder rule is appended by the MCP branch of start_child.
fn main_rule(cfg: &Config) -> String {
    let orch_mode = std::env::var(orch::ENV_KEY).unwrap_or_default();
    main_rule_with(cfg, &orch_mode)
}

// Go start() rule order: delegation, forward, category, orch. orch_mode is the CHAUDE_ORCH value.
fn main_rule_with(cfg: &Config, orch_mode: &str) -> String {
    let delegation = if cfg.no_delegation_rule {
        ""
    } else {
        DELEGATION_RULE
    };
    let category = if cfg.no_category_rotate {
        ""
    } else {
        CATEGORY_RULE
    };
    join_rules(&[
        delegation,
        forward_rule_for(&cfg.model),
        category,
        &orch::rule(orch_mode),
    ])
}

// Go delegationRule: default delegation of bounded code changes to the haiku-coder agent.
pub const DELEGATION_RULE: &str = "Default to delegating bounded code changes (implementation steps, small fixes, wiring, tests) to the haiku-coder agent with an exact spec; implement yourself only design-level or tightly coupled parts, then verify Haiku's diff. Log each delegation (agent, tokens, duration, outcome, retries).";

// Go turn row: the subagent spend fields (sub_*, spawns) are merged into the row.
fn with_spend(mut row: Value, spend: &TurnSpend) -> Value {
    if let (Some(r), Some(f)) = (row.as_object_mut(), spend.fields().as_object()) {
        for (k, v) in f {
            r.insert(k.clone(), v.clone());
        }
    }
    row
}

// Go TrimSpace(rule + " " + part): non-empty parts joined by one space.
fn join_rules(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

// S21: API errors that another send can fix: rate limit (429) and overloaded (529). Anything else is permanent.
fn retryable_api_error(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    ["429", "529", "rate limit", "rate_limit", "overloaded"]
        .iter()
        .any(|m| t.contains(m))
}

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
fn agent_slot(label: &str) -> String {
    format!("agent:{label}")
}

// Go subagentLabel (resident_coder.go:60): the subagent_type input, else "agent".
// The model an assistant line of a delegated agent ran on (message.model); None for other lines.
fn stream_model(raw: &Value) -> Option<String> {
    if raw.get("type").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    raw.get("message")
        .and_then(|m| m.get("model"))
        .and_then(Value::as_str)
        .filter(|m| !m.is_empty())
        .map(str::to_string)
}

// An agent named with a model prefix (haiku-scout, opus-runner, sonnet-x) runs on that model family.
fn agent_type_model(name: &str) -> Option<String> {
    ["haiku", "opus", "sonnet"]
        .into_iter()
        .find(|p| name.starts_with(&format!("{p}-")))
        .map(str::to_string)
}

fn subagent_label(input: &Value) -> String {
    input
        .get("subagent_type")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("agent")
        .to_string()
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

// A carried agent brief cut to CARRIED_BRIEF_CHARS (its head kept); a cut brief says so.
fn cap_brief(brief: &str) -> String {
    if brief.chars().count() <= CARRIED_BRIEF_CHARS {
        return brief.to_string();
    }
    format!(
        "{}\n[brief truncated: first {CARRIED_BRIEF_CHARS} chars shown]",
        clip(brief, CARRIED_BRIEF_CHARS)
    )
}

// The last max chars of s.
fn tail(s: &str, max: usize) -> String {
    let n = s.chars().count();
    s.chars().skip(n.saturating_sub(max)).collect()
}

// Go outPathRe `notes/runs/[^\s"'`)]+\.md`: the longest such run, ending at its last ".md".
// The result path is the notes/runs file the prompt asks the agent to write: the first path on a line that names
// an output action. A path the prompt only reads or cites is not the result (it was taken for one before).
fn out_path_in(prompt: &str) -> String {
    const OUTPUT_WORDS: [&str; 7] = ["write", "save", "output", "result", "작성", "저장", "기록"];
    prompt
        .lines()
        .filter(|l| {
            let lower = l.to_lowercase();
            OUTPUT_WORDS.iter().any(|w| lower.contains(w))
        })
        .find_map(first_run_path)
        .unwrap_or_default()
}

// The first notes/runs/ .md path in text that is a file, not a glob pattern (a glob names many files, never one result).
fn first_run_path(text: &str) -> Option<String> {
    let mut from = 0;
    while let Some(off) = text[from..].find("notes/runs/") {
        let start = from + off;
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | ')'))
            .unwrap_or(rest.len());
        let run = &rest[..end];
        if let Some(i) = run.rfind(".md") {
            let path = &run[..i + 3];
            if i > "notes/runs/".len() && !path.contains(['*', '?', '[', '{']) {
                return Some(path.to_string());
            }
        }
        from = start + "notes/runs/".len();
    }
    None
}

fn elapsed_ms(since: Option<Instant>) -> u64 {
    since.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0)
}

// Local wall-clock time for the record file name (Go recordSave), from the shared localtime helper.
fn local_stamp() -> Option<Stamp> {
    let [y, mo, d, h, mi, s] = crate::localtime::local_civil(now_secs() as i64);
    Some(Stamp::new(
        y as i32, mo as u32, d as u32, h as u32, mi as u32, s as u32,
    ))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Go effTTL: the observed TTL when known, else the --cache-ttl flag.
fn effective_ttl<'a>(seen: &'a str, cfg: &'a str) -> &'a str {
    if seen.is_empty() { cfg } else { seen }
}

// Go livechild.go: an assistant event with non-empty usage updates ttlSeen. None when the line does not apply.
fn observe_ttl(line: &str, seen: &str) -> Option<String> {
    if !line.contains("usage") {
        return None;
    }
    let v: Value = serde_json::from_str(line).ok()?;
    if v["type"] != "assistant" {
        return None;
    }
    let u = &v["message"]["usage"];
    if !u.as_object().is_some_and(|m| !m.is_empty()) {
        return None;
    }
    Some(observed_ttl(u, seen))
}

// Go switchChild: the handoff turn runs only for a switch that starts a fresh child, with no pending snapshot
// (a kept import snapshot wins), turns to hand off, a running session, and no handoff already in flight.
// Go warmDeadline (switchchild.go:26-31): the cache window (clients::idle_deadline) when set, else NEVER_COLD.
// Used by pickChild.
fn warm_deadline(idle: Duration) -> Duration {
    if idle.is_zero() { NEVER_COLD } else { idle }
}

// pickChild warm window: idle_rotate (3570 s) whatever TTL the cache showed. An observed 5m cache does not
// shorten it (user rule 261109). estimate_next keeps its own 290 s cache window.
fn pick_warm_window(cfg: &Config) -> Duration {
    warm_deadline(cfg.idle_rotate)
}

// Main session idle rotation: idle_rotate only. An observed 5m cache does not shorten it (user rule: base
// sessions idle out after 3570 s). Extra coders use coder_idle_ttl instead.
fn main_idle_rotation(cfg: &Config) -> Duration {
    cfg.idle_rotate
}

fn switch_needs_handoff(
    pending: &str,
    recent: usize,
    running: bool,
    pick: ChildAction,
    handoff_active: bool,
) -> bool {
    pending.is_empty() && recent > 0 && running && !handoff_active && pick == ChildAction::StartNew
}

// Go parkedIdleEvent for a non-coder resident (tui, prompter): coderMayUse is false for it, so the request
// is always denied with the parked note. The write error is ignored, as Go does (`_ = s.RespondPermission`).
fn deny_parked(proc: Option<&mut ClaudeProc>, request_id: &str) {
    if let Some(p) = proc {
        let _ = p.write_permission(
            request_id,
            false,
            Value::Null,
            "denied: session parked (another model is active)",
        );
    }
}

// Go coderMayUse: a busy resident coder may use Bash, and Read/Edit/Write on paths inside cwd. An idle
// coder gets no approval. Relative paths join cwd; ".." segments are cleaned before the cwd check.
fn coder_may_use(busy: bool, cwd: Option<&Path>, tool: &str, input: &Value) -> bool {
    if !busy {
        return false;
    }
    match tool {
        "Bash" => true,
        "Read" | "Edit" | "Write" => {
            let Some(cwd) = cwd else { return false };
            let Some(p) = input.get("file_path").and_then(Value::as_str) else {
                return false;
            };
            if p.is_empty() {
                return false;
            }
            let p = if Path::new(p).is_absolute() {
                PathBuf::from(p)
            } else {
                cwd.join(p)
            };
            lexically_clean(&p).starts_with(lexically_clean(cwd))
        }
        _ => false,
    }
}

// Drops "." and resolves ".." against earlier normal segments, without touching the filesystem.
fn lexically_clean(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir
                if matches!(out.components().next_back(), Some(Component::Normal(_))) =>
            {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

// Go recordProjectName: an unset cwd resolves to the process cwd (os.Getwd), never ".".
fn resolve_cwd(cwd: Option<PathBuf>) -> PathBuf {
    cwd.or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests;
