//! The eframe app: drains engine events into GuiState once per frame, draws the output pane and the input box.

use std::sync::mpsc::Receiver;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use chaude_core::api::{EngineHandle, Role, UiEvent};
use chaude_core::ui_state::BarInput;
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
        if let Ok(cwd) = std::env::current_dir() {
            state.load_history(&cwd);
        }
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
        egui::Window::new("permission")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.strong(format!("Allow {}?", p.tool));
                ui.label(&p.summary);
                ui.horizontal(|ui| {
                    if ui.button("Allow (y)").clicked() {
                        choice = Some(true);
                    }
                    if ui.button("Deny (n)").clicked() {
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
        ui.weak(status_line(&bar));
        let activity = activity_line(&bar);
        if !activity.is_empty() {
            ui.weak(activity);
        }
    }

    fn input_box(&mut self, ui: &mut egui::Ui) {
        // No input while a permission prompt is open; the dialog takes the keys.
        let ready = self.engine.is_some() && self.state.permission.is_none();
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
        self.shortcuts(ui.ctx());
        // Bottom panels stack in show order: the status bar sits below the input box.
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::bottom("input").show(ui, |ui| self.input_box(ui));
        egui::CentralPanel::default().show(ui, |ui| self.output(ui));
        self.permission_dialog(ui.ctx());
    }
}

// Compact token count: 999 stays as is, 1500 becomes "1.5k".
fn kfmt(n: u64) -> String {
    if n >= 1000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

// Model, context percent, tokens, cost, and the permission mode when it is not the default.
fn status_line(bar: &BarInput) -> String {
    let mut parts = vec![
        bar.model.clone(),
        format!("ctx {}%", bar.context_pct),
        format!("{} tok", kfmt(bar.tokens)),
        format!("${:.2}", bar.cost_usd),
    ];
    if !bar.mode.is_empty() && bar.mode != "default" {
        parts.push(bar.mode.clone());
    }
    if bar.rot > 0 {
        parts.push(format!("rot {}", bar.rot));
    }
    if let Some(rate) = &bar.rate {
        parts.push(rate.clone());
    }
    parts.join(" · ")
}

// Turn timer, running agents, queue, held reply, per-role context, and the last turn's tokens. Empty when idle.
fn activity_line(bar: &BarInput) -> String {
    let mut parts = Vec::new();
    if let Some(secs) = bar.turn_secs {
        parts.push(format!("turn {secs}s"));
    }
    if bar.fg + bar.bg > 0 {
        parts.push(format!("agents fg {} bg {}", bar.fg, bar.bg));
    }
    if bar.queued > 0 {
        parts.push(format!("queued {}", bar.queued));
    }
    if let Some(n) = bar.waiting {
        parts.push(format!("held, waiting {n}"));
    }
    for (role, tokens) in &bar.role_tokens {
        parts.push(format!("{role} {}", kfmt(*tokens)));
    }
    if let Some((input, output, cache)) = bar.turn_tokens {
        parts.push(format!(
            "last in {} out {} cache {}",
            kfmt(input),
            kfmt(output),
            kfmt(cache)
        ));
    }
    parts.join(" · ")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kfmt_keeps_small_and_shortens_large() {
        assert_eq!(kfmt(999), "999");
        assert_eq!(kfmt(1500), "1.5k");
    }

    #[test]
    fn status_line_hides_default_mode() {
        let bar = BarInput {
            model: "opus".into(),
            context_pct: 12,
            tokens: 2500,
            cost_usd: 0.5,
            mode: "default".into(),
            ..Default::default()
        };
        assert_eq!(status_line(&bar), "opus · ctx 12% · 2.5k tok · $0.50");
        let plan = BarInput {
            mode: "plan".into(),
            ..bar
        };
        assert!(status_line(&plan).ends_with("· plan"));
    }

    #[test]
    fn activity_line_is_empty_when_idle() {
        assert_eq!(activity_line(&BarInput::default()), "");
        let busy = BarInput {
            turn_secs: Some(7),
            queued: 2,
            ..Default::default()
        };
        assert_eq!(activity_line(&busy), "turn 7s · queued 2");
    }
}
