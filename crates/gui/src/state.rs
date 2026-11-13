//! Pure GUI state: applies core UiEvents to the transcript, the status-bar data and the open permission prompt.
//! No egui types here, so the reducer is unit-tested without a window.

use std::path::{Path, PathBuf};
use std::time::Instant;

use chaude_core::api::{AgentRow, Role, TaskStatus, UiEvent};
use chaude_core::input_history::{append_history, history_file_for, read_history};
use chaude_core::ui_state::{BarInput, push_history};

// One row of the output pane. Text rows keep the role so main and agent text stay in their own blocks.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    User(String),
    Text {
        role: Role,
        text: String,
        done: bool,
    },
    Tool {
        id: String,
        name: String,
        summary: String,
        ok: Option<bool>,
    },
    Agent {
        id: String,
        description: String,
        done: bool,
        summary: String,
    },
    Notice(String),
    Error(String),
}

// A can_use_tool request waiting for the user (UiEvent::PermissionRequest).
#[derive(Debug, Clone, PartialEq)]
pub struct Permission {
    pub id: String,
    pub tool: String,
    pub summary: String,
}

// A main Agent/Task call the reply waits for (UiEvent::TaskQueued, TaskResolved, TaskStats).
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: String,
    pub desc: String,
    pub status: Option<TaskStatus>,
    pub tokens: Option<u64>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Default)]
pub struct GuiState {
    pub transcript: Vec<Entry>,
    // Status-bar data. turn_secs and the agent list are derived in bar_at, not stored here.
    pub bar: BarInput,
    pub busy: bool,
    pub busy_since: Option<Instant>,
    pub agent_rows: Vec<AgentRow>,
    pub permission: Option<Permission>,
    pub tasks: Vec<Task>,
    // Prompt history, newest last; same rule and file as the TUI (core input_history).
    pub history: Vec<String>,
    // <cwd>/.chaude/history.jsonl, set by load_history; None means history is kept in memory only.
    pub history_file: Option<PathBuf>,
}

impl GuiState {
    // Loads the recall list from <cwd>/.chaude/history.jsonl, as the TUI does at start.
    pub fn load_history(&mut self, cwd: &Path) {
        self.history_file = history_file_for(cwd);
        self.history = self
            .history_file
            .as_deref()
            .map(read_history)
            .unwrap_or_default();
    }

    // Echoes a prompt the user sent and records it in the history. The history gets the trimmed text, like the TUI.
    pub fn push_user(&mut self, text: &str) {
        self.transcript.push(Entry::User(text.to_string()));
        let entry = text.trim();
        if push_history(&mut self.history, entry.to_string())
            && let Some(path) = &self.history_file
        {
            // Best effort, like the TUI: recall still works in memory.
            let _ = append_history(path, entry);
        }
    }

    // Applies one engine event. now is the time it arrived (turn timer start).
    pub fn apply(&mut self, ev: UiEvent, now: Instant) {
        match ev {
            UiEvent::TextDelta { role, text } => {
                let i = self.open_text(&role);
                if let Entry::Text { text: t, .. } = &mut self.transcript[i] {
                    t.push_str(&text);
                }
            }
            UiEvent::MessageDone { role } => {
                if let Some(i) = self.open_text_index(&role)
                    && let Entry::Text { done, .. } = &mut self.transcript[i]
                {
                    *done = true;
                }
            }
            UiEvent::ToolStart { id, name, summary } => {
                self.transcript.push(Entry::Tool {
                    id,
                    name,
                    summary,
                    ok: None,
                });
            }
            UiEvent::ToolEnd { id, ok } => {
                if let Some(Entry::Tool { ok: slot, .. }) = self
                    .transcript
                    .iter_mut()
                    .rev()
                    .find(|e| matches!(e, Entry::Tool { id: i, .. } if *i == id))
                {
                    *slot = Some(ok);
                }
            }
            UiEvent::AgentStart {
                id, description, ..
            } => {
                self.transcript.push(Entry::Agent {
                    id,
                    description,
                    done: false,
                    summary: String::new(),
                });
            }
            UiEvent::AgentResumed {
                id, description, ..
            } => match self.find_agent(&id) {
                Some(Entry::Agent { done, .. }) => *done = false,
                _ => self.transcript.push(Entry::Agent {
                    id,
                    description,
                    done: false,
                    summary: String::new(),
                }),
            },
            UiEvent::AgentEnd { id, summary, .. } => {
                if let Some(Entry::Agent {
                    done, summary: s, ..
                }) = self.find_agent(&id)
                {
                    *done = true;
                    if !summary.is_empty() {
                        *s = summary;
                    }
                }
            }
            UiEvent::AgentRows { rows } => self.agent_rows = rows,
            UiEvent::Status {
                model,
                tokens,
                cost_usd,
                context_pct,
            } => {
                self.bar.model = model;
                self.bar.tokens = tokens;
                self.bar.cost_usd = cost_usd;
                self.bar.context_pct = context_pct;
            }
            UiEvent::Busy(on) => {
                self.busy = on;
                self.busy_since = match (on, self.busy_since) {
                    (false, _) => None,
                    (true, Some(t)) => Some(t),
                    (true, None) => Some(now),
                };
            }
            UiEvent::TurnEnd { .. } => {
                self.busy = false;
                self.busy_since = None;
            }
            UiEvent::Queued(n) => self.bar.queued = n,
            UiEvent::ModeChanged { mode } => self.bar.mode = mode,
            UiEvent::TurnTokens {
                input,
                output,
                cache,
            } => self.bar.turn_tokens = Some((input, output, cache)),
            UiEvent::ReplyHeld { pending } => self.bar.waiting = Some(pending),
            UiEvent::ReplyReleased => self.bar.waiting = None,
            UiEvent::RoleContext { role, tokens } => {
                self.bar.role_tokens.insert(role, tokens);
            }
            UiEvent::CoderContexts { coders } => self.bar.coders = coders,
            UiEvent::Rotated { reason } => {
                self.bar.rot += 1;
                self.transcript.push(Entry::Notice(reason));
            }
            UiEvent::Notice { text } => self.transcript.push(Entry::Notice(text)),
            UiEvent::Error(text) => self.transcript.push(Entry::Error(text)),
            UiEvent::PermissionRequest { id, tool, summary } => {
                self.permission = Some(Permission { id, tool, summary });
            }
            UiEvent::TaskQueued { id, desc } => self.tasks.push(Task {
                id,
                desc,
                status: None,
                tokens: None,
                duration_ms: None,
            }),
            UiEvent::TaskResolved { id, status } => {
                if let Some(t) = self.task_mut(&id) {
                    t.status = Some(status);
                }
            }
            UiEvent::TaskStats {
                id,
                tokens,
                duration_ms,
            } => {
                if let Some(t) = self.task_mut(&id) {
                    t.tokens = tokens;
                    t.duration_ms = duration_ms;
                }
            }
            _ => {}
        }
    }

    // Status-bar input at a given time. now_unix is Unix seconds (agent elapsed time).
    pub fn bar_at(&self, now: Instant, now_unix: u64) -> BarInput {
        let mut bar = self.bar.clone();
        bar.turn_secs = self
            .busy_since
            .map(|t| now.saturating_duration_since(t).as_secs());
        let running: Vec<&AgentRow> = self.agent_rows.iter().filter(|r| !r.done).collect();
        bar.fg = running.iter().filter(|r| !r.background).count();
        bar.bg = running.iter().filter(|r| r.background).count();
        bar.agents = running
            .iter()
            .map(|r| (r.task.clone(), now_unix.saturating_sub(r.started_at)))
            .collect();
        bar
    }

    // Takes the open permission prompt and returns (id, allow) for EngineHandle::answer_permission.
    pub fn answer_permission(&mut self, allow: bool) -> Option<(String, bool)> {
        self.permission.take().map(|p| (p.id, allow))
    }

    fn open_text_index(&self, role: &Role) -> Option<usize> {
        self.transcript
            .iter()
            .rposition(|e| matches!(e, Entry::Text { role: r, done: false, .. } if r == role))
    }

    // The open text row of this role, started when none is open.
    fn open_text(&mut self, role: &Role) -> usize {
        self.open_text_index(role).unwrap_or_else(|| {
            self.transcript.push(Entry::Text {
                role: role.clone(),
                text: String::new(),
                done: false,
            });
            self.transcript.len() - 1
        })
    }

    fn find_agent(&mut self, id: &str) -> Option<&mut Entry> {
        self.transcript
            .iter_mut()
            .rev()
            .find(|e| matches!(e, Entry::Agent { id: i, .. } if i == id))
    }

    fn task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn text(role: Role, s: &str) -> UiEvent {
        UiEvent::TextDelta {
            role,
            text: s.into(),
        }
    }

    #[test]
    fn deltas_join_until_message_done_then_a_new_row_starts() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(text(Role::Main, "he"), now);
        st.apply(text(Role::Main, "llo"), now);
        st.apply(UiEvent::MessageDone { role: Role::Main }, now);
        st.apply(text(Role::Main, "next"), now);
        assert_eq!(
            st.transcript,
            [
                Entry::Text {
                    role: Role::Main,
                    text: "hello".into(),
                    done: true
                },
                Entry::Text {
                    role: Role::Main,
                    text: "next".into(),
                    done: false
                },
            ]
        );
    }

    #[test]
    fn agent_text_is_its_own_open_row() {
        let mut st = GuiState::default();
        let now = Instant::now();
        let agent = Role::Agent("t1".into());
        st.apply(text(Role::Main, "a"), now);
        st.apply(text(agent.clone(), "b"), now);
        st.apply(text(Role::Main, "c"), now);
        assert_eq!(st.transcript.len(), 2);
        assert_eq!(
            st.transcript[0],
            Entry::Text {
                role: Role::Main,
                text: "ac".into(),
                done: false
            }
        );
        st.apply(UiEvent::MessageDone { role: agent }, now);
        assert!(matches!(&st.transcript[1], Entry::Text { done: true, .. }));
    }

    #[test]
    fn tool_end_sets_ok_on_the_matching_row() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::ToolStart {
                id: "a".into(),
                name: "Read".into(),
                summary: "x.rs".into(),
            },
            now,
        );
        st.apply(
            UiEvent::ToolStart {
                id: "b".into(),
                name: "Bash".into(),
                summary: "ls".into(),
            },
            now,
        );
        st.apply(
            UiEvent::ToolEnd {
                id: "a".into(),
                ok: false,
            },
            now,
        );
        assert!(matches!(
            &st.transcript[0],
            Entry::Tool {
                ok: Some(false),
                ..
            }
        ));
        assert!(matches!(&st.transcript[1], Entry::Tool { ok: None, .. }));
    }

    #[test]
    fn agent_end_marks_done_and_keeps_the_summary() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::AgentStart {
                id: "g1".into(),
                description: "scan".into(),
                started_at: 0,
            },
            now,
        );
        st.apply(
            UiEvent::AgentEnd {
                id: "g1".into(),
                elapsed_ms: 5,
                model: String::new(),
                name: String::new(),
                summary: "found 3".into(),
            },
            now,
        );
        assert_eq!(
            st.transcript[0],
            Entry::Agent {
                id: "g1".into(),
                description: "scan".into(),
                done: true,
                summary: "found 3".into()
            }
        );
    }

    #[test]
    fn busy_starts_and_ends_the_turn_timer() {
        let mut st = GuiState::default();
        let t0 = Instant::now();
        st.apply(UiEvent::Busy(true), t0);
        let bar = st.bar_at(t0 + Duration::from_secs(7), 0);
        assert_eq!(bar.turn_secs, Some(7));
        st.apply(UiEvent::TurnEnd { duration_ms: 1 }, t0);
        assert_eq!(st.bar_at(t0, 0).turn_secs, None);
        assert!(!st.busy);
    }

    #[test]
    fn status_and_bar_fields_follow_their_events() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::Status {
                model: "opus".into(),
                tokens: 1200,
                cost_usd: 0.5,
                context_pct: 3,
            },
            now,
        );
        st.apply(UiEvent::Queued(2), now);
        st.apply(
            UiEvent::ModeChanged {
                mode: "plan".into(),
            },
            now,
        );
        st.apply(UiEvent::ReplyHeld { pending: 1 }, now);
        st.apply(
            UiEvent::RoleContext {
                role: "P".into(),
                tokens: 9,
            },
            now,
        );
        st.apply(
            UiEvent::Rotated {
                reason: "limit".into(),
            },
            now,
        );
        let bar = st.bar_at(now, 0);
        assert_eq!(bar.model, "opus");
        assert_eq!((bar.tokens, bar.context_pct), (1200, 3));
        assert_eq!(bar.queued, 2);
        assert_eq!(bar.mode, "plan");
        assert_eq!(bar.waiting, Some(1));
        assert_eq!(bar.role_tokens.get("P"), Some(&9));
        assert_eq!(bar.rot, 1);
        st.apply(UiEvent::ReplyReleased, now);
        assert_eq!(st.bar_at(now, 0).waiting, None);
    }

    #[test]
    fn agent_rows_split_into_foreground_and_background_counts() {
        let mut st = GuiState::default();
        let now = Instant::now();
        let row = |id: &str, bg: bool, done: bool| AgentRow {
            id: id.into(),
            task: format!("task {id}"),
            prompt: String::new(),
            name: String::new(),
            model: String::new(),
            background: bg,
            done,
            failed: false,
            interrupted: false,
            agent_id: String::new(),
            result_path: String::new(),
            started_at: 100,
            last_active: 0,
        };
        st.apply(
            UiEvent::AgentRows {
                rows: vec![
                    row("a", false, false),
                    row("b", true, false),
                    row("c", true, true),
                ],
            },
            now,
        );
        let bar = st.bar_at(now, 130);
        assert_eq!((bar.fg, bar.bg), (1, 1));
        assert_eq!(
            bar.agents,
            [("task a".to_string(), 30), ("task b".to_string(), 30)]
        );
    }

    #[test]
    fn permission_request_is_answered_once() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::PermissionRequest {
                id: "p1".into(),
                tool: "Bash".into(),
                summary: "rm x".into(),
            },
            now,
        );
        assert_eq!(
            st.permission.as_ref().map(|p| p.tool.as_str()),
            Some("Bash")
        );
        assert_eq!(st.answer_permission(false), Some(("p1".into(), false)));
        assert_eq!(st.answer_permission(true), None);
    }

    #[test]
    fn task_events_fill_one_row() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::TaskQueued {
                id: "t".into(),
                desc: "build".into(),
            },
            now,
        );
        st.apply(
            UiEvent::TaskStats {
                id: "t".into(),
                tokens: Some(4000),
                duration_ms: Some(2000),
            },
            now,
        );
        st.apply(
            UiEvent::TaskResolved {
                id: "t".into(),
                status: TaskStatus::Done,
            },
            now,
        );
        assert_eq!(
            st.tasks,
            [Task {
                id: "t".into(),
                desc: "build".into(),
                status: Some(TaskStatus::Done),
                tokens: Some(4000),
                duration_ms: Some(2000),
            }]
        );
    }

    #[test]
    fn errors_and_user_prompts_go_to_the_transcript_and_history() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.push_user("hi");
        st.push_user("");
        st.push_user("hi");
        st.apply(UiEvent::Error("boom".into()), now);
        assert_eq!(
            st.transcript,
            [
                Entry::User("hi".into()),
                Entry::User("".into()),
                Entry::User("hi".into()),
                Entry::Error("boom".into()),
            ]
        );
        assert_eq!(st.history, ["hi"]);
    }

    // The GUI reads and writes <cwd>/.chaude/history.jsonl like the TUI: trimmed, saved, loaded at start.
    #[test]
    fn prompt_history_is_saved_and_loaded_from_the_cwd() {
        let dir = std::env::temp_dir().join(format!("chaude-gui-history-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut st = GuiState::default();
        st.load_history(&dir);
        st.push_user("  one  ");
        st.push_user("two");
        st.push_user("two");
        let mut again = GuiState::default();
        again.load_history(&dir);
        assert_eq!(again.history, ["one", "two"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
