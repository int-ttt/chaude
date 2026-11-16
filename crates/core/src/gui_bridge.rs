//! The GUI bridge's message shapes, shared by the window and the core: events Rust sends to the page and commands
//! the page sends back. Plain serde types, JSON tagged by "type". `Mirror` is the producer: it compares the core
//! state (Transcript, RunState, BarInput) with what the page already has and returns only the events that changed.

use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::api::Role;
use crate::recent_folders::RecentFolder;
use crate::session_view;
use crate::ui_state::{BarInput, Entry, RunState, Transcript};
use crate::usage_poll::UsageMeter;

// The session kind the window runs. Chat: a scratch dir, no tools, no harness agents. Code: the folder is the cwd
// and the full harness runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Chat,
    Code,
}

// One transcript row as the page gets it (serde mirror of ui_state::Entry; text rows carry the agent id when the
// row is an agent's, and no agent for the main session).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntryView {
    User {
        text: String,
    },
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        agent: Option<String>,
        text: String,
        done: bool,
    },
    Tool {
        id: String,
        name: String,
        summary: String,
        ok: Option<bool>,
        start: i64,
        end: Option<ToolEnd>,
    },
    ToolResult {
        id: String,
        text: String,
        at: i64,
    },
    Agent {
        id: String,
        description: String,
        done: bool,
        summary: String,
    },
    Notice {
        text: String,
    },
    Status {
        text: String,
    },
    Error {
        text: String,
    },
    Blank,
}

// When a tool call finished: the unix time and the elapsed seconds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolEnd {
    pub at: i64,
    pub secs: u64,
}

impl From<&Entry> for EntryView {
    fn from(e: &Entry) -> Self {
        match e {
            Entry::User(text) => Self::User { text: text.clone() },
            Entry::Text { role, text, done } => Self::Text {
                agent: match role {
                    Role::Main => None,
                    Role::Agent(id) => Some(id.clone()),
                },
                text: text.clone(),
                done: *done,
            },
            Entry::Tool {
                id,
                name,
                summary,
                ok,
                start,
                end,
            } => Self::Tool {
                id: id.clone(),
                name: name.clone(),
                summary: summary.clone(),
                ok: *ok,
                start: *start,
                end: end.as_ref().map(|&(at, secs)| ToolEnd { at, secs }),
            },
            Entry::ToolResult { id, text, at } => Self::ToolResult {
                id: id.clone(),
                text: text.clone(),
                at: *at,
            },
            Entry::Agent {
                id,
                description,
                done,
                summary,
            } => Self::Agent {
                id: id.clone(),
                description: description.clone(),
                done: *done,
                summary: summary.clone(),
            },
            Entry::Notice(text) => Self::Notice { text: text.clone() },
            Entry::Status(text) => Self::Status { text: text.clone() },
            Entry::Error(text) => Self::Error { text: text.clone() },
            Entry::Blank => Self::Blank,
        }
    }
}

// The permission prompt waiting for the user (ui_state side: GuiState::permission).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PermissionView {
    pub id: String,
    pub tool: String,
    pub summary: String,
}

// The status a page shows beside the chat: engine progress (RunState) and the bar data (BarInput).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StateView {
    pub busy: bool,
    pub running: usize,
    pub queued: usize,
    pub held: Option<usize>,
    pub rotations: u32,
    pub model: String,
    pub tokens: u64,
    pub cost_usd: f64,
    pub context_pct: u32,
    // The rate window text (BarInput rate); None hides it.
    pub rate: Option<String>,
    pub session_id: String,
    pub permission: Option<PermissionView>,
}

impl StateView {
    pub fn new(
        run: &RunState,
        bar: &BarInput,
        session_id: &str,
        permission: Option<PermissionView>,
    ) -> Self {
        Self {
            busy: run.busy,
            running: run.running_n(),
            queued: run.queue_texts.len(),
            held: run.held,
            rotations: run.rot,
            model: bar.model.clone(),
            tokens: bar.tokens,
            cost_usd: bar.cost_usd,
            context_pct: bar.context_pct,
            rate: bar.rate.clone(),
            session_id: session_id.to_string(),
            permission,
        }
    }
}

// One past session of a Claude Code project dir: the row that resumes it. first is its first prompt in one line;
// used is the transcript's modified time in unix seconds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PastSession {
    pub id: String,
    pub turns: usize,
    pub first: String,
    pub used: u64,
}

// How many past sessions past_sessions keeps (the newest; open_plan takes the first).
pub const MAX_PAST: usize = 50;

// First prompts the harness sends itself, not the user: a rotated session's carry-over, the prompter and relay
// summaries, handoff notes and the harness's own probes. A session whose first prompt starts with one is no row.
// Matched on the one-line prompt (whitespace collapsed).
const HARNESS_PROMPTS: &[&str] = &[
    "## Previous recommendation (verbatim)",
    "[Context carried over from a previous session",
    "[prompter summary",
    "Opus main return follows",
    "lang: ko Opus main return follows",
    "Opus 메인 반환입니다",
    "Write a handoff note for a fresh session",
    "Summarize the following user input",
    "Workstream chaude-",
    "Remember this codeword",
    "Count from 1 to",
    "Count slowly from",
    "Reply with exactly",
    "Use the Write tool to create",
    "Run the bash command `echo harness-ok`",
    "List the files in the current directory using the Bash tool",
];

// True when a one-line first prompt is empty or one the harness sent (see HARNESS_PROMPTS).
pub fn is_harness_prompt(first: &str) -> bool {
    let first = first.trim();
    first.is_empty() || HARNESS_PROMPTS.iter().any(|p| first.starts_with(p))
}

// The user's past sessions of the project dir of cwd under home, newest first. Sub-agent transcripts sit under
// <session>/subagents/ (not listed) and sidechain-only files hold no prompt, so list_sessions drops them; this also
// drops the empty and harness-sent sessions.
pub fn past_sessions(home: &Path, cwd: &str) -> Vec<PastSession> {
    session_view::list_sessions(&session_view::project_dir(home, cwd))
        .into_iter()
        .filter(|s| !is_harness_prompt(&s.first))
        .take(MAX_PAST)
        .map(|s| PastSession {
            id: s.id,
            turns: s.turns,
            first: s
                .first
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(60)
                .collect(),
            used: s
                .modified
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        })
        .collect()
}

// What opening a sidebar row does: the mode, the engine cwd (a code folder, or a chat scratch dir under chat_root)
// and the session to resume (empty when the dir has none yet). None when the row is not one to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPlan {
    pub mode: Mode,
    pub cwd: String,
    pub id: String,
}

// Plans open_session. An empty id takes the newest past session of the folder.
pub fn open_plan(
    mode: Mode,
    folder: &str,
    id: &str,
    home: &Path,
    chat_root: &Path,
) -> Option<OpenPlan> {
    if folder.is_empty() || (mode == Mode::Chat && !Path::new(folder).starts_with(chat_root)) {
        return None;
    }
    let id = if id.is_empty() {
        past_sessions(home, folder)
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_default()
    } else {
        id.to_string()
    };
    Some(OpenPlan {
        mode,
        cwd: folder.to_string(),
        id,
    })
}

// A message to the page (Rust -> JS). entry_cut keeps the first `len` rows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Pong,
    EntryPush {
        index: usize,
        entry: EntryView,
    },
    EntryUpdate {
        index: usize,
        entry: EntryView,
    },
    EntryCut {
        len: usize,
    },
    State {
        state: StateView,
    },
    PickerOpen {
        picker: String,
    },
    PickerClose {
        picker: String,
    },
    Note {
        text: String,
    },
    // The session on screen. mode None = the start screen. folder is the code cwd ("" for chat); cwd is the engine's
    // dir (the scratch dir for chat); home shortens paths to ~. recent = the recent folders (code); chats = the chat
    // scratch dirs, newest first (used = the dir's mtime).
    Session {
        mode: Option<Mode>,
        folder: String,
        cwd: String,
        home: String,
        recent: Vec<RecentFolder>,
        chats: Vec<RecentFolder>,
    },
    // The /usage numbers for the sidebar footer; None until a /usage answer parses. Sent only when it changes.
    Usage {
        meter: Option<UsageMeter>,
    },
}

impl Event {
    // The event's type name, for logs.
    pub fn kind(&self) -> &'static str {
        match self {
            Event::Pong => "pong",
            Event::EntryPush { .. } => "entry_push",
            Event::EntryUpdate { .. } => "entry_update",
            Event::EntryCut { .. } => "entry_cut",
            Event::State { .. } => "state",
            Event::PickerOpen { .. } => "picker_open",
            Event::PickerClose { .. } => "picker_close",
            Event::Note { .. } => "note",
            Event::Session { .. } => "session",
            Event::Usage { .. } => "usage",
        }
    }
}

// A message from the page (JS -> Rust). Each one is routed by the window into the core logic.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Ping,
    // A prompt or a /command line, as typed (the core decides which).
    Submit {
        text: String,
    },
    // Stops the running turn (the Esc / stop button).
    Interrupt,
    // A slash command by name with its argument (the slash menu pick).
    Slash {
        name: String,
        #[serde(default)]
        arg: String,
    },
    // Picks a row of an open picker (row None = the cursor row, as Enter does).
    PickerPick {
        picker: String,
        row: Option<usize>,
    },
    // Answers the permission prompt; interrupt also stops the turn (Ctrl+C).
    PermissionAnswer {
        allow: bool,
        #[serde(default)]
        interrupt: bool,
    },
    // A copy the page made of `text`; the window shows the copied-count note.
    Copy {
        text: String,
    },
    // Starts a session: chat in a new scratch dir, or code with `folder` as the engine cwd (a recent-folder row or
    // the smoke hook). The folder is ignored for chat. A non-empty `text` is submitted in the new session (the start
    // screen's composer).
    Start {
        mode: Mode,
        #[serde(default)]
        folder: String,
        #[serde(default)]
        text: String,
    },
    // Opens the native folder dialog; a picked folder starts a code session.
    OpenFolder,
    // Opens a past session (a sidebar row): its folder (code) or chat scratch dir becomes the engine cwd, and its
    // transcript is loaded and installed to resume. An empty id takes the newest session there.
    OpenSession {
        mode: Mode,
        folder: String,
        #[serde(default)]
        id: String,
    },
    // Opens the /usage window (the sidebar meter's click). Needs a running session; nothing happens without one.
    Usage,
}

// Keeps the page in step with the core. Each sync compares the new core state with the last one it returned and
// returns the events for what changed: transcript rows (push, update, cut), state, pickers opened or closed, note.
#[derive(Debug, Default)]
pub struct Mirror {
    entries: Vec<Entry>,
    state: Option<StateView>,
    pickers: Vec<String>,
    note: Option<String>,
}

impl Mirror {
    pub fn sync(
        &mut self,
        transcript: &Transcript,
        state: StateView,
        pickers: &[&str],
        note: Option<&str>,
    ) -> Vec<Event> {
        let mut out = Vec::new();
        let new = &transcript.entries;
        let old_len = self.entries.len();
        for (index, (old, fresh)) in self.entries.iter_mut().zip(new).enumerate() {
            if *old != *fresh {
                *old = fresh.clone();
                out.push(Event::EntryUpdate {
                    index,
                    entry: EntryView::from(fresh),
                });
            }
        }
        if new.len() < old_len {
            self.entries.truncate(new.len());
            out.push(Event::EntryCut { len: new.len() });
        }
        for (index, e) in new.iter().enumerate().skip(old_len) {
            self.entries.push(e.clone());
            out.push(Event::EntryPush {
                index,
                entry: EntryView::from(e),
            });
        }
        if self.state.as_ref() != Some(&state) {
            out.push(Event::State {
                state: state.clone(),
            });
            self.state = Some(state);
        }
        for p in pickers {
            if !self.pickers.iter().any(|q| q == p) {
                out.push(Event::PickerOpen {
                    picker: p.to_string(),
                });
            }
        }
        for q in &self.pickers {
            if !pickers.contains(&q.as_str()) {
                out.push(Event::PickerClose { picker: q.clone() });
            }
        }
        self.pickers = pickers.iter().map(|p| p.to_string()).collect();
        if self.note.as_deref() != note {
            out.push(Event::Note {
                text: note.unwrap_or("").to_string(),
            });
            self.note = note.map(str::to_string);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(ev: &Event) -> String {
        serde_json::to_string(ev).unwrap()
    }

    fn idle() -> StateView {
        StateView::new(&RunState::default(), &BarInput::default(), "", None)
    }

    #[test]
    fn entry_push_snapshot() {
        let ev = Event::EntryPush {
            index: 0,
            entry: EntryView::from(&Entry::User("hi".into())),
        };
        assert_eq!(
            json(&ev),
            r#"{"type":"entry_push","index":0,"entry":{"kind":"user","text":"hi"}}"#
        );
    }

    #[test]
    fn entry_update_snapshot_keeps_agent_id() {
        let ev = Event::EntryUpdate {
            index: 2,
            entry: EntryView::from(&Entry::Text {
                role: Role::Agent("toolu_1".into()),
                text: "ok".into(),
                done: true,
            }),
        };
        assert_eq!(
            json(&ev),
            r#"{"type":"entry_update","index":2,"entry":{"kind":"text","agent":"toolu_1","text":"ok","done":true}}"#
        );
    }

    #[test]
    fn tool_entry_snapshot_has_end() {
        let entry = Entry::Tool {
            id: "t1".into(),
            name: "Bash".into(),
            summary: "ls".into(),
            ok: Some(true),
            start: 10,
            end: Some((12, 2)),
        };
        assert_eq!(
            json(&Event::EntryPush {
                index: 1,
                entry: EntryView::from(&entry),
            }),
            r#"{"type":"entry_push","index":1,"entry":{"kind":"tool","id":"t1","name":"Bash","summary":"ls","ok":true,"start":10,"end":{"at":12,"secs":2}}}"#
        );
    }

    #[test]
    fn entry_cut_snapshot() {
        assert_eq!(
            json(&Event::EntryCut { len: 1 }),
            r#"{"type":"entry_cut","len":1}"#
        );
    }

    #[test]
    fn state_snapshot() {
        assert_eq!(
            json(&Event::State { state: idle() }),
            r#"{"type":"state","state":{"busy":false,"running":0,"queued":0,"held":null,"rotations":0,"model":"","tokens":0,"cost_usd":0.0,"context_pct":0,"rate":null,"session_id":"","permission":null}}"#
        );
    }

    #[test]
    fn session_snapshot() {
        let recent = vec![RecentFolder {
            path: "/w/a".into(),
            used: 7,
        }];
        let chats = vec![RecentFolder {
            path: "/h/.chaude/chat/c1".into(),
            used: 9,
        }];
        assert_eq!(
            json(&Event::Session {
                mode: Some(Mode::Code),
                folder: "/w/a".into(),
                cwd: "/w/a".into(),
                home: "/h".into(),
                recent: recent.clone(),
                chats: chats.clone(),
            }),
            r#"{"type":"session","mode":"code","folder":"/w/a","cwd":"/w/a","home":"/h","recent":[{"path":"/w/a","used":7}],"chats":[{"path":"/h/.chaude/chat/c1","used":9}]}"#
        );
        assert_eq!(
            json(&Event::Session {
                mode: None,
                folder: String::new(),
                cwd: String::new(),
                home: "/h".into(),
                recent,
                chats: Vec::new(),
            }),
            r#"{"type":"session","mode":null,"folder":"","cwd":"","home":"/h","recent":[{"path":"/w/a","used":7}],"chats":[]}"#
        );
    }

    #[test]
    fn usage_snapshot() {
        let meter = UsageMeter {
            session_pct: 18.0,
            session_resets: Some("Oct 30 at 12:49pm (Asia/Seoul)".into()),
            week_pct: Some(27.0),
            week_resets: None,
        };
        assert_eq!(
            json(&Event::Usage { meter: Some(meter) }),
            r#"{"type":"usage","meter":{"session_pct":18.0,"session_resets":"Oct 30 at 12:49pm (Asia/Seoul)","week_pct":27.0,"week_resets":null}}"#
        );
        assert_eq!(
            json(&Event::Usage { meter: None }),
            r#"{"type":"usage","meter":null}"#
        );
        assert_eq!(
            serde_json::from_str::<Command>(r#"{"type":"usage"}"#).unwrap(),
            Command::Usage
        );
    }

    #[test]
    fn start_commands_parse_from_page_json() {
        let parse = |s: &str| serde_json::from_str::<Command>(s).unwrap();
        assert_eq!(
            parse(r#"{"type":"start","mode":"chat"}"#),
            Command::Start {
                mode: Mode::Chat,
                folder: String::new(),
                text: String::new()
            }
        );
        assert_eq!(
            parse(r#"{"type":"start","mode":"code","folder":"/w/a"}"#),
            Command::Start {
                mode: Mode::Code,
                folder: "/w/a".into(),
                text: String::new()
            }
        );
        assert_eq!(
            parse(r#"{"type":"start","mode":"chat","text":"hi"}"#),
            Command::Start {
                mode: Mode::Chat,
                folder: String::new(),
                text: "hi".into()
            }
        );
        assert_eq!(parse(r#"{"type":"open_folder"}"#), Command::OpenFolder);
        assert_eq!(
            parse(r#"{"type":"open_session","mode":"code","folder":"/w/a","id":"s1"}"#),
            Command::OpenSession {
                mode: Mode::Code,
                folder: "/w/a".into(),
                id: "s1".into()
            }
        );
        assert_eq!(
            parse(r#"{"type":"open_session","mode":"chat","folder":"/h/.chaude/chat/c1"}"#),
            Command::OpenSession {
                mode: Mode::Chat,
                folder: "/h/.chaude/chat/c1".into(),
                id: String::new()
            }
        );
        assert!(serde_json::from_str::<Command>(r#"{"type":"start","mode":"x"}"#).is_err());
    }

    #[test]
    fn picker_and_note_snapshots() {
        assert_eq!(
            json(&Event::PickerOpen {
                picker: "lang".into()
            }),
            r#"{"type":"picker_open","picker":"lang"}"#
        );
        assert_eq!(
            json(&Event::PickerClose {
                picker: "lang".into()
            }),
            r#"{"type":"picker_close","picker":"lang"}"#
        );
        assert_eq!(
            json(&Event::Note {
                text: "copied (3 chars)".into()
            }),
            r#"{"type":"note","text":"copied (3 chars)"}"#
        );
        assert_eq!(json(&Event::Pong), r#"{"type":"pong"}"#);
    }

    #[test]
    fn commands_parse_from_page_json() {
        let parse = |s: &str| serde_json::from_str::<Command>(s).unwrap();
        assert_eq!(parse(r#"{"type":"ping"}"#), Command::Ping);
        assert_eq!(
            parse(r#"{"type":"submit","text":"hi"}"#),
            Command::Submit { text: "hi".into() }
        );
        assert_eq!(parse(r#"{"type":"interrupt"}"#), Command::Interrupt);
        assert_eq!(
            parse(r#"{"type":"slash","name":"usage"}"#),
            Command::Slash {
                name: "usage".into(),
                arg: String::new()
            }
        );
        assert_eq!(
            parse(r#"{"type":"picker_pick","picker":"lang","row":1}"#),
            Command::PickerPick {
                picker: "lang".into(),
                row: Some(1)
            }
        );
        assert_eq!(
            parse(r#"{"type":"permission_answer","allow":true}"#),
            Command::PermissionAnswer {
                allow: true,
                interrupt: false
            }
        );
        assert_eq!(
            parse(r#"{"type":"copy","text":"abc"}"#),
            Command::Copy { text: "abc".into() }
        );
        assert!(serde_json::from_str::<Command>(r#"{"type":"no_such"}"#).is_err());
    }

    #[test]
    fn mirror_sends_only_changes() {
        let mut m = Mirror::default();
        let mut t = Transcript::default();
        t.push_user("a");
        t.push_notice("b");
        let kinds = |evs: &[Event]| evs.iter().map(Event::kind).collect::<Vec<_>>();
        assert_eq!(
            kinds(&m.sync(&t, idle(), &[], None)),
            ["entry_push", "entry_push", "state"]
        );
        assert!(m.sync(&t, idle(), &[], None).is_empty());

        t.push_text(Role::Main, "x", false);
        assert_eq!(kinds(&m.sync(&t, idle(), &[], None)), ["entry_push"]);

        if let Entry::Text { done, .. } = &mut t.entries[2] {
            *done = true;
        }
        let evs = m.sync(&t, idle(), &[], None);
        assert_eq!(kinds(&evs), ["entry_update"]);
        assert!(matches!(evs[0], Event::EntryUpdate { index: 2, .. }));

        t.entries.truncate(1);
        assert_eq!(kinds(&m.sync(&t, idle(), &[], None)), ["entry_cut"]);
    }

    #[test]
    fn streaming_chunks_send_updates_with_the_full_text() {
        let mut m = Mirror::default();
        let mut t = Transcript::default();
        t.push_text(Role::Main, "", false);
        m.sync(&t, idle(), &[], None);
        let mut shown = String::new();
        for chunk in ["he", "llo", " there"] {
            if let Entry::Text { text, .. } = &mut t.entries[0] {
                text.push_str(chunk);
            }
            shown.push_str(chunk);
            let evs = m.sync(&t, idle(), &[], None);
            match evs.as_slice() {
                [
                    Event::EntryUpdate {
                        index: 0,
                        entry: EntryView::Text { text, done, .. },
                    },
                ] => {
                    assert_eq!(text, &shown);
                    assert!(!done);
                }
                other => panic!("expected one entry_update, got {other:?}"),
            }
        }
    }

    #[test]
    fn mirror_tracks_pickers_state_and_note() {
        let mut m = Mirror::default();
        let t = Transcript::default();
        assert_eq!(
            kinds_of(&m.sync(&t, idle(), &["lang"], Some("copied (2 chars)"))),
            ["state", "picker_open", "note"]
        );
        assert_eq!(
            kinds_of(&m.sync(&t, idle(), &[], Some(""))),
            ["picker_close", "note"]
        );
        assert!(m.sync(&t, idle(), &[], Some("")).is_empty());
    }

    #[test]
    fn open_plan_sets_mode_cwd_and_the_session_to_resume() {
        use std::time::{Duration, SystemTime};
        let home = std::env::temp_dir().join(format!("chaude-bridge-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let cwd = "/w/proj";
        let dir = session_view::project_dir(&home, cwd);
        std::fs::create_dir_all(&dir).unwrap();
        let line = |p: &str| {
            format!(
                "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"{p}\"}}}}\n{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"ok\"}}]}}}}\n"
            )
        };
        let old = dir.join("old.jsonl");
        let new = dir.join("new.jsonl");
        std::fs::write(&old, line("first old prompt")).unwrap();
        std::fs::write(&new, line("newest prompt")).unwrap();
        let now = SystemTime::now();
        std::fs::File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_modified(now - Duration::from_secs(600))
            .unwrap();
        std::fs::File::options()
            .write(true)
            .open(&new)
            .unwrap()
            .set_modified(now)
            .unwrap();
        let chat_root = home.join(".chaude").join("chat");
        let scratch = chat_root.join("c1").display().to_string();

        let past = past_sessions(&home, cwd);
        assert_eq!(
            past.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["new", "old"]
        );
        assert_eq!(past[0].first, "newest prompt");
        assert_eq!(past[0].turns, 1);

        // A code folder with no id resumes its newest session; an explicit id is kept.
        assert_eq!(
            open_plan(Mode::Code, cwd, "", &home, &chat_root),
            Some(OpenPlan {
                mode: Mode::Code,
                cwd: cwd.into(),
                id: "new".into()
            })
        );
        assert_eq!(
            open_plan(Mode::Code, cwd, "old", &home, &chat_root),
            Some(OpenPlan {
                mode: Mode::Code,
                cwd: cwd.into(),
                id: "old".into()
            })
        );
        // A chat scratch dir keeps chat mode and its own cwd; with no session there the id is empty.
        assert_eq!(
            open_plan(Mode::Chat, &scratch, "", &home, &chat_root),
            Some(OpenPlan {
                mode: Mode::Chat,
                cwd: scratch.clone(),
                id: String::new()
            })
        );
        // Not a chat dir, or no folder: nothing is opened.
        assert_eq!(open_plan(Mode::Chat, cwd, "", &home, &chat_root), None);
        assert_eq!(open_plan(Mode::Code, "", "", &home, &chat_root), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn past_sessions_lists_only_the_users_top_level_sessions() {
        let home =
            std::env::temp_dir().join(format!("chaude-bridge-filter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let cwd = "/w/filter";
        let dir = session_view::project_dir(&home, cwd);
        std::fs::create_dir_all(dir.join("real").join("subagents")).unwrap();
        let user = |p: &str| {
            format!("{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"{p}\"}}}}\n")
        };
        let side = |p: &str| {
            format!(
                "{{\"type\":\"user\",\"isSidechain\":true,\"message\":{{\"role\":\"user\",\"content\":\"{p}\"}}}}\n"
            )
        };
        let write = |name: &str, body: &str| std::fs::write(dir.join(name), body).unwrap();
        write("real.jsonl", &user("the user's real prompt"));
        write("helper.jsonl", &user("[prompter summary] Intent: x"));
        write(
            "carry.jsonl",
            &user("[Context carried over from a previous session that was rotated"),
        );
        write("side.jsonl", &side("only a sub-agent prompt"));
        write("blank.jsonl", &user("   "));
        write(
            "empty.jsonl",
            "{\"type\":\"queue-operation\",\"operation\":\"enqueue\"}\n",
        );
        // A sub-agent transcript sits under its session dir; it is never a top-level row.
        std::fs::write(
            dir.join("real").join("subagents").join("agent-a.jsonl"),
            side("a sub-agent"),
        )
        .unwrap();

        let past = past_sessions(&home, cwd);
        assert_eq!(
            past.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["real"]
        );
        assert_eq!(past[0].first, "the user's real prompt");
        assert!(is_harness_prompt(
            "lang: ko Opus main return follows. Use the relay"
        ));
        assert!(!is_harness_prompt("fix the build"));
        let _ = std::fs::remove_dir_all(&home);
    }

    fn kinds_of(evs: &[Event]) -> Vec<&'static str> {
        evs.iter().map(Event::kind).collect()
    }
}
