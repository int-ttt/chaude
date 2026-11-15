//! The eframe app: drains engine events into GuiState once per frame, draws the output pane and the input box.

use std::sync::mpsc::Receiver;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use chaude_core::api::{EngineHandle, Role, UiEvent};
use chaude_core::bar_text;
use chaude_core::i18n::{self, Msg};
use chaude_core::ui_state::Entry;
use eframe::egui::text::{CCursor, CCursorRange};
use eframe::egui::{self, Key, Modifiers};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::engine;
use crate::input::Input;
use crate::state::GuiState;

// Stable id so the input box's TextEdit state (cursor) can be set from the editor.
const INPUT_ID: &str = "chaude-input";

pub struct ChaudeApp {
    // Kept alive for the app's lifetime; dropping it shuts the engine down.
    engine: Option<EngineHandle>,
    rx: Option<Receiver<UiEvent>>,
    state: GuiState,
    md: CommonMarkCache,
    // The input box text, cursor and prompt history (core Editor).
    input: Input,
    // Whether the input box had focus last frame; decides if Enter and arrows are ours before the TextEdit runs.
    input_focused: bool,
    // Set on submit: the output pane jumps to the tail for one frame.
    jump_tail: bool,
}

impl ChaudeApp {
    pub fn new(ctx: egui::Context) -> Self {
        let mut state = GuiState::default();
        let cwd = std::env::current_dir().unwrap_or_default();
        let (engine, rx) = match engine::start(ctx) {
            Ok((handle, rx)) => (Some(handle), Some(rx)),
            Err(e) => {
                state.apply(UiEvent::Error(e), Instant::now());
                (None, None)
            }
        };
        Self {
            engine,
            rx,
            state,
            md: CommonMarkCache::default(),
            input: Input::new(&cwd),
            input_focused: false,
            jump_tail: false,
        }
    }

    fn drain(&mut self) {
        if let Some(rx) = &self.rx {
            while let Ok(ev) = rx.try_recv() {
                self.state.apply(ev, Instant::now());
            }
        }
    }

    // Sends the input as a prompt (or /command) and echoes it in the transcript.
    fn submit(&mut self) {
        let Some(engine) = &self.engine else { return };
        let Some(text) = self.input.submit() else {
            return;
        };
        engine.submit(&text);
        self.state.push_user(&text);
        self.jump_tail = true;
    }

    // Moves the widget's cursor to the editor's cursor after the editor changed the text (history, Enter).
    fn place_cursor(&self, ui: &egui::Ui) {
        let id = ui.make_persistent_id(INPUT_ID);
        let mut st = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        let at = CCursor::new(self.input.cursor());
        st.cursor.set_char_range(Some(CCursorRange::one(at)));
        st.store(ui.ctx(), id);
    }

    // Answers the open permission prompt and tells the engine.
    fn answer(&mut self, allow: bool) {
        let Some((id, allow)) = self.state.answer_permission(allow) else {
            return;
        };
        if let Some(engine) = &self.engine {
            engine.answer_permission(&id, allow);
        }
    }

    // Global keys, consumed before any widget sees them: y/n while a permission prompt is open, Shift+Tab cycles mode.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let (mut yes, mut no, mut mode) = (false, false, false);
        ctx.input_mut(|i| {
            if self.state.permission.is_some() {
                yes = i.consume_key(Modifiers::NONE, Key::Y);
                no = i.consume_key(Modifiers::NONE, Key::N);
            }
            mode = i.consume_key(Modifiers::SHIFT, Key::Tab);
        });
        if yes || no {
            self.answer(yes);
        }
        if mode && let Some(engine) = &self.engine {
            engine.set_mode("next");
        }
    }

    // Modal y/n dialog for a can_use_tool request. Buttons and the y/n keys both answer it.
    fn permission_dialog(&mut self, ctx: &egui::Context) {
        let Some(p) = self.state.permission.clone() else {
            return;
        };
        let mut choice = None;
        egui::Window::new(i18n::t(Msg::PermTitle))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.strong(i18n::tf(Msg::PermAsk, &[&p.tool]));
                ui.label(&p.summary);
                ui.horizontal(|ui| {
                    if ui.button(i18n::t(Msg::PermAllow)).clicked() {
                        choice = Some(true);
                    }
                    if ui.button(i18n::t(Msg::PermDeny)).clicked() {
                        choice = Some(false);
                    }
                });
            });
        if let Some(allow) = choice {
            self.answer(allow);
        }
    }

    // Two status rows from the core BarInput: model, context, tokens and cost; then turn, agents and queue.
    fn status_bar(&self, ui: &mut egui::Ui) {
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let bar = self.state.bar_at(Instant::now(), now_unix);
        let lang = i18n::current();
        ui.weak(bar_text::status_line(lang, &bar));
        let activity = bar_text::activity_line(lang, &bar);
        if !activity.is_empty() {
            ui.weak(activity);
        }
    }

    fn input_box(&mut self, ui: &mut egui::Ui) {
        // No input while a permission prompt is open; the dialog takes the keys.
        let ready = self.engine.is_some() && self.state.permission.is_none();
        let focused = ready && self.input_focused;
        let single_line = self.input.is_single_line();
        let (mut enter, mut up, mut down) = (false, false, false);
        // Keys are consumed before the TextEdit sees them: plain Enter sends instead of inserting a newline.
        if focused {
            ui.input_mut(|i| {
                enter = i.consume_key(Modifiers::NONE, Key::Enter);
                if single_line {
                    up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
                    down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
                }
            });
        }
        // Editor-side changes (history recall, Enter) move the widget's cursor with them.
        let mut moved = false;
        if up {
            self.input.history_prev();
            moved = true;
        }
        if down {
            self.input.history_next();
            moved = true;
        }
        if enter {
            // A trailing backslash turns Enter into a newline (the TUI rule); anything else sends.
            if !self.input.backslash_newline() {
                self.submit();
            }
            moved = true;
        }
        if moved {
            self.place_cursor(ui);
        }
        // The widget edits a copy of the editor text (IME preedit and commit, paste, Shift+Enter); the edit is copied back.
        let mut shown = self.input.text();
        let out = egui::TextEdit::multiline(&mut shown)
            .id_salt(INPUT_ID)
            .desired_rows(3)
            .desired_width(f32::INFINITY)
            .interactive(ready)
            .show(ui);
        let cursor = out
            .cursor_range
            .map_or(self.input.cursor(), |r| r.primary.index.into());
        self.input.apply_widget(&shown, cursor);
        self.input_focused = out.response.has_focus();
    }

    fn output(&mut self, ui: &mut egui::Ui) {
        let mut area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true);
        if std::mem::take(&mut self.jump_tail) {
            area = area.vertical_scroll_offset(f32::INFINITY);
        }
        area.show(ui, |ui| {
            for entry in &self.state.transcript {
                render_entry(ui, &mut self.md, entry);
            }
        });
    }
}

impl eframe::App for ChaudeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain();
        self.shortcuts(ui.ctx());
        // Bottom panels stack in show order: the status bar sits below the input box.
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::bottom("input").show(ui, |ui| self.input_box(ui));
        egui::CentralPanel::default().show(ui, |ui| self.output(ui));
        self.permission_dialog(ui.ctx());
    }
}

fn render_entry(ui: &mut egui::Ui, md: &mut CommonMarkCache, entry: &Entry) {
    match entry {
        Entry::User(t) => {
            ui.strong(format!("> {t}"));
        }
        Entry::Text { role, text, .. } => {
            if *role != Role::Main {
                ui.weak("agent");
            }
            CommonMarkViewer::new().show(ui, md, text);
        }
        Entry::Tool {
            name, summary, ok, ..
        } => {
            let status = match ok {
                None => "...",
                Some(true) => "ok",
                Some(false) => "failed",
            };
            ui.weak(format!("{name} {summary} [{status}]"));
        }
        Entry::Agent {
            description,
            done,
            summary,
            ..
        } => {
            let status = if *done { "done" } else { "running" };
            ui.weak(format!("agent {description} [{status}] {summary}"));
        }
        Entry::ToolResult { text, .. } => {
            ui.weak(format!("  ⎿  {text}"));
        }
        Entry::Notice(t) => {
            ui.label(t);
        }
        Entry::Status(t) => {
            ui.weak(t);
        }
        Entry::Error(t) => {
            ui.colored_label(egui::Color32::RED, format!("error: {t}"));
        }
        Entry::Blank => {
            ui.add_space(ui.spacing().item_spacing.y);
        }
    }
}
