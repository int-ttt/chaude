//! The GUI's side of the bridge. Page commands go into the core logic (GuiState, slash dispatch, the engine); core
//! changes come back as page events through the core Mirror. No window types here, so the window thread only moves
//! messages.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use chaude_core::api::{Config, EngineHandle, InterruptSource, UiEvent};
use chaude_core::gui_bridge::{Command, Event, Mirror, Mode, PermissionView, StateView, open_plan};
use chaude_core::handoff::session_mode_note;
use chaude_core::i18n::{self, Msg};
use chaude_core::osenv;
use chaude_core::recent_folders::{self, RecentFolder};
use chaude_core::settings::pickers;
use chaude_core::slash;
use chaude_core::usage_poll::UsageMeter;

use crate::commands::{self, Effect};
use crate::engine;
use crate::state::GuiState;

// Writes the text to the OS clipboard. False when the clipboard is not reachable (then no copied note is shown).
fn copy_to_clipboard(text: &str) -> bool {
    match arboard::Clipboard::new().and_then(|mut clip| clip.set_text(text)) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("copy: clipboard failed: {e}");
            false
        }
    }
}

pub struct Front {
    pub state: GuiState,
    // Set by /exit and /quit; the window closes after the events of that command went out.
    pub quit: bool,
    engine: Option<EngineHandle>,
    rx: Option<Receiver<UiEvent>>,
    // Background jobs (doctor, update) send their text here and call wake.
    jobs_tx: Sender<String>,
    jobs_rx: Receiver<String>,
    wake: Arc<dyn Fn() + Send + Sync>,
    mirror: Mirror,
    // The chaude config dir (recent folders, chat scratch dirs). Tests pass a temp dir.
    config: PathBuf,
    // The session on screen: its mode and the code folder ("" for chat). None = the start screen.
    session: Option<(Mode, String)>,
    // The engine's dir (the scratch dir for chat) and the user's home (paths show as ~).
    cwd: String,
    home: String,
    // Recent folders, newest first, as recent.json holds them.
    recent: Vec<RecentFolder>,
    // Chat scratch dirs, newest first (the sidebar's chats).
    chats: Vec<RecentFolder>,
    // The Session event the page last got, so it is sent only when it changes.
    sent_session: Option<Event>,
    // The sidebar meter the page last got (Event::Usage), so it is sent only when it changes.
    sent_usage: Option<UsageMeter>,
    // Engine busy at the last sync; a true -> false edge is the turn end that notifies.
    was_busy: bool,
}

impl Front {
    // Starts on the start screen: no engine until the user picks New chat or a folder. wake is called from any thread
    // when the engine or a job has something for the window.
    pub fn start(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self::with_config(wake, recent_folders::config_dir())
    }

    fn with_config(wake: Arc<dyn Fn() + Send + Sync>, config: PathBuf) -> Self {
        let recent = recent_folders::load(&recent_folders::recent_file(&config));
        let chats = recent_folders::chat_dirs(&config);
        let (jobs_tx, jobs_rx) = mpsc::channel();
        Self {
            state: GuiState::default(),
            quit: false,
            engine: None,
            rx: None,
            jobs_tx,
            jobs_rx,
            wake,
            mirror: Mirror::default(),
            config,
            session: None,
            cwd: String::new(),
            home: osenv::home_dir().display().to_string(),
            recent,
            chats,
            sent_session: None,
            sent_usage: None,
            was_busy: false,
        }
    }

    // Starts a session: chat in a new scratch dir, or code with the folder as the engine cwd. A previous engine is
    // shut down first. A boot failure shows as an error row in the new session.
    fn start_session(&mut self, mode: Mode, folder: &str, now: Instant) {
        let cwd = match mode {
            Mode::Chat => {
                let made = recent_folders::new_chat_dir(&self.config, &chat_id())
                    .map_err(|e| format!("chat: cannot make the scratch dir: {e}"));
                self.chats = recent_folders::chat_dirs(&self.config);
                made
            }
            Mode::Code => {
                let file = recent_folders::recent_file(&self.config);
                if let Ok(rows) = recent_folders::touch(&file, folder, now_secs()) {
                    self.recent = rows;
                }
                Ok(PathBuf::from(folder))
            }
        };
        self.enter(mode, folder, cwd, now);
    }

    // Back to the start screen: the engine stops and the session on screen is cleared. Nothing is made on disk.
    fn show_start(&mut self) {
        if let Some(old) = self.engine.take() {
            old.shutdown();
        }
        self.rx = None;
        self.state = GuiState::default();
        self.cwd.clear();
        self.session = None;
        self.was_busy = false;
    }

    // Puts a session on screen (folder is the code folder, "" for chat): the old engine stops, the state resets and the
    // engine starts in cwd. A failed cwd (no scratch dir) shows as an error row.
    fn enter(&mut self, mode: Mode, folder: &str, cwd: Result<PathBuf, String>, now: Instant) {
        if let Some(old) = self.engine.take() {
            old.shutdown();
        }
        self.rx = None;
        self.state = GuiState::default();
        self.cwd.clear();
        match cwd {
            Ok(cwd) => {
                self.cwd = cwd.display().to_string();
                self.boot(cwd, mode == Mode::Chat, now);
            }
            Err(e) => self.state.apply(UiEvent::Error(e), now),
        }
        self.session = Some((
            mode,
            match mode {
                Mode::Chat => String::new(),
                Mode::Code => folder.to_string(),
            },
        ));
        self.was_busy = false;
    }

    // Opens a past session (a sidebar row): its folder or chat scratch dir becomes the engine cwd, and the session's
    // transcript is shown and installed for the next prompt, as /import does. A dir with no session starts empty.
    fn open_session(&mut self, mode: Mode, folder: &str, id: &str, now: Instant) {
        let chat_root = self.config.join("chat");
        let Some(plan) = open_plan(mode, folder, id, &osenv::home_dir(), &chat_root) else {
            return;
        };
        if mode == Mode::Code
            && let Ok(rows) = recent_folders::touch(
                &recent_folders::recent_file(&self.config),
                folder,
                now_secs(),
            )
        {
            self.recent = rows;
        }
        self.enter(mode, folder, Ok(PathBuf::from(&plan.cwd)), now);
        if plan.id.is_empty() {
            self.state
                .transcript
                .push_notice("no past session in this folder yet".to_string());
        } else if let (Some(imp), Some(engine)) = (self.state.import(&plan.id), &self.engine) {
            engine.install_import(imp);
        }
    }

    // Starts the engine in cwd and fills the state the TUI shows at start (the killed-run notice and the session note).
    fn boot(&mut self, cwd: PathBuf, chat: bool, now: Instant) {
        self.state.cwd = cwd.display().to_string();
        // Read before the engine starts: its live marker would hide a killed run (TUI killed_sid).
        self.state.killed_sid = chaude_core::group_pick::killed_session(&self.state.cwd);
        let cfg = Config::default();
        let handoff = engine::handoff_path(&cwd);
        self.state.session_mode =
            session_mode_note(Some(&handoff), &cfg.model, cfg.rotate_threshold, false);
        if let Some(mode) = self.state.session_mode.clone() {
            self.state.transcript.push_notice(mode);
        }
        let w = self.wake.clone();
        match engine::start(cwd, chat, move || w()) {
            Ok((handle, rx)) => {
                handle.usage_meter();
                self.engine = Some(handle);
                self.rx = Some(rx);
            }
            Err(e) => self.state.apply(UiEvent::Error(e), now),
        }
    }

    // One OS notification when a turn ends (the window may be behind). A failed notification is ignored.
    fn notify_turn_end(&self) {
        let name = match &self.session {
            Some((Mode::Code, folder)) => folder_name(folder),
            _ => "chat".to_string(),
        };
        // The macOS send blocks on the notification center, so it runs off the window thread.
        std::thread::spawn(move || {
            let _ = notify_rust::Notification::new()
                .summary("chaude")
                .body(&format!("Reply ready in {name}"))
                .show();
        });
    }

    // The Session event for the page: the mode and folder on screen, and the recent folders and chat dirs.
    fn session_event(&self) -> Event {
        let (mode, folder) = match &self.session {
            Some((mode, folder)) => (Some(*mode), folder.clone()),
            None => (None, String::new()),
        };
        Event::Session {
            mode,
            folder,
            cwd: self.cwd.clone(),
            home: self.home.clone(),
            recent: self.recent.clone(),
            chats: self.chats.clone(),
        }
    }

    // Applies what the engine and the background jobs sent since the last call; returns the page events.
    pub fn pump(&mut self, now: Instant) -> Vec<Event> {
        while let Ok(text) = self.jobs_rx.try_recv() {
            self.state.transcript.push_notice(text);
        }
        if let Some(rx) = &self.rx {
            while let Ok(ev) = rx.try_recv() {
                self.state.apply(ev, now);
            }
        }
        self.sync(now)
    }

    // Routes one page command into the core logic; returns the page events it caused.
    pub fn command(&mut self, cmd: Command, now: Instant) -> Vec<Event> {
        match cmd {
            // The page's first ping (on load) gets the whole transcript and state, so nothing sent before it is lost.
            Command::Ping => {
                let mut out = vec![Event::Pong];
                out.extend(self.sync(now));
                return out;
            }
            // A chat makes its scratch dir only with a message; an empty New chat just shows the start screen.
            Command::Start { mode, folder, text } => {
                if mode == Mode::Chat && text.trim().is_empty() {
                    self.show_start();
                } else if mode == Mode::Chat || !folder.is_empty() {
                    self.start_session(mode, &folder, now);
                    if !text.is_empty() {
                        self.submit(&text);
                    }
                }
            }
            Command::OpenSession { mode, folder, id } => {
                self.open_session(mode, &folder, &id, now);
            }
            Command::OpenFolder => {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    self.start_session(Mode::Code, &dir.display().to_string(), now);
                }
            }
            Command::Submit { text } => self.submit(&text),
            // The sidebar meter's click: the same window as /usage, without a line in the transcript.
            Command::Usage => {
                if self.engine.is_some() {
                    self.run(Effect::OpenUsage, "");
                }
            }
            Command::Slash { name, arg } => {
                let line = if arg.is_empty() {
                    format!("/{name}")
                } else {
                    format!("/{name} {arg}")
                };
                self.submit(&line);
            }
            Command::Interrupt => {
                if let (Some(engine), true) = (&self.engine, self.state.run.busy) {
                    engine.interrupt(InterruptSource::EscKey);
                }
            }
            Command::PermissionAnswer { allow, interrupt } => self.answer(allow, interrupt),
            Command::PickerPick { picker, row } => self.pick(&picker, row),
            Command::Copy { text } => {
                if copy_to_clipboard(&text) {
                    self.state.copy_done(text.chars().count(), now);
                }
            }
        }
        self.sync(now)
    }

    // Runs one line the way the TUI does: core dispatch decides what it is, the GUI runs its own effects, the rest
    // goes to the engine. The line is echoed as a user row either way.
    fn submit(&mut self, line: &str) {
        let text = line.trim();
        if text.is_empty() {
            return;
        }
        let outcome = slash::dispatch(
            text,
            &commands::ctx(&self.state.bar, self.state.run.busy, &self.state.session_id),
        );
        let effect = commands::effect(outcome, Path::new(&self.state.cwd));
        self.run(effect, text);
        self.state.push_user(text);
    }

    fn run(&mut self, effect: Effect, text: &str) {
        match effect {
            Effect::Note(s) => self.state.transcript.push_notice(s),
            Effect::OpenView(arg) => self.state.open_view(&arg),
            Effect::Quit => self.quit = true,
            Effect::SetMode(p) => {
                if let Some(engine) = &self.engine {
                    engine.set_mode(p.name());
                }
            }
            Effect::OpenUsage => {
                self.state.open_usage();
                if let Some(engine) = &self.engine {
                    engine.usage();
                }
            }
            Effect::OpenLang => {
                let note = pickers::lang_current_note(None);
                self.state.transcript.push_notice(note);
                self.state.open_lang();
            }
            Effect::SetLang(code) => {
                let note = pickers::lang_apply(None, &code);
                self.state.transcript.push_notice(note);
                if pickers::lang_is_code(&code) {
                    self.engine_submit(&format!("/lang {code}"));
                }
            }
            Effect::OpenSettings => {
                let cwd = self.state.cwd.clone();
                self.state.open_settings(Some(Path::new(&cwd)));
            }
            Effect::Resume(args) => {
                if let (Some(imp), Some(engine)) = (self.state.resume(&args), &self.engine) {
                    engine.install_import(imp);
                }
            }
            Effect::Import(arg) => {
                if let (Some(imp), Some(engine)) = (self.state.import(&arg), &self.engine) {
                    engine.install_import(imp);
                }
            }
            Effect::Doctor => {
                self.state
                    .transcript
                    .push_notice(i18n::t(Msg::DoctorChecking));
                let (tx, wake) = (self.jobs_tx.clone(), self.wake.clone());
                std::thread::spawn(move || {
                    let lines = chaude_core::doctor::render(&chaude_core::doctor::checks_now());
                    let _ = tx.send(lines.join("\n"));
                    wake();
                });
            }
            Effect::Update => {
                self.state.transcript.push_notice(slash::UPDATE_START_NOTE);
                let (tx, wake) = (self.jobs_tx.clone(), self.wake.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(chaude_update::text(env!("CARGO_PKG_VERSION")));
                    wake();
                });
            }
            Effect::Send => self.engine_submit(text),
        }
    }

    fn engine_submit(&self, line: &str) {
        if let Some(engine) = &self.engine {
            engine.submit(line);
        }
    }

    // Answers the open permission prompt and tells the engine. An answer that is not an interrupt is echoed as
    // allowed or denied (the TUI key()); interrupt (Ctrl+C) also stops the turn.
    fn answer(&mut self, allow: bool, interrupt: bool) {
        let Some((id, allow)) = self.state.answer_permission(allow) else {
            return;
        };
        if !interrupt {
            self.state
                .transcript
                .push_status(if allow { "allowed" } else { "denied" });
            self.state.transcript.push_blank();
        }
        if let Some(engine) = &self.engine {
            engine.answer_permission(&id, allow);
            if interrupt {
                engine.interrupt(InterruptSource::CtrlC);
            }
        }
    }

    // A pick in an open picker. Lang, record and session-group pickers run here; rewind and /view come with their
    // steps (6), so a pick there gets the not-yet note.
    fn pick(&mut self, picker: &str, row: Option<usize>) {
        match picker {
            "lang" => {
                if let Some(code) = self.state.lang_pick(row) {
                    self.run(Effect::SetLang(code.to_string()), "");
                }
            }
            "records" => {
                let _ = self.state.records_pick(row);
            }
            "groups" => {
                if let (Some(imp), Some(engine)) = (self.state.groups_pick(row), &self.engine) {
                    engine.install_import(imp);
                }
            }
            other => self.state.transcript.push_notice(commands::not_yet(other)),
        }
    }

    // The page events for what changed since the last sync: the transcript rows, the state, the open pickers, the
    // copy note.
    fn sync(&mut self, now: Instant) -> Vec<Event> {
        let mut pickers: Vec<&str> = Vec::new();
        if self.state.rewind.is_open() {
            pickers.push("rewind");
        }
        if self.state.records.is_some() {
            pickers.push("records");
        }
        if self.state.groups.is_some() {
            pickers.push("groups");
        }
        if self.state.sess.is_some() {
            pickers.push("view");
        }
        if self.state.usage.is_some() {
            pickers.push("usage");
        }
        if self.state.lang.is_some() {
            pickers.push("lang");
        }
        if self.state.settings.is_some() {
            pickers.push("settings");
        }
        let permission = self.state.permission.as_ref().map(|p| PermissionView {
            id: p.id.clone(),
            tool: p.tool.clone(),
            summary: p.summary.clone(),
        });
        let state = StateView::new(
            &self.state.run,
            &self.state.bar,
            &self.state.session_id,
            permission,
        );
        let note = self.state.copy_note_text(now).map(str::to_string);
        let busy = self.state.run.busy;
        if self.was_busy && !busy && self.session.is_some() {
            self.notify_turn_end();
            // The core throttles this to one background /usage run per interval.
            if let Some(engine) = &self.engine {
                engine.usage_meter();
            }
        }
        self.was_busy = busy;
        let mut out = Vec::new();
        let session = self.session_event();
        if self.sent_session.as_ref() != Some(&session) {
            self.sent_session = Some(session.clone());
            out.push(session);
        }
        if self.state.meter != self.sent_usage {
            self.sent_usage = self.state.meter.clone();
            out.push(Event::Usage {
                meter: self.sent_usage.clone(),
            });
        }
        out.extend(
            self.mirror
                .sync(&self.state.transcript, state, &pickers, note.as_deref()),
        );
        out
    }
}

// A unique id for a new chat's scratch dir (nanoseconds since the epoch, so two chats in one second differ).
fn chat_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}-{}", std::process::id())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// The last path part of a folder, for the header and the notification.
fn folder_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // A fresh temp config dir per test; never the real home.
    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chaude-front-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn front(name: &str) -> (Front, PathBuf) {
        let dir = temp(name);
        (Front::with_config(Arc::new(|| {}), dir.clone()), dir)
    }

    #[test]
    fn start_screen_sends_session_once_then_no_repeat() {
        let (mut f, dir) = front("session");
        let now = Instant::now();
        let first = f.pump(now);
        assert_eq!(
            first
                .iter()
                .filter(|e| matches!(e, Event::Session { .. }))
                .count(),
            1
        );
        assert!(first.contains(&Event::Session {
            mode: None,
            folder: String::new(),
            cwd: String::new(),
            home: osenv::home_dir().display().to_string(),
            recent: Vec::new(),
            chats: Vec::new(),
        }));
        let again = f.pump(now);
        assert!(!again.iter().any(|e| matches!(e, Event::Session { .. })));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recent_folders_load_from_config_dir() {
        let (f, dir) = front("recent");
        recent_folders::touch(&recent_folders::recent_file(&dir), "/w/proj", 5).unwrap();
        let f2 = Front::with_config(Arc::new(|| {}), dir.clone());
        assert_eq!(f2.recent.len(), 1);
        assert_eq!(f2.recent[0].path, "/w/proj");
        assert!(f.recent.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chat_dirs_reach_the_session_event() {
        let (_, dir) = front("chats");
        recent_folders::new_chat_dir(&dir, "c1").unwrap();
        let f = Front::with_config(Arc::new(|| {}), dir.clone());
        match f.session_event() {
            Event::Session { mode, chats, .. } => {
                assert_eq!(mode, None);
                assert_eq!(chats.len(), 1);
                assert!(chats[0].path.ends_with("c1"));
            }
            other => panic!("not a session event: {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_session_ignores_a_chat_row_outside_the_chat_dir() {
        let (mut f, dir) = front("open-session");
        let now = Instant::now();
        let out = f.command(
            Command::OpenSession {
                mode: Mode::Chat,
                folder: "/w/proj".into(),
                id: String::new(),
            },
            now,
        );
        // No session starts, so no engine and no session on screen; the page still gets the state.
        assert!(f.session.is_none());
        assert!(f.engine.is_none());
        assert!(
            !out.iter()
                .any(|e| matches!(e, Event::Session { mode: Some(_), .. }))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_new_chat_makes_no_scratch_dir_and_shows_the_start_screen() {
        let (mut f, dir) = front("empty-chat");
        let now = Instant::now();
        for text in ["", "   \n\t"] {
            f.command(
                Command::Start {
                    mode: Mode::Chat,
                    folder: String::new(),
                    text: text.into(),
                },
                now,
            );
            assert!(f.session.is_none());
            assert!(f.engine.is_none());
            assert!(f.chats.is_empty());
            assert!(!dir.join("chat").exists());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_name_is_last_part() {
        assert_eq!(folder_name("/w/my-proj"), "my-proj");
        assert_eq!(folder_name("/"), "/");
    }

    #[test]
    fn chat_ids_differ() {
        assert_ne!(chat_id(), chat_id());
    }
}
