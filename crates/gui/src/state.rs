//! Pure GUI state: applies core UiEvents to the transcript, the status-bar data and the open permission prompt.
//! No egui types here, so the reducer is unit-tested without a window.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use chaude_core::api::{AgentRow, Role, UiEvent};
use chaude_core::bar_text;
use chaude_core::handoff;
use chaude_core::i18n::{self, Msg};
use chaude_core::localtime::{clock_hhmmss, unix_now};
use chaude_core::resident::StagePhase;
use chaude_core::settings::pickers::{LangPicker, SettingsPicker};
use chaude_core::ui_state::{
    BarInput, Entry, RunState, Transcript, stage_line, stage_text, tool_result_text, tool_summary,
};
use chaude_core::usage_text;

// A can_use_tool request waiting for the user (UiEvent::PermissionRequest).
#[derive(Debug, Clone, PartialEq)]
pub struct Permission {
    pub id: String,
    pub tool: String,
    pub summary: String,
}

// An open stage row (Working) waiting for its Done or Failed; idx is its transcript index.
#[derive(Debug, Clone)]
struct StageRow {
    idx: usize,
    text: String,
    start: i64,
    at: Instant,
}

#[derive(Debug, Default)]
pub struct GuiState {
    pub transcript: Transcript,
    // Status-bar data. turn_secs and the agent list are derived in bar_at, not stored here.
    pub bar: BarInput,
    // Engine progress shared with the TUI: busy turn, agent rows, tasks, rotations (core ui_state::RunState).
    pub run: RunState,
    pub permission: Option<Permission>,
    // Working directory shown in the orphan notice (set by the app at start).
    pub cwd: String,
    // Session id the next rotation replaces (UiEvent::RotatedFrom), read by the Rotated notice.
    rot_from: String,
    // Open stage rows by resident slot.
    stage_rows: HashMap<String, StageRow>,
    // Agents whose tool call ended in error (ToolEnd ok=false), by tool id until their AgentEnd (TUI agent_outcome).
    agent_failed: HashSet<String>,
    // Main session id (UiEvent::SessionIds); empty after a rotation until the new one. /status shows it.
    pub session_id: String,
    // The /usage window text while it is open (None = closed; the loading text until the one-shot run answers).
    pub usage: Option<String>,
    // True from open until the one-shot run answers; its failure then goes into the window (TUI usage.pending).
    usage_pending: bool,
    // The open /lang picker (None = closed). Keys and the apply live in the app; the model is core (settings::pickers).
    pub lang: Option<LangPicker>,
    // The open /settings picker over a working copy (None = closed). Closing saves it (core settings::pickers).
    pub settings: Option<SettingsPicker>,
}

impl GuiState {
    // Echoes a prompt the user sent. Prompt history is kept by the input box (crate::input, core Editor).
    pub fn push_user(&mut self, text: &str) {
        self.transcript.push_user(text);
    }

    // Opens the /usage window with the loading text (TUI usage.open). The one-shot run answers through apply.
    pub fn open_usage(&mut self) {
        self.usage = Some(i18n::t(Msg::UsageLoading).to_string());
        self.usage_pending = true;
    }

    // Closes the /usage window; a late answer then goes to the transcript as before (TUI usage.close).
    pub fn close_usage(&mut self) {
        self.usage = None;
        self.usage_pending = false;
    }

    // Opens the /lang picker on the language in use (TUI LangPicker::open).
    pub fn open_lang(&mut self) {
        self.lang = Some(LangPicker::open());
    }

    // Moves the /lang cursor up or down (no-op while closed).
    pub fn lang_step(&mut self, up: bool) {
        if let Some(p) = self.lang.as_mut() {
            p.step(up);
        }
    }

    // Closes the /lang picker without a pick (TUI Key::Cancel).
    pub fn close_lang(&mut self) {
        self.lang = None;
    }

    // Opens the /settings picker on the project settings of cwd (TUI settings::Picker::open). Only reads the file.
    pub fn open_settings(&mut self, cwd: Option<&Path>) {
        self.settings = Some(SettingsPicker::open(cwd));
    }

    // Moves the /settings cursor up or down (no-op while closed).
    pub fn settings_step(&mut self, up: bool) {
        if let Some(p) = self.settings.as_mut() {
            p.step(up);
        }
    }

    // Steps the value under the /settings cursor by dir (+1 or -1), wrapping (no-op while closed).
    pub fn settings_change(&mut self, dir: i64) {
        if let Some(p) = self.settings.as_mut() {
            p.change(dir);
        }
    }

    // Saves the /settings working copy and closes the picker (TUI close). Returns the note to append; None when closed.
    pub fn close_settings(&mut self) -> Option<String> {
        self.settings.take().map(|mut p| p.close())
    }

    // Picks a row (Some(index) for 1/2 or a click, None for Enter on the cursor row) and closes the picker.
    // Returns the settings code to apply; None when the picker was closed.
    pub fn lang_pick(&mut self, row: Option<usize>) -> Option<&'static str> {
        let mut p = self.lang.take()?;
        if let Some(i) = row {
            p.sel = i;
        }
        Some(p.picked())
    }

    // Applies one engine event. now is the time it arrived (turn timer start).
    pub fn apply(&mut self, ev: UiEvent, now: Instant) {
        match ev {
            UiEvent::TextDelta { role, text } => {
                let i = self.open_text(&role);
                if let Entry::Text { text: t, .. } = &mut self.transcript.entries[i] {
                    t.push_str(&text);
                }
            }
            UiEvent::MessageDone { role } => {
                if let Some(i) = self.open_text_index(&role)
                    && let Entry::Text { done, .. } = &mut self.transcript.entries[i]
                {
                    *done = true;
                }
            }
            UiEvent::ToolStart { id, name, summary } => {
                self.transcript.push_tool(id, name, summary, unix_now());
            }
            // The input JSON gives the row its argument summary (TUI ToolInput, tool_summary).
            UiEvent::ToolInput { id, input } => {
                if let Some(Entry::Tool { summary, .. }) = self.find_tool(&id) {
                    *summary = tool_summary(&input);
                }
            }
            // The tool row gets its end time and elapsed seconds, as the TUI stamps them.
            UiEvent::ToolEnd { id, ok } => {
                let now_unix = unix_now();
                if !ok && self.find_agent(&id).is_some() {
                    self.agent_failed.insert(id.clone());
                }
                if let Some(Entry::Tool {
                    ok: slot,
                    start,
                    end,
                    ..
                }) = self.find_tool(&id)
                {
                    *slot = Some(ok);
                    if end.is_none() {
                        *end = Some((now_unix, (now_unix - *start).max(0) as u64));
                    }
                }
            }
            // Tool result under its call (first line, the TUI rule); a caveman return is hidden.
            UiEvent::ToolResult { id, text } => {
                if let Some(head) = tool_result_text(&text) {
                    let rows = vec![
                        Entry::ToolResult {
                            id: id.clone(),
                            text: head,
                            at: unix_now(),
                        },
                        Entry::Blank,
                    ];
                    let at = self
                        .transcript
                        .entries
                        .iter()
                        .rposition(|e| matches!(e, Entry::Tool { id: i, .. } if *i == id))
                        .map_or(self.transcript.len(), |i| i + 1);
                    self.insert_rows(at, rows);
                }
            }
            // Resident stage: a Working row is stamped and closed on Done or Failed, then a note line (TUI stage()).
            UiEvent::Stage { slot, phase } => {
                let text = stage_text(i18n::current(), &slot, &phase);
                match &phase {
                    StagePhase::Working => {
                        let start = unix_now();
                        let idx = self.transcript.len();
                        self.transcript
                            .push(Entry::Notice(stage_line(start, &text, None)));
                        self.stage_rows.insert(
                            slot,
                            StageRow {
                                idx,
                                text,
                                start,
                                at: now,
                            },
                        );
                    }
                    StagePhase::Done | StagePhase::Failed(_) => {
                        if let Some(r) = self.stage_rows.remove(&slot) {
                            let end = (unix_now(), now.saturating_duration_since(r.at).as_secs());
                            if let Some(Entry::Notice(line)) =
                                self.transcript.entries.get_mut(r.idx)
                            {
                                *line = stage_line(r.start, &r.text, Some(end));
                            }
                        }
                        self.transcript.push_notice(text);
                    }
                }
                self.transcript.push_blank();
            }
            UiEvent::Interrupted => {
                self.transcript.push_status("[interrupted]");
                self.transcript.push_blank();
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
            // The agent row is marked done, then its completion notice follows with the local arrival time (TUI agent_done).
            UiEvent::AgentEnd {
                id,
                model,
                name,
                summary,
                ..
            } => {
                let failed = self.agent_failed.remove(&id);
                if let Some(Entry::Agent {
                    done, summary: s, ..
                }) = self.find_agent(&id)
                {
                    *done = true;
                    if !summary.is_empty() {
                        *s = summary.clone();
                    }
                }
                let line = bar_text::done_line(i18n::current(), failed, &model, &name, &summary);
                self.transcript
                    .push_notice(format!("{} {line}", clock_hhmmss(unix_now())));
            }
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
            // Busy keeps the start of a turn already running and clears the last tool (core RunState::set_busy, shared with the TUI).
            UiEvent::Busy(on) => self.run.set_busy(on, now),
            // TurnEnd ends the turn only; a held reply is cleared by ReplyReleased, as before.
            // The engine drains the queue into the next turn: the merged prompt is echoed once here and becomes the
            // turn's label (the TUI's QueueTexts arm).
            UiEvent::QueueTexts(texts) => {
                let drained = std::mem::replace(&mut self.run.queue_texts, texts);
                if !drained.is_empty() && self.run.queue_texts.is_empty() && self.run.busy {
                    let merged = drained.join("\n\n");
                    self.run.turn_prompt = merged
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .chars()
                        .take(60)
                        .collect();
                    self.push_user(&merged);
                }
            }
            UiEvent::TurnEnd { .. } => self.run.set_busy(false, now),
            UiEvent::Queued(n) => self.bar.queued = n,
            UiEvent::ModeChanged { mode } => {
                self.transcript.push_status(format!("mode: {mode}"));
                self.transcript.push_blank();
                self.bar.mode = mode;
            }
            UiEvent::TurnTokens {
                input,
                output,
                cache,
            } => self.bar.turn_tokens = Some((input, output, cache)),
            UiEvent::RoleContext { role, tokens } => {
                self.bar.role_tokens.insert(role, tokens);
            }
            UiEvent::CoderContexts { coders } => self.bar.coders = coders,
            // Go repl.go setRate: the rate text is recomputed on every rate_limit_event (shared with the TUI).
            UiEvent::RateLimit { info } => {
                self.bar.rate = bar_text::rate_status(&info, unix_now()).filter(|s| !s.is_empty());
            }
            UiEvent::SessionIds { main, .. } => self.session_id = main,
            UiEvent::RotatedFrom { old_session } => {
                self.session_id.clear();
                self.rot_from = old_session;
            }
            // The /usage answer fills the open window (TUI UiEvent::Usage); a late answer after close is dropped.
            UiEvent::Usage { text } if self.usage_pending => {
                self.usage_pending = false;
                let text = if text.trim().is_empty() {
                    i18n::t(Msg::UsageEmpty).to_string()
                } else {
                    usage_text::decorate_usage(&text, self.bar.tokens as i64)
                };
                self.usage = Some(text);
            }
            UiEvent::Usage { .. } => {}
            UiEvent::Rotated { reason } => {
                self.run.rot += 1;
                let old = std::mem::take(&mut self.rot_from);
                let lang = i18n::current();
                let text = bar_text::rotated_notice(lang, self.run.rot, &reason, &old);
                self.transcript.push_notice(text);
            }
            // A claude child a dead harness left running: an empty model falls back to the main model or the role name.
            UiEvent::OrphanNotice { role, pid, model } => {
                let model = if !model.is_empty() {
                    model
                } else if role == handoff::MAIN_ROLE {
                    self.bar.model.clone()
                } else {
                    role
                };
                let text = bar_text::orphan_notice(i18n::current(), pid, &model, &self.cwd);
                self.transcript.push_notice(text);
            }
            UiEvent::BudgetStop { spent_usd, cap_usd } => {
                let text = bar_text::budget_notice(i18n::current(), spent_usd, cap_usd);
                self.transcript.push_notice(text);
            }
            UiEvent::Notice { text } => self.transcript.push_notice(text),
            UiEvent::Error(text) if self.usage_pending => {
                self.usage_pending = false;
                self.usage = Some(usage_text::usage_failed(&text));
            }
            UiEvent::Error(text) => self.transcript.push_error(text),
            UiEvent::PermissionRequest { id, tool, summary } => {
                self.permission = Some(Permission { id, tool, summary });
            }
            // Agent rows, tasks, opus/resident state and the rest of the progress events fold into the run state.
            ev => self.run.apply(ev, now),
        }
    }

    // Status-bar input at a given time. now_unix is Unix seconds (agent elapsed time).
    pub fn bar_at(&self, now: Instant, now_unix: u64) -> BarInput {
        let mut bar = self.bar.clone();
        bar.turn_secs = self
            .run
            .turn_start
            .map(|t| now.saturating_duration_since(t).as_secs());
        bar.rot = self.run.rot;
        bar.waiting = self.run.held;
        let running: Vec<&AgentRow> = self.run.agent_rows.iter().filter(|r| !r.done).collect();
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
            .entries
            .iter()
            .rposition(|e| matches!(e, Entry::Text { role: r, done: false, .. } if r == role))
    }

    // The open text row of this role, started when none is open.
    fn open_text(&mut self, role: &Role) -> usize {
        self.open_text_index(role).unwrap_or_else(|| {
            self.transcript.push_text(role.clone(), "", false);
            self.transcript.len() - 1
        })
    }

    fn find_agent(&mut self, id: &str) -> Option<&mut Entry> {
        self.transcript
            .entries
            .iter_mut()
            .rev()
            .find(|e| matches!(e, Entry::Agent { id: i, .. } if i == id))
    }

    fn find_tool(&mut self, id: &str) -> Option<&mut Entry> {
        self.transcript
            .entries
            .iter_mut()
            .rev()
            .find(|e| matches!(e, Entry::Tool { id: i, .. } if i == id))
    }

    // Inserts rows at a transcript index and moves open stage rows that sit after it.
    fn insert_rows(&mut self, at: usize, rows: Vec<Entry>) {
        let n = rows.len();
        for r in self.stage_rows.values_mut() {
            if r.idx >= at {
                r.idx += n;
            }
        }
        self.transcript.entries.splice(at..at, rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use chaude_core::api::TaskStatus;
    use chaude_core::ui_state::{TaskRow, TaskState};

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
            st.transcript.entries,
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
            st.transcript.entries[0],
            Entry::Text {
                role: Role::Main,
                text: "ac".into(),
                done: false
            }
        );
        st.apply(UiEvent::MessageDone { role: agent }, now);
        assert!(matches!(
            &st.transcript.entries[1],
            Entry::Text { done: true, .. }
        ));
    }

    #[test]
    fn tool_rows_are_stamped_and_closed_with_an_end_time() {
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
            UiEvent::ToolEnd {
                id: "a".into(),
                ok: true,
            },
            now,
        );
        match &st.transcript.entries[0] {
            Entry::Tool {
                name,
                summary,
                ok,
                start,
                end,
                ..
            } => {
                assert_eq!(
                    (name.as_str(), summary.as_str(), *ok),
                    ("Read", "x.rs", Some(true))
                );
                assert!(*start > 0);
                let (at, secs) = end.expect("end stamped");
                assert!(at >= *start && secs == (at - *start) as u64);
            }
            other => panic!("expected Tool, got {other:?}"),
        }
    }

    #[test]
    fn tool_result_follows_its_tool_and_hides_caveman_returns() {
        let mut st = GuiState::default();
        let now = Instant::now();
        let start = |id: &str| UiEvent::ToolStart {
            id: id.into(),
            name: "Bash".into(),
            summary: "ls".into(),
        };
        st.apply(start("a"), now);
        st.apply(start("b"), now);
        st.apply(
            UiEvent::ToolResult {
                id: "a".into(),
                text: "one\ntwo".into(),
            },
            now,
        );
        st.apply(
            UiEvent::ToolResult {
                id: "b".into(),
                text: "status: done\nreason: -\nclaim: a.go:1 | x".into(),
            },
            now,
        );
        assert!(
            matches!(&st.transcript.entries[1], Entry::ToolResult { id, text, .. }
            if id == "a" && text == "one … (+1 lines)")
        );
        assert_eq!(st.transcript.entries[2], Entry::Blank);
        // The caveman return adds no row: only the two tool rows and the first result pair remain.
        assert_eq!(st.transcript.len(), 4);
    }

    #[test]
    fn stage_row_is_closed_by_done_and_keeps_its_slot_index() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::ToolStart {
                id: "a".into(),
                name: "Read".into(),
                summary: "x".into(),
            },
            now,
        );
        st.apply(
            UiEvent::Stage {
                slot: "agent:haiku-coder".into(),
                phase: StagePhase::Working,
            },
            now,
        );
        // A tool result inserted before the stage row moves its index.
        st.apply(
            UiEvent::ToolResult {
                id: "a".into(),
                text: "ok".into(),
            },
            now,
        );
        st.apply(
            UiEvent::Stage {
                slot: "agent:haiku-coder".into(),
                phase: StagePhase::Done,
            },
            now,
        );
        let lang = i18n::current();
        let done = stage_text(lang, "agent:haiku-coder", &StagePhase::Done);
        assert_eq!(st.transcript.entries.last(), Some(&Entry::Blank));
        assert_eq!(
            st.transcript.entries[st.transcript.len() - 2],
            Entry::Notice(done)
        );
        // The working row (moved to index 3 by the result pair) is rewritten with an end stamp.
        match &st.transcript.entries[3] {
            Entry::Notice(line) => assert!(line.starts_with("· "), "{line}"),
            other => panic!("expected stage Notice, got {other:?}"),
        }
        assert!(st.stage_rows.is_empty());
    }

    #[test]
    fn interrupted_and_mode_rows_are_status_lines_with_a_blank() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::ModeChanged {
                mode: "auto".into(),
            },
            now,
        );
        st.apply(UiEvent::Interrupted, now);
        assert_eq!(
            st.transcript.entries,
            [
                Entry::Status("mode: auto".into()),
                Entry::Blank,
                Entry::Status("[interrupted]".into()),
                Entry::Blank,
            ]
        );
        assert_eq!(st.bar.mode, "auto");
    }

    #[test]
    fn rotation_orphan_and_budget_notices_use_the_shared_text() {
        let mut st = GuiState::default();
        let now = Instant::now();
        let lang = i18n::current();
        st.apply(
            UiEvent::RotatedFrom {
                old_session: "s1".into(),
            },
            now,
        );
        st.apply(
            UiEvent::Rotated {
                reason: "context".into(),
            },
            now,
        );
        st.cwd = "/work".into();
        st.apply(
            UiEvent::OrphanNotice {
                role: "main".into(),
                pid: 42,
                model: String::new(),
            },
            now,
        );
        st.apply(
            UiEvent::BudgetStop {
                spent_usd: 2.0,
                cap_usd: 1.0,
            },
            now,
        );
        assert_eq!(st.run.rot, 1);
        assert_eq!(
            st.transcript.entries,
            [
                Entry::Notice(bar_text::rotated_notice(lang, 1, "context", "s1")),
                Entry::Notice(bar_text::orphan_notice(lang, 42, "", "/work")),
                Entry::Notice(bar_text::budget_notice(lang, 2.0, 1.0)),
            ]
        );
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
            &st.transcript.entries[0],
            Entry::Tool {
                ok: Some(false),
                ..
            }
        ));
        assert!(matches!(
            &st.transcript.entries[1],
            Entry::Tool { ok: None, .. }
        ));
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
            st.transcript.entries[0],
            Entry::Agent {
                id: "g1".into(),
                description: "scan".into(),
                done: true,
                summary: "found 3".into()
            }
        );
    }

    #[test]
    fn tool_input_sets_the_summary_and_agent_end_writes_a_done_notice() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::ToolStart {
                id: "t1".into(),
                name: "Bash".into(),
                summary: String::new(),
            },
            now,
        );
        st.apply(
            UiEvent::ToolInput {
                id: "t1".into(),
                input: serde_json::json!({"command": "ls\n-la"}),
            },
            now,
        );
        assert!(
            matches!(&st.transcript.entries[0], Entry::Tool { summary, .. } if summary == "ls -la")
        );

        st.apply(
            UiEvent::AgentStart {
                id: "a1".into(),
                description: "scan".into(),
                started_at: 0,
            },
            now,
        );
        st.apply(
            UiEvent::ToolEnd {
                id: "a1".into(),
                ok: false,
            },
            now,
        );
        st.apply(
            UiEvent::AgentEnd {
                id: "a1".into(),
                elapsed_ms: 5,
                model: "haiku".into(),
                name: "scan".into(),
                summary: "boom".into(),
            },
            now,
        );
        let Some(Entry::Notice(line)) = st.transcript.entries.last() else {
            panic!("agent end writes a notice");
        };
        assert!(line.ends_with(&bar_text::done_line(
            i18n::current(),
            true,
            "haiku",
            "scan",
            "boom"
        )));
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
        assert!(!st.run.busy);
    }

    #[test]
    fn rate_limit_event_sets_the_rate_window_text() {
        let mut st = GuiState::default();
        let info = serde_json::json!({"unifiedWindows": {"five_hour": {"utilization": 0.37}}});
        st.apply(UiEvent::RateLimit { info }, Instant::now());
        let rate = st.bar.rate.clone().expect("rate text");
        assert!(rate.ends_with(" · 37%"), "{rate}");
        let text = bar_text::bottom_text(i18n::Lang::En, &st.bar_at(Instant::now(), 0));
        assert!(text.contains(&rate), "{text}");
    }

    // The merged queue is echoed once, when the engine drains the queue into a busy turn (the TUI's QueueTexts arm).
    #[test]
    fn queued_prompts_echo_once_when_the_turn_takes_them() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(UiEvent::Busy(true), now);
        st.apply(UiEvent::QueueTexts(vec!["one".into(), "two".into()]), now);
        assert_eq!(st.run.queue_texts.len(), 2);
        assert_eq!(user_echoes(&st), 0);
        st.apply(UiEvent::QueueTexts(vec![]), now);
        assert_eq!(user_echoes(&st), 1);
        assert_eq!(st.run.turn_prompt, "one");
        st.apply(UiEvent::QueueTexts(vec![]), now);
        assert_eq!(user_echoes(&st), 1);
    }

    fn user_echoes(st: &GuiState) -> usize {
        st.transcript
            .entries
            .iter()
            .filter(|e| matches!(e, chaude_core::ui_state::Entry::User(_)))
            .count()
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
        let rows: &[TaskRow] = &st.run.tasks;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "t");
        assert_eq!(rows[0].desc, "build");
        assert_eq!(rows[0].state, TaskState::Done);
        assert_eq!(rows[0].tokens, Some(4000));
        assert_eq!(rows[0].duration_ms, Some(2000));
    }

    #[test]
    fn errors_and_user_prompts_go_to_the_transcript() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.push_user("hi");
        st.push_user("");
        st.push_user("hi");
        st.apply(UiEvent::Error("boom".into()), now);
        assert_eq!(
            st.transcript.entries,
            [
                Entry::User("hi".into()),
                Entry::User("".into()),
                Entry::User("hi".into()),
                Entry::Error("boom".into()),
            ]
        );
    }

    #[test]
    fn usage_window_opens_loading_then_takes_the_answer_once() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.open_usage();
        assert_eq!(st.usage.as_deref(), Some(i18n::t(Msg::UsageLoading)));
        st.apply(
            UiEvent::Usage {
                text: "Current session: 45% used".into(),
            },
            now,
        );
        assert!(st.usage.as_deref().is_some_and(|t| t.contains("45%")));
        st.apply(
            UiEvent::Usage {
                text: "late".into(),
            },
            now,
        );
        assert!(!st.usage.as_deref().unwrap_or("").contains("late"));
        st.close_usage();
        assert_eq!(st.usage, None);
        assert!(
            st.transcript.entries.is_empty(),
            "a closed window adds no notice"
        );
    }

    #[test]
    fn lang_window_steps_picks_and_closes() {
        let mut st = GuiState::default();
        st.lang_step(true);
        assert!(st.lang.is_none(), "stepping while closed does nothing");
        st.open_lang();
        let start = st.lang.as_ref().map(|p| p.sel);
        st.lang_step(false);
        assert_eq!(st.lang.as_ref().map(|p| p.sel), start.map(|s| (s + 1) % 2));
        assert_eq!(st.lang_pick(Some(1)), Some("en"));
        assert!(st.lang.is_none(), "a pick closes the window");
        assert_eq!(st.lang_pick(Some(0)), None, "no pick while closed");
        st.open_lang();
        st.close_lang();
        assert!(st.lang.is_none(), "esc closes without a pick");
    }

    #[test]
    fn settings_window_steps_changes_and_closes_once() {
        let mut st = GuiState::default();
        st.settings_step(true);
        assert!(st.settings.is_none(), "stepping while closed does nothing");
        // A missing file loads the defaults and nothing is written.
        st.open_settings(Some(Path::new("/nonexistent/chaude-gui-test")));
        assert_eq!(st.settings.as_ref().map(|p| p.sel), Some(0));
        st.settings_step(true);
        assert_eq!(st.settings.as_ref().map(|p| p.sel), Some(2), "up wraps");
        let before = st.settings.as_ref().map(|p| p.s.main_model.clone());
        st.settings_change(1);
        assert_ne!(st.settings.as_ref().map(|p| p.s.main_model.clone()), before);
        // The path is cleared so closing never writes a settings file from a test.
        if let Some(p) = st.settings.as_mut() {
            p.path = None;
        }
        assert!(st.close_settings().is_some(), "close returns the note");
        assert!(st.settings.is_none());
        assert_eq!(st.close_settings(), None, "a second close does nothing");
    }

    #[test]
    fn usage_failure_goes_into_the_open_window() {
        let mut st = GuiState::default();
        st.open_usage();
        st.apply(UiEvent::Error("boom".into()), Instant::now());
        assert_eq!(st.usage, Some(usage_text::usage_failed("boom")));
        assert!(st.transcript.entries.is_empty());
    }
}
