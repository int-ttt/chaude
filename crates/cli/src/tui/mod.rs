// Full-screen terminal UI on top of chaude_core::api, laid out like the Go TUI. The alternate screen holds,
// top to bottom: the transcript viewport, the running row, the input line and the model/status row.
// `frame` is pure and snapshot-tested at fixed sizes; `draw` rewrites only the rows that changed.

pub mod agentview;
pub mod bar;
pub mod commands;
pub mod edit;
pub mod overlay;
pub mod records;
pub mod render;
pub mod rewind;
mod runewidth_table;
pub mod sessview;
pub mod settings;
pub mod usage;

use std::collections::HashSet;
use std::io::{self, BufRead, IsTerminal, Write};
use std::panic;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use chaude_core::api::{Config, Engine, InterruptSource, Role, UiEvent};
use chaude_core::resident::StagePhase;
use crossterm::cursor::{MoveTo, Show};
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use bar::BarInput;
use commands::{Ctx, Dispatch, PermMode};
use edit::Editor;
use render::{BOLD_OFF, BOLD_ON, FG_OFF, GREY_ON, ORANGE_ON, RESET};

const TICK: Duration = Duration::from_millis(80);
const PROMPT: &str = "> ";
// Rows below the viewport when idle: input box top, input line, input box bottom, status row.
// A running row sits above them while busy (`App::shows_bar`).
const CHROME: usize = 4;
// Go maxInputRows (tui.go:988).
const MAX_INPUT_ROWS: usize = 5;
// Reverse-video cell for the cursor (tui.go View, cursor style).
const REV_ON: &str = "\x1b[7m";
const WHEEL_LINES: isize = 3;
// Go copyNote lifetime (selection.go copyNoteMsg, 2s).
const COPY_NOTE_TTL: Duration = Duration::from_secs(2);
// Shown on the running row when output arrived while the view was held above the tail (scroll-bottom.md).
const NEW_MSG: &str = "↓ 새 메시지";
// Prefix for agent text lines.
const AGENT_PREFIX: &str = "  │ ";
// Go tui.go render(): orange "● " opens an assistant block, green "● " opens a tool block.
const ASST_FIRST: &str = "\x1b[38;2;217;119;87m● \x1b[39m";
const TOOL_ON: &str = "\x1b[38;2;95;175;135m";

// What the event loop must do after a key press.
#[derive(Debug, PartialEq)]
pub enum Action {
    Continue,
    Submit(String),
    Interrupt(InterruptSource),
    Quit,
    // /usage: the caller asks the engine for the one-shot run (UiEvent::Usage). Never a turn.
    Usage,
    // Answer to the open permission prompt (Go tui.go key(): y/Enter allow, n/Esc deny, Ctrl+C deny + interrupt).
    Answer {
        id: String,
        allow: bool,
        interrupt: bool,
    },
    // Shift+Tab: the engine cycles the permission mode (Go setMode(Next())); it answers with ModeChanged.
    CycleMode,
}

pub struct App {
    editor: Editor,
    // Terminal size as last seen by the event loop. Key handling needs it to page and clamp the viewport.
    cols: usize,
    rows: usize,
    bar: BarInput,
    busy: bool,
    turn_start: Option<Instant>,
    // First line of the prompt that started the turn (Go curPrompt), shown in the busy row.
    turn_prompt: String,
    agents: Vec<(String, String, Instant)>,
    // Ids of the agents launched in the background (UiEvent::AgentBackground); they have no AgentEnd until done.
    bg_agents: Vec<String>,
    // Rotations seen this session (UiEvent::Rotated); Go m.r.rotations.
    rot: u32,
    // Time of the last tick. Elapsed times and the spinner are computed from it, so `frame` stays pure.
    clock: Instant,
    spin: char,
    transcript: Vec<String>,
    // True while the main assistant block has emitted its first line (the "● " lead is written once).
    main_open: bool,
    // Unfinished text (no newline yet) per role. Agent text shares one buffer; it is flushed at its MessageDone.
    pending_main: String,
    pending_agent: String,
    // First visible row while held. While `follow` is set the view sticks to the tail instead.
    top: usize,
    follow: bool,
    unseen: bool,
    // Mouse wheel scrolling (/mouse). When off, the terminal keeps the mouse for text selection.
    mouse: bool,
    // Selected row of the slash menu (overlay::slash_popup). Reset when the input changes.
    sugg_sel: usize,
    // Full-screen /usage window (Go m.usage); replaces the transcript viewport while open.
    usage: usage::Usage,
    // Open /settings picker (Go m.settings). Its save note goes to the transcript when it closes.
    settings: Option<settings::Picker>,
    // Open /resume record picker (Go m.rpick) and /view session browser (Go m.sv). Both replace the body.
    recs: Option<records::RecPicker>,
    sess: Option<sessview::SessView>,
    // Open Ctrl+T agent view (Go m.av). Read-only; replaces the body.
    av: Option<agentview::AgentView>,
    // Go m.rw: Esc Esc rewind picker. prompts = (transcript row, text) of each echoed prompt, in order.
    rw: rewind::Rewind,
    prompts: Vec<(usize, String)>,
    // Time of the last Esc on an empty input (Go m.lastEsc); a second Esc within 600 ms opens the picker.
    last_esc: Option<Instant>,
    // Transcript indices of note rows that wrap at width + 1 (Go shows /view errors at the full width).
    wide: HashSet<usize>,
    // Id of the permission request waiting for y/n (UiEvent::PermissionRequest). While set, keys answer it.
    perm: Option<String>,
    // Drag selection in body coordinates (wrapped body row as `line`, cell column as `col`); the Go textSel model.
    sel: render::TextSel,
    dragging: bool,
    // "복사됨 (N자)" or the copy error, shown on the status row for 2s (Go copyNote); the Instant is when it was set.
    copy_note: Option<(String, Instant)>,
    welcome: Welcome,
}

// What the welcome box shows (tui.go welcome()).
pub struct Welcome {
    pub cwd: String,
    pub model: String,
    pub threshold: u64,
}

impl App {
    pub fn new(cols: usize, rows: usize) -> Self {
        App {
            welcome: Welcome {
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                model: "claude-opus-5-5".to_string(),
                threshold: 200_000,
            },
            editor: Editor::default(),
            cols,
            rows,
            bar: BarInput {
                mode: "auto".to_string(),
                ..BarInput::default()
            },
            busy: false,
            turn_start: None,
            turn_prompt: String::new(),
            agents: Vec::new(),
            bg_agents: Vec::new(),
            rot: 0,
            clock: Instant::now(),
            spin: ' ',
            transcript: Vec::new(),
            main_open: false,
            pending_main: String::new(),
            pending_agent: String::new(),
            top: 0,
            follow: true,
            unseen: false,
            mouse: true,
            sugg_sel: 0,
            usage: usage::Usage::default(),
            settings: None,
            recs: None,
            sess: None,
            av: None,
            rw: rewind::Rewind::default(),
            prompts: Vec::new(),
            last_esc: None,
            wide: HashSet::new(),
            perm: None,
            sel: render::TextSel::default(),
            dragging: false,
            copy_note: None,
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
            } => feed(
                &mut self.pending_main,
                &text,
                (ASST_FIRST, "  "),
                &mut self.main_open,
                &mut self.transcript,
                asst_text,
            ),
            UiEvent::TextDelta {
                role: Role::Agent(_),
                text,
            } => feed(
                &mut self.pending_agent,
                &text,
                (AGENT_PREFIX, AGENT_PREFIX),
                &mut false,
                &mut self.transcript,
                plain_text,
            ),
            UiEvent::MessageDone { role: Role::Main } => self.end_main_block(),
            UiEvent::MessageDone {
                role: Role::Agent(_),
            } => flush(
                &mut self.pending_agent,
                (AGENT_PREFIX, AGENT_PREFIX),
                &mut false,
                &mut self.transcript,
                plain_text,
            ),
            UiEvent::ToolStart { name, summary, .. } => {
                self.end_main_block();
                let head = if summary.is_empty() {
                    name
                } else {
                    format!("{name}({summary})")
                };
                self.transcript.push(format!("{TOOL_ON}● {head}{FG_OFF}"));
            }
            // Go shows the tool_result text, not a done/failed word; that row comes from ToolResult.
            UiEvent::ToolEnd { .. } => {}
            // Go toolResultMsg: the first line cut to 160 runes, then "… (+N lines)" for the rest.
            UiEvent::ToolResult { text, .. } => {
                let text = text.trim();
                // Go hides an agent's caveman return from the transcript (tui.go:481).
                if render::is_caveman_block(text) {
                    return;
                }
                let lines: Vec<&str> = text.split('\n').collect();
                let mut head = render::trunc_chars(lines[0], 160);
                if lines.len() > 1 {
                    head.push_str(&format!(" … (+{} lines)", lines.len() - 1));
                }
                self.transcript
                    .push(format!("{GREY_ON}  ⎿  {head}{FG_OFF}"));
                self.transcript.push(String::new());
            }
            UiEvent::AgentBackground { id } => self.bg_agents.push(id),
            UiEvent::AgentStart {
                id, description, ..
            } => {
                self.transcript
                    .push(format!("◆ agent started: {description}"));
                self.agents.push((id, description, Instant::now()));
            }
            // A background agent resumed by SendMessage runs again under its original tool id (core keeps background=true).
            UiEvent::AgentResumed {
                id, description, ..
            } => {
                if !self.agents.iter().any(|(aid, _, _)| *aid == id) {
                    self.agents.push((id.clone(), description, Instant::now()));
                }
                if !self.bg_agents.contains(&id) {
                    self.bg_agents.push(id);
                }
            }
            UiEvent::AgentEnd { id, elapsed_ms } => {
                self.agents.retain(|(aid, _, _)| *aid != id);
                self.bg_agents.retain(|aid| *aid != id);
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
            // Go shows no per-turn "done" row; its footer comes from engine stats that UiEvent does not carry yet.
            UiEvent::TurnEnd { .. } => self.end_main_block(),
            UiEvent::Rotated { reason } => {
                self.rot += 1;
                self.transcript.push(format!("↻ session rotated: {reason}"))
            }
            UiEvent::Usage { text } => {
                let (w, h) = self.usage_geom();
                let text = if text.trim().is_empty() {
                    usage::EMPTY.to_string()
                } else {
                    render::decorate_usage(&text, self.bar.tokens as i64)
                };
                self.usage.set_text(&text, w, h);
            }
            // While the window waits for /usage, its failure goes into the window (Go usageFetch).
            // Go repl.go stage(): one grey note per stage change, same as the Go stageMsg block.
            UiEvent::Stage { slot, phase } => {
                self.end_main_block();
                self.note(&stage_text(&slot, &phase));
            }
            UiEvent::Error(msg) if self.usage.pending() => {
                let (w, h) = self.usage_geom();
                self.usage.set_text(&usage::failed(&msg), w, h);
            }
            UiEvent::Error(msg) => self.transcript.push(format!("! {msg}")),
            // Go tuiWriter note: the interrupted turn keeps its text, then the dim marker.
            UiEvent::Interrupted => {
                // The engine drops the abandoned turn's prompts (core pending_perm), so none can be answered now.
                self.perm = None;
                self.end_main_block();
                self.transcript
                    .push(format!("{GREY_ON}[interrupted]{FG_OFF}"));
                self.transcript.push(String::new());
            }
            // Go record_turn.go: the measurement line follows the turn's assistant text, at column 0.
            UiEvent::TurnStats {
                duration_ms,
                cost_turn_usd,
                cost_usd,
                ctx,
                delegations,
            } => {
                let line = format!(
                    "[측정] duration_ms={duration_ms} cost_turn_usd={} cost_usd={} ctx={ctx} rotations={} delegations={delegations}",
                    bar::fmt_g4(cost_turn_usd),
                    bar::fmt_g4(cost_usd),
                    self.rot
                );
                self.end_main_block();
                self.transcript.push(line);
                self.transcript.push(String::new());
            }
            // Go modeLine / setMode: the start state is not announced, so only later changes print.
            UiEvent::ModeChanged { mode } => {
                self.transcript
                    .push(format!("{GREY_ON}mode: {mode}{FG_OFF}"));
                self.transcript.push(String::new());
                self.bar.mode = mode;
            }
            // Go ui.ask: the prompt is a note; the answer key path is not wired yet.
            UiEvent::PermissionRequest { id, tool, summary } => {
                self.perm = Some(id);
                let note = render::perm_note(&tool, &summary);
                self.transcript.push(format!("{GREY_ON}{note}{FG_OFF}"));
                self.transcript.push(String::new());
            }
            // Go repl.go setRate: the status-line rate text is recomputed on every rate_limit_event.
            UiEvent::RateLimit { info } => {
                self.bar.rate = rate_status(&info, unix_now()).filter(|s| !s.is_empty());
            }
            // Go repl.go budgetStop: one dim line in the transcript (no blank row after it).
            UiEvent::BudgetStop { spent_usd, cap_usd } => {
                let notice = bar::budget_notice(spent_usd, cap_usd);
                self.transcript.push(format!("{GREY_ON}{notice}{FG_OFF}"));
            }
            // Variants added after the TUI wiring (non_exhaustive in core) render nothing yet.
            _ => {}
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
        // The copy note lasts 2s (Go copyNoteMsg timer).
        if self
            .copy_note
            .as_ref()
            .is_some_and(|(_, at)| now.duration_since(*at) >= COPY_NOTE_TTL)
        {
            self.copy_note = None;
        }
    }

    // Maps one key press to an action.
    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        if self.usage.is_open() {
            let (w, h) = self.usage_geom();
            if self.usage.key(key, w, h) {
                self.follow_tail();
            }
            return Action::Continue;
        }
        // Go settingsKey / rpick / sessView keys: each picker reads every key while it is open.
        if let Some(note) = self.settings.as_mut().map(|p| p.key(key)) {
            if let Some(note) = note {
                self.settings = None;
                self.note(&note);
            }
            return Action::Continue;
        }
        if let Some(act) = self.recs.as_mut().map(|rp| rp.key(key)) {
            match act {
                records::Action::None => {}
                records::Action::Close => self.recs = None,
                records::Action::Load(p) => {
                    self.recs = None;
                    self.load_record(&p);
                }
            }
            return Action::Continue;
        }
        if let Some(act) = self.sess.as_mut().map(|sv| sv.key(key)) {
            match act {
                sessview::Action::None => {}
                sessview::Action::Close => {
                    self.sess = None;
                    self.follow_tail();
                }
                sessview::Action::Note(n) => self.note(&n),
            }
            return Action::Continue;
        }
        // Go tui.go rewind branch: while the picker is open it reads every key.
        if self.rw.is_open() {
            let refs: Vec<(&str, &str)> = self.prompts.iter().map(|(_, t)| ("user", t.as_str())).collect();
            if let Some(cut) = self.rw.key(key, &refs) {
                self.transcript.truncate(self.prompts[cut.cut].0);
                self.wide.retain(|&i| i < self.transcript.len());
                self.prompts.truncate(cut.cut);
                self.editor.clear();
                self.editor.insert_str(&cut.text);
                self.note(rewind::REWOUND);
            }
            return Action::Continue;
        }
        // Go avKey: read-only agent view. Only close, switch and scroll keys act; the rest is dropped.
        if self.av.is_some() {
            let recs = self.agent_recs();
            let vp_h = self.rows.saturating_sub(CHROME);
            if let Some(av) = self.av.as_mut()
                && av.key(key, &recs, vp_h) == agentview::Action::Close
            {
                self.av = None;
                self.follow_tail();
            }
            return Action::Continue;
        }
        // Go tui.go key(): while a permission prompt is open, only y/Enter, n/Esc and Ctrl+C are read.
        if let Some(id) = self.perm.clone() {
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let (allow, interrupt) = match key.code {
                KeyCode::Char('y' | 'Y') | KeyCode::Enter => (true, false),
                KeyCode::Char('n' | 'N') | KeyCode::Esc => (false, false),
                KeyCode::Char('c') if ctrl => (false, true),
                _ => return Action::Continue,
            };
            self.perm = None;
            if !interrupt {
                let word = if allow { "allowed" } else { "denied" };
                self.transcript.push(format!("{GREY_ON}{word}{FG_OFF}"));
                self.transcript.push(String::new());
            }
            return Action::Answer {
                id,
                allow,
                interrupt,
            };
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let page = self.rows.saturating_sub(CHROME).max(1) as isize;
        match key.code {
            // Go ctrl+t: the agent view opens on the first agent of the sorted list.
            KeyCode::Char('t') if ctrl => {
                let recs = self.agent_recs();
                match agentview::AgentView::open_first(&recs) {
                    Some(av) => self.av = Some(av),
                    None => self.note(agentview::NO_AGENTS),
                }
                Action::Continue
            }
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
            // Go tui.go: Ctrl+D quits at once, busy or not.
            KeyCode::Char('d') if ctrl => Action::Quit,
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
                self.sugg_sel = 0;
                Action::Continue
            }
            KeyCode::Enter if alt => {
                self.editor.newline();
                Action::Continue
            }
            KeyCode::Enter => self.submit(),
            // Go tui.go: a single Esc interrupts while busy.
            KeyCode::Esc if self.busy => Action::Interrupt(InterruptSource::EscKey),
            KeyCode::Esc => {
                // Go tui.go esc: on an empty input a second Esc within 600 ms opens the rewind picker.
                let now = Instant::now();
                let double = self.editor.is_empty()
                    && self.last_esc.is_some_and(|t| now.duration_since(t) < Duration::from_millis(600));
                self.last_esc = Some(now);
                if double {
                    self.last_esc = None;
                    let opened = {
                        let refs: Vec<(&str, &str)> =
                            self.prompts.iter().map(|(_, t)| ("user", t.as_str())).collect();
                        self.rw.open(&refs)
                    };
                    if !opened {
                        self.note(rewind::NOTHING);
                    }
                }
                Action::Continue
            }
            KeyCode::Backspace => {
                self.editor.backspace();
                self.sugg_sel = 0;
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
            KeyCode::Up if !self.suggest().is_empty() => {
                self.sugg_sel = self.sugg_sel.saturating_sub(1);
                Action::Continue
            }
            KeyCode::Down if !self.suggest().is_empty() => {
                let last = self.suggest().len() - 1;
                self.sugg_sel = (self.sugg_sel + 1).min(last);
                Action::Continue
            }
            KeyCode::Tab if !self.suggest().is_empty() => {
                let text = self.editor.text();
                if let Some(done) = commands::complete(&text, self.sugg_sel, self.busy, false) {
                    self.editor.clear();
                    self.editor.insert_str(&done);
                }
                self.sugg_sel = 0;
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
            KeyCode::BackTab => Action::CycleMode,
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
        self.turn_prompt = render::trunc_chars(text.lines().next().unwrap_or("").trim(), 60);
        self.end_main_block();
        let first = text.split_whitespace().next().unwrap_or("");
        // Go tui.go /view is a local note and echoes nothing into the transcript.
        if first != "/view" {
            self.prompts.push((self.transcript.len(), text.clone()));
            // Go tui.go render(): the user block is grey, "> " on its first row, and is followed by a blank row.
            for (i, line) in text.split('\n').enumerate() {
                let lead = if i == 0 { PROMPT } else { "  " };
                self.transcript
                    .push(format!("{GREY_ON}{lead}{line}{FG_OFF}"));
            }
            self.transcript.push(String::new());
            self.follow_tail();
        }
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
            Some(Dispatch::OpenUsage) => {
                self.usage.open();
                return Action::Usage;
            }
            Some(Dispatch::OpenSettings) => {
                self.settings = Some(settings::Picker::open(
                    std::env::current_dir().ok().as_deref(),
                ));
                return Action::Continue;
            }
            // TODO(update-check): Go runs the release check off the UI thread (tui.go:1100). No update client in cli yet.
            Some(Dispatch::Update) => {
                self.note(commands::UPDATE_START_NOTE);
                return Action::Continue;
            }
            Some(Dispatch::OpenView { arg }) => {
                self.open_view(&arg);
                return Action::Continue;
            }
            // /resume claude (Claude Code sessions) still goes to the engine; only record lookups open here.
            Some(Dispatch::Resume { args })
                if args.first().map(String::as_str) != Some("claude") =>
            {
                self.resume(&args.join(" "));
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

    // Go /view: the sessions this process spawned. TODO(view-owned): Go ownSessionList needs the spawned
    // session ids (main, rotated, resident child), which no UiEvent carries yet, so the owned set is empty.
    fn open_view(&mut self, arg: &str) {
        let result = sessview::session_list(&self.welcome.cwd)
            .and_then(|all| sessview::own_sessions(all, &HashSet::new()))
            .and_then(|list| sessview::SessView::open(list, arg));
        match result {
            Ok(sv) => self.sess = Some(sv),
            // Go prints the note grey and resets the colour on its own row after the text.
            Err(msg) => {
                self.note_wide(&format!("{GREY_ON}{msg}"));
                self.transcript.push(FG_OFF.to_string());
            }
        }
    }

    // Go /resume <arg> over the record folder: several hits open the picker, one hit or a number loads.
    fn resume(&mut self, arg: &str) {
        let root = records::record_dir(&std::env::current_dir().unwrap_or_else(|_| ".".into()));
        match records::pick(&root, arg) {
            Err(e) => self.note(&e),
            Ok(Some(ps)) => self.recs = Some(records::RecPicker::new(ps)),
            Ok(None) => match records::resume(&root, arg) {
                Err(e) => self.note(&e),
                Ok((lines, entries)) => {
                    for l in &lines {
                        self.note(l);
                    }
                    for e in &entries {
                        self.record_block(&e.kind, &e.text);
                    }
                    self.follow_tail();
                }
            },
        }
    }

    // Enter in the record picker: the whole folder is appended to the transcript (Go loadRecordPick).
    fn load_record(&mut self, p: &records::RecordProject) {
        match records::load_project(&p.dir) {
            Err(e) => self.note(&e),
            Ok(entries) => {
                for e in &entries {
                    self.record_block(&e.kind, &e.text);
                }
                self.follow_tail();
            }
        }
    }

    // One record block in the transcript. User blocks use the prompt echo; other kinds are plain text rows.
    // TODO(record-kinds): Go renders assistant blocks through the markdown path; plain rows for now.
    fn record_block(&mut self, kind: &str, text: &str) {
        if kind == "user" {
            for (i, line) in text.split('\n').enumerate() {
                let lead = if i == 0 { PROMPT } else { "  " };
                self.transcript
                    .push(format!("{GREY_ON}{lead}{line}{FG_OFF}"));
            }
        } else {
            self.note(text);
        }
        self.transcript.push(String::new());
    }

    // Body rows of the open /settings, /resume or /view picker (same geometry as /usage). None when none is open.
    // Running agents as the agent view sees them (Go tracker). The name is the description, never the id.
    fn agent_recs(&self) -> Vec<agentview::AgentRec> {
        let mut recs: Vec<agentview::AgentRec> = self
            .agents
            .iter()
            .map(|(id, desc, _)| agentview::AgentRec {
                id: id.clone(),
                task: desc.clone(),
                name: desc.clone(),
                started: true,
                ..Default::default()
            })
            .collect();
        agentview::sort_all(&mut recs);
        recs
    }

    fn overlay_lines(&self, width: usize, h: usize) -> Option<Vec<String>> {
        if self.rw.is_open() {
            let refs: Vec<(&str, &str)> = self.prompts.iter().map(|(_, t)| ("user", t.as_str())).collect();
            return Some(self.rw.lines(&refs, width, h));
        }
        if let Some(av) = &self.av {
            let recs = self.agent_recs();
            let rec = recs.iter().find(|r| r.id == av.id).cloned().unwrap_or_default();
            return Some(av.clone().lines(&rec, &self.bar.model, width, h));
        }
        if let Some(p) = &self.settings {
            return Some(p.lines(width, h));
        }
        if let Some(rp) = &self.recs {
            return Some(rp.lines(width, h));
        }
        self.sess.as_ref().map(|sv| match sv.turns {
            Some(_) => sv.prompt_lines(width, h),
            None => sv.list_lines(width, std::time::SystemTime::now()),
        })
    }

    // Width and body rows of the viewport, as the usage window sees them (no slash menu while it is open).
    fn usage_geom(&self) -> (usize, usize) {
        (
            text_width(self.cols),
            self.rows.saturating_sub(self.chrome_len()).max(1),
        )
    }

    // Closes the open assistant block: flushes its unfinished line and adds Go's blank row after it.
    fn end_main_block(&mut self) {
        flush(
            &mut self.pending_main,
            (ASST_FIRST, "  "),
            &mut self.main_open,
            &mut self.transcript,
            asst_text,
        );
        if self.main_open {
            self.transcript.push(String::new());
        }
        self.main_open = false;
    }

    fn note(&mut self, text: &str) {
        self.transcript.extend(text.split('\n').map(str::to_string));
    }

    // A note whose rows wrap at the full width instead of the chat width.
    fn note_wide(&mut self, text: &str) {
        let at = self.transcript.len();
        self.note(text);
        self.wide.extend(at..self.transcript.len());
    }

    // Go mainStatusText + the spinner (tui.go:385-390): `✶ 생각중-model-메인(prompt) · Mm SSs`, orange frame, grey text.
    fn busy_row(&self) -> String {
        let mut text = format!("생각중-{}-메인", self.welcome.model);
        if !self.turn_prompt.is_empty() {
            text.push_str(&format!("({})", self.turn_prompt));
        }
        if let Some(t) = self.turn_start {
            text.push_str(&bar::elapsed_suffix(
                self.clock.duration_since(t).as_secs(),
                "",
            ));
        }
        format!("{ORANGE_ON}{}{FG_OFF}{GREY_ON} {text}{FG_OFF}", self.spin)
    }

    // Lines in the viewport: transcript, then the unfinished text of each role, wrapped to `width`.
    fn body(&self, width: usize) -> Vec<String> {
        let agent = format!("{AGENT_PREFIX}{}", self.pending_agent);
        let busy_row = self.busy_row();
        let mut lines: Vec<&str> = self.transcript.iter().map(String::as_str).collect();
        if !self.pending_main.is_empty() {
            lines.push(&self.pending_main);
        }
        if !self.pending_agent.is_empty() {
            lines.push(&agent);
        }
        // Go tui.go View(): the main session row sits in the transcript flow while busy.
        if self.busy {
            lines.push(&busy_row);
        }
        // Text width is one less than the full width, so the welcome box gets the full width back.
        let mut rows = self.welcome_rows(width + 1);
        for (i, l) in lines.into_iter().enumerate() {
            let w = if self.wide.contains(&i) { width + 1 } else { width };
            rows.extend(render::wrap_cells(l, w).split('\n').map(str::to_string));
        }
        rows
    }

    // The status row (with the copy note), the input box and the bar above them, drawn at width `w`.
    fn chrome(&self, w: usize) -> Vec<String> {
        let width = text_width(w);
        let bi = self.bar_input();
        let [mut top, mut status] = bar::rows(&bi, self.spin, width);
        if self.unseen && !self.follow {
            top = format!("{top} · {NEW_MSG}");
        }
        // Go tui.go footer: the copy note follows the status text.
        if let Some((note, _)) = &self.copy_note {
            status = format!("{status} · {note}");
        }
        let mut chrome = Vec::with_capacity(CHROME + 1);
        if self.shows_bar() {
            chrome.push(bar::clip(&top, width));
        }
        chrome.extend(self.input_box(w));
        chrome.push(format!("{GREY_ON}{}{FG_OFF}", bar::clip(&status, width)));
        chrome
    }

    // Rows of the transcript viewport on a `w` x `h` screen: what is left after the chrome and the slash menu.
    fn body_h(&self, w: usize, h: usize) -> usize {
        let chrome_h = self.chrome(w).len();
        let menu_max = h.saturating_sub(chrome_h + 1);
        let menu = overlay::slash_popup(&self.suggest(), self.sugg_sel, w, menu_max);
        h.saturating_sub(chrome_h + menu.len())
    }

    // Body cell under screen cell (x, y). A drag clamps into the viewport; a press outside it gives None.
    fn body_at(&self, x: u16, y: u16, clamp: bool) -> Option<render::TextPos> {
        let body_h = self.body_h(self.cols, self.rows);
        let y = y as usize;
        if body_h == 0 || (y >= body_h && !clamp) {
            return None;
        }
        let start = self.start_row(self.total_rows(self.cols), body_h);
        Some(render::TextPos {
            line: start + y.min(body_h - 1),
            col: x as usize,
        })
    }

    // Left press in the transcript starts a selection (Go selectMouse). Usage and the permission prompt keep the mouse.
    fn select_press(&mut self, x: u16, y: u16) {
        if self.usage.is_open() || self.perm.is_some() {
            return;
        }
        if let Some(p) = self.body_at(x, y, false) {
            self.sel = render::TextSel {
                set: true,
                anchor: p,
                head: p,
            };
            self.dragging = true;
        }
    }

    fn select_drag(&mut self, x: u16, y: u16) {
        if !self.dragging {
            return;
        }
        if let Some(p) = self.body_at(x, y, true) {
            self.sel.head = p;
        }
    }

    // Left release ends the drag. A drag over text returns that text to copy; a plain click returns None.
    fn select_release(&mut self, x: u16, y: u16) -> Option<String> {
        if !self.dragging {
            return None;
        }
        self.select_drag(x, y);
        self.dragging = false;
        let text = self.selected_text();
        (!text.is_empty()).then_some(text)
    }

    // The plain text under the selection, one line per body row (Go textSel.text).
    fn selected_text(&self) -> String {
        let plain: Vec<String> = self
            .body(text_width(self.cols))
            .iter()
            .map(|r| render::strip_sgr(r))
            .collect();
        self.sel.text(&plain)
    }

    // Body row `i` with its selected cells in reverse video (Go highlightView).
    fn selected_row(&self, i: usize, row: String) -> String {
        let plain = render::strip_sgr(&row);
        match self.sel.span(i, UnicodeWidthStr::width(plain.as_str())) {
            Some((from, to)) => render::highlight_line(&row, from, to),
            None => row,
        }
    }

    // Records the outcome of a copy: "복사됨 (N자)" on success, the error otherwise. Shown for 2s.
    fn copy_done(&mut self, chars: usize, result: Result<(), String>) {
        let note = match result {
            Ok(()) => format!("복사됨 ({chars}자)"),
            Err(e) => format!("복사 실패: {e}"),
        };
        self.copy_note = Some((note, self.clock));
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
    // Go wheelSel on the open /resume picker or /view list. Returns false when neither is open.
    fn overlay_wheel(&mut self, up: bool) -> bool {
        if let Some(sv) = self.sess.as_mut() {
            sv.wheel(up);
            return true;
        }
        if let Some(rp) = self.recs.as_mut() {
            rp.wheel(up);
            return true;
        }
        false
    }

    fn scroll(&mut self, delta: isize) {
        if self.usage.is_open() {
            let (w, h) = self.usage_geom();
            self.usage.scroll_by(delta, w, h);
            return;
        }
        let total = self.total_rows(self.cols);
        let start = self.start_row(total, self.rows.saturating_sub(self.chrome_len()));
        self.scroll_to((start as isize + delta).max(0) as usize);
    }

    fn scroll_to(&mut self, row: usize) {
        let max = self
            .total_rows(self.cols)
            .saturating_sub(self.rows.saturating_sub(self.chrome_len()));
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
        b.limit = self.welcome.threshold;
        b.agents = self
            .agents
            .iter()
            .map(|(_, d, t)| (d.clone(), self.clock.duration_since(*t).as_secs()))
            .collect();
        b.bg = self
            .agents
            .iter()
            .filter(|(id, _, _)| self.bg_agents.contains(id))
            .count();
        b.fg = self.agents.len() - b.bg;
        b.rot = self.rot;
        b
    }

    // Slash menu rows for the current input (empty when no menu applies).
    fn suggest(&self) -> Vec<&'static commands::SlashCmd> {
        commands::suggestions(&self.editor.text(), self.busy, false)
    }

    // Go inputBox.rows() (input.go:149): logical lines soft-wrapped at `w` cells. Returns the visual rows,
    // and the visual row and cell column of the cursor.
    fn input_rows(&self, w: usize) -> (Vec<String>, usize, usize) {
        let w = w.max(2);
        let pos = self.editor.cursor();
        let (mut rows, mut line) = (Vec::new(), String::new());
        let (mut cells, mut cur) = (0, (0, 0));
        for (i, c) in self.editor.text().chars().enumerate() {
            if c == '\n' {
                if i == pos {
                    cur = (rows.len(), cells);
                }
                rows.push(std::mem::take(&mut line));
                cells = 0;
                continue;
            }
            let cw = UnicodeWidthChar::width(c).unwrap_or(0);
            if cells + cw > w && cells > 0 {
                rows.push(std::mem::take(&mut line));
                cells = 0;
            }
            if i == pos {
                cur = (rows.len(), cells);
            }
            line.push(c);
            cells += cw;
        }
        if pos >= self.editor.text().chars().count() {
            cur = (rows.len(), cells);
        }
        rows.push(line);
        (rows, cur.0, cur.1)
    }

    // Visual rows the input box shows: at least 1, at most MAX_INPUT_ROWS.
    fn input_height(&self, w: usize) -> usize {
        let inner = w.saturating_sub(2).max(1);
        let pw = UnicodeWidthStr::width(PROMPT);
        self.input_rows(inner.saturating_sub(pw))
            .0
            .len()
            .min(MAX_INPUT_ROWS)
    }

    // Rows below the viewport: the top bar (unseen output or agents), then the input box. Busy state is in the body.
    fn shows_bar(&self) -> bool {
        self.unseen || !self.agents.is_empty()
    }

    // Status row, the input box's top and bottom borders, and the input rows.
    fn chrome_len(&self) -> usize {
        CHROME - 1 + self.input_height(self.cols) + usize::from(self.shows_bar())
    }

    // Go welcome() (tui.go:295-311): a rounded orange box at most 66 columns wide, then one blank row.
    // `w` is the full terminal width. The interior is clamped to 20..=64 cells.
    fn welcome_rows(&self, w: usize) -> Vec<String> {
        // Shrinks to narrow terminals; long content lines wrap inside the box instead of overflowing it.
        let inner = w.saturating_sub(2).clamp(4, 64);
        let text = inner - 2; // one padding cell on each side inside the border
        let edge = |s: &str| format!("{ORANGE_ON}{s}{FG_OFF}");
        let grey = |s: String| format!("{GREY_ON}{s}{FG_OFF}");
        let title = format!("{BOLD_ON}{ORANGE_ON}✻ Welcome to Claude Harness!{FG_OFF}{BOLD_OFF}");
        let cwd = render::trunc_chars(&self.welcome.cwd, inner.saturating_sub(12));
        let model = format!(
            "  model: {} · rotate at {} tokens",
            self.welcome.model,
            commands::commas(self.welcome.threshold)
        );
        let lines = [
            title,
            String::new(),
            grey("  /help for help, /rotate to rotate session, /quit to exit".to_string()),
            String::new(),
            grey(format!("  cwd: {cwd}")),
            grey(model),
        ];
        let mut rows = vec![edge(&format!("╭{}╮", "─".repeat(inner)))];
        for l in lines {
            for piece in render::wrap_cells(&l, text).split('\n') {
                let pad = text.saturating_sub(render::cell_width(piece));
                rows.push(format!(
                    "{} {piece}{}{}",
                    edge("│"),
                    " ".repeat(pad + 1),
                    edge("│")
                ));
            }
        }
        rows.push(edge(&format!("╰{}╯", "─".repeat(inner))));
        rows.push(String::new());
        rows
    }

    // Go View() input box (tui.go:1222-1340): rounded grey border, the prompt, a reversed cursor cell and,
    // while the input is empty, the placeholder (tui.go:879). `w` is the full terminal width.
    fn input_box(&self, w: usize) -> Vec<String> {
        let inner = w.saturating_sub(2).max(1);
        let edge = |l: &str, r: &str| format!("{GREY_ON}{l}{}{r}{FG_OFF}", "─".repeat(inner));
        let side = |mid: &str| {
            let pad = inner.saturating_sub(render::cell_width(mid));
            format!(
                "{GREY_ON}│{FG_OFF}{mid}{}{GREY_ON}│{FG_OFF}",
                " ".repeat(pad)
            )
        };
        let mut out = vec![edge("╭", "╮")];
        if self.editor.is_empty() {
            let ph = render::input_placeholder(self.busy);
            // The cursor cell takes one column after the prompt, so the placeholder gets the rest.
            let ph = bar::clip(ph, inner.saturating_sub(PROMPT.len() + 1));
            out.push(side(&format!(
                "{PROMPT}{REV_ON} {RESET}{GREY_ON}{ph}{FG_OFF}"
            )));
        } else {
            let pw = UnicodeWidthStr::width(PROMPT);
            let (rows, cur_row, cur_col) = self.input_rows(inner.saturating_sub(pw));
            let (top, shown) = input_window(rows.len(), cur_row);
            for (i, row) in rows.iter().enumerate().skip(top).take(shown) {
                let pre = if i == 0 { PROMPT } else { "  " };
                // Go input.go View: reverse-video cursor cell (I33 input_view_row), byte index from the cell column.
                let cursor = (i == cur_row).then(|| split_cells(row, cur_col).0.len());
                out.push(side(&format!("{pre}{}", render::input_view_row(row, cursor))));
            }
        }
        out.push(edge("╰", "╯"));
        out
    }
}

// Go stage lines (repl.go stage calls): "· <slot> 처리중" on start, "· <slot> 완료" on success,
// "· <slot> 실패: <err>" on failure. Agent slots use Go's "호출" on start and "agent <desc>" as the name.
fn stage_text(slot: &str, phase: &StagePhase) -> String {
    let (name, start) = match slot.strip_prefix("agent:") {
        Some(desc) => (format!("agent {desc}"), "호출"),
        None if slot == "tui" => (slot.to_string(), "번역중"),
        None => (slot.to_string(), "처리중"),
    };
    match phase {
        StagePhase::Working => format!("· {name} {start}"),
        StagePhase::Done => format!("· {name} 완료"),
        StagePhase::Failed(err) => format!("· {name} 실패: {err}"),
    }
}

// Go inputBox.View window (input.go:211): at most MAX_INPUT_ROWS rows, scrolled so the cursor row stays visible.
// Returns the first row and the number of rows shown.
fn input_window(n: usize, cur: usize) -> (usize, usize) {
    let shown = n.min(MAX_INPUT_ROWS);
    let top = (cur + 1).saturating_sub(shown).min(n - shown);
    (top, shown)
}

// Copies `text` like Go copyToClipboard: OSC 52 to the terminal (reaches the local clipboard over SSH) and,
// on macOS, pbcopy. The error is the native copy's; elsewhere only OSC 52 is written, so success there is unverified.
fn copy_text(out: &mut impl Write, text: &str) -> Result<(), String> {
    let osc = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
    out.write_all(osc.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|e| e.to_string())?;
    native_copy(text)
}

#[cfg(target_os = "macos")]
fn native_copy(text: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("pbcopy {status}"))
    }
}

#[cfg(not(target_os = "macos"))]
fn native_copy(_text: &str) -> Result<(), String> {
    Ok(())
}

// Standard base64 (RFC 4648, padded) for the OSC 52 payload.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = |k: usize| u32::from(*chunk.get(k).unwrap_or(&0));
        let n = (b(0) << 16) | (b(1) << 8) | b(2);
        for i in 0..4u32 {
            if i as usize <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

// Splits `s` at display column `col`. Used to place the cursor cell inside the input line.
fn split_cells(s: &str, col: usize) -> (&str, &str) {
    let mut acc = 0;
    for (i, c) in s.char_indices() {
        if acc >= col {
            return s.split_at(i);
        }
        acc += UnicodeWidthChar::width(c).unwrap_or(0);
    }
    (s, "")
}

// Columns usable for text. The last column stays empty so a full row never auto-wraps.
fn text_width(w: usize) -> usize {
    w.saturating_sub(1).max(1)
}

// The whole screen as `h` rows of at most `w` columns. Pure: the same App and size give the same rows.
pub fn frame(app: &App, w: usize, h: usize) -> Vec<String> {
    let width = text_width(w);
    let chrome = app.chrome(w);
    let chrome_h = chrome.len();
    // The slash menu sits right above the input box and never takes the last row of the body.
    let menu_max = h.saturating_sub(chrome_h + 1);
    // Go draws the menu rows at the full terminal width (no spare last column).
    let menu = overlay::slash_popup(&app.suggest(), app.sugg_sel, w, menu_max);
    let body_h = h.saturating_sub(chrome_h + menu.len());
    let mut rows = Vec::with_capacity(h);
    let overlay = if body_h > 0 && !app.usage.is_open() {
        app.overlay_lines(width, body_h)
    } else {
        None
    };
    if body_h > 0 && app.usage.is_open() {
        rows.extend(app.usage.lines(width, body_h));
    } else if let Some(lines) = overlay {
        rows.extend(lines);
    } else if body_h > 0 {
        let body = app.body(width);
        let start = app.start_row(body.len(), body_h);
        for i in 0..body_h {
            let row = body.get(start + i).cloned().unwrap_or_default();
            rows.push(app.selected_row(start + i, row));
        }
    }
    rows.extend(menu);
    // Too short for the whole chrome: keep the bottom rows, input line included.
    rows.extend(chrome.into_iter().skip(chrome_h - h.min(chrome_h)));
    rows
}

// Terminal cell that holds the cursor: inside the input box (after its left border), on the input row.
fn cursor(app: &App, w: usize, h: usize) -> (u16, u16) {
    let inner = w.saturating_sub(2).max(1);
    let pw = UnicodeWidthStr::width(PROMPT);
    let (rows, cur_row, cur_col) = app.input_rows(inner.saturating_sub(pw));
    let (top, shown) = input_window(rows.len(), cur_row);
    // The input rows sit above the box's bottom border and the status row.
    let y = h.saturating_sub(2 + shown) + (cur_row - top);
    ((1 + pw + cur_col) as u16, y as u16)
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

// Appends text to `buf`, emitting each completed line to `out`. `lead` is (first line, later lines); `open`
// records whether the block already has a line, so the first-line lead is written only once per block.
fn feed(
    buf: &mut String,
    text: &str,
    lead: (&str, &str),
    open: &mut bool,
    out: &mut Vec<String>,
    style: fn(&str) -> String,
) {
    buf.push_str(text);
    while let Some(i) = buf.find('\n') {
        let line: String = buf.drain(..=i).collect();
        out.push(format!(
            "{}{}",
            if *open { lead.1 } else { lead.0 },
            style(line.trim_end_matches('\n'))
        ));
        *open = true;
    }
}

// Assistant text as Go shows it: **bold** pairs, then link styling (tui.go:374 mdBold + wrapLinked).
fn asst_text(line: &str) -> String {
    render::linkify(&render::md_bold(line))
}

fn plain_text(line: &str) -> String {
    line.to_string()
}

// Emits whatever is left in `buf` as a final line.
fn flush(
    buf: &mut String,
    lead: (&str, &str),
    open: &mut bool,
    out: &mut Vec<String>,
    style: fn(&str) -> String,
) {
    if !buf.is_empty() {
        out.push(format!(
            "{}{}",
            if *open { lead.1 } else { lead.0 },
            style(&std::mem::take(buf))
        ));
        *open = true;
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
    let model = settings::Settings {
        main_model: cfg.model.clone(),
        ..settings::Settings::default()
    }
    .model_id()
    .to_string();
    let threshold = cfg.context_window;
    let (engine, rx) = Engine::start(cfg);
    let _guard = TermGuard::enter()?;
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let mut app = App::new(cols as usize, rows as usize);
    app.welcome.model = model;
    app.welcome.threshold = threshold;
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
                Event::Key(key) if key.kind == KeyEventKind::Press => match app.on_key(key) {
                    Action::Continue => {}
                    Action::Submit(text) => engine.submit(&text),
                    Action::Interrupt(src) => engine.interrupt(src),
                    Action::Quit => break 'main,
                    Action::Usage => engine.usage(),
                    Action::Answer {
                        id,
                        allow,
                        interrupt,
                    } => {
                        engine.answer_permission(&id, allow);
                        if interrupt {
                            engine.interrupt(InterruptSource::CtrlC);
                        }
                    }
                    Action::CycleMode => engine.set_mode("next"),
                },
                Event::Paste(text) => app.editor.insert_str(&text),
                Event::Mouse(m) if app.mouse => match m.kind {
                    MouseEventKind::ScrollUp if app.overlay_wheel(true) => {}
                    MouseEventKind::ScrollDown if app.overlay_wheel(false) => {}
                    MouseEventKind::ScrollUp => app.scroll(-WHEEL_LINES),
                    MouseEventKind::ScrollDown => app.scroll(WHEEL_LINES),
                    MouseEventKind::Down(MouseButton::Left) => app.select_press(m.column, m.row),
                    MouseEventKind::Drag(MouseButton::Left) => app.select_drag(m.column, m.row),
                    MouseEventKind::Up(MouseButton::Left) => {
                        if let Some(text) = app.select_release(m.column, m.row) {
                            let result = copy_text(&mut out, &text);
                            app.copy_done(text.chars().count(), result);
                        }
                    }
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

// Go ratestatus.go rateNum: a JSON number, or a numeric string.
fn rate_num(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

// Go ratestatus.go fmtRateStatus: "now/reset · N%" from the five-hour window; None when no percent is known.
// Top-level fields only count for the five_hour window (rateLimitType is "" or "five_hour").
fn rate_status(info: &serde_json::Value, now: i64) -> Option<String> {
    let w = &info["unifiedWindows"]["five_hour"];
    let top = info["rateLimitType"].as_str().unwrap_or("");
    let top_is_5h = top.is_empty() || top == "five_hour";
    let mut v = rate_num(&w["utilization"]).or_else(|| {
        if top_is_5h {
            rate_num(&info["utilization"])
        } else {
            None
        }
    })?;
    if v < 1.0 {
        v *= 100.0; // fraction scale (0.37 -> 37%)
    }
    let mut reset = rate_num(&w["resetsAt"]);
    if reset.is_none() && top_is_5h {
        reset = rate_num(&info["resetsAt"]);
    }
    let reset = match reset {
        Some(r) if r > 1e12 => (r / 1000.0) as i64, // milliseconds
        Some(r) => r as i64,
        None => now,
    };
    Some(format!(
        "{}/{} · {}%",
        clock_hhmm(now),
        clock_hhmm(reset),
        v.round() as i64
    ))
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// Local "HH:MM" (Go time.Format("15:04")).
#[cfg(unix)]
fn clock_hhmm(secs: i64) -> String {
    let t = secs as libc::time_t;
    // SAFETY: localtime_r writes into the zeroed tm we own and reads only the local `t`.
    let tm = unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm
    };
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}

// No libc off unix: UTC clock, so the rate text stays well-formed.
#[cfg(not(unix))]
fn clock_hhmm(secs: i64) -> String {
    let day = secs.rem_euclid(86_400);
    format!("{:02}:{:02}", day / 3600, day % 3600 / 60)
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

    // Double Esc on an empty editor opens the rewind picker; Enter cuts the transcript to the chosen prompt
    // and puts that prompt back into the editor (Go tui.go rewind branch).
    #[test]
    fn double_esc_rewinds_to_selected_prompt() {
        let mut app = App::new(100, 30);
        for (i, text) in ["first", "second"].iter().enumerate() {
            app.prompts.push((app.transcript.len(), text.to_string()));
            app.transcript.push(format!("> {text}"));
            app.transcript.push(format!("answer {i}"));
        }
        assert!(matches!(app.on_key(key(KeyCode::Esc, KeyModifiers::NONE)), Action::Continue));
        assert!(matches!(app.on_key(key(KeyCode::Esc, KeyModifiers::NONE)), Action::Continue));
        assert!(app.rw.is_open());
        // The picker starts on the newest prompt; step back to the first one, then confirm.
        app.on_key(key(KeyCode::Up, KeyModifiers::NONE));
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.rw.is_open());
        assert_eq!(app.prompts.len(), 0);
        // Only the rewound note remains; the cut turns are gone.
        assert!(app.transcript.iter().all(|l| !l.starts_with("> ") && !l.starts_with("answer")));
        assert!(app.transcript.iter().any(|l| l.contains("rewound")));
        assert_eq!(app.editor.text(), "first");
    }

    // An app at 30x5 (two viewport rows) with lines "line 0" .. "line {n-1}".
    fn app_with_lines(n: usize) -> App {
        let mut app = App::new(30, 6);
        for i in 0..n {
            app.transcript.push(format!("line {i}"));
        }
        app
    }

    const IDLE_STATUS: &str = " M 0/200k P 0/80k T 0/80k C 0/80k · rot 0 · mode auto";

    // Frame rows with the SGR escapes removed, so tests compare the visible text.
    fn screen(app: &App, w: usize, h: usize) -> Vec<String> {
        frame(app, w, h)
            .into_iter()
            .map(|r| {
                let mut out = String::new();
                let mut chars = r.chars();
                while let Some(c) = chars.next() {
                    if c == '\x1b' {
                        for d in chars.by_ref() {
                            if d == 'm' {
                                break;
                            }
                        }
                    } else {
                        out.push(c);
                    }
                }
                out
            })
            .collect()
    }

    // The idle status row clipped to the text width (w - 1 columns).
    fn idle_status(w: usize) -> String {
        IDLE_STATUS.chars().take(w - 1).collect()
    }

    // Expected chrome rows at width 30: optional bar row, the input box, then the status row.
    fn chrome(bar: Option<&str>, input: &str) -> Vec<String> {
        let inner = 28;
        // Empty input shows the cursor cell, then the idle placeholder.
        let mid = if input.is_empty() {
            ">  대기 - 프롬프트 입력".to_string()
        } else {
            format!("> {input}")
        };
        let pad = inner - UnicodeWidthStr::width(mid.as_str());
        let mut rows: Vec<String> = bar.map(String::from).into_iter().collect();
        rows.push(format!("╭{}╮", "─".repeat(inner)));
        rows.push(format!("│{mid}{}│", " ".repeat(pad)));
        rows.push(format!("╰{}╯", "─".repeat(inner)));
        rows.push(idle_status(30));
        rows
    }

    fn type_str(app: &mut App, s: &str) {
        for c in s.chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
    }

    #[test]
    fn slash_menu_sits_above_the_input_box_and_the_cursor_stays_on_input() {
        let mut app = App::new(30, 8);
        type_str(&mut app, "/mo");
        let rows = screen(&app, 30, 8);
        assert_eq!(rows.len(), 8);
        assert!(rows[1].starts_with("  /model "), "{rows:?}");
        assert!(rows[2].starts_with("  /mode "), "{rows:?}");
        assert!(rows[3].starts_with("  /mouse "), "{rows:?}");
        assert!(rows[5].starts_with("│> /mo"), "{rows:?}");
        assert_eq!(rows[7], idle_status(30));
        assert_eq!(cursor(&app, 30, 8), (6, 5));
    }

    // Go inputBox.View: a second logical line adds a row to the box, the cursor follows it.
    #[test]
    fn multiline_input_grows_the_box_and_the_cursor_follows_the_last_line() {
        let mut app = App::new(30, 10);
        type_str(&mut app, "line one");
        app.editor.newline();
        type_str(&mut app, "line two");
        let rows = screen(&app, 30, 10);
        assert_eq!(rows[5], format!("╭{}╮", "─".repeat(28)), "{rows:?}");
        assert!(rows[6].starts_with("│> line one"), "{rows:?}");
        assert!(rows[7].starts_with("│  line two"), "{rows:?}");
        assert!(rows[8].starts_with("╰"), "{rows:?}");
        assert_eq!(rows[9], idle_status(30));
        assert_eq!(UnicodeWidthStr::width(rows[6].as_str()), 30);
        assert_eq!(UnicodeWidthStr::width(rows[7].as_str()), 30);
        assert_eq!(cursor(&app, 30, 10), (11, 7));
    }

    // draw() ends by showing the real cursor at the editor cell (1-based MoveTo: row+1, col+1).
    #[test]
    fn draw_shows_the_real_cursor_on_the_editor_cell() {
        let mut app = App::new(30, 10);
        type_str(&mut app, "ab");
        app.editor.newline();
        type_str(&mut app, "cd");
        let mut buf = Vec::new();
        draw(&mut buf, &app, &mut Vec::new()).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.ends_with("\x1b[?25h"), "{out:?}");
        // Cursor after "cd" on the second input row: 0-based (5, 7), so MoveTo is 1-based row 8, column 6.
        assert!(out.contains("\x1b[8;6H\x1b[?25h"), "{out:?}");
    }

    #[test]
    fn down_moves_the_menu_selection_and_tab_completes_it() {
        let mut app = App::new(30, 8);
        type_str(&mut app, "/mo");
        app.on_key(key(KeyCode::Down, KeyModifiers::NONE));
        assert!(screen(&app, 30, 8)[2].starts_with("  /mode "));
        assert!(frame(&app, 30, 8)[2].starts_with(ORANGE_ON));
        app.on_key(key(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.editor.text(), "/mode ");
        assert!(screen(&app, 30, 8).iter().all(|r| !r.contains("/mouse")));
    }

    #[test]
    fn no_menu_without_a_slash_word_and_up_still_walks_history() {
        let mut app = App::new(30, 8);
        type_str(&mut app, "hi");
        assert_eq!(screen(&app, 30, 8).len(), 8);
        assert!(screen(&app, 30, 8)[5].starts_with("│> hi"));
        app.on_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.editor.text(), "hi");
    }

    #[test]
    fn empty_screen_shows_the_input_box_and_the_status_row() {
        let app = App::new(30, 5);
        // Five rows: one body row (the welcome box's trailing blank) above the chrome.
        let mut want = vec!["".to_string()];
        want.extend(chrome(None, ""));
        assert_eq!(screen(&app, 30, 5), want);
    }

    #[test]
    fn long_output_shows_the_tail_until_scrolled() {
        let mut app = app_with_lines(10);
        let mut want = vec!["line 8".to_string(), "line 9".to_string()];
        want.extend(chrome(None, ""));
        assert_eq!(screen(&app, 30, 6), want);

        app.scroll(-3);
        let mut want = vec!["line 5".to_string(), "line 6".to_string()];
        want.extend(chrome(None, ""));
        assert_eq!(screen(&app, 30, 6), want);
    }

    #[test]
    fn new_output_while_held_keeps_position_and_shows_the_indicator() {
        let mut app = app_with_lines(10);
        app.scroll(-3);
        app.apply(text("line 10\n"));
        let mut want = vec!["line 5".to_string(), "line 6".to_string()];
        want.extend(chrome(Some("idle · ↓ 새 메시지"), ""));
        assert_eq!(screen(&app, 30, 7), want);
    }

    #[test]
    fn typing_while_scrolled_keeps_position() {
        let mut app = app_with_lines(10);
        app.scroll(-3);
        app.on_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        app.on_key(key(KeyCode::Char('b'), KeyModifiers::NONE));
        let rows = screen(&app, 30, 6);
        assert_eq!(rows[0], "line 5");
        assert_eq!(rows[1], "line 6");
        assert!(rows[3].starts_with("│> ab"), "{rows:?}");
    }

    #[test]
    fn new_output_at_the_bottom_follows_the_tail() {
        let mut app = app_with_lines(10);
        app.transcript.push("line 10".to_string());
        let mut want = vec!["line 9".to_string(), "line 10".to_string()];
        want.extend(chrome(None, ""));
        assert_eq!(screen(&app, 30, 6), want);
    }

    #[test]
    fn end_key_returns_to_the_tail_and_clears_the_indicator() {
        let mut app = app_with_lines(10);
        app.scroll(-3);
        app.transcript.push("line 10".to_string());
        app.on_key(key(KeyCode::End, KeyModifiers::NONE));
        let rows = screen(&app, 30, 6);
        assert_eq!(rows[0], "line 9");
        assert_eq!(rows[1], "line 10");
        assert!(rows[2].starts_with('╭'), "{rows:?}");
    }

    #[test]
    fn resize_clamps_the_view_and_rewraps_wide_text() {
        let mut app = app_with_lines(10);
        app.on_key(key(KeyCode::Home, KeyModifiers::NONE));
        // Narrower and shorter: one viewport row, and the status row is clipped to the width.
        app.cols = 12;
        app.rows = 5;
        let rows = screen(&app, 12, 5);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[4], " M 0/200k P");
        for r in &rows {
            assert!(UnicodeWidthStr::width(r.as_str()) <= 12, "{r:?}");
        }

        let mut app = App::new(30, 5);
        app.transcript.push("가나다라마바사".to_string());
        let rows = screen(&app, 7, 6);
        assert_eq!(rows[0], "라마바");
        assert_eq!(rows[1], "사");
    }

    #[test]
    fn transcript_blocks_follow_go_leads_and_blank_rows() {
        let mut app = App::new(40, 12);
        app.apply(text("hi\n"));
        app.apply(UiEvent::MessageDone { role: Role::Main });
        app.apply(UiEvent::ToolStart {
            id: "t1".into(),
            name: "Agent".into(),
            summary: "scan repo".into(),
        });
        app.apply(UiEvent::ToolEnd {
            id: "t1".into(),
            ok: true,
        });
        app.apply(UiEvent::ToolResult {
            id: "t1".into(),
            text: "done".into(),
        });
        app.apply(UiEvent::TurnEnd { duration_ms: 1000 });
        let rows: Vec<String> = app
            .transcript
            .iter()
            .map(|l| render::strip_sgr(l))
            .collect();
        assert_eq!(
            rows,
            vec!["● hi", "", "● Agent(scan repo)", "  ⎿  done", ""]
        );
    }

    // R04 (tui.go:481): an agent caveman return adds no transcript row; a plain tool result still does.
    #[test]
    fn caveman_tool_result_is_hidden_from_the_transcript() {
        let mut app = app_with_lines(0);
        let before = app.transcript.len();
        app.apply(UiEvent::ToolResult {
            id: "t1".to_string(),
            text: "status: done\nreason: -\nclaim: a.go:1 | x".to_string(),
        });
        assert_eq!(app.transcript.len(), before);
        app.apply(UiEvent::ToolResult {
            id: "t2".to_string(),
            text: "plain result".to_string(),
        });
        assert!(app.transcript.len() > before);
    }

    // R03/R08 (tui.go:374): assistant text gets **bold** and link styling, and keeps its visible text.
    #[test]
    fn assistant_text_is_bold_and_linked_per_line() {
        let mut app = app_with_lines(0);
        app.apply(UiEvent::TextDelta {
            role: Role::Main,
            text: "**hi** see https://a.io\n".to_string(),
        });
        let row = app.transcript.last().unwrap().clone();
        assert!(row.contains(render::BOLD_ON));
        assert_eq!(
            render::strip_sgr(&row)
                .trim_start_matches("● ")
                .trim_start(),
            "hi see https://a.io"
        );
    }

    #[test]
    fn enter_submits_and_echoes_while_blank_enter_does_nothing() {
        let mut app = App::new(80, 24);
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Continue
        );
        for c in "hi".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.on_key(key(KeyCode::Char('j'), KeyModifiers::CONTROL));
        app.on_key(key(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Submit("hi\nx".into())
        );
        assert_eq!(
            app.transcript,
            vec![
                format!("{GREY_ON}> hi{FG_OFF}"),
                format!("{GREY_ON}  x{FG_OFF}"),
                String::new(),
            ]
        );
    }

    #[test]
    fn quit_and_mouse_are_handled_locally() {
        let mut app = App::new(80, 24);
        for c in "/mouse".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Continue
        );
        assert!(!app.mouse);
        for c in "/quit".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Quit
        );
    }

    #[test]
    fn busy_row_sits_in_the_transcript_flow_with_go_text() {
        let mut app = App::new(100, 30);
        type_str(&mut app, "long task");
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        app.apply(UiEvent::Busy(true));
        let body = app.body(100);
        assert!(
            body.iter()
                .any(|r| r.contains("생각중-claude-opus-5-5-메인(long task) · 0m00s")),
            "{body:?}"
        );
        // The busy state no longer adds a chrome row, so only the top bar (unseen or agents) can.
        assert!(!app.shows_bar());
    }

    #[test]
    fn a_single_escape_interrupts_only_while_busy() {
        let mut app = App::new(80, 24);
        let esc = || key(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(app.on_key(esc()), Action::Continue);
        app.apply(UiEvent::Busy(true));
        assert_eq!(
            app.on_key(esc()),
            Action::Interrupt(InterruptSource::EscKey)
        );
        app.apply(UiEvent::Busy(false));
        assert_eq!(app.on_key(esc()), Action::Continue);
    }

    #[test]
    fn ctrl_c_interrupts_when_busy_clears_when_idle_and_quits_when_idle_and_empty() {
        let mut app = App::new(80, 24);
        let ctrl_c = || key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        app.on_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(app.on_key(ctrl_c()), Action::Continue);
        assert!(app.editor.is_empty());
        assert_eq!(app.on_key(ctrl_c()), Action::Quit);
        app.apply(UiEvent::Busy(true));
        assert_eq!(
            app.on_key(ctrl_c()),
            Action::Interrupt(InterruptSource::CtrlC)
        );
    }

    #[test]
    fn permission_prompt_takes_y_n_enter_esc_and_swallows_other_keys() {
        let mut app = App::new(80, 24);
        let ask = |app: &mut App| {
            app.apply(UiEvent::PermissionRequest {
                id: "p1".into(),
                tool: "Bash".into(),
                summary: "ls".into(),
            })
        };
        ask(&mut app);
        assert_eq!(
            app.on_key(key(KeyCode::Char('x'), KeyModifiers::NONE)),
            Action::Continue
        );
        assert!(app.editor.is_empty());
        assert_eq!(
            app.on_key(key(KeyCode::Char('y'), KeyModifiers::NONE)),
            Action::Answer {
                id: "p1".into(),
                allow: true,
                interrupt: false
            }
        );
        ask(&mut app);
        assert_eq!(
            app.on_key(key(KeyCode::Esc, KeyModifiers::NONE)),
            Action::Answer {
                id: "p1".into(),
                allow: false,
                interrupt: false
            }
        );
        ask(&mut app);
        assert_eq!(
            app.on_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Action::Answer {
                id: "p1".into(),
                allow: false,
                interrupt: true
            }
        );
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Continue
        );
    }

    #[test]
    fn interrupt_drops_an_open_permission_prompt() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::PermissionRequest {
            id: "p2".into(),
            tool: "Bash".into(),
            summary: "rm".into(),
        });
        app.apply(UiEvent::Interrupted);
        assert_eq!(
            app.on_key(key(KeyCode::Char('y'), KeyModifiers::NONE)),
            Action::Continue
        );
    }

    #[test]
    fn shift_tab_cycles_mode_and_ctrl_d_quits() {
        let mut app = App::new(80, 24);
        assert_eq!(
            app.on_key(key(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Action::CycleMode
        );
        assert_eq!(
            app.on_key(key(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Action::Quit
        );
    }

    #[test]
    fn rate_status_follows_go_fmt_rate_status() {
        use serde_json::json;
        let now = 1_700_000_000;
        let reset = 1_700_003_600;
        let want = format!("{}/{} · 37%", clock_hhmm(now), clock_hhmm(reset));
        // Fraction scale, ISO-less epoch seconds.
        let frac =
            json!({"unifiedWindows": {"five_hour": {"utilization": 0.37, "resetsAt": reset}}});
        assert_eq!(rate_status(&frac, now).as_deref(), Some(want.as_str()));
        // Percent scale with milliseconds and a string value.
        let pct = json!({"unifiedWindows": {"five_hour": {"utilization": "37", "resetsAt": reset * 1000}}});
        assert_eq!(rate_status(&pct, now).as_deref(), Some(want.as_str()));
        // No reset time: the reset column shows now.
        let no_reset = json!({"unifiedWindows": {"five_hour": {"utilization": 0.37}}});
        assert_eq!(
            rate_status(&no_reset, now),
            Some(format!("{0}/{0} · 37%", clock_hhmm(now)))
        );
        // No percent, or a top-level percent for another window: nothing to show.
        assert_eq!(rate_status(&json!({"unifiedWindows": {}}), now), None);
        let weekly = json!({"rateLimitType": "seven_day", "utilization": 0.37});
        assert_eq!(rate_status(&weekly, now), None);
    }

    #[test]
    fn rate_event_sets_the_status_rate_and_budget_stop_prints_one_line() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::RateLimit {
            info: serde_json::json!({"unifiedWindows": {"five_hour": {"utilization": 0.5}}}),
        });
        assert!(
            app.bar
                .rate
                .as_deref()
                .is_some_and(|r| r.ends_with("· 50%"))
        );
        app.apply(UiEvent::BudgetStop {
            spent_usd: 1.5,
            cap_usd: 2.0,
        });
        assert_eq!(
            app.transcript.last().map(String::as_str),
            Some(
                format!("{GREY_ON}[budget] $1.5000 / $2.0000 reached, task stopped{FG_OFF}")
                    .as_str()
            )
        );
    }

    #[test]
    fn drag_over_body_rows_selects_text_and_release_returns_it() {
        let mut app = app_with_lines(3);
        assert_eq!(screen(&app, 30, 6)[..2], ["line 1", "line 2"]);
        app.select_press(0, 0);
        app.select_drag(3, 1);
        assert_eq!(app.select_release(3, 1), Some("line 1\nlin".to_string()));
    }

    #[test]
    fn reverse_drag_highlights_in_reverse_video_and_a_click_copies_nothing() {
        let mut app = app_with_lines(3);
        app.select_press(0, 1);
        assert_eq!(app.select_release(0, 1), None);
        // Press on row 1, drag up to row 0: the selection is ordered from row 0 col 2 to row 1 col 5.
        app.select_press(5, 1);
        app.select_drag(2, 0);
        assert!(frame(&app, 30, 6)[0].contains("\x1b[7m"));
        assert_eq!(screen(&app, 30, 6)[..2], ["line 1", "line 2"]);
        assert_eq!(app.select_release(2, 0), Some("ne 1\nline ".to_string()));
    }

    #[test]
    fn presses_outside_the_viewport_or_under_a_prompt_do_not_select() {
        let mut app = app_with_lines(3);
        app.select_press(0, 5); // input and status rows, below the two viewport rows
        assert!(!app.sel.set);
        app.perm = Some("p1".into());
        app.select_press(0, 0);
        assert!(!app.sel.set);
    }

    #[test]
    fn copy_note_shows_the_result_on_the_status_row_for_two_seconds() {
        let mut app = app_with_lines(3);
        app.copy_done(5, Ok(()));
        assert!(screen(&app, 120, 6)[5].ends_with("복사됨 (5자)"));
        app.copy_done(0, Err("boom".into()));
        assert!(screen(&app, 120, 6)[5].ends_with("복사 실패: boom"));
        let t = app.clock;
        app.tick(t + Duration::from_millis(1999));
        assert!(app.copy_note.is_some());
        app.tick(t + Duration::from_millis(2000));
        assert!(app.copy_note.is_none());
    }

    #[test]
    fn base64_matches_rfc4648_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64("복사".as_bytes()), "67O17IKs");
    }

    #[test]
    fn draw_writes_only_changed_rows() {
        let app = App::new(20, 5);
        let mut prev = Vec::new();
        let mut buf: Vec<u8> = Vec::new();
        draw(&mut buf, &app, &mut prev).unwrap();
        let text = String::from_utf8_lossy(&buf);
        assert!(text.contains("╭"), "{text:?}");
        assert!(text.contains("M 0/200k"), "{text:?}");
        assert_eq!(prev, frame(&app, 20, 5));

        let mut again: Vec<u8> = Vec::new();
        draw(&mut again, &app, &mut prev).unwrap();
        assert!(!String::from_utf8_lossy(&again).contains("M 0/200k"));
    }
}
