// Inline terminal UI on top of chaude_core::api. Finished lines go to normal scrollback (no alternate
// screen). A 3-row area at the bottom (two status rows and the input row) is redrawn in place. Its cursor
// always rests on the input row, so scrolling back up two rows reaches the top of the area.

pub mod bar;
pub mod commands;
pub mod edit;
pub mod render;

use std::io::{self, BufRead, IsTerminal, Write};
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use chaude_core::api::{Config, Engine, InterruptSource, Role, UiEvent};
use crossterm::cursor::{MoveToColumn, MoveUp};
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers,
};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{execute, queue};
use unicode_width::UnicodeWidthStr;

use bar::BarInput;
use edit::Editor;

const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const TICK: Duration = Duration::from_millis(80);
const DOUBLE_ESC: Duration = Duration::from_secs(1);
const PROMPT: &str = "> ";
// Status rows above the input row. The cursor rests on the input row, so the top of the area is two rows up.
const ABOVE_INPUT: u16 = 2;

// What the event loop must do after a key press.
#[derive(Debug, PartialEq)]
pub enum Action {
    Continue,
    Submit(String),
    Interrupt(InterruptSource),
    Quit,
}

pub struct App {
    pub editor: Editor,
    pub cols: usize,
    bar: BarInput,
    busy: bool,
    turn_start: Option<Instant>,
    agents: Vec<(String, String, Instant)>,
    // Unfinished text (no newline yet) per role. Agent text shares one buffer; it is flushed at its MessageDone.
    pending_main: String,
    pending_agent: String,
    last_esc: Option<Instant>,
}

impl App {
    pub fn new(cols: usize) -> Self {
        App {
            editor: Editor::default(),
            cols,
            bar: BarInput::default(),
            busy: false,
            turn_start: None,
            agents: Vec::new(),
            pending_main: String::new(),
            pending_agent: String::new(),
            last_esc: None,
        }
    }

    // Folds one engine event into the view state. Lines that belong in scrollback are appended to `out`.
    pub fn apply(&mut self, ev: UiEvent, out: &mut Vec<String>) {
        match ev {
            UiEvent::TextDelta {
                role: Role::Main,
                text,
            } => feed(&mut self.pending_main, &text, "", out),
            UiEvent::TextDelta {
                role: Role::Agent(_),
                text,
            } => feed(&mut self.pending_agent, &text, "  │ ", out),
            UiEvent::MessageDone { role: Role::Main } => flush(&mut self.pending_main, "", out),
            UiEvent::MessageDone {
                role: Role::Agent(_),
            } => flush(&mut self.pending_agent, "  │ ", out),
            UiEvent::ToolStart { name, summary, .. } => out.push(format!("● {name} {summary}")),
            UiEvent::ToolEnd { ok, .. } => out.push(if ok {
                "  ↳ ok".into()
            } else {
                "  ↳ failed".into()
            }),
            UiEvent::AgentStart {
                id, description, ..
            } => {
                out.push(format!("◆ agent started: {description}"));
                self.agents.push((id, description, Instant::now()));
            }
            UiEvent::AgentEnd { id, elapsed_ms } => {
                self.agents.retain(|(aid, _, _)| *aid != id);
                out.push(format!(
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
                flush(&mut self.pending_main, "", out);
                out.push(format!("── done in {}", bar::fmt_secs(duration_ms / 1000)));
            }
            UiEvent::Rotated { reason } => out.push(format!("↻ session rotated: {reason}")),
            UiEvent::Error(msg) => out.push(format!("! {msg}")),
        }
    }

    // Maps one key press to an action. Lines to scroll (the echoed prompt) go to `out`.
    pub fn on_key(&mut self, key: KeyEvent, now: Instant, out: &mut Vec<String>) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
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
            KeyCode::Char(c) if !ctrl => {
                self.editor.insert_char(c);
                Action::Continue
            }
            KeyCode::Enter => {
                let text = self.editor.take();
                if text.trim().is_empty() {
                    return Action::Continue;
                }
                for (i, line) in text.split('\n').enumerate() {
                    out.push(format!("{}{line}", if i == 0 { "› " } else { "  " }));
                }
                Action::Submit(text)
            }
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
            KeyCode::Home => {
                self.editor.home();
                Action::Continue
            }
            KeyCode::End => {
                self.editor.end();
                Action::Continue
            }
            KeyCode::Up => {
                self.editor.history_prev();
                Action::Continue
            }
            KeyCode::Down => {
                self.editor.history_next();
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    // The bar's view of the current state. Elapsed times are computed at draw time.
    fn bar_input(&self, now: Instant) -> BarInput {
        let mut b = self.bar.clone();
        b.turn_secs = self.turn_start.map(|t| now.duration_since(t).as_secs());
        b.agents = self
            .agents
            .iter()
            .map(|(_, d, t)| (d.clone(), now.duration_since(*t).as_secs()))
            .collect();
        b
    }

    // Redraws the 3-row area. Expects the cursor on the top row of the area (or at the first free row).
    // Leaves the cursor on the input row, at the editing position.
    pub fn draw(&self, out: &mut impl Write, tick: usize, now: Instant) -> io::Result<()> {
        let width = self.cols.saturating_sub(1).max(1);
        let spin = if self.busy {
            SPINNER[tick % SPINNER.len()]
        } else {
            ' '
        };
        let [top, status] = bar::rows(&self.bar_input(now), spin, width);
        let prompt_w = UnicodeWidthStr::width(PROMPT);
        let (view, cur) = self.editor.view(width.saturating_sub(prompt_w));
        queue!(out, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
        write!(out, "{top}\r\n")?;
        queue!(out, Clear(ClearType::CurrentLine))?;
        write!(out, "{status}\r\n")?;
        queue!(out, Clear(ClearType::CurrentLine))?;
        write!(out, "{PROMPT}{view}")?;
        queue!(out, MoveToColumn((prompt_w + cur) as u16))?;
        out.flush()
    }

    // Moves to the top of the area, writes finished lines into scrollback, and redraws the area below them.
    pub fn scroll(&self, out: &mut impl Write, lines: &[String], tick: usize) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }
        queue!(
            out,
            MoveUp(ABOVE_INPUT),
            MoveToColumn(0),
            Clear(ClearType::FromCursorDown)
        )?;
        for line in lines {
            write!(out, "{line}\r\n")?;
        }
        self.draw(out, tick, Instant::now())
    }
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

// Restores the terminal on drop, including after an error or panic unwind.
struct RawGuard;

impl RawGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnableBracketedPaste)?;
        Ok(RawGuard)
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableBracketedPaste);
        let _ = terminal::disable_raw_mode();
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
    let _guard = RawGuard::enter()?;
    let mut out = io::stdout();
    let (cols, _) = terminal::size().unwrap_or((80, 24));
    let mut app = App::new(cols as usize);
    let mut tick = 0usize;
    app.draw(&mut out, tick, Instant::now())?;
    'main: loop {
        let mut lines = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(ev) => app.apply(ev, &mut lines),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break 'main,
            }
        }
        if event::poll(TICK)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match app.on_key(key, Instant::now(), &mut lines) {
                        Action::Continue => {}
                        Action::Submit(text) => engine.submit(&text),
                        Action::Interrupt(src) => engine.interrupt(src),
                        Action::Quit => break 'main,
                    }
                }
                Event::Paste(text) => app.editor.insert_str(&text),
                Event::Resize(c, _) => app.cols = c as usize,
                _ => {}
            }
        }
        tick = tick.wrapping_add(1);
        app.scroll(&mut out, &lines, tick)?;
    }
    // Clear the area so the shell prompt starts on a clean line.
    queue!(
        out,
        MoveUp(ABOVE_INPUT),
        MoveToColumn(0),
        Clear(ClearType::FromCursorDown)
    )?;
    out.flush()?;
    engine.shutdown();
    Ok(())
}

// One prompt per stdin line, answered before the next line is read. Engine events are printed as plain text.
fn plain(cfg: Config) -> io::Result<()> {
    let (engine, rx) = Engine::start(cfg);
    let mut app = App::new(80);
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        engine.submit(&line);
        while let Ok(ev) = rx.recv() {
            let finished = ev == UiEvent::Busy(false);
            let mut lines = Vec::new();
            app.apply(ev, &mut lines);
            for l in lines {
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

    #[test]
    fn text_is_emitted_per_completed_line_and_flushed_at_message_end() {
        let mut app = App::new(80);
        let mut out = Vec::new();
        app.apply(
            UiEvent::TextDelta {
                role: Role::Main,
                text: "가나\n다".into(),
            },
            &mut out,
        );
        assert_eq!(out, vec!["가나".to_string()]);
        app.apply(UiEvent::MessageDone { role: Role::Main }, &mut out);
        assert_eq!(out.last().unwrap(), "다");
    }

    #[test]
    fn enter_submits_and_echoes_while_blank_enter_does_nothing() {
        let mut app = App::new(80);
        let mut out = Vec::new();
        let now = Instant::now();
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE), now, &mut out),
            Action::Continue
        );
        for c in "hi".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE), now, &mut out);
        }
        app.on_key(
            key(KeyCode::Char('j'), KeyModifiers::CONTROL),
            now,
            &mut out,
        );
        app.on_key(key(KeyCode::Char('x'), KeyModifiers::NONE), now, &mut out);
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE), now, &mut out),
            Action::Submit("hi\nx".into())
        );
        assert_eq!(out, vec!["› hi".to_string(), "  x".to_string()]);
    }

    #[test]
    fn two_escapes_within_a_second_interrupt_only_while_busy() {
        let mut app = App::new(80);
        let mut out = Vec::new();
        let t0 = Instant::now();
        let esc = || key(KeyCode::Esc, KeyModifiers::NONE);
        app.apply(UiEvent::Busy(true), &mut out);
        assert_eq!(app.on_key(esc(), t0, &mut out), Action::Continue);
        assert_eq!(
            app.on_key(esc(), t0 + Duration::from_millis(400), &mut out),
            Action::Interrupt(InterruptSource::EscKey)
        );
        assert_eq!(
            app.on_key(esc(), t0 + Duration::from_secs(5), &mut out),
            Action::Continue
        );
        app.apply(UiEvent::Busy(false), &mut out);
        assert_eq!(
            app.on_key(esc(), t0 + Duration::from_secs(5), &mut out),
            Action::Continue
        );
        assert_eq!(
            app.on_key(esc(), t0 + Duration::from_secs(5), &mut out),
            Action::Continue
        );
    }

    #[test]
    fn ctrl_c_interrupts_when_busy_clears_when_idle_and_quits_when_idle_and_empty() {
        let mut app = App::new(80);
        let mut out = Vec::new();
        let now = Instant::now();
        let ctrl_c = || key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        app.on_key(key(KeyCode::Char('a'), KeyModifiers::NONE), now, &mut out);
        assert_eq!(app.on_key(ctrl_c(), now, &mut out), Action::Continue);
        assert!(app.editor.is_empty());
        assert_eq!(app.on_key(ctrl_c(), now, &mut out), Action::Quit);
        app.apply(UiEvent::Busy(true), &mut out);
        assert_eq!(
            app.on_key(ctrl_c(), now, &mut out),
            Action::Interrupt(InterruptSource::CtrlC)
        );
    }

    #[test]
    fn draw_writes_three_rows_and_no_row_is_wider_than_the_terminal() {
        let app = App::new(20);
        let mut buf: Vec<u8> = Vec::new();
        app.draw(&mut buf, 0, Instant::now()).unwrap();
        let text = String::from_utf8_lossy(&buf);
        assert!(text.contains("idle"), "{text:?}");
        assert!(text.contains("no status yet"), "{text:?}");
        assert!(text.contains(PROMPT), "{text:?}");
    }
}
