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

mod events;
mod misc;
mod persist;
mod retry;
mod route;
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
