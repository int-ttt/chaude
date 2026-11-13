//! The eframe app: drains engine events into GuiState once per frame, draws the output pane and the input box.

use std::sync::mpsc::Receiver;
use std::time::Instant;

use chaude_core::api::{EngineHandle, Role, UiEvent};
use eframe::egui::{self, Key, Modifiers};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::engine;
use crate::state::{Entry, GuiState};

pub struct ChaudeApp {
    // Kept alive for the app's lifetime; dropping it shuts the engine down.
    engine: Option<EngineHandle>,
    rx: Option<Receiver<UiEvent>>,
    state: GuiState,
    md: CommonMarkCache,
    input: String,
    // Whether the input box had focus last frame; decides if Enter and arrows are ours before the TextEdit runs.
    input_focused: bool,
    // Index into state.history while recalling with Up/Down; None while editing a new prompt.
    recall: Option<usize>,
    // The unsent text kept while recalling history.
    draft: String,
    // Set on submit: the output pane jumps to the tail for one frame.
    jump_tail: bool,
}

impl ChaudeApp {
    pub fn new(ctx: egui::Context) -> Self {
        let mut state = GuiState::default();
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
            input: String::new(),
            input_focused: false,
            recall: None,
            draft: String::new(),
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
        if self.input.trim().is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.input);
        engine.submit(&text);
        self.state.push_user(&text);
        self.recall = None;
        self.draft.clear();
        self.jump_tail = true;
    }

    // Steps through prompt history: older on Up, newer on Down. The unsent text is kept as the draft.
    fn step_history(&mut self, older: bool) {
        let len = self.state.history.len();
        if len == 0 {
            return;
        }
        let next = match (self.recall, older) {
            (None, true) => {
                self.draft = self.input.clone();
                Some(len - 1)
            }
            (None, false) => return,
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) => (i + 1 < len).then_some(i + 1),
        };
        self.input = match next {
            Some(i) => self.state.history[i].clone(),
            None => std::mem::take(&mut self.draft),
        };
        self.recall = next;
    }

    fn input_box(&mut self, ui: &mut egui::Ui) {
        let ready = self.engine.is_some();
        let focused = ready && self.input_focused;
        let single_line = !self.input.contains('\n');
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
        if up {
            self.step_history(true);
        }
        if down {
            self.step_history(false);
        }
        if enter {
            // A trailing backslash turns Enter into a newline (the TUI rule); anything else sends.
            if let Some(rest) = self.input.strip_suffix('\\') {
                self.input = format!("{rest}\n");
            } else {
                self.submit();
            }
        }
        let out = ui.add(
            egui::TextEdit::multiline(&mut self.input)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .interactive(ready),
        );
        self.input_focused = out.has_focus();
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
        egui::Panel::bottom("input").show(ui, |ui| self.input_box(ui));
        egui::CentralPanel::default().show(ui, |ui| self.output(ui));
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
        Entry::Notice(t) => {
            ui.label(t);
        }
        Entry::Error(t) => {
            ui.colored_label(egui::Color32::RED, format!("error: {t}"));
        }
    }
}
