//! UI-neutral state shared by the TUI and the GUI: the user Action enum, the status-bar data (BarInput) and the
//! prompt-history push rule. No terminal, render or editor types here; the TUI keeps Editor, RefCell and render.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::time::Instant;

use crate::api::{AgentRow, CoderContext, InterruptSource, Role, TaskStatus, UiEvent};
use crate::bar_text::fmt_secs;
use crate::i18n::{self, Lang, Msg};
use crate::localtime::clock_hhmmss;
use crate::resident::StagePhase;
use crate::slash::PermMode;

// One row of a front end's transcript (GUI output pane, TUI transcript model). Text rows keep the role so main and
// agent text stay in their own blocks. Renderer-free: each front end draws it its own way (the TUI wraps and colours
// them; the GUI shows them as widgets). Kinds map 1:1 to the TUI push sites in tui/mod.rs (see the S4 step-1 note).
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    // Prompt echo (TUI echo_prompt, record "user" block).
    User(String),
    // Assistant markdown (main or agent), streamed then done. TUI feed_main/feed/flush, record "asst" block.
    Text {
        role: Role,
        text: String,
        done: bool,
    },
    // Tool call row (TUI ToolStart, resume row); ToolInput/ToolEnd update summary, ok and end in place.
    // start is the local arrival time (unix seconds); end is (unix seconds, elapsed seconds) once the call finished.
    Tool {
        id: String,
        name: String,
        summary: String,
        ok: Option<bool>,
        start: i64,
        end: Option<(i64, u64)>,
    },
    // Tool result under its call (TUI ToolResult: "  ⎿  <time> <head>"), the first line only. at is the unix arrival time.
    ToolResult {
        id: String,
        text: String,
        at: i64,
    },
    // Background or foreground agent summary row (GUI). The TUI shows agents in its panel and writes a Notice plus an
    // in-place Tool update on AgentEnd (agent_done).
    Agent {
        id: String,
        description: String,
        done: bool,
        summary: String,
    },
    // Dim note line (TUI note/note_wide: Notice, SessionResumed, PermissionRequest, BudgetStop, record other kinds).
    Notice(String),
    // Short state line (TUI ModeChanged "mode: x", Interrupted "[interrupted]", permission answer "allowed"/"denied").
    Status(String),
    // Error line (TUI UiEvent::Error, dim note block).
    Error(String),
    // Blank separator row (TUI String::new() pushes between blocks).
    Blank,
}

// The shared transcript: an ordered list of Entry rows. Push helpers keep call sites one line per kind.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Transcript {
    pub entries: Vec<Entry>,
}

impl Transcript {
    pub fn push(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn push_user(&mut self, text: impl Into<String>) {
        self.push(Entry::User(text.into()));
    }

    pub fn push_text(&mut self, role: Role, text: impl Into<String>, done: bool) {
        self.push(Entry::Text {
            role,
            text: text.into(),
            done,
        });
    }

    pub fn push_tool(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        summary: impl Into<String>,
        start: i64,
    ) {
        self.push(Entry::Tool {
            id: id.into(),
            name: name.into(),
            summary: summary.into(),
            ok: None,
            start,
            end: None,
        });
    }

    pub fn push_tool_result(&mut self, id: impl Into<String>, text: impl Into<String>, at: i64) {
        self.push(Entry::ToolResult {
            id: id.into(),
            text: text.into(),
            at,
        });
    }

    pub fn push_notice(&mut self, text: impl Into<String>) {
        self.push(Entry::Notice(text.into()));
    }

    pub fn push_status(&mut self, text: impl Into<String>) {
        self.push(Entry::Status(text.into()));
    }

    pub fn push_error(&mut self, text: impl Into<String>) {
        self.push(Entry::Error(text.into()));
    }

    pub fn push_blank(&mut self) {
        self.push(Entry::Blank);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// Tool row head: "name(summary)", or the name alone when there is no summary (the ToolRow head rule).
pub fn tool_head(name: &str, summary: &str) -> String {
    if summary.is_empty() {
        name.to_string()
    } else {
        format!("{name}({summary})")
    }
}

// An action row with its time: `<lead>HH:MM:SS <body>`, and once finished ` → HH:MM:SS (elapsed)` at the end.
// The lead ("● " or "· ") stays first, so the wrap hang and the "● Name(arg)" text are unchanged.
pub fn action_line(lead: &str, start: i64, body: &str, end: Option<(i64, u64)>) -> String {
    let mut s = format!("{lead}{} {body}", clock_hhmmss(start));
    if let Some((at, secs)) = end {
        s.push_str(&format!(" → {} ({})", clock_hhmmss(at), fmt_secs(secs)));
    }
    s
}

// A tool call row as plain text (the TUI colours it green): "● HH:MM:SS name(summary)", with the end stamp once done.
pub fn tool_line(start: i64, name: &str, summary: &str, end: Option<(i64, u64)>) -> String {
    action_line("● ", start, &tool_head(name, summary), end)
}

// A tool result row as plain text: "  ⎿  HH:MM:SS <first line>", stamped with its arrival time.
pub fn tool_result_line(at: i64, text: &str) -> String {
    format!("  ⎿  {} {text}", clock_hhmmss(at))
}

// A stage row as plain text: "· HH:MM:SS <stage text>". The stage text's own "· " lead is not repeated.
pub fn stage_line(start: i64, text: &str, end: Option<(i64, u64)>) -> String {
    action_line("· ", start, text.strip_prefix("· ").unwrap_or(text), end)
}

// Go repl.go stage(): "· <slot> 처리중" on start, "· <slot> 완료" on success, "· <slot> 실패: <err>" on failure.
// Agent slots use Go's "호출" on start and "agent <label>" as the name, where the label is Go subagentLabel.
pub fn stage_text(lang: Lang, slot: &str, phase: &StagePhase) -> String {
    let (name, start) = match slot.strip_prefix("agent:") {
        Some(desc) => (format!("agent {desc}"), Msg::StageCall),
        None if slot == "tui" => (slot.to_string(), Msg::StageTranslate),
        None => (slot.to_string(), Msg::StageStart),
    };
    let fill = |msg: Msg, args: &[&dyn Display]| i18n::fill(i18n::text(lang, msg), args);
    match phase {
        StagePhase::Working => fill(start, &[&name]),
        StagePhase::Done => fill(Msg::StageDone, &[&name]),
        StagePhase::Failed(err) => fill(Msg::StageFailed, &[&name, err]),
    }
}

// R04 cavemanStatusRe (tui.go:97): ^status:\s*(done|partial|fail|blocked)\b, applied to a trimmed line.
fn is_status_line(ln: &str) -> bool {
    let Some(rest) = ln.strip_prefix("status:") else {
        return false;
    };
    let rest = rest.trim_start_matches([' ', '\t', '\n', '\x0c', '\r']);
    for word in ["done", "partial", "fail", "blocked"] {
        if let Some(tail) = rest.strip_prefix(word) {
            return !tail
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        }
    }
    false
}

// R04 isCavemanBlock (tui.go:100): a status line and a reason line anywhere in the text.
pub fn is_caveman_block(s: &str) -> bool {
    let mut status = false;
    let mut reason = false;
    for ln in s.split('\n') {
        let ln = ln.trim();
        if is_status_line(ln) {
            status = true;
        }
        if ln.starts_with("reason:") {
            reason = true;
        }
    }
    status && reason
}

// R05 tool result text (toolResultMsg, tui.go:485): None when hidden as a caveman block; otherwise the first
// line cut to 160 chars, plus " … (+N lines)" when there is more.
pub fn tool_result_text(s: &str) -> Option<String> {
    let s = s.trim();
    if is_caveman_block(s) {
        return None;
    }
    let lines: Vec<&str> = s.split('\n').collect();
    let mut t: String = lines[0].chars().take(160).collect();
    if lines.len() > 1 {
        t.push_str(&format!(" … (+{} lines)", lines.len() - 1));
    }
    Some(t)
}

// history.go maxHistory
pub const MAX_HISTORY: usize = 500;

// Go inputHistory.Add: blanks and a repeat of the last entry are skipped; only the newest MAX_HISTORY are kept.
// Returns whether the entry was added.
pub fn push_history(items: &mut Vec<String>, s: String) -> bool {
    if !s.is_empty() && items.last() != Some(&s) {
        items.push(s);
        if items.len() > MAX_HISTORY {
            items.drain(..items.len() - MAX_HISTORY);
        }
        return true;
    }
    false
}

// What the event loop must do after a key press.
#[derive(Debug, PartialEq)]
pub enum Action {
    Continue,
    Submit(String),
    Interrupt(InterruptSource),
    Quit,
    // Esc while a reply is held: abandon its queued-task wait (Engine::cancel_wait).
    CancelWait,
    // /usage: the caller asks the engine for the one-shot run (UiEvent::Usage). Never a turn.
    Usage,
    // /update: the caller runs the release check off the UI thread; the result returns as Wake::Note (tui.go:1166).
    Update,
    // /doctor: the caller runs doctor::checks_now() off the UI thread; the rendered lines return as Wake::Note.
    Doctor,
    // Answer to the open permission prompt (Go tui.go key(): y/Enter allow, n/Esc deny, Ctrl+C deny + interrupt).
    Answer {
        id: String,
        allow: bool,
        interrupt: bool,
    },
    // Shift+Tab: the engine cycles the permission mode (Go setMode(Next())); it answers with ModeChanged.
    CycleMode,
    // /mode <name>: the engine sets the permission mode and answers with ModeChanged.
    SetMode(PermMode),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BarInput {
    // Seconds since the current turn started; None while idle.
    pub turn_secs: Option<u64>,
    // Running subagents as (description, elapsed seconds).
    pub agents: Vec<(String, u64)>,
    pub model: String,
    pub tokens: u64,
    pub cost_usd: f64,
    pub context_pct: u32,
    pub queued: usize,
    // Main-context limit in tokens (the welcome threshold); used by the idle Go-style status row.
    pub limit: u64,
    // Status line extras (Go statusLine): rate window text, rotations, permission mode ("default" or empty is
    // hidden), and foreground / background agent counts.
    pub rate: Option<String>,
    pub rot: u32,
    pub mode: String,
    pub fg: usize,
    pub bg: usize,
    // Agents a held reply waits for (UiEvent::ReplyHeld pending); None when the reply is not held.
    pub waiting: Option<usize>,
    // Latest context tokens per role (UiEvent::RoleContext): P = prompter, T = tui, C = coder.
    pub role_tokens: BTreeMap<String, u64>,
    // Live resident coders in spawn order (UiEvent::CoderContexts); empty hides the coder line.
    pub coders: Vec<CoderContext>,
    // Main's token counts of the last finished turn as (input, output, cache) (UiEvent::TurnTokens); shown on the idle row.
    pub turn_tokens: Option<(u64, u64, u64)>,
}

// One main Agent/Task call in the task list. Queued until its launch ack (TaskResolved Running), then Running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Running,
    Done,
    Failed,
    Timeout,
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: String,
    pub desc: String,
    pub state: TaskState,
    // The agent's total tokens and wall time from its tool result (UiEvent::TaskStats); None until it arrives.
    pub tokens: Option<u64>,
    pub duration_ms: Option<u64>,
}

// Engine progress shared by the TUI and the GUI: what runs, what is queued, what a held reply waits for.
// Each front end keeps its own transcript and render state beside it.
#[derive(Debug, Default)]
pub struct RunState {
    // A main turn is running; turn_start is when it began (None while idle).
    pub busy: bool,
    pub turn_start: Option<Instant>,
    // First line of the prompt that started the turn (Go curPrompt), shown in the busy row.
    pub turn_prompt: String,
    // (id, description, elapsed clock, started_at in ms since epoch: AgentStart carries Unix seconds).
    pub agents: Vec<(String, String, Instant, u64)>,
    // Ids of the agents launched in the background (UiEvent::AgentBackground); they have no AgentEnd until done.
    pub bg_agents: Vec<String>,
    // Go opusRunning / residentBusy: an opus Agent is running, and the resident roles with a command in flight.
    pub opus_busy: bool,
    // Busy resident sessions as running rows (Go residentBusy), oldest first; id and name are the role.
    pub resident_busy: Vec<AgentRow>,
    // Panel rows of the started agents, oldest first (UiEvent::AgentRows; Go tracker rows).
    pub agent_rows: Vec<AgentRow>,
    // Go lastRun: running count (running_n) at the last refresh.
    pub last_run: usize,
    // Agents an earlier harness left unfinished, listed after the running rows (UiEvent::AgentsRestored).
    pub restored_rows: Vec<AgentRow>,
    // Last tool call of the running turn, cut to 40 runes (Go lastTool); the busy row shows it.
    pub last_tool: String,
    // Prompts waiting for the next turn, oldest first (UiEvent::QueueTexts; Go m.queue).
    pub queue_texts: Vec<String>,
    // Main's Agent/Task calls of the current round, oldest first (UiEvent::TaskQueued / TaskResolved).
    pub tasks: Vec<TaskRow>,
    // Pending task count while main's final reply is held (UiEvent::ReplyHeld); None once released.
    pub held: Option<usize>,
    // Rotations seen this session (UiEvent::Rotated); Go m.r.rotations.
    pub rot: u32,
}

impl RunState {
    // Go runningN (tui.go:1445): running agents plus busy resident sessions.
    pub fn running_n(&self) -> usize {
        self.agent_rows.iter().filter(|r| !r.done).count() + self.resident_busy.len()
    }

    // Busy flag, turn start and last tool, shared by the TUI and the GUI. A turn already running keeps its start,
    // so a repeated Busy(true) does not restart the clock. Off clears the start. Every Busy(true) clears the last
    // tool (Go sendPrompt). The held reply is the caller's to apply (TurnEnd does not clear it here).
    pub fn set_busy(&mut self, on: bool, now: Instant) {
        self.busy = on;
        self.turn_start = match (on, self.turn_start) {
            (false, _) => None,
            (true, Some(t)) => Some(t),
            (true, None) => Some(now),
        };
        if on {
            // Go sendPrompt resets lastTool at the start of each turn.
            self.last_tool.clear();
        }
    }

    // Folds the progress events that touch only this state; other events are ignored. now is the arrival time
    // (turn start, agent start). A front end that also writes a transcript row for an event does that itself.
    pub fn apply(&mut self, ev: UiEvent, now: Instant) {
        match ev {
            UiEvent::Busy(on) => self.set_busy(on, now),
            UiEvent::AgentBackground { id } => self.bg_agents.push(id),
            UiEvent::AgentStart {
                id,
                description,
                started_at,
            } => self.agents.push((id, description, now, started_at * 1000)),
            // A new round starts once every earlier task has settled, as the engine's own list does.
            UiEvent::TaskQueued { id, desc } => {
                if self
                    .tasks
                    .iter()
                    .all(|t| !matches!(t.state, TaskState::Queued | TaskState::Running))
                {
                    self.tasks.clear();
                }
                self.tasks.push(TaskRow {
                    id,
                    desc,
                    state: TaskState::Queued,
                    tokens: None,
                    duration_ms: None,
                });
            }
            UiEvent::TaskStats {
                id,
                tokens,
                duration_ms,
            } => {
                if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                    t.tokens = tokens;
                    t.duration_ms = duration_ms;
                }
            }
            UiEvent::TaskResolved { id, status } => {
                if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                    t.state = match status {
                        TaskStatus::Running => TaskState::Running,
                        TaskStatus::Done => TaskState::Done,
                        TaskStatus::Failed => TaskState::Failed,
                        TaskStatus::Timeout => TaskState::Timeout,
                    };
                }
            }
            UiEvent::ReplyHeld { pending } => self.held = Some(pending),
            UiEvent::ReplyReleased => self.held = None,
            UiEvent::TurnEnd { .. } => self.held = None,
            UiEvent::Rotated { .. } => self.rot += 1,
            // Go agentRows / m.queue: the panels are redrawn from these lists each frame.
            UiEvent::AgentRows { rows } => {
                self.agent_rows = rows;
                self.last_run = self.running_n();
            }
            UiEvent::AgentsRestored { rows } => self.restored_rows = rows,
            // Go opusRunning / residentBusy: state only, read by the thinking row.
            UiEvent::OpusBusy(on) => self.opus_busy = on,
            UiEvent::ResidentBusy {
                role,
                busy,
                model,
                prompt,
                started_at,
            } => {
                self.resident_busy.retain(|r| r.id != role);
                if busy {
                    self.resident_busy.push(AgentRow {
                        id: role.clone(),
                        task: String::new(),
                        prompt,
                        name: role,
                        // A resident row never falls back to the session (main) model; residents default to haiku.
                        model: if model.is_empty() {
                            "haiku".to_string()
                        } else {
                            model
                        },
                        background: false,
                        done: false,
                        failed: false,
                        interrupted: false,
                        last_active: 0,
                        agent_id: String::new(),
                        result_path: String::new(),
                        started_at,
                    });
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caveman_blocks_are_detected_and_hidden() {
        let cases = [
            ("status: done\nreason: -\nclaim: a.go:1 | x", true),
            ("[harness: note]\nstatus: partial\nreason: cap", true),
            ("status: done", false),
            ("status: running\nreason: -", false),
            ("읽기 결과입니다.", false),
            ("", false),
        ];
        for (input, want) in cases {
            assert_eq!(is_caveman_block(input), want, "input {input:?}");
        }
        assert_eq!(
            tool_result_text("status: done\nreason: -\nclaim: a.go:1 | x"),
            None
        );
        assert_eq!(
            tool_result_text("ok\nmore").as_deref(),
            Some("ok … (+1 lines)")
        );
        assert_eq!(tool_result_text("  one  ").as_deref(), Some("one"));
    }

    #[test]
    fn tool_and_result_rows_stamp_their_times() {
        let start = 1_700_000_000;
        assert_eq!(tool_head("Bash", ""), "Bash");
        assert_eq!(tool_head("Read", "a.rs"), "Read(a.rs)");
        assert_eq!(
            tool_line(start, "Read", "a.rs", None),
            format!("● {} Read(a.rs)", clock_hhmmss(start))
        );
        assert_eq!(
            tool_line(start, "Read", "a.rs", Some((start + 89, 89))),
            format!(
                "● {} Read(a.rs) → {} (1m29s)",
                clock_hhmmss(start),
                clock_hhmmss(start + 89)
            )
        );
        assert_eq!(
            tool_result_line(start, "one line"),
            format!("  ⎿  {} one line", clock_hhmmss(start))
        );
    }

    #[test]
    fn stage_rows_drop_the_stage_lead_and_follow_the_phase() {
        let start = 1_700_000_000;
        assert_eq!(
            stage_line(start, "· tui 번역중", None),
            format!("· {} tui 번역중", clock_hhmmss(start))
        );
        assert_eq!(
            stage_text(Lang::Ko, "agent:haiku-coder", &StagePhase::Working),
            "· agent haiku-coder 호출"
        );
        assert_eq!(
            stage_text(Lang::Ko, "agent:agent", &StagePhase::Done),
            "· agent agent 완료"
        );
        let failed = stage_text(Lang::Ko, "prompter", &StagePhase::Failed("boom".into()));
        assert!(
            failed.starts_with("· prompter") && failed.ends_with("boom"),
            "{failed}"
        );
        assert_eq!(
            stage_text(Lang::En, "prompter", &StagePhase::Working),
            "· prompter working"
        );
    }

    #[test]
    fn run_state_busy_sets_turn_start_and_clears_last_tool() {
        let mut run = RunState {
            last_tool: "Read(a)".into(),
            ..RunState::default()
        };
        let now = Instant::now();
        run.apply(UiEvent::Busy(true), now);
        assert!(run.busy);
        assert_eq!(run.turn_start, Some(now));
        assert!(run.last_tool.is_empty());
        run.apply(UiEvent::Busy(false), now);
        assert!(!run.busy);
        assert_eq!(run.turn_start, None);
    }

    #[test]
    fn run_state_set_busy_keeps_a_running_turn_start() {
        let mut run = RunState::default();
        let t0 = Instant::now();
        let t1 = t0 + std::time::Duration::from_secs(5);
        run.set_busy(true, t0);
        run.set_busy(true, t1);
        assert!(run.busy);
        assert_eq!(run.turn_start, Some(t0));
        run.set_busy(false, t1);
        assert!(!run.busy);
        assert_eq!(run.turn_start, None);
    }

    #[test]
    fn run_state_repeated_busy_keeps_turn_start_and_clears_last_tool() {
        // TUI (apply) and GUI (set_busy) share the rule: a second Busy(true) keeps the first start.
        let mut run = RunState::default();
        let t0 = Instant::now();
        let t1 = t0 + std::time::Duration::from_secs(5);
        run.apply(UiEvent::Busy(true), t0);
        run.last_tool = "Read(a)".into();
        run.apply(UiEvent::Busy(true), t1);
        assert_eq!(run.turn_start, Some(t0));
        assert!(run.last_tool.is_empty());
        run.last_tool = "Bash(b)".into();
        run.set_busy(true, t1);
        assert_eq!(run.turn_start, Some(t0));
        assert!(run.last_tool.is_empty());
    }

    #[test]
    fn run_state_task_round_restarts_once_settled() {
        let mut run = RunState::default();
        let now = Instant::now();
        run.apply(
            UiEvent::TaskQueued {
                id: "t1".into(),
                desc: "a".into(),
            },
            now,
        );
        run.apply(
            UiEvent::TaskResolved {
                id: "t1".into(),
                status: TaskStatus::Done,
            },
            now,
        );
        run.apply(
            UiEvent::TaskStats {
                id: "t1".into(),
                tokens: Some(5),
                duration_ms: Some(9),
            },
            now,
        );
        assert_eq!(run.tasks.len(), 1);
        assert_eq!(run.tasks[0].state, TaskState::Done);
        assert_eq!(run.tasks[0].tokens, Some(5));
        run.apply(
            UiEvent::TaskQueued {
                id: "t2".into(),
                desc: "b".into(),
            },
            now,
        );
        assert_eq!(run.tasks.len(), 1);
        assert_eq!(run.tasks[0].id, "t2");
    }

    #[test]
    fn run_state_held_and_rot() {
        let mut run = RunState::default();
        let now = Instant::now();
        run.apply(UiEvent::ReplyHeld { pending: 2 }, now);
        assert_eq!(run.held, Some(2));
        run.apply(UiEvent::ReplyReleased, now);
        assert_eq!(run.held, None);
        run.apply(
            UiEvent::Rotated {
                reason: "ctx".into(),
            },
            now,
        );
        run.apply(
            UiEvent::Rotated {
                reason: "ctx".into(),
            },
            now,
        );
        assert_eq!(run.rot, 2);
    }

    #[test]
    fn push_history_skips_blank_and_repeat_of_last() {
        let mut items = Vec::new();
        assert!(push_history(&mut items, "a".into()));
        assert!(!push_history(&mut items, "".into()));
        assert!(!push_history(&mut items, "a".into()));
        assert!(push_history(&mut items, "b".into()));
        assert!(push_history(&mut items, "a".into()));
        assert_eq!(items, ["a", "b", "a"]);
    }

    #[test]
    fn push_history_keeps_only_the_newest_max_entries() {
        let mut items = Vec::new();
        for i in 0..MAX_HISTORY + 3 {
            push_history(&mut items, format!("e{i}"));
        }
        assert_eq!(items.len(), MAX_HISTORY);
        assert_eq!(items[0], "e3");
    }

    #[test]
    fn bar_input_defaults_to_idle() {
        let b = BarInput::default();
        assert_eq!(b.turn_secs, None);
        assert!(b.coders.is_empty());
        assert_eq!(b.turn_tokens, None);
    }

    #[test]
    fn action_carries_its_payload() {
        assert_eq!(Action::Submit("hi".into()), Action::Submit("hi".into()));
        assert_eq!(
            Action::SetMode(PermMode::Plan),
            Action::SetMode(PermMode::Plan)
        );
    }

    #[test]
    fn transcript_user_and_text_kinds() {
        let mut t = Transcript::default();
        t.push_user("hi");
        t.push_text(Role::Main, "answer", true);
        t.push_text(Role::Agent("a1".into()), "partial", false);
        assert_eq!(
            t.entries,
            vec![
                Entry::User("hi".into()),
                Entry::Text {
                    role: Role::Main,
                    text: "answer".into(),
                    done: true
                },
                Entry::Text {
                    role: Role::Agent("a1".into()),
                    text: "partial".into(),
                    done: false
                },
            ]
        );
    }

    #[test]
    fn transcript_tool_call_and_result_kinds() {
        let mut t = Transcript::default();
        t.push_tool("t1", "Read", "a.rs", 100);
        t.push_tool_result("t1", "12 lines", 101);
        assert_eq!(
            t.entries,
            vec![
                Entry::Tool {
                    id: "t1".into(),
                    name: "Read".into(),
                    summary: "a.rs".into(),
                    ok: None,
                    start: 100,
                    end: None
                },
                Entry::ToolResult {
                    id: "t1".into(),
                    text: "12 lines".into(),
                    at: 101
                },
            ]
        );
    }

    #[test]
    fn transcript_agent_kind_keeps_fields() {
        let mut t = Transcript::default();
        t.push(Entry::Agent {
            id: "g1".into(),
            description: "scan".into(),
            done: true,
            summary: "ok".into(),
        });
        assert_eq!(t.len(), 1);
        assert!(matches!(
            &t.entries[0],
            Entry::Agent { id, done: true, .. } if id == "g1"
        ));
    }

    #[test]
    fn transcript_notice_status_error_blank_kinds() {
        let mut t = Transcript::default();
        assert!(t.is_empty());
        t.push_notice("session resumed");
        t.push_status("mode: plan");
        t.push_error("boom");
        t.push_blank();
        assert_eq!(
            t.entries,
            vec![
                Entry::Notice("session resumed".into()),
                Entry::Status("mode: plan".into()),
                Entry::Error("boom".into()),
                Entry::Blank,
            ]
        );
        assert!(!t.is_empty());
    }
}
