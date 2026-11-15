//! UI-neutral state shared by the TUI and the GUI: the user Action enum, the status-bar data (BarInput) and the
//! prompt-history push rule. No terminal, render or editor types here; the TUI keeps Editor, RefCell and render.

use std::collections::BTreeMap;
use std::time::Instant;

use crate::api::{AgentRow, CoderContext, InterruptSource, TaskStatus, UiEvent};
use crate::slash::PermMode;

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
}
