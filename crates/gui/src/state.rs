//! Pure GUI state: applies core UiEvents to the transcript, the status-bar data and the open permission prompt.
//! No egui types here, so the reducer is unit-tested without a window.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chaude_core::api::{AgentRow, Role, UiEvent};
use chaude_core::bar_text;
use chaude_core::group_pick::{self, GroupPicker};
use chaude_core::handoff;
use chaude_core::i18n::{self, Msg};
use chaude_core::import_claude::{self, ClaudeImport};
use chaude_core::localtime::{clock_hhmmss, unix_now};
use chaude_core::osenv;
use chaude_core::record;
use chaude_core::record_folder;
use chaude_core::record_pick::{self, Picker};
use chaude_core::record_turn;
use chaude_core::resident::StagePhase;
use chaude_core::rewind::{NOTHING, REWOUND, Rewind};
use chaude_core::session_view::{self, SessView};
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

// The copy note lasts 2s (TUI COPY_NOTE_TTL in cli tui/mod.rs; the TUI keeps its own copy).
pub const COPY_NOTE_TTL: Duration = Duration::from_secs(2);

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
    // The copy note after a drag-select copy in the output (TUI copy_note): its text and when it was set.
    copy_note: Option<(String, Instant)>,
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
    // The open Esc Esc rewind picker (TUI m.rw). Its list, selection and cut are the core model (chaude_core::rewind).
    pub rewind: Rewind,
    // The open /resume record picker (TUI m.recs). Its folder list and cursor are the core model (chaude_core::record_pick).
    pub records: Option<Picker>,
    // The open /resume session group picker (TUI pick). Its list and cursor are the core model (chaude_core::group_pick).
    pub groups: Option<GroupPicker>,
    // The session id the previous run did not finish, read by the app before the engine starts (TUI killed_sid).
    // A bare /resume imports it.
    pub killed_sid: Option<String>,
    // Which mode the previous session loads with (TUI session_mode), noted at start and by /view.
    pub session_mode: Option<String>,
    // The open /view browser (TUI sess): the session list, then one session's prompts. The model is core (session_view).
    pub sess: Option<SessView>,
    // Sessions this run owns (SessionIds; TUI this_run) and their purposes (TUI session_purpose).
    this_run: HashSet<String>,
    session_purpose: HashMap<String, String>,
    // Where /view finds the Claude Code projects: None = the user's home (tests set a temp dir).
    pub home: Option<PathBuf>,
}

// The transcript as the core rewind model reads it: each user prompt is a "user" block, the rest carry no text.
fn rewind_blocks(t: &Transcript) -> Vec<(&str, &str)> {
    t.entries
        .iter()
        .map(|e| match e {
            Entry::User(s) => ("user", s.as_str()),
            _ => ("", ""),
        })
        .collect()
}

impl GuiState {
    // Echoes a prompt the user sent. Prompt history is kept by the input box (crate::input, core Editor).
    pub fn push_user(&mut self, text: &str) {
        self.transcript.push_user(text);
    }

    // Opens the rewind picker over the transcript's prompts (TUI rewind open). With no prompt it notes that there is
    // nothing to rewind to.
    pub fn open_rewind(&mut self) {
        if !self.rewind.open(&rewind_blocks(&self.transcript)) {
            self.transcript.push_notice(NOTHING);
        }
    }

    // Moves the rewind cursor up or down (no-op while closed).
    pub fn rewind_step(&mut self, up: bool) {
        self.rewind.move_by(if up { -1 } else { 1 });
    }

    // Closes the rewind picker without a cut (TUI Cancel).
    pub fn close_rewind(&mut self) {
        self.rewind.cancel();
    }

    // The picker rows as the window shows them: "N) prompt", newlines folded and the prompt cut at 100 chars (TUI rewind lines).
    pub fn rewind_rows(&self) -> Vec<String> {
        let blocks = rewind_blocks(&self.transcript);
        self.rewind
            .turns()
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                let text: String = blocks[b].1.replace('\n', " ").chars().take(100).collect();
                format!("{:>2}) {text}", i + 1)
            })
            .collect()
    }

    // Picks a row (Some(index) for a click, None for Enter on the cursor row) and cuts the transcript before that
    // prompt (TUI rewind confirm). Returns the prompt text to put back in the input; None when the picker is closed.
    pub fn rewind_pick(&mut self, row: Option<usize>) -> Option<String> {
        if let Some(i) = row {
            self.rewind.select(i);
        }
        let cut = self.rewind.confirm(&rewind_blocks(&self.transcript))?;
        self.transcript.entries.truncate(cut.cut);
        let n = self.transcript.len();
        self.stage_rows.retain(|_, r| r.idx < n);
        self.transcript.push_notice(REWOUND);
        Some(cut.text)
    }

    // /resume <args> (TUI resume): a bare /resume after a killed run imports that session; a query that no record
    // answers opens the session group picker; otherwise the record folders answer. Returns the import the engine installs.
    pub fn resume(&mut self, args: &[String]) -> Option<ClaudeImport> {
        if args.is_empty()
            && let Some(sid) = self.killed_sid.clone()
        {
            let key = group_pick::killed_key(self.home().as_deref(), &self.cwd, &sid);
            return self.import_view(&key, true);
        }
        let root = record::dir_for(Path::new(&self.cwd));
        let (terms, record) = record_turn::resume_target_in(&root, args);
        if !record {
            return self.open_groups(&terms);
        }
        self.open_records(&root, &args.join(" "));
        None
    }

    // /import [id|latest|a,b] (TUI import_view without the digest): the transcript replaces the view and the import
    // goes to the engine for the next prompt. A read error is a notice.
    pub fn import(&mut self, arg: &str) -> Option<ClaudeImport> {
        self.import_view(arg, false)
    }

    // Opens the /resume record picker over the folders under root for arg (TUI resume). With no folder that matches,
    // the core resume answers instead: its notes and blocks go to the transcript.
    pub fn open_records(&mut self, root: &Path, arg: &str) {
        match Picker::open(root, arg) {
            Err(e) => self.transcript.push_notice(e.to_string()),
            Ok(Some(p)) => self.records = Some(p),
            Ok(None) => match record_turn::resume(root, arg, &record_pick::list_date) {
                Err(e) => self.transcript.push_notice(e.to_string()),
                Ok((lines, entries)) => {
                    for l in lines {
                        self.transcript.push_notice(l);
                    }
                    for e in &entries {
                        self.record_block(&e.kind, &e.text);
                    }
                }
            },
        }
    }

    // Lists the session groups for terms (TUI open_groups): one group imports at once, several open the picker, an
    // error is a notice.
    fn open_groups(&mut self, terms: &[String]) -> Option<ClaudeImport> {
        match group_pick::groups(self.home().as_deref(), &self.cwd, terms) {
            Err(e) => {
                self.transcript.push_notice(e);
                None
            }
            Ok(gs) if gs.len() == 1 => {
                let key = gs[0].key();
                self.import_view(&key, true)
            }
            Ok(gs) => {
                self.groups = Some(GroupPicker::new(gs));
                None
            }
        }
    }

    // Reads the import of key (TUI import_view). digest writes the resume note of a /resume pick. The transcript shows
    // its blocks; the import is returned for the engine. A read error is a notice.
    fn import_view(&mut self, key: &str, digest: bool) -> Option<ClaudeImport> {
        let home = self.home().unwrap_or_else(osenv::home_dir);
        match import_claude::import_for_resume_in(&home, key, &self.cwd, digest) {
            Err(e) => {
                self.transcript.push_notice(e.to_string());
                None
            }
            Ok(imp) => {
                self.show_import(&imp);
                Some(imp)
            }
        }
    }

    // Replaces the transcript with the blocks of an import (TUI show_import): the last 100 turns, with a note first.
    fn show_import(&mut self, imp: &ClaudeImport) {
        self.transcript.entries.clear();
        self.stage_rows.clear();
        for (kind, text) in import_claude::import_blocks(&imp.view) {
            self.record_block(kind, &text);
        }
        self.transcript.push_notice(import_claude::INSTALL_NOTE);
    }

    // Moves the session group cursor up or down (no-op while closed).
    pub fn groups_step(&mut self, up: bool) {
        if let Some(p) = self.groups.as_mut() {
            p.move_by(if up { -1 } else { 1 });
        }
    }

    // A digit key: moves the session group cursor to row i (out-of-range rows are ignored by the model).
    pub fn groups_select(&mut self, i: usize) {
        if let Some(p) = self.groups.as_mut() {
            p.select(i);
        }
    }

    // Closes the session group picker without a pick (TUI Close).
    pub fn close_groups(&mut self) {
        self.groups = None;
    }

    // The group rows as the window shows them: "N) <files> · date · first prompt" (TUI resume_pick::lines).
    pub fn groups_rows(&self) -> Vec<String> {
        self.groups
            .as_ref()
            .map(|p| {
                p.groups
                    .iter()
                    .enumerate()
                    .map(|(i, g)| format!("{:>2}) {}", i + 1, g.label(&group_pick::stamp)))
                    .collect()
            })
            .unwrap_or_default()
    }

    // Imports the group at row (Some for a click, None for Enter on the cursor row) and closes the picker (TUI Pick).
    // An empty list keeps the picker open and imports nothing.
    pub fn groups_pick(&mut self, row: Option<usize>) -> Option<ClaudeImport> {
        if let Some(i) = row {
            self.groups_select(i);
        }
        let key = self.groups.as_ref()?.chosen_key()?;
        self.groups = None;
        self.import_view(&key, true)
    }

    // SessionIds (TUI SessionIds arm): the purposes of this run's sessions and the set of ids it owns.
    fn session_ids(
        &mut self,
        main: String,
        residents: BTreeMap<String, String>,
        owned: Vec<String>,
    ) {
        self.session_id = main.clone();
        if !main.is_empty() {
            self.session_purpose
                .insert(main.clone(), "main".to_string());
        }
        for (role, id) in &residents {
            if !id.is_empty() {
                self.session_purpose.insert(id.clone(), role.clone());
            }
        }
        self.this_run = std::iter::once(main)
            .chain(residents.into_values())
            .chain(owned)
            .filter(|s| !s.is_empty())
            .collect();
    }

    // /view <arg> (TUI open_view): the session list, or one session's prompts for a prefix. A failure is a notice.
    // The Claude Code home: the injected one (tests), else $HOME / USERPROFILE.
    fn home(&self) -> Option<PathBuf> {
        self.home.clone().or_else(session_view::home_dir)
    }

    pub fn open_view(&mut self, arg: &str) {
        if let Some(mode) = self.session_mode.clone() {
            self.transcript.push_notice(mode);
        }
        let Some(home) = self.home() else {
            self.transcript
                .push_notice("$HOME is not defined".to_string());
            return;
        };
        let dir = session_view::project_dir(&home, &self.cwd);
        let mut owned = session_view::handoff_owned(&self.cwd);
        owned.extend(self.this_run.iter().cloned());
        match session_view::open_in(&dir, &owned, arg) {
            Ok(mut sv) => {
                sv.purposes = self.session_purpose.clone();
                self.sess = Some(sv);
            }
            Err(msg) => self.transcript.push_notice(msg),
        }
    }

    // Level 1 cursor up or down (no-op while closed or at level 2).
    pub fn sess_step(&mut self, up: bool) {
        if let Some(sv) = self.sess.as_mut().filter(|s| !s.in_prompts()) {
            sv.move_sel(up);
        }
    }

    // A digit key: moves the level 1 cursor to row i (out-of-range rows are ignored).
    pub fn sess_select(&mut self, i: usize) {
        if let Some(sv) = self.sess.as_mut().filter(|s| !s.in_prompts())
            && i < sv.list.len()
        {
            sv.sel = i;
        }
    }

    // Enter (row None) or a click (Some row) at level 1 opens that session's prompts (TUI Enter). A failure is a notice.
    pub fn sess_open(&mut self, row: Option<usize>) {
        let Some(sv) = self.sess.as_mut() else {
            return;
        };
        if sv.in_prompts() {
            return;
        }
        if let Some(i) = row.filter(|&i| i < sv.list.len()) {
            sv.sel = i;
        }
        let i = sv.sel;
        if i >= sv.list.len() {
            return;
        }
        if let Err(e) = sv.open_prompts(i) {
            self.transcript.push_notice(e);
        }
    }

    // Level 2: the previous or next prompt (TUI Left and Right).
    pub fn sess_prompt(&mut self, next: bool) {
        if let Some(sv) = self.sess.as_mut().filter(|s| s.in_prompts()) {
            if next {
                sv.next_prompt();
            } else {
                sv.prev_prompt();
            }
        }
    }

    // Level 2 back to the list (TUI Esc at level 2).
    pub fn sess_back(&mut self) {
        if let Some(sv) = self.sess.as_mut() {
            sv.leave_prompts();
        }
    }

    // Closes /view (TUI Close).
    pub fn close_sess(&mut self) {
        self.sess = None;
    }

    // Moves the record cursor up or down (no-op while closed).
    pub fn records_step(&mut self, up: bool) {
        if let Some(p) = self.records.as_mut() {
            p.move_by(if up { -1 } else { 1 });
        }
    }

    // A digit key: moves the record cursor to row i (out-of-range rows are ignored by the model).
    pub fn records_select(&mut self, i: usize) {
        if let Some(p) = self.records.as_mut() {
            p.select(i);
        }
    }

    // Closes the record picker without loading (TUI Close).
    pub fn close_records(&mut self) {
        self.records = None;
    }

    // The picker rows as the window shows them: "N) <name> · 파일 n개 · date" (TUI records::lines).
    pub fn records_rows(&self) -> Vec<String> {
        self.records
            .as_ref()
            .map(|p| {
                p.projects
                    .iter()
                    .enumerate()
                    .map(|(i, r)| format!("{:>2}) {}", i + 1, record_pick::row_text(r)))
                    .collect()
            })
            .unwrap_or_default()
    }

    // Loads the folder at row (Some for a click, None for Enter on the cursor row) into the transcript and closes the
    // picker (TUI load_record). Returns true when the folder loaded.
    pub fn records_pick(&mut self, row: Option<usize>) -> bool {
        if let Some(i) = row {
            self.records_select(i);
        }
        let Some(picker) = self.records.take() else {
            return false;
        };
        let Some(p) = picker.chosen() else {
            return false;
        };
        match record_folder::load_project(&p.dir) {
            Err(e) => {
                self.transcript.push_notice(e.to_string());
                false
            }
            Ok(entries) => {
                for e in &entries {
                    self.record_block(&e.kind, &e.text);
                }
                true
            }
        }
    }

    // One record block in the transcript (TUI record_block): a user block is a prompt, an asst block a finished
    // answer, any other kind a note; each block ends with a blank row.
    fn record_block(&mut self, kind: &str, text: &str) {
        match kind {
            "user" => self.transcript.push_user(text),
            "asst" => self.transcript.push_text(Role::Main, text, true),
            _ => self.transcript.push_notice(text),
        }
        self.transcript.push_blank();
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
            UiEvent::SessionIds {
                main,
                residents,
                owned,
            } => self.session_ids(main, residents, owned),
            // Core relaunched the main child with --resume: the /view list opens so the switch is visible (TUI).
            UiEvent::SessionResumed {
                old_session,
                session_id,
            } => {
                self.transcript
                    .push_notice(format!("session resumed: {session_id} (was {old_session})"));
                self.open_view("");
            }
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
            // With residents the tui answer is the main reply (TUI feed_main + end_main_block); it lands in the open
            // main row, or a new one, and that row is done.
            UiEvent::TuiAnswer { text } => {
                let i = self.open_text(&Role::Main);
                if let Entry::Text { text: t, done, .. } = &mut self.transcript.entries[i] {
                    t.push_str(&text);
                    *done = true;
                }
            }
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
    // Records a copy of `chars` characters: "copied (N chars)" (TUI copy_done, Ok case).
    pub fn copy_done(&mut self, chars: usize, now: Instant) {
        self.copy_note = Some((i18n::tf(Msg::CopyDone, &[&chars]), now));
    }

    // The copy note while it is fresh: shown for COPY_NOTE_TTL after it was set (TUI copy_note expiry).
    pub fn copy_note_text(&self, now: Instant) -> Option<&str> {
        self.copy_note
            .as_ref()
            .filter(|(_, at)| now.saturating_duration_since(*at) < COPY_NOTE_TTL)
            .map(|(text, _)| text.as_str())
    }

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

    #[test]
    fn copy_note_shows_for_two_seconds_then_clears() {
        let mut st = GuiState::default();
        let t0 = Instant::now();
        assert_eq!(st.copy_note_text(t0), None);
        st.copy_done(5, t0);
        let want = i18n::tf(Msg::CopyDone, &[&5usize]);
        assert_eq!(st.copy_note_text(t0), Some(want.as_str()));
        assert_eq!(
            st.copy_note_text(t0 + COPY_NOTE_TTL - Duration::from_millis(1)),
            Some(want.as_str())
        );
        assert_eq!(st.copy_note_text(t0 + COPY_NOTE_TTL), None);
        st.copy_done(0, t0 + Duration::from_secs(5));
        assert!(st.copy_note_text(t0 + Duration::from_secs(6)).is_some());
    }

    fn text(role: Role, s: &str) -> UiEvent {
        UiEvent::TextDelta {
            role,
            text: s.into(),
        }
    }

    #[test]
    fn tui_answer_is_the_done_main_reply() {
        let mut st = GuiState::default();
        let now = Instant::now();
        st.apply(
            UiEvent::TuiAnswer {
                text: "answer".into(),
            },
            now,
        );
        assert_eq!(
            st.transcript.entries,
            [Entry::Text {
                role: Role::Main,
                text: "answer".into(),
                done: true
            }]
        );
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

    // Transcript: a slash command, "one", a status row, "two", a status row. Prompts 1 and 2 are listed.
    fn rewind_fixture() -> GuiState {
        let mut st = GuiState::default();
        st.push_user("/help");
        st.push_user("one");
        st.transcript.push_status("ok");
        st.push_user("two");
        st.transcript.push_status("done");
        st
    }

    #[test]
    fn rewind_window_opens_on_the_newest_prompt_and_steps() {
        let mut st = rewind_fixture();
        st.rewind_step(true);
        assert!(!st.rewind.is_open(), "stepping while closed does nothing");
        st.open_rewind();
        assert!(st.rewind.is_open());
        assert_eq!(st.rewind_rows(), [" 1) one", " 2) two"]);
        assert_eq!(st.rewind.sel(), 1, "newest prompt is selected");
        st.rewind_step(true);
        assert_eq!(st.rewind.sel(), 0);
        st.rewind_step(true); // wraps
        assert_eq!(st.rewind.sel(), 1);
    }

    #[test]
    fn rewind_pick_cuts_before_the_prompt_and_returns_its_text() {
        let mut st = rewind_fixture();
        st.open_rewind();
        st.rewind_step(true);
        assert_eq!(st.rewind_pick(None), Some("one".to_string()));
        assert!(!st.rewind.is_open(), "a pick closes the window");
        assert_eq!(
            st.transcript.entries,
            vec![Entry::User("/help".into()), Entry::Notice(REWOUND.into()),],
            "the cut keeps the rows before the picked prompt, then the note"
        );
        assert_eq!(st.rewind_pick(None), None, "no pick while closed");
    }

    #[test]
    fn rewind_pick_by_row_and_cancel_leave_the_transcript_alone() {
        let mut st = rewind_fixture();
        let before = st.transcript.clone();
        st.open_rewind();
        st.close_rewind();
        assert!(!st.rewind.is_open(), "cancel closes");
        assert_eq!(st.transcript, before, "a cancel cuts nothing");
        st.open_rewind();
        assert_eq!(
            st.rewind_pick(Some(0)),
            Some("one".to_string()),
            "a click picks its row"
        );
        assert_eq!(st.transcript.len(), 2);
    }

    #[test]
    fn rewind_with_no_prompt_notes_nothing_to_rewind_to() {
        let mut st = GuiState::default();
        st.transcript.push_status("ok");
        st.open_rewind();
        assert!(!st.rewind.is_open());
        assert_eq!(
            st.transcript.entries.last(),
            Some(&Entry::Notice(NOTHING.into()))
        );
    }

    // One folder with one record file: a user "q" and an assistant answer "answer of <name>". Written under root
    // (a temp dir, never ~/.claude).
    fn write_record(root: &std::path::Path, name: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let body = format!(
            "<!-- rec #0001 user 2026-10-31T10:00 cat={name} -->\n## #0001 · 10:00 · {name}\n\nq\n\n\
             <!-- rec #0001 asst 2026-10-31T10:00 cat={name} -->\n## #0001 · 10:00 · {name}\n\nanswer of {name}\n\n"
        );
        std::fs::write(dir.join("261031-100000-0001.md"), body).unwrap();
    }

    // A temp record root with folders alpha and beta; the test removes it at its end.
    fn records_root(tag: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("chaude-gui-records-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write_record(&root, "alpha");
        write_record(&root, "beta");
        root
    }

    #[test]
    fn records_window_opens_lists_and_steps() {
        let root = records_root("open");
        let mut st = GuiState::default();
        st.records_step(true);
        assert!(st.records.is_none(), "stepping while closed does nothing");
        st.open_records(&root, "");
        assert!(st.records.is_some());
        let rows = st.records_rows();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with(" 1) "), "{rows:?}");
        assert_eq!(st.records.as_ref().unwrap().sel, 0);
        st.records_step(false);
        assert_eq!(st.records.as_ref().unwrap().sel, 1);
        st.records_step(false); // wraps
        assert_eq!(st.records.as_ref().unwrap().sel, 0);
        st.records_step(true); // wraps
        assert_eq!(st.records.as_ref().unwrap().sel, 1);
        st.records_select(9);
        assert_eq!(
            st.records.as_ref().unwrap().sel,
            1,
            "past the list is ignored"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn records_with_no_folder_stay_closed_and_note() {
        let root = std::env::temp_dir().join(format!(
            "chaude-gui-records-{}-none-missing",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut st = GuiState::default();
        st.open_records(&root, "");
        assert!(st.records.is_none(), "no folder, no window");
        assert!(
            matches!(st.transcript.entries.last(), Some(Entry::Notice(_))),
            "{:?}",
            st.transcript.entries
        );
    }

    #[test]
    fn records_cancel_closes_without_loading() {
        let root = records_root("cancel");
        let mut st = GuiState::default();
        st.open_records(&root, "");
        st.close_records();
        assert!(st.records.is_none(), "cancel closes");
        assert!(st.transcript.is_empty(), "nothing loaded");
        let _ = std::fs::remove_dir_all(&root);
    }

    fn session_group(id: &str) -> chaude_core::resume_group::SessionGroup {
        chaude_core::resume_group::SessionGroup {
            files: vec![chaude_core::resume_group::SessionInfo {
                id: id.into(),
                path: std::path::PathBuf::new(),
                modified: std::time::SystemTime::UNIX_EPOCH,
                turns: 2,
                first: "first prompt".into(),
                last: "last prompt".into(),
                start: None,
                cwd: String::new(),
                name: String::new(),
                cleared: false,
                clear_head: false,
            }],
        }
    }

    #[test]
    fn groups_window_steps_selects_and_closes() {
        let mut st = GuiState::default();
        st.groups_step(true);
        assert!(st.groups.is_none(), "stepping while closed does nothing");
        st.groups = Some(GroupPicker::new(vec![
            session_group("a"),
            session_group("b"),
        ]));
        assert_eq!(st.groups_rows().len(), 2);
        assert!(st.groups_rows()[1].starts_with(" 2) "));
        st.groups_step(true); // wraps to the last row
        assert_eq!(st.groups.as_ref().unwrap().sel, 1);
        st.groups_select(9);
        assert_eq!(
            st.groups.as_ref().unwrap().sel,
            1,
            "past the list is ignored"
        );
        st.close_groups();
        assert!(st.groups.is_none(), "cancel closes");
        assert!(st.transcript.is_empty(), "nothing imported");
    }

    #[test]
    fn groups_enter_on_an_empty_list_keeps_the_picker_open() {
        let mut st = GuiState {
            groups: Some(GroupPicker::new(Vec::new())),
            ..Default::default()
        };
        assert!(st.groups_pick(None).is_none());
        assert!(st.groups.is_some(), "nothing to pick, so it stays open");
    }

    #[test]
    fn records_enter_loads_the_cursor_folder_into_the_transcript() {
        let root = records_root("enter");
        let mut st = GuiState::default();
        st.open_records(&root, "");
        st.records_step(false);
        let name = st.records.as_ref().unwrap().projects[1].name.clone();
        assert!(st.records_pick(None), "enter loads the cursor row");
        assert!(st.records.is_none(), "a pick closes the window");
        let answer = format!("answer of {name}");
        assert!(st.transcript.entries.contains(&Entry::User("q".into())));
        assert!(st.transcript.entries.iter().any(|e| matches!(
            e,
            Entry::Text { text, done: true, .. } if *text == answer
        )));
        let other = if name == "alpha" { "beta" } else { "alpha" };
        assert!(
            !st.transcript.entries.iter().any(|e| matches!(
                e,
                Entry::Text { text, .. } if *text == format!("answer of {other}")
            )),
            "only the chosen folder loads"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn records_click_loads_the_clicked_row() {
        let root = records_root("click");
        let mut st = GuiState::default();
        st.open_records(&root, "");
        let name = st.records.as_ref().unwrap().projects[0].name.clone();
        assert!(st.records_pick(Some(0)), "a click loads that row");
        assert!(st.records.is_none());
        let answer = format!("answer of {name}");
        assert!(st.transcript.entries.iter().any(|e| matches!(
            e,
            Entry::Text { text, .. } if *text == answer
        )));
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod view_tests {
    use super::*;

    // A temp home whose Claude Code project dir for cwd holds one session per (id, prompts); the test removes it.
    fn home_with(tag: &str, cwd: &str, sessions: &[(&str, &[&str])]) -> PathBuf {
        let home =
            std::env::temp_dir().join(format!("chaude-gui-view-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let dir = session_view::project_dir(&home, cwd);
        std::fs::create_dir_all(&dir).unwrap();
        for (id, prompts) in sessions {
            let mut body = String::new();
            for p in *prompts {
                body += &format!(
                    "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"{p}\"}}}}\n"
                );
                body += "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"ok\"}]}}\n";
            }
            std::fs::write(dir.join(format!("{id}.jsonl")), body).unwrap();
        }
        home
    }

    // A state on cwd and home, whose run owns aaaa1111 (main) and bbbb2222 (owned).
    fn owned_state(home: &Path, cwd: &str) -> GuiState {
        let mut st = GuiState {
            cwd: cwd.into(),
            home: Some(home.to_path_buf()),
            ..Default::default()
        };
        st.apply(
            UiEvent::SessionIds {
                main: "aaaa1111".into(),
                residents: BTreeMap::new(),
                owned: vec!["bbbb2222".into()],
            },
            Instant::now(),
        );
        st
    }

    fn sessions() -> [(&'static str, &'static [&'static str]); 2] {
        [
            ("aaaa1111", &["first one", "second one"]),
            ("bbbb2222", &["only one"]),
        ]
    }

    #[test]
    fn view_opens_drills_backs_picks_and_cancels() {
        let cwd = "/chaude-gui-view/work-open";
        let home = home_with("open", cwd, &sessions());
        let mut st = owned_state(&home, cwd);
        st.sess_open(None);
        assert!(st.sess.is_none(), "enter while closed does nothing");
        st.open_view("");
        assert_eq!(st.sess.as_ref().map(|s| s.list.len()), Some(2));
        let bbbb = st
            .sess
            .as_ref()
            .unwrap()
            .list
            .iter()
            .position(|s| s.id == "bbbb2222")
            .unwrap();
        st.sess_open(Some(bbbb));
        let sv = st.sess.as_ref().unwrap();
        assert!(sv.in_prompts(), "a pick drills into the prompts");
        assert_eq!(sv.shown(), Some("only one"));
        st.sess_back();
        assert!(!st.sess.as_ref().unwrap().in_prompts(), "back to the list");
        let aaaa = st
            .sess
            .as_ref()
            .unwrap()
            .list
            .iter()
            .position(|s| s.id == "aaaa1111")
            .unwrap();
        st.sess_select(aaaa);
        st.sess_open(None);
        assert_eq!(
            st.sess.as_ref().unwrap().id,
            "aaaa1111",
            "enter drills the cursor row"
        );
        st.sess_prompt(true);
        assert_eq!(
            st.sess.as_ref().unwrap().cur,
            1,
            "right steps to the next prompt"
        );
        st.close_sess();
        assert!(st.sess.is_none(), "cancel closes the browser");
        let _ = std::fs::remove_dir_all(&home);
    }

    // /import (TUI import_view without the digest): a known id reads its transcript, shows it and returns the import
    // for the engine; a miss is a notice and returns nothing. The temp home holds the transcripts.
    #[test]
    fn import_reads_the_transcript_and_a_miss_is_a_notice() {
        let cwd = "/chaude-gui-view/work-import";
        let home = home_with("import", cwd, &sessions());
        let mut st = owned_state(&home, cwd);
        let imp = st.import("bbbb2222").expect("a known id imports");
        assert_eq!(imp.view[0].0, "only one", "the import holds the prompt");
        assert!(
            st.transcript.entries.iter().any(
                |e| matches!(e, Entry::Notice(t) if t.as_str() == import_claude::INSTALL_NOTE)
            ),
            "the install note follows the view"
        );
        st.transcript.entries.clear();
        assert!(st.import("zzzz").is_none(), "a miss returns nothing");
        assert!(
            st.transcript
                .entries
                .iter()
                .any(|e| matches!(e, Entry::Notice(_))),
            "the miss is a notice"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn view_with_a_prefix_opens_its_prompts_and_a_miss_is_a_notice() {
        let cwd = "/chaude-gui-view/work-prefix";
        let home = home_with("prefix", cwd, &sessions());
        let mut st = owned_state(&home, cwd);
        st.open_view("bbbb");
        assert_eq!(st.sess.as_ref().unwrap().shown(), Some("only one"));
        st.close_sess();
        st.open_view("zzzz");
        assert!(st.sess.is_none(), "no match, no browser");
        assert!(
            st.transcript
                .entries
                .iter()
                .any(|e| matches!(e, Entry::Notice(t) if t.contains("zzzz"))),
            "the miss is a notice"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn view_notes_the_session_mode_before_the_list() {
        let cwd = "/chaude-gui-view/work-mode";
        let home = home_with("mode", cwd, &sessions());
        let mut st = owned_state(&home, cwd);
        st.session_mode = Some("previous session: fresh start, handoff note carried over".into());
        st.open_view("zzzz");
        assert!(st.sess.is_none(), "a miss still opens no browser");
        assert!(
            st.transcript
                .entries
                .iter()
                .any(|e| matches!(e, Entry::Notice(t) if t.starts_with("previous session"))),
            "the mode is noted on every /view, as the TUI does"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_mid_run_switch_reopens_the_session_list() {
        let cwd = "/chaude-gui-view/work-resumed";
        let home = home_with("resumed", cwd, &sessions());
        let mut st = owned_state(&home, cwd);
        st.apply(
            UiEvent::SessionResumed {
                old_session: "aaaa1111".into(),
                session_id: "bbbb2222".into(),
            },
            Instant::now(),
        );
        let sv = st.sess.as_ref().expect("the list reopens after the switch");
        assert!(!sv.in_prompts());
        assert!(
            st.transcript
                .entries
                .iter()
                .any(|e| matches!(e, Entry::Notice(t) if t.contains("(was aaaa1111)"))),
            "the switch is noted"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
