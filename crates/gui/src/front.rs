//! The GUI's side of the bridge. Page commands go into the core logic (GuiState, slash dispatch, the engine); core
//! changes come back as page events through the core Mirror. No window types here, so the window thread only moves
//! messages.

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use chaude_core::api::{Config, EngineHandle, InterruptSource, UiEvent};
use chaude_core::gui_bridge::{Command, Event, Mirror, PermissionView, StateView};
use chaude_core::handoff::session_mode_note;
use chaude_core::i18n::{self, Msg};
use chaude_core::settings::pickers;
use chaude_core::slash;

use crate::commands::{self, Effect};
use crate::engine;
use crate::state::GuiState;

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
}

impl Front {
    // Starts the GUI state and the engine in the current folder, as the TUI starts. wake is called from any thread
    // when the engine or a job has something for the window.
    pub fn start(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut state = GuiState::default();
        state.cwd = cwd.display().to_string();
        // Read before the engine starts: its live marker would hide a killed run (TUI killed_sid).
        state.killed_sid = chaude_core::group_pick::killed_session(&state.cwd);
        let cfg = Config::default();
        let handoff = engine::handoff_path(&cwd);
        state.session_mode =
            session_mode_note(Some(&handoff), &cfg.model, cfg.rotate_threshold, false);
        if let Some(mode) = state.session_mode.clone() {
            state.transcript.push_notice(mode);
        }
        let w = wake.clone();
        let (engine, rx) = match engine::start(move || w()) {
            Ok((handle, rx)) => (Some(handle), Some(rx)),
            Err(e) => {
                state.apply(UiEvent::Error(e), Instant::now());
                (None, None)
            }
        };
        let (jobs_tx, jobs_rx) = mpsc::channel();
        Self {
            state,
            quit: false,
            engine,
            rx,
            jobs_tx,
            jobs_rx,
            wake,
            mirror: Mirror::default(),
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
            Command::Ping => return vec![Event::Pong],
            Command::Submit { text } => self.submit(&text),
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
            Command::Copy { text } => self.state.copy_done(text.chars().count(), now),
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
        self.mirror
            .sync(&self.state.transcript, state, &pickers, note.as_deref())
    }
}
