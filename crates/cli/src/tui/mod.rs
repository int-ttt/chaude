// Full-screen terminal UI on top of chaude_core::api, laid out like the Go TUI. The alternate screen holds,
// top to bottom: the transcript viewport, the running row, the input line and the model/status row.
// `frame` is pure and snapshot-tested at fixed sizes; `draw` rewrites only the rows that changed.

pub mod bar;
pub mod commands;
pub mod edit;
pub mod render;

use std::io::{self, BufRead, IsTerminal, Write};
use std::panic;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use chaude_core::api::{Config, Engine, InterruptSource, Role, UiEvent};
use crossterm::cursor::{MoveTo, Show};
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use unicode_width::UnicodeWidthStr;

use bar::BarInput;
use commands::{Ctx, Dispatch, PermMode};
use edit::Editor;

const TICK: Duration = Duration::from_millis(80);
const DOUBLE_ESC: Duration = Duration::from_secs(1);
const PROMPT: &str = "> ";
// Rows below the viewport: running row, input line, status row.
const CHROME: usize = 3;
const WHEEL_LINES: isize = 3;
// Shown on the running row when output arrived while the view was held above the tail (scroll-bottom.md).
const NEW_MSG: &str = "↓ 새 메시지";
// Prefix for agent text lines.
const AGENT_PREFIX: &str = "  │ ";

// What the event loop must do after a key press.
#[derive(Debug, PartialEq)]
pub enum Action {
    Continue,
    Submit(String),
    Interrupt(InterruptSource),
    Quit,
}

pub struct App {
    editor: Editor,
    // Terminal size as last seen by the event loop. Key handling needs it to page and clamp the viewport.
    cols: usize,
    rows: usize,
    bar: BarInput,
    busy: bool,
    turn_start: Option<Instant>,
    agents: Vec<(String, String, Instant)>,
    // Time of the last tick. Elapsed times and the spinner are computed from it, so `frame` stays pure.
    clock: Instant,
    spin: char,
    transcript: Vec<String>,
    // Unfinished text (no newline yet) per role. Agent text shares one buffer; it is flushed at its MessageDone.
    pending_main: String,
    pending_agent: String,
    // First visible row while held. While `follow` is set the view sticks to the tail instead.
    top: usize,
    follow: bool,
    unseen: bool,
    // Mouse wheel scrolling (/mouse). When off, the terminal keeps the mouse for text selection.
    mouse: bool,
    last_esc: Option<Instant>,
}

impl App {
    pub fn new(cols: usize, rows: usize) -> Self {
        App {
            editor: Editor::default(),
            cols,
            rows,
            bar: BarInput::default(),
            busy: false,
            turn_start: None,
            agents: Vec::new(),
            clock: Instant::now(),
            spin: ' ',
            transcript: Vec::new(),
            pending_main: String::new(),
            pending_agent: String::new(),
            top: 0,
            follow: true,
            unseen: false,
            mouse: true,
            last_esc: None,
        }
    }

    // Folds one engine event into the transcript and status state.
    pub fn apply(&mut self, ev: UiEvent) {
        let content = !matches!(
            ev,
            UiEvent::Status { .. } | UiEvent::Busy(_) | UiEvent::Queued(_)
        );
        match ev {
            UiEvent::TextDelta {
                role: Role::Main,
                text,
            } => feed(&mut self.pending_main, &text, "", &mut self.transcript),
            UiEvent::TextDelta {
                role: Role::Agent(_),
                text,
            } => feed(
                &mut self.pending_agent,
                &text,
                AGENT_PREFIX,
                &mut self.transcript,
            ),
            UiEvent::MessageDone { role: Role::Main } => {
                flush(&mut self.pending_main, "", &mut self.transcript)
            }
            UiEvent::MessageDone {
                role: Role::Agent(_),
            } => flush(&mut self.pending_agent, AGENT_PREFIX, &mut self.transcript),
            UiEvent::ToolStart { name, summary, .. } => {
                self.transcript.push(format!("● {name} {summary}"))
            }
            UiEvent::ToolEnd { ok, .. } => self.transcript.push(if ok {
                "  ↳ ok".into()
            } else {
                "  ↳ failed".into()
            }),
            UiEvent::AgentStart {
                id, description, ..
            } => {
                self.transcript
                    .push(format!("◆ agent started: {description}"));
                self.agents.push((id, description, Instant::now()));
            }
            UiEvent::AgentEnd { id, elapsed_ms } => {
                self.agents.retain(|(aid, _, _)| *aid != id);
                self.transcript.push(format!(
                    "◆ agent done in {}",
                    bar::fmt_secs(elapsed_ms / 1000)
                ));
            }
            UiEvent::Status {
                model,
                tokens,
                cost_usd,
                context_pct,
            } => {
                self.bar.has_status = true;
                self.bar.model = model;
                self.bar.tokens = tokens;
                self.bar.cost_usd = cost_usd;
                self.bar.context_pct = context_pct;
            }
            UiEvent::Busy(b) => {
                self.busy = b;
                self.turn_start = if b { Some(Instant::now()) } else { None };
            }
            UiEvent::Queued(n) => self.bar.queued = n,
            UiEvent::TurnEnd { duration_ms } => {
                flush(&mut self.pending_main, "", &mut self.transcript);
                self.transcript
                    .push(format!("── done in {}", bar::fmt_secs(duration_ms / 1000)));
            }
            UiEvent::Rotated { reason } => {
                self.transcript.push(format!("↻ session rotated: {reason}"))
            }
            UiEvent::Error(msg) => self.transcript.push(format!("! {msg}")),
        }
        if content && !self.follow {
            self.unseen = true;
        }
    }

    // Records the time for elapsed counters and the spinner (Go spinner: one frame per 125 ms).
    pub fn tick(&mut self, now: Instant) {
        self.clock = now;
        self.spin = match self.turn_start {
            Some(t) if self.busy => bar::spinner_frame(now.duration_since(t)),
            _ => ' ',
        };
    }

    // Maps one key press to an action.
    pub fn on_key(&mut self, key: KeyEvent, now: Instant) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let page = self.rows.saturating_sub(CHROME).max(1) as isize;
        match key.code {
            KeyCode::Char('c') if ctrl => {
                if self.busy {
                    Action::Interrupt(InterruptSource::CtrlC)
                } else if self.editor.is_empty() {
                    Action::Quit
                } else {
                    self.editor.clear();
                    Action::Continue
                }
            }
            KeyCode::Char('j') if ctrl => {
                self.editor.newline();
                Action::Continue
            }
            KeyCode::Char('a') if ctrl => {
                self.editor.home();
                Action::Continue
            }
            KeyCode::Char('e') if ctrl => {
                self.editor.end();
                Action::Continue
            }
            KeyCode::Char('w') if ctrl => {
                self.editor.delete_word();
                Action::Continue
            }
            KeyCode::Char('u') if ctrl => {
                self.editor.kill_to_start();
                Action::Continue
            }
            KeyCode::Char('k') if ctrl => {
                self.editor.kill_to_end();
                Action::Continue
            }
            KeyCode::Char('b') if ctrl => {
                self.scroll(-page);
                Action::Continue
            }
            KeyCode::Char('f') if ctrl => {
                self.scroll(page);
                Action::Continue
            }
            KeyCode::Char(c) if !ctrl => {
                self.editor.insert_char(c);
                Action::Continue
            }
            KeyCode::Enter if alt => {
                self.editor.newline();
                Action::Continue
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Esc => {
                let double = self
                    .last_esc
                    .is_some_and(|t| now.duration_since(t) < DOUBLE_ESC);
                self.last_esc = Some(now);
                if self.busy && double {
                    self.last_esc = None;
                    Action::Interrupt(InterruptSource::EscKey)
                } else {
                    Action::Continue
                }
            }
            KeyCode::Backspace => {
                self.editor.backspace();
                Action::Continue
            }
            KeyCode::Delete => {
                self.editor.delete();
                Action::Continue
            }
            KeyCode::Left => {
                self.editor.left();
                Action::Continue
            }
            KeyCode::Right => {
                self.editor.right();
                Action::Continue
            }
            KeyCode::Home if self.editor.is_empty() => {
                self.scroll_to(0);
                Action::Continue
            }
            KeyCode::Home => {
                self.editor.home();
                Action::Continue
            }
            KeyCode::End if self.editor.is_empty() => {
                self.follow_tail();
                Action::Continue
            }
            KeyCode::End => {
                self.editor.end();
                Action::Continue
            }
            KeyCode::PageUp => {
                self.scroll(-page);
                Action::Continue
            }
            KeyCode::PageDown => {
                self.scroll(page);
                Action::Continue
            }
            KeyCode::Up if shift => {
                self.scroll(-1);
                Action::Continue
            }
            KeyCode::Down if shift => {
                self.scroll(1);
                Action::Continue
            }
            KeyCode::Up => {
                if !self.editor.move_row(-1) {
                    self.editor.history_prev();
                }
                Action::Continue
            }
            KeyCode::Down => {
                if !self.editor.move_row(1) {
                    self.editor.history_next();
                }
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    // Enter: a trailing backslash becomes a newline; otherwise the text is echoed and sent (or handled locally).
    fn submit(&mut self) -> Action {
        if self.editor.backslash_newline() {
            return Action::Continue;
        }
        let text = self.editor.take();
        if text.trim().is_empty() {
            return Action::Continue;
        }
        for (i, line) in text.split('\n').enumerate() {
            self.transcript
                .push(format!("{}{line}", if i == 0 { "› " } else { "  " }));
        }
        self.follow_tail();
        let first = text.split_whitespace().next().unwrap_or("");
        let dispatched = {
            let ctx = Ctx {
                model: &self.bar.model,
                last_ctx: 0,
                threshold: 0,
                rotations: 0,
                session_id: None,
                mode: PermMode::Default,
                chaude_version: env!("CARGO_PKG_VERSION"),
                claude_version: "",
                busy: self.busy,
            };
            commands::dispatch(&text, &ctx)
        };
        // /status, /version and /mode need engine state that is not in UiEvent yet, so they go to the engine.
        match dispatched {
            Some(Dispatch::Quit) => return Action::Quit,
            Some(Dispatch::ToggleMouse) => {
                self.mouse = !self.mouse;
                self.note(commands::mouse_note(self.mouse));
                return Action::Continue;
            }
            Some(Dispatch::Note(s)) if !matches!(first, "/status" | "/version" | "/mode") => {
                self.note(&s);
                return Action::Continue;
            }
            _ => {}
        }
        Action::Submit(text)
    }

    fn note(&mut self, text: &str) {
        self.transcript.extend(text.split('\n').map(str::to_string));
    }

    // Lines in the viewport: transcript, then the unfinished text of each role, wrapped to `width`.
    fn body(&self, width: usize) -> Vec<String> {
        let agent = format!("{AGENT_PREFIX}{}", self.pending_agent);
        let mut lines: Vec<&str> = self.transcript.iter().map(String::as_str).collect();
        if !self.pending_main.is_empty() {
            lines.push(&self.pending_main);
        }
        if !self.pending_agent.is_empty() {
            lines.push(&agent);
        }
        let mut rows = Vec::new();
        for l in lines {
            rows.extend(render::wrap_cells(l, width).split('\n').map(str::to_string));
        }
        rows
    }

    fn total_rows(&self, w: usize) -> usize {
        self.body(text_width(w)).len()
    }

    // First visible row for a body of `total` rows shown in `body_h` rows.
    fn start_row(&self, total: usize, body_h: usize) -> usize {
        let max = total.saturating_sub(body_h);
        if self.follow { max } else { self.top.min(max) }
    }

    // Moves the viewport by `delta` rows. Landing on the last page follows the tail again.
    fn scroll(&mut self, delta: isize) {
        let total = self.total_rows(self.cols);
        let start = self.start_row(total, self.rows.saturating_sub(CHROME));
        self.scroll_to((start as isize + delta).max(0) as usize);
    }

    fn scroll_to(&mut self, row: usize) {
        let max = self
            .total_rows(self.cols)
            .saturating_sub(self.rows.saturating_sub(CHROME));
        self.top = row.min(max);
        self.follow = self.top >= max;
        if self.follow {
            self.unseen = false;
        }
    }

    fn follow_tail(&mut self) {
        self.follow = true;
        self.unseen = false;
    }

    // The bar's view of the current state. Elapsed times are taken from the last tick.
    fn bar_input(&self) -> BarInput {
        let mut b = self.bar.clone();
        b.turn_secs = self
            .turn_start
            .map(|t| self.clock.duration_since(t).as_secs());
        b.agents = self
            .agents
            .iter()
            .map(|(_, d, t)| (d.clone(), self.clock.duration_since(*t).as_secs()))
            .collect();
        b
    }

    // The input row text and the cursor column inside it.
    fn input_line(&self, width: usize) -> (String, usize) {
        let pw = UnicodeWidthStr::width(PROMPT);
        let (view, cur) = self.editor.view(width.saturating_sub(pw));
        (format!("{PROMPT}{view}"), pw + cur)
    }
}

// Columns usable for text. The last column stays empty so a full row never auto-wraps.
fn text_width(w: usize) -> usize {
    w.saturating_sub(1).max(1)
}

// The whole screen as `h` rows of at most `w` columns. Pure: the same App and size give the same rows.
pub fn frame(app: &App, w: usize, h: usize) -> Vec<String> {
    let width = text_width(w);
    let bi = app.bar_input();
    let [mut top, status] = bar::rows(&bi, app.spin, width);
    if app.unseen && !app.follow {
        top = format!("{top} · {NEW_MSG}");
    }
    let (input, _) = app.input_line(width);
    let chrome = [bar::clip(&top, width), input, bar::clip(&status, width)];
    let body_h = h.saturating_sub(CHROME);
    let mut rows = Vec::with_capacity(h);
    if body_h > 0 {
        let body = app.body(width);
        let start = app.start_row(body.len(), body_h);
        for i in 0..body_h {
            rows.push(body.get(start + i).cloned().unwrap_or_default());
        }
    }
    // Too short for the whole chrome: keep the bottom rows, input line included.
    rows.extend(chrome.into_iter().skip(CHROME - h.min(CHROME)));
    rows
}

// Terminal cell that holds the cursor: the input line, at the editing column.
fn cursor(app: &App, w: usize, h: usize) -> (u16, u16) {
    let (_, col) = app.input_line(text_width(w));
    (col as u16, h.saturating_sub(2) as u16)
}

// Writes only the rows that differ from `prev` (the rows last drawn), then parks the cursor on the input line.
pub fn draw(out: &mut impl Write, app: &App, prev: &mut Vec<String>) -> io::Result<()> {
    let rows = frame(app, app.cols, app.rows);
    for (i, row) in rows.iter().enumerate() {
        if prev.get(i) != Some(row) {
            queue!(out, MoveTo(0, i as u16), Clear(ClearType::CurrentLine))?;
            write!(out, "{row}")?;
        }
    }
    let (x, y) = cursor(app, app.cols, app.rows);
    queue!(out, MoveTo(x, y), Show)?;
    *prev = rows;
    out.flush()
}

// Appends text to `buf`, emitting each completed line (with `prefix`) to `out`.
fn feed(buf: &mut String, text: &str, prefix: &str, out: &mut Vec<String>) {
    buf.push_str(text);
    while let Some(i) = buf.find('\n') {
        let line: String = buf.drain(..=i).collect();
        out.push(format!("{prefix}{}", line.trim_end_matches('\n')));
    }
}

// Emits whatever is left in `buf` as a final line.
fn flush(buf: &mut String, prefix: &str, out: &mut Vec<String>) {
    if !buf.is_empty() {
        out.push(format!("{prefix}{}", std::mem::take(buf)));
    }
}

fn restore_terminal() {
    let _ = execute!(
        io::stdout(),
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen,
        Show
    );
    let _ = terminal::disable_raw_mode();
}

// Enters raw mode and the alternate screen. Dropping it (exit, error, or unwind) restores the terminal.
struct TermGuard;

impl TermGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            Clear(ClearType::All),
            EnableBracketedPaste,
            EnableMouseCapture
        )?;
        // The default hook prints while the alternate screen is still up, so restore before it runs.
        let hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            restore_terminal();
            hook(info)
        }));
        Ok(TermGuard)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

// Entry point for `chaude` with no arguments. Non-TTY stdin falls back to plain line mode.
pub fn run(cfg: Config) -> io::Result<()> {
    if io::stdin().is_terminal() {
        tui(cfg)
    } else {
        plain(cfg)
    }
}

fn tui(cfg: Config) -> io::Result<()> {
    let (engine, rx) = Engine::start(cfg);
    let _guard = TermGuard::enter()?;
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let mut app = App::new(cols as usize, rows as usize);
    if let Ok(cwd) = std::env::current_dir() {
        app.editor.load_history(&cwd);
    }
    let mut out = io::stdout();
    let mut prev = Vec::new();
    // The guard turned mouse capture on; this tracks whether it is still on.
    let mut captured = true;
    draw(&mut out, &app, &mut prev)?;
    'main: loop {
        loop {
            match rx.try_recv() {
                Ok(ev) => app.apply(ev),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break 'main,
            }
        }
        if event::poll(TICK)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match app.on_key(key, Instant::now()) {
                        Action::Continue => {}
                        Action::Submit(text) => engine.submit(&text),
                        Action::Interrupt(src) => engine.interrupt(src),
                        Action::Quit => break 'main,
                    }
                }
                Event::Paste(text) => app.editor.insert_str(&text),
                Event::Mouse(m) if app.mouse => match m.kind {
                    MouseEventKind::ScrollUp => app.scroll(-WHEEL_LINES),
                    MouseEventKind::ScrollDown => app.scroll(WHEEL_LINES),
                    _ => {}
                },
                Event::Resize(c, r) => {
                    app.cols = c as usize;
                    app.rows = r as usize;
                    queue!(out, Clear(ClearType::All))?;
                    prev.clear();
                }
                _ => {}
            }
        }
        app.tick(Instant::now());
        if app.mouse != captured {
            if app.mouse {
                execute!(out, EnableMouseCapture)?;
            } else {
                execute!(out, DisableMouseCapture)?;
            }
            captured = app.mouse;
        }
        draw(&mut out, &app, &mut prev)?;
    }
    engine.shutdown();
    Ok(())
}

// One prompt per stdin line, answered before the next line is read. Engine events are printed as plain text.
fn plain(cfg: Config) -> io::Result<()> {
    let (engine, rx) = Engine::start(cfg);
    let mut app = App::new(80, 24);
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        engine.submit(&line);
        while let Ok(ev) = rx.recv() {
            let finished = ev == UiEvent::Busy(false);
            let before = app.transcript.len();
            app.apply(ev);
            for l in &app.transcript[before..] {
                println!("{l}");
            }
            if finished {
                break;
            }
        }
    }
    engine.shutdown();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn text(s: &str) -> UiEvent {
        UiEvent::TextDelta {
            role: Role::Main,
            text: s.into(),
        }
    }

    // An app at 30x5 (two viewport rows) with lines "line 0" .. "line {n-1}".
    fn app_with_lines(n: usize) -> App {
        let mut app = App::new(30, 5);
        for i in 0..n {
            app.apply(text(&format!("line {i}\n")));
        }
        app
    }

    fn chrome(top: &str) -> Vec<String> {
        vec![top.into(), "> ".into(), "no status yet".into()]
    }

    #[test]
    fn empty_screen_shows_idle_and_the_input_line() {
        let app = App::new(30, 5);
        let mut want = vec!["".to_string(), "".to_string()];
        want.extend(chrome("idle"));
        assert_eq!(frame(&app, 30, 5), want);
    }

    #[test]
    fn long_output_shows_the_tail_until_scrolled() {
        let mut app = app_with_lines(10);
        let mut want = vec!["line 8".to_string(), "line 9".to_string()];
        want.extend(chrome("idle"));
        assert_eq!(frame(&app, 30, 5), want);

        app.scroll(-3);
        let mut want = vec!["line 5".to_string(), "line 6".to_string()];
        want.extend(chrome("idle"));
        assert_eq!(frame(&app, 30, 5), want);
    }

    #[test]
    fn new_output_while_held_keeps_position_and_shows_the_indicator() {
        let mut app = app_with_lines(10);
        app.scroll(-3);
        app.apply(text("line 10\n"));
        let mut want = vec!["line 5".to_string(), "line 6".to_string()];
        want.extend(chrome("idle · ↓ 새 메시지"));
        assert_eq!(frame(&app, 30, 5), want);
    }

    #[test]
    fn typing_while_scrolled_keeps_position() {
        let mut app = app_with_lines(10);
        app.scroll(-3);
        let now = Instant::now();
        app.on_key(key(KeyCode::Char('a'), KeyModifiers::NONE), now);
        app.on_key(key(KeyCode::Char('b'), KeyModifiers::NONE), now);
        let rows = frame(&app, 30, 5);
        assert_eq!(rows[0], "line 5");
        assert_eq!(rows[1], "line 6");
        assert_eq!(rows[3], "> ab");
    }

    #[test]
    fn new_output_at_the_bottom_follows_the_tail() {
        let mut app = app_with_lines(10);
        app.apply(text("line 10\n"));
        let mut want = vec!["line 9".to_string(), "line 10".to_string()];
        want.extend(chrome("idle"));
        assert_eq!(frame(&app, 30, 5), want);
    }

    #[test]
    fn end_key_returns_to_the_tail_and_clears_the_indicator() {
        let mut app = app_with_lines(10);
        app.scroll(-3);
        app.apply(text("line 10\n"));
        app.on_key(key(KeyCode::End, KeyModifiers::NONE), Instant::now());
        let rows = frame(&app, 30, 5);
        assert_eq!(rows[0], "line 9");
        assert_eq!(rows[1], "line 10");
        assert_eq!(rows[2], "idle");
    }

    #[test]
    fn resize_clamps_the_view_and_rewraps_wide_text() {
        let mut app = app_with_lines(10);
        app.on_key(key(KeyCode::Home, KeyModifiers::NONE), Instant::now());
        // Narrower and shorter: one viewport row, so the tail row is shown and the status row is clipped.
        app.cols = 12;
        app.rows = 4;
        let rows = frame(&app, 12, 4);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0], "line 0");
        assert_eq!(rows[3], "no status y");
        for r in &rows {
            assert!(UnicodeWidthStr::width(r.as_str()) <= 12, "{r:?}");
        }

        let mut app = App::new(30, 5);
        app.apply(text("가나다라마바사\n"));
        let rows = frame(&app, 7, 5);
        assert_eq!(rows[0], "라마바");
        assert_eq!(rows[1], "사");
    }

    #[test]
    fn enter_submits_and_echoes_while_blank_enter_does_nothing() {
        let mut app = App::new(80, 24);
        let now = Instant::now();
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE), now),
            Action::Continue
        );
        for c in "hi".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE), now);
        }
        app.on_key(key(KeyCode::Char('j'), KeyModifiers::CONTROL), now);
        app.on_key(key(KeyCode::Char('x'), KeyModifiers::NONE), now);
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE), now),
            Action::Submit("hi\nx".into())
        );
        assert_eq!(app.transcript, vec!["› hi".to_string(), "  x".to_string()]);
    }

    #[test]
    fn quit_and_mouse_are_handled_locally() {
        let mut app = App::new(80, 24);
        let now = Instant::now();
        for c in "/mouse".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE), now);
        }
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE), now),
            Action::Continue
        );
        assert!(!app.mouse);
        for c in "/quit".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE), now);
        }
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE), now),
            Action::Quit
        );
    }

    #[test]
    fn two_escapes_within_a_second_interrupt_only_while_busy() {
        let mut app = App::new(80, 24);
        let t0 = Instant::now();
        let esc = || key(KeyCode::Esc, KeyModifiers::NONE);
        app.apply(UiEvent::Busy(true));
        assert_eq!(app.on_key(esc(), t0), Action::Continue);
        assert_eq!(
            app.on_key(esc(), t0 + Duration::from_millis(400)),
            Action::Interrupt(InterruptSource::EscKey)
        );
        assert_eq!(
            app.on_key(esc(), t0 + Duration::from_secs(5)),
            Action::Continue
        );
        app.apply(UiEvent::Busy(false));
        assert_eq!(
            app.on_key(esc(), t0 + Duration::from_secs(5)),
            Action::Continue
        );
        assert_eq!(
            app.on_key(esc(), t0 + Duration::from_secs(5)),
            Action::Continue
        );
    }

    #[test]
    fn ctrl_c_interrupts_when_busy_clears_when_idle_and_quits_when_idle_and_empty() {
        let mut app = App::new(80, 24);
        let now = Instant::now();
        let ctrl_c = || key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        app.on_key(key(KeyCode::Char('a'), KeyModifiers::NONE), now);
        assert_eq!(app.on_key(ctrl_c(), now), Action::Continue);
        assert!(app.editor.is_empty());
        assert_eq!(app.on_key(ctrl_c(), now), Action::Quit);
        app.apply(UiEvent::Busy(true));
        assert_eq!(
            app.on_key(ctrl_c(), now),
            Action::Interrupt(InterruptSource::CtrlC)
        );
    }

    #[test]
    fn draw_writes_only_changed_rows() {
        let app = App::new(20, 5);
        let mut prev = Vec::new();
        let mut buf: Vec<u8> = Vec::new();
        draw(&mut buf, &app, &mut prev).unwrap();
        let text = String::from_utf8_lossy(&buf);
        assert!(text.contains("idle"), "{text:?}");
        assert!(text.contains("no status yet"), "{text:?}");
        assert_eq!(prev, frame(&app, 20, 5));

        let mut again: Vec<u8> = Vec::new();
        draw(&mut again, &app, &mut prev).unwrap();
        assert!(!String::from_utf8_lossy(&again).contains("idle"));
    }
}
