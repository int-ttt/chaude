//! The eframe app: drains engine events into GuiState once per frame, draws the output pane and the input box.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chaude_core::api::{AgentRow, EngineHandle, InterruptSource, Role, UiEvent};
use chaude_core::bar_text;
use chaude_core::i18n::{self, Msg};
use chaude_core::slash;
use chaude_core::ui_state::{
    Action, Entry, WELCOME_MODEL, WELCOME_THRESHOLD, ctrl_c_action, tool_line, tool_result_line,
    welcome_lines,
};
use eframe::egui::text::{CCursor, CCursorRange};
use eframe::egui::{self, Key, Modifiers};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::commands::{self, Effect};
use crate::engine;
use crate::input::Input;
use crate::state::GuiState;

// Stable id so the input box's TextEdit state (cursor) can be set from the editor.
const INPUT_ID: &str = "chaude-input";

// Row caps of the panels above the input box (the TUI's MAX_QUEUE_ROWS, MAX_TASK_ROWS, MAX_PANEL_ROWS).
const MAX_QUEUE_ROWS: usize = 5;
const MAX_TASK_ROWS: usize = 8;
const MAX_PANEL_ROWS: usize = 4;
const MAX_SLASH_ROWS: usize = 8;

// Desktop notification for a finished turn (sent only while the window is in the background). A failed send is
// ignored: the transcript already shows the reply.
fn turn_notice() {
    let _ = notify_rust::Notification::new()
        .summary("chaude")
        .body(i18n::t(Msg::NotifyTurnDone))
        .show();
}

// One dim row that is clipped to the panel width (the TUI clips its panel rows the same way).
fn weak_row(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.add(egui::Label::new(egui::RichText::new(text).weak()).truncate());
}

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
    // Output pane metrics from the last frame (scroll offset, largest offset, viewport height in points).
    view: ScrollView,
    // Keyboard scroll target for the next frame (PageUp/PageDown/Home), in points.
    scroll_to: Option<f32>,
    // True while the output shows the tail; false once the user scrolls up.
    follow: bool,
    // Output arrived while not following (the TUI's unseen-output bar).
    unseen: bool,
    // Results of /doctor and /update checks, run off the UI thread; drained into the transcript each frame.
    jobs_tx: Sender<String>,
    jobs_rx: Receiver<String>,
    // Selected row of the slash popup (clamped to the matches when drawn).
    slash_sel: usize,
}

// Output pane geometry from the last frame, in points.
#[derive(Clone, Copy, Default)]
struct ScrollView {
    offset: f32,
    max: f32,
    viewport: f32,
}

// Scroll keys: PageUp/PageDown move a page; Home/End go to the top or the tail (when the input box is empty).
#[derive(Clone, Copy)]
enum Jump {
    Up,
    Down,
    Home,
    End,
}

// Events that add visible transcript rows (the TUI's content rule for the unseen-output bar).
fn adds_output(ev: &UiEvent) -> bool {
    matches!(
        ev,
        UiEvent::TextDelta { .. }
            | UiEvent::ToolStart { .. }
            | UiEvent::ToolResult { .. }
            | UiEvent::Notice { .. }
            | UiEvent::Error(_)
    )
}

impl ChaudeApp {
    pub fn new(ctx: egui::Context) -> Self {
        let mut state = GuiState::default();
        let cwd = std::env::current_dir().unwrap_or_default();
        state.cwd = cwd.display().to_string();
        let (engine, rx) = match engine::start(ctx) {
            Ok((handle, rx)) => (Some(handle), Some(rx)),
            Err(e) => {
                state.apply(UiEvent::Error(e), Instant::now());
                (None, None)
            }
        };
        let (jobs_tx, jobs_rx) = mpsc::channel();
        Self {
            engine,
            rx,
            jobs_tx,
            jobs_rx,
            state,
            md: CommonMarkCache::default(),
            input: Input::new(&cwd),
            input_focused: false,
            jump_tail: false,
            view: ScrollView::default(),
            scroll_to: None,
            follow: true,
            unseen: false,
            slash_sel: 0,
        }
    }

    fn drain(&mut self, ctx: &egui::Context) {
        while let Ok(text) = self.jobs_rx.try_recv() {
            self.state.transcript.push_notice(text);
        }
        if let Some(rx) = &self.rx {
            while let Ok(ev) = rx.try_recv() {
                if !self.follow && adds_output(&ev) {
                    self.unseen = true;
                }
                // A turn that ends while the window is in the background notifies the desktop.
                if matches!(ev, UiEvent::TurnEnd { .. }) && !ctx.input(|i| i.focused) {
                    turn_notice();
                }
                self.state.apply(ev, Instant::now());
            }
        }
    }

    // Sends the input as a prompt or /command and echoes it in the transcript. Commands the GUI runs itself
    // (notes, /exit, /mode, /doctor, /update) stay here; the rest go to the engine.
    fn submit(&mut self, ctx: &egui::Context) {
        let Some(engine) = &self.engine else { return };
        let Some(text) = self.input.submit() else {
            return;
        };
        let outcome = slash::dispatch(
            &text,
            &commands::ctx(&self.state.bar, self.state.run.busy, &self.state.session_id),
        );
        match commands::effect(outcome, Path::new(&self.state.cwd)) {
            Effect::Note(s) => self.state.transcript.push_notice(s),
            Effect::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Effect::SetMode(p) => engine.set_mode(p.name()),
            Effect::OpenUsage => {
                self.state.open_usage();
                engine.usage();
            }
            Effect::Doctor => {
                self.state
                    .transcript
                    .push_notice(i18n::t(Msg::DoctorChecking));
                let (tx, ctx) = (self.jobs_tx.clone(), ctx.clone());
                std::thread::spawn(move || {
                    let lines = chaude_core::doctor::render(&chaude_core::doctor::checks_now());
                    let _ = tx.send(lines.join("\n"));
                    ctx.request_repaint();
                });
            }
            Effect::Update => {
                self.state.transcript.push_notice(slash::UPDATE_START_NOTE);
                let (tx, ctx) = (self.jobs_tx.clone(), ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(chaude_update::text(env!("CARGO_PKG_VERSION")));
                    ctx.request_repaint();
                });
            }
            Effect::Send => engine.submit(&text),
        }
        self.state.push_user(&text);
        self.jump_tail = true;
        self.slash_sel = 0;
    }

    // Moves the output by delta points from the last frame's offset, clamped to the scroll range.
    fn scroll_by(&mut self, delta: f32) {
        let to = (self.view.offset + delta).clamp(0.0, self.view.max);
        self.scroll_to = Some(to);
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
    // interrupt (Ctrl+C) also stops the turn. An answer that is not an interrupt is echoed as allowed or denied (TUI key()).
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

    // Keys of the open /usage window (TUI usage.rs usageKey): Esc, q, Enter and Ctrl+C close it. They run before
    // shortcuts, so Esc does not interrupt a turn and Ctrl+C does not quit while the window is open.
    fn window_keys(&mut self, ctx: &egui::Context) {
        if self.state.permission.is_some() || self.state.usage.is_none() {
            return;
        }
        let mut close = false;
        ctx.input_mut(|i| {
            for (m, k) in [
                (Modifiers::NONE, Key::Escape),
                (Modifiers::NONE, Key::Q),
                (Modifiers::NONE, Key::Enter),
                (Modifiers::CTRL, Key::C),
            ] {
                close |= i.consume_key(m, k);
            }
        });
        if close {
            self.state.close_usage();
        }
    }

    // Global keys, consumed before any widget sees them. While a permission prompt is open: y/Enter allow,
    // n/Esc deny, Ctrl+C deny and interrupt. Otherwise Esc cancels a held reply's wait or interrupts a busy turn,
    // Ctrl+C interrupts or quits, Ctrl+D quits, Shift+Tab cycles the mode, and PageUp/PageDown (plus Home/End on an
    // empty input box, as the TUI) scroll the output.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let perm = self.state.permission.is_some();
        let input_empty = self.input.text().is_empty();
        let (mut allow, mut deny, mut ctrl_c, mut ctrl_d, mut esc, mut mode) =
            (false, false, false, false, false, false);
        let mut jump = None;
        ctx.input_mut(|i| {
            if perm {
                allow = i.consume_key(Modifiers::NONE, Key::Y)
                    || i.consume_key(Modifiers::NONE, Key::Enter);
                deny = i.consume_key(Modifiers::NONE, Key::N)
                    || i.consume_key(Modifiers::NONE, Key::Escape);
                ctrl_c = i.consume_key(Modifiers::CTRL, Key::C);
            } else {
                esc = i.consume_key(Modifiers::NONE, Key::Escape);
                ctrl_c = i.consume_key(Modifiers::CTRL, Key::C);
                ctrl_d = i.consume_key(Modifiers::CTRL, Key::D);
                for (k, j) in [(Key::PageUp, Jump::Up), (Key::PageDown, Jump::Down)] {
                    if i.consume_key(Modifiers::NONE, k) {
                        jump = Some(j);
                    }
                }
                if input_empty {
                    for (k, j) in [(Key::Home, Jump::Home), (Key::End, Jump::End)] {
                        if i.consume_key(Modifiers::NONE, k) {
                            jump = Some(j);
                        }
                    }
                }
            }
            mode = i.consume_key(Modifiers::SHIFT, Key::Tab);
        });
        match jump {
            Some(Jump::Up) => self.scroll_by(-self.view.viewport),
            Some(Jump::Down) => self.scroll_by(self.view.viewport),
            Some(Jump::Home) => self.scroll_to = Some(0.0),
            Some(Jump::End) => self.jump_tail = true,
            None => {}
        }
        if perm {
            if ctrl_c {
                self.answer(false, true);
            } else if allow || deny {
                self.answer(allow, false);
            }
        } else {
            if esc && let Some(engine) = &self.engine {
                if self.state.run.held.is_some() {
                    engine.cancel_wait();
                } else if self.state.run.busy {
                    engine.interrupt(InterruptSource::EscKey);
                }
            }
            if ctrl_c {
                match ctrl_c_action(self.state.run.busy) {
                    Action::Interrupt(src) => {
                        if let Some(engine) = &self.engine {
                            engine.interrupt(src);
                        }
                    }
                    _ => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                }
            }
            if ctrl_d {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if mode && let Some(engine) = &self.engine {
            engine.set_mode("next");
        }
    }

    // The /usage window (TUI usage.rs): the text of the one-shot run, scrollable. Its X button closes it too.
    fn usage_window(&mut self, ctx: &egui::Context) {
        let Some(text) = self.state.usage.clone() else {
            return;
        };
        let mut open = true;
        egui::Window::new("Usage")
            .collapsible(false)
            .default_size([560.0, 360.0])
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(text);
                });
            });
        if !open {
            self.state.close_usage();
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
            self.answer(allow, false);
        }
    }

    // The status rows drawn from the core bar text (TUI bar.rs): the busy row while a turn runs, the running state
    // with its agent segments, then M/P/T/C with the rate window and the queue. The spinner turns while busy.
    fn status_bar(&self, ui: &mut egui::Ui) {
        let now = Instant::now();
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let bar = self.state.bar_at(now, now_unix);
        let run = &self.state.run;
        let lang = i18n::current();
        if run.busy {
            let model = if bar.model.is_empty() {
                WELCOME_MODEL
            } else {
                &bar.model
            };
            let busy = bar_text::busy_text(
                lang,
                model,
                &run.turn_prompt,
                bar.turn_secs,
                &run.last_tool,
                true,
            );
            ui.weak(busy);
            ui.ctx().request_repaint_after(Duration::from_millis(125));
        }
        let spin = run.turn_start.map_or(' ', |t| {
            bar_text::spinner_frame(now.saturating_duration_since(t))
        });
        let mut top = bar_text::top_text(lang, &bar, spin);
        for (desc, secs) in &bar.agents {
            top.push_str(&bar_text::agent_segment(lang, desc, *secs));
        }
        // Unseen output while scrolled up: the TUI's "↓ new message" marker on the top row.
        if self.unseen && !self.follow {
            top = format!("{top} · {}", i18n::t(Msg::NewMessage));
        }
        ui.weak(top);
        ui.weak(format!(
            "{} · {}",
            bar_text::bottom_text(lang, &bar),
            bar_text::cost_text(bar.cost_usd)
        ));
    }

    // Foreground (bg false) or background agent rows: running agents, busy residents (foreground only), then the
    // restored rows of that kind (the TUI panel order).
    fn agent_rows(&self, bg: bool) -> Vec<&AgentRow> {
        let run = &self.state.run;
        run.agent_rows
            .iter()
            .filter(|r| r.background == bg && !r.done)
            .chain(run.resident_busy.iter().filter(|_| !bg))
            .chain(run.restored_rows.iter().filter(|r| r.background == bg))
            .collect()
    }

    // Agent panel rows; past MAX_PANEL_ROWS the last row is the scroll range text (the GUI has no panel scroll yet, so
    // the window starts at the first row).
    fn agent_panel(&self, ui: &mut egui::Ui, bg: bool) {
        let list = self.agent_rows(bg);
        let lang = i18n::current();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let shown = if list.len() > MAX_PANEL_ROWS {
            MAX_PANEL_ROWS - 1
        } else {
            list.len()
        };
        for r in list.iter().take(shown) {
            let text = format!("● {}", bar_text::agent_row_text(lang, r, now));
            weak_row(ui, text);
        }
        if list.len() > shown {
            weak_row(ui, bar_text::scroll_range_text(lang, 0, shown, list.len()));
        }
    }

    // Queued prompts above the input box, oldest first (core queue_rows).
    fn queue_panel(&self, ui: &mut egui::Ui) {
        let rows =
            bar_text::queue_rows(i18n::current(), &self.state.run.queue_texts, MAX_QUEUE_ROWS);
        for row in rows {
            weak_row(ui, row);
        }
    }

    // Main's Agent/Task calls grouped 대기 / 진행 중 / 완료 / 실패 above the input box (core task_rows).
    fn task_panel(&self, ui: &mut egui::Ui) {
        let lang = i18n::current();
        weak_row(ui, i18n::t(Msg::TasksTitle));
        for row in bar_text::task_rows(lang, &self.state.run.tasks, MAX_TASK_ROWS - 1) {
            weak_row(ui, row);
        }
    }

    fn input_box(&mut self, ui: &mut egui::Ui) {
        // No input while a permission prompt or a /usage window is open; they take the keys.
        let ready =
            self.engine.is_some() && self.state.permission.is_none() && self.state.usage.is_none();
        let focused = ready && self.input_focused;
        let single_line = self.input.is_single_line();
        // Slash popup: the matches for a command word being typed (none while busy or a permission prompt is open).
        let text = self.input.text();
        let busy = self.state.run.busy;
        let perm_open = self.state.permission.is_some();
        let sugg = slash::suggestions(&text, busy, perm_open);
        let sel = self.slash_sel.min(sugg.len().saturating_sub(1));
        let (mut enter, mut up, mut down) = (false, false, false);
        let (mut tab, mut sel_up, mut sel_down) = (false, false, false);
        // Editor keys (core Editor, the TUI's emacs-style keys): Ctrl+A/E home/end, Ctrl+W delete word,
        // Ctrl+U/K kill to start/end, Ctrl+J or Alt+Enter newline, Alt+Backspace delete word.
        let mut edits: Vec<&'static str> = Vec::new();
        // Keys are consumed before the TextEdit sees them: plain Enter sends instead of inserting a newline.
        if focused {
            ui.input_mut(|i| {
                // Modifier combinations first: a plain Enter or Backspace match would also take them.
                for (m, k, op) in [
                    (Modifiers::ALT, Key::Enter, "newline"),
                    (Modifiers::ALT, Key::Backspace, "delete_word"),
                    (Modifiers::CTRL, Key::J, "newline"),
                    (Modifiers::CTRL, Key::A, "home"),
                    (Modifiers::CTRL, Key::E, "end"),
                    (Modifiers::CTRL, Key::W, "delete_word"),
                    (Modifiers::CTRL, Key::U, "kill_to_start"),
                    (Modifiers::CTRL, Key::K, "kill_to_end"),
                ] {
                    if i.consume_key(m, k) {
                        edits.push(op);
                    }
                }
                enter = i.consume_key(Modifiers::NONE, Key::Enter);
                if single_line && sugg.is_empty() {
                    up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
                    down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
                }
                // With the popup open, Tab completes the selected command and the arrows move the selection.
                if !sugg.is_empty() {
                    tab = i.consume_key(Modifiers::NONE, Key::Tab);
                    sel_up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
                    sel_down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
                }
            });
        }
        // Editor-side changes (history recall, Enter, editor keys) move the widget's cursor with them.
        let mut moved = !edits.is_empty();
        if sel_up {
            self.slash_sel = sel.saturating_sub(1);
        }
        if sel_down {
            self.slash_sel = (sel + 1).min(sugg.len().saturating_sub(1));
        }
        if tab && let Some(done) = slash::complete(&text, sel, busy, perm_open) {
            self.input.apply_widget(&done, done.chars().count());
            self.slash_sel = 0;
            moved = true;
        }
        for op in edits {
            self.input.apply_key(op);
        }
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
                self.submit(ui.ctx());
            }
            moved = true;
        }
        if moved {
            self.place_cursor(ui);
        }
        for c in sugg.iter().take(MAX_SLASH_ROWS) {
            let row = egui::RichText::new(format!("{:<10} {}", c.name, c.desc));
            ui.label(if std::ptr::eq(*c, sugg[sel]) {
                row.strong()
            } else {
                row.weak()
            });
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
        if let Some(to) = self.scroll_to.take() {
            area = area.vertical_scroll_offset(to);
        }
        let out = area.show(ui, |ui| {
            welcome(ui, &self.state);
            for entry in &self.state.transcript.entries {
                render_entry(ui, &mut self.md, entry);
            }
        });
        let viewport = out.inner_rect.height();
        let max = (out.content_size.y - viewport).max(0.0);
        self.view = ScrollView {
            offset: out.state.clamped_offset().y,
            max,
            viewport,
        };
        self.follow = max - out.state.clamped_offset().y <= 1.0;
        if self.follow {
            self.unseen = false;
        }
    }
}

impl eframe::App for ChaudeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain(ui.ctx());
        self.window_keys(ui.ctx());
        self.shortcuts(ui.ctx());
        // Bottom panels stack in show order (later is higher). Bottom to top this is the TUI chrome order reversed:
        // status, background agents, foreground agents, coder line, input, tasks, queue.
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        if !self.agent_rows(true).is_empty() {
            egui::Panel::bottom("agents_bg").show(ui, |ui| self.agent_panel(ui, true));
        }
        if !self.agent_rows(false).is_empty() {
            egui::Panel::bottom("agents_fg").show(ui, |ui| self.agent_panel(ui, false));
        }
        // The coder line sits under the input box, above the status rows (TUI chrome order).
        if let Some(line) = bar_text::coder_text(&self.state.bar.coders) {
            egui::Panel::bottom("coders").show(ui, |ui| ui.weak(line));
        }
        egui::Panel::bottom("input").show(ui, |ui| self.input_box(ui));
        if !self.state.run.tasks.is_empty() {
            egui::Panel::bottom("tasks").show(ui, |ui| self.task_panel(ui));
        }
        if !self.state.run.queue_texts.is_empty() {
            egui::Panel::bottom("queue").show(ui, |ui| self.queue_panel(ui));
        }
        egui::CentralPanel::default().show(ui, |ui| self.output(ui));
        self.permission_dialog(ui.ctx());
        self.usage_window(ui.ctx());
    }
}

// The welcome box at the top of the transcript: the shared core text rows (TUI welcome_rows), title first.
fn welcome(ui: &mut egui::Ui, state: &GuiState) {
    let model = if state.bar.model.is_empty() {
        WELCOME_MODEL
    } else {
        &state.bar.model
    };
    let rows = welcome_lines(&state.cwd, 80, model, WELCOME_THRESHOLD);
    ui.group(|ui| {
        for (i, row) in rows.iter().enumerate() {
            if i == 0 {
                ui.strong(row);
            } else {
                ui.weak(row);
            }
        }
    });
    ui.add_space(ui.spacing().item_spacing.y);
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
            name,
            summary,
            start,
            end,
            ..
        } => {
            ui.weak(tool_line(*start, name, summary, *end));
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
        Entry::ToolResult { text, at, .. } => {
            ui.weak(tool_result_line(*at, text));
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
