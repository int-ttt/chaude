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
pub mod resume_pick;
pub mod rewind;
mod runewidth_table;
pub mod sessview;
pub mod settings;
pub mod usage;

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, Write};
use std::panic;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chaude_core::api::{AgentRow, Config, Engine, InterruptSource, Role, UiEvent};
use chaude_core::handoff;
use chaude_core::import_claude::{self, ClaudeImport};
use chaude_core::resident::StagePhase;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, ScrollDown, ScrollUp,
};
use crossterm::{execute, queue};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use bar::BarInput;
use commands::{Ctx, Dispatch, PermMode};
use edit::Editor;
use render::{BOLD_OFF, BOLD_ON, FG_OFF, GREY_ON, ORANGE_ON, RESET};

const TICK: Duration = Duration::from_millis(80);
// Shortest gap between two frames (about 60 fps). Output that arrives faster is coalesced into the next frame.
const FRAME: Duration = Duration::from_millis(16);
const PROMPT: &str = "> ";
// Rows below the viewport when idle: input box top, input line, input box bottom, status row.
// A running row sits above them while busy (`App::shows_bar`).
const CHROME: usize = 4;
// Go maxInputRows (tui.go:988).
const MAX_INPUT_ROWS: usize = 5;
// Go queueLines (tui.go:929): most waiting prompts shown above the input box.
const MAX_QUEUE_ROWS: usize = 5;
// Go maxPanel (tui.go:1452): most rows in one agent panel (foreground or background).
const MAX_PANEL_ROWS: usize = 4;
// Go lastTool: `trunc(t, 40)` in runes.
const LAST_TOOL_RUNES: usize = 40;
// Go minViewRows: the transcript keeps at least this many rows before the queue and panels take space.
const MIN_VIEW_ROWS: usize = 3;
// Reverse-video cell for the cursor (tui.go View, cursor style).
const REV_ON: &str = "\x1b[7m";
const WHEEL_LINES: isize = 3;
// Go copyNote lifetime (selection.go copyNoteMsg, 2s).
const COPY_NOTE_TTL: Duration = Duration::from_secs(2);
// Go defaultIdleNotice (repl.go:620): quiet time with no open tool call before the "still waiting" notice.
const IDLE_NOTICE: Duration = Duration::from_secs(30 * 60);
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
    // /update: the caller runs the release check off the UI thread; the result returns as Wake::Note (tui.go:1166).
    Update,
    // Answer to the open permission prompt (Go tui.go key(): y/Enter allow, n/Esc deny, Ctrl+C deny + interrupt).
    Answer {
        id: String,
        allow: bool,
        interrupt: bool,
    },
    // Shift+Tab: the engine cycles the permission mode (Go setMode(Next())); it answers with ModeChanged.
    CycleMode,
    // /mode <name>: the engine sets the permission mode and answers with ModeChanged.
    SetMode(PermMode),
}

pub struct App {
    // Cursor column shift for IME preedit (Go imeShift, read once at start from the env).
    ime_shift: i32,
    // Keep the hardware cursor hidden (parked for IME only). JetBrains-JediTerm would show it next to the drawn cell.
    hide_cursor: bool,
    // /import or --from-claude: the loaded transcript, taken by the event loop and installed into the engine.
    import_install: Option<ClaudeImport>,
    editor: Editor,
    // Terminal size as last seen by the event loop. Key handling needs it to page and clamp the viewport.
    cols: usize,
    rows: usize,
    bar: BarInput,
    busy: bool,
    turn_start: Option<Instant>,
    // First line of the prompt that started the turn (Go curPrompt), shown in the busy row.
    turn_prompt: String,
    // (id, description, elapsed clock, started_at in ms since epoch: AgentStart carries Unix seconds).
    agents: Vec<(String, String, Instant, u64)>,
    // Ids of the agents launched in the background (UiEvent::AgentBackground); they have no AgentEnd until done.
    bg_agents: Vec<String>,
    // Go opusRunning / residentBusy: an opus Agent is running, and the resident roles with a command in flight.
    opus_busy: bool,
    // Busy resident sessions as running rows (Go residentBusy), oldest first; id and name are the role.
    resident_busy: Vec<AgentRow>,
    // Go tracker, shared with the engine: the tick re-reads it into agent_rows. None without an engine.
    agent_src: Option<Arc<Mutex<Vec<AgentRow>>>>,
    // Panel rows of the started agents, oldest first (UiEvent::AgentRows; Go tracker rows).
    agent_rows: Vec<AgentRow>,
    // Go lastRun: running count (running_n) at the last refresh; the idle tick skips when it is unchanged and not busy.
    last_run: usize,
    // Agents an earlier harness left unfinished, listed after the running rows (UiEvent::AgentsRestored).
    restored_rows: Vec<AgentRow>,
    // Agent-view log by agent tool id (UiEvent::AgentLog; Go agentRec.log, newest MAX_AGENT_LOG lines).
    agent_logs: HashMap<String, Vec<String>>,
    // First agent row shown in the panels (Go panOff), moved by the wheel over them.
    pan_off: usize,
    // Last tool call of the running turn, cut to 40 runes (Go lastTool); the busy row shows it.
    last_tool: String,
    // Prompts waiting for the next turn, oldest first (UiEvent::QueueTexts; Go m.queue).
    queue_texts: Vec<String>,
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
    // Last content event (Go seen counter) and open tool call ids. The idle notice needs both (Go openTools).
    quiet_since: Option<Instant>,
    open_tools: HashSet<String>,
    quiet_noted: bool,
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
    // Open /resume session picker (Go m.pick): the Claude Code session groups of this project.
    pick: Option<resume_pick::GroupPicker>,
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
    // Transcript indices of assistant and user rows; only these get link styles in body() (Go tui.go:374, 376).
    linked: HashSet<usize>,
    // Id of the permission request waiting for y/n (UiEvent::PermissionRequest). While set, keys answer it.
    perm: Option<String>,
    // Drag selection in body coordinates (wrapped body row as `line`, cell column as `col`); the Go textSel model.
    sel: render::TextSel,
    dragging: bool,
    // "복사됨 (N자)" or the copy error, shown on the status row for 2s (Go copyNote); the Instant is when it was set.
    copy_note: Option<(String, Instant)>,
    // Which mode the previous session loads with (session_mode_note), shown at start and by /view.
    session_mode: Option<String>,
    // Session ids of this run (UiEvent::SessionIds: main, residents, owned). /view lists them with the previous run's.
    this_run: BTreeSet<String>,
    // Session id the next rotation replaces (UiEvent::RotatedFrom), read by the Rotated notice.
    rot_from: String,
    // Current main session id (UiEvent::SessionIds main; empty after a rotation until the new one). /status shows it.
    session_id: String,
    // Tool rows by tool id: transcript index and tool name. ToolInput rewrites the row in place.
    tool_rows: HashMap<String, (usize, String)>,
    // Agents whose tool_result was an error (ToolEnd ok=false), by tool id until their AgentEnd.
    agent_outcome: HashSet<String>,
    // Foreground agents already finished by an interrupt (Go finishForeground); core's later AgentEnd for them is dropped.
    finished_fg: Vec<String>,
    // Main text streamed in this turn (Go gotText). Cleared when a turn starts.
    got_text: bool,
    welcome: Welcome,
    // Wrapped rows of each transcript line (see WrapCache). Filled by body(); the frame only reads it.
    wraps: RefCell<WrapCache>,
}

// Wrapped rows per transcript line, each kept with a hash of what produced them (text, width, link style).
// A frame rewraps only the lines whose hash changed, so a long history costs a hash per row, not a rewrap.
#[derive(Default)]
struct WrapCache {
    rows: Vec<(u64, Vec<String>)>,
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
            agent_rows: Vec::new(),
            last_run: 0,
            agent_src: None,
            restored_rows: Vec::new(),
            agent_logs: HashMap::new(),
            pan_off: 0,
            last_tool: String::new(),
            queue_texts: Vec::new(),
            rot: 0,
            session_id: String::new(),
            clock: Instant::now(),
            spin: ' ',
            transcript: Vec::new(),
            main_open: false,
            pending_main: String::new(),
            pending_agent: String::new(),
            top: 0,
            follow: true,
            unseen: false,
            quiet_since: None,
            open_tools: HashSet::new(),
            opus_busy: false,
            resident_busy: Vec::new(),
            quiet_noted: false,
            mouse: true,
            ime_shift: 0,
            hide_cursor: false,
            sugg_sel: 0,
            usage: usage::Usage::default(),
            settings: None,
            recs: None,
            pick: None,
            sess: None,
            av: None,
            rw: rewind::Rewind::default(),
            prompts: Vec::new(),
            import_install: None,
            last_esc: None,
            wide: HashSet::new(),
            linked: HashSet::new(),
            perm: None,
            sel: render::TextSel::default(),
            dragging: false,
            copy_note: None,
            session_mode: None,
            this_run: BTreeSet::new(),
            rot_from: String::new(),
            tool_rows: HashMap::new(),
            agent_outcome: HashSet::new(),
            finished_fg: Vec::new(),
            got_text: false,
            wraps: RefCell::new(WrapCache::default()),
        }
    }

    // Folds one engine event into the transcript and status state.
    pub fn apply(&mut self, ev: UiEvent) {
        let content = !matches!(
            ev,
            UiEvent::Status { .. }
                | UiEvent::Busy(_)
                | UiEvent::Queued(_)
                | UiEvent::AgentRows { .. }
                | UiEvent::AgentsRestored { .. }
                | UiEvent::AgentLog { .. }
                | UiEvent::QueueTexts(_)
                | UiEvent::OpusBusy(_)
                | UiEvent::ResidentBusy { .. }
        );
        if matches!(
            ev,
            UiEvent::TextDelta {
                role: Role::Main,
                ..
            } | UiEvent::TuiAnswer { .. }
        ) {
            self.got_text = true;
        }
        match ev {
            UiEvent::TextDelta {
                role: Role::Main,
                text,
            } => self.feed_main(&text),
            // With residents the tui answer is the whole main reply (Go viaTUI): it replaces the streamed text.
            UiEvent::TuiAnswer { text } => {
                self.feed_main(&text);
                self.end_main_block();
            }
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
            UiEvent::ToolStart { id, name, summary } => {
                self.open_tools.insert(id.clone());
                self.end_main_block();
                let head = if summary.is_empty() {
                    name.clone()
                } else {
                    format!("{name}({summary})")
                };
                self.last_tool = trunc_runes(&head, LAST_TOOL_RUNES);
                self.tool_rows.insert(id, (self.transcript.len(), name));
                self.transcript.push(format!("{TOOL_ON}● {head}{FG_OFF}"));
            }
            // Go tool_summary: the input JSON gives the row its argument summary. Rewrites only a row still in place.
            UiEvent::ToolInput { id, input } => {
                let summary = render::tool_summary(&input);
                if let Some((row, name)) = self.tool_rows.get(&id).cloned()
                    && let Some(line) = self.transcript.get_mut(row)
                {
                    let head = if summary.is_empty() {
                        name
                    } else {
                        format!("{name}({summary})")
                    };
                    self.last_tool = trunc_runes(&head, LAST_TOOL_RUNES);
                    *line = format!("{TOOL_ON}● {head}{FG_OFF}");
                }
            }
            // Go shows the tool_result text, not a done/failed word; that row comes from ToolResult.
            UiEvent::ToolEnd { id, ok } => {
                self.open_tools.remove(&id);
                if !ok && self.is_agent_tool(&id) {
                    self.agent_outcome.insert(id);
                }
            }
            // Go toolResultMsg: the first line cut to 160 runes, then "… (+N lines)" for the rest.
            UiEvent::ToolResult { text, .. } => {
                // Go hides an agent's caveman return from the transcript (tui.go:481); None means hidden.
                let Some(head) = render::tool_result_text(&text) else {
                    return;
                };
                self.transcript
                    .push(format!("{GREY_ON}  ⎿  {head}{FG_OFF}"));
                self.transcript.push(String::new());
            }
            UiEvent::AgentBackground { id } => self.bg_agents.push(id),
            UiEvent::AgentStart {
                id,
                description,
                started_at,
            } => {
                self.agents
                    .push((id, description, Instant::now(), started_at * 1000));
            }
            // A background agent resumed by SendMessage runs again under its original tool id (core keeps background=true).
            UiEvent::AgentResumed {
                id,
                description,
                started_at,
            } => {
                if !self.agents.iter().any(|(aid, _, _, _)| *aid == id) {
                    self.agents
                        .push((id.clone(), description, Instant::now(), started_at * 1000));
                }
                if !self.bg_agents.contains(&id) {
                    self.bg_agents.push(id);
                }
            }
            UiEvent::AgentEnd {
                id,
                model,
                name,
                summary,
                ..
            } => {
                // Already finished by an interrupt (finish_foreground): drop the late duplicate.
                if let Some(pos) = self.finished_fg.iter().position(|f| *f == id) {
                    self.finished_fg.remove(pos);
                    self.agent_outcome.remove(&id);
                } else {
                    self.agent_done(&id, &model, &name, &summary);
                }
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
                if b {
                    self.got_text = false;
                    // Go sendPrompt resets lastTool at the start of each turn.
                    self.last_tool.clear();
                    // A new turn is past any late AgentEnd of the interrupted one.
                    self.finished_fg.clear();
                } else {
                    // Go doneMsg (tui.go:517): turn over, no foreground agent can still be running.
                    self.finish_foreground();
                }
                // A new turn starts its quiet count now, not at the last event of the previous turn.
                self.quiet_since = Some(self.clock);
                self.quiet_noted = false;
            }
            UiEvent::Queued(n) => self.bar.queued = n,
            // Go shows no per-turn "done" row; its footer comes from engine stats that UiEvent does not carry yet.
            UiEvent::TurnEnd { .. } => self.end_main_block(),
            UiEvent::RoleContext { role, tokens } => {
                self.bar.role_tokens.insert(role, tokens);
            }
            // Go /view: this run's main, resident and owned session ids.
            UiEvent::SessionIds {
                main,
                residents,
                owned,
            } => {
                self.session_id = main.clone();
                self.this_run = std::iter::once(main)
                    .chain(residents.into_values())
                    .chain(owned)
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            // Go repl.go:1412: /clear starts a new session.
            UiEvent::NewSession { .. } => self.note(bar::NEW_SESSION_NOTICE),
            UiEvent::RotatedFrom { old_session } => {
                self.session_id.clear();
                self.rot_from = old_session;
            }
            // Go repl.go:1436: a claude child left by a dead harness. Core sends the model alias when the handoff
            // file had one; otherwise it is the main model for main and the role name for a resident.
            UiEvent::OrphanNotice { role, pid, model } => {
                let model = if !model.is_empty() {
                    model
                } else if role == handoff::MAIN_ROLE {
                    self.welcome.model.clone()
                } else {
                    role
                };
                let cwd = self.welcome.cwd.clone();
                self.note(&bar::orphan_notice(pid, &model, &cwd));
            }
            UiEvent::Rotated { reason } => {
                self.rot += 1;
                let old = std::mem::take(&mut self.rot_from);
                self.note(&bar::rotated_notice(self.rot, &reason, &old));
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
            // Go resident.go notice(): a failed prompter or tui pass, one note line.
            UiEvent::Notice { text } => {
                self.end_main_block();
                self.note(&text);
            }
            UiEvent::Error(msg) if self.usage.pending() => {
                let (w, h) = self.usage_geom();
                self.usage.set_text(&usage::failed(&msg), w, h);
            }
            // Go repl.go error paths: a dim note block, then a blank row (tui.go:379 note rendering).
            UiEvent::Error(msg) => {
                self.end_main_block();
                self.transcript.push(format!("{GREY_ON}{msg}{FG_OFF}"));
                self.transcript.push(String::new());
            }
            // Core relaunched the idle main child with --resume; the view moves to the session list.
            UiEvent::SessionResumed {
                old_session,
                session_id,
            } => {
                self.note(&format!(
                    "session resumed: {session_id} (was {old_session})"
                ));
                self.open_sessions("");
            }
            // Go tuiWriter note: the interrupted turn keeps its text, then the dim marker.
            UiEvent::Interrupted => {
                // The engine drops the abandoned turn's prompts (core pending_perm), so none can be answered now.
                self.perm = None;
                self.end_main_block();
                self.transcript
                    .push(format!("{GREY_ON}[interrupted]{FG_OFF}"));
                self.transcript.push(String::new());
            }
            // Go record_turn.go writes the [측정] line to the record file only; the Go TUI never showed it
            // (repl.go:1143 prints to os.Stdout under bubbletea), so the transcript has no row for it.
            UiEvent::TurnStats { .. } => {}
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
            // Go agentRows / m.queue: the panels and the queue are redrawn from these lists each frame.
            UiEvent::AgentRows { rows } => {
                self.agent_rows = rows;
                self.last_run = self.running_n();
            }
            UiEvent::AgentsRestored { rows } => self.restored_rows = rows,
            UiEvent::AgentLog { id, lines } => {
                agentview::push_log(self.agent_logs.entry(id).or_default(), lines)
            }
            UiEvent::QueueTexts(texts) => self.queue_texts = texts,
            // Go opusRunning / residentBusy: state only, read by the thinking row.
            UiEvent::OpusBusy(on) => self.opus_busy = on,
            UiEvent::ResidentBusy {
                role,
                busy,
                prompt,
                started_at,
            } => {
                self.resident_busy.retain(|r| r.id != role);
                if busy {
                    self.resident_busy.push(AgentRow {
                        id: role.clone(),
                        task: String::new(),
                        prompt,
                        name: role,
                        model: String::new(),
                        background: false,
                        done: false,
                        agent_id: String::new(),
                        result_path: String::new(),
                        started_at,
                    });
                }
            }
            // Variants added after the TUI wiring (non_exhaustive in core) render nothing yet.
            _ => {}
        }
        if content && !self.follow {
            self.unseen = true;
        }
        if content {
            self.quiet_since = Some(self.clock);
            self.quiet_noted = false;
        }
    }

    // True while something changes with time (spinner, elapsed clock, copy note). The loop wakes every TICK only then.
    pub fn animating(&self) -> bool {
        self.busy || self.copy_note.is_some()
    }

    // Hands the TUI the engine's agent list. The tick re-reads it (Go tick: m.r.tracker.running()).
    pub fn set_agent_source(&mut self, src: Arc<Mutex<Vec<AgentRow>>>) {
        self.agent_src = Some(src);
    }

    // Records the time for elapsed counters and the spinner (Go spinner: one frame per 125 ms).
    pub fn tick(&mut self, now: Instant) {
        self.clock = now;
        // Go tick gate (tui.go:454): re-read the agent list only while busy or when the running count moved since
        // the last refresh. Rust lays out per frame, so the list is enough.
        if let Some(src) = &self.agent_src {
            let live = src.lock().unwrap_or_else(|e| e.into_inner());
            let n = live.iter().filter(|r| !r.done).count() + self.resident_busy.len();
            if self.busy || n != self.last_run {
                self.agent_rows = live.clone();
                self.last_run = n;
            }
        }
        // Go quiet(): one notice after a quiet stretch with no open tool call (Agent/Task calls are waited for).
        let quiet_for = self.quiet_since.map(|since| now.duration_since(since));
        if self.busy
            && self.open_tools.is_empty()
            && !self.quiet_noted
            && quiet_for.is_some_and(|d| d >= IDLE_NOTICE)
        {
            self.quiet_noted = true;
            self.note(&bar::quiet_notice(IDLE_NOTICE));
        }
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
        // Go tui.go key(): while a permission prompt is open, only y/Enter, n/Esc and Ctrl+C are read.
        // It runs before every overlay, so an open overlay cannot swallow the answer.
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
        if let Some(act) = self.pick.as_mut().map(|gp| gp.key(key)) {
            match act {
                resume_pick::Action::None => {}
                resume_pick::Action::Close => self.pick = None,
                resume_pick::Action::Pick(key) => {
                    self.pick = None;
                    self.import_view(&key);
                }
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
        let (sw, sh) = (text_width(self.cols), self.body_h(self.cols, self.rows));
        if let Some(act) = self.sess.as_mut().map(|sv| sv.key(key, sw, sh)) {
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
            let refs: Vec<(&str, &str)> = self
                .prompts
                .iter()
                .map(|(_, t)| ("user", t.as_str()))
                .collect();
            if let Some(cut) = self.rw.key(key, &refs) {
                self.transcript.truncate(self.prompts[cut.cut].0);
                self.tool_rows
                    .retain(|_, (row, _)| *row < self.transcript.len());
                self.wide.retain(|&i| i < self.transcript.len());
                self.linked.retain(|&i| i < self.transcript.len());
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
            let vp_h = self.body_h(self.cols, self.rows);
            if let Some(av) = self.av.as_mut()
                && av.key(key, &recs, vp_h) == agentview::Action::Close
            {
                self.av = None;
                self.follow_tail();
            }
            return Action::Continue;
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
                // Go tui.go: Ctrl+C interrupts when busy, otherwise quits even with input text.
                if self.busy {
                    Action::Interrupt(InterruptSource::CtrlC)
                } else {
                    Action::Quit
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
                    && self
                        .last_esc
                        .is_some_and(|t| now.duration_since(t) < Duration::from_millis(600));
                self.last_esc = Some(now);
                if double {
                    self.last_esc = None;
                    let opened = {
                        let refs: Vec<(&str, &str)> = self
                            .prompts
                            .iter()
                            .map(|(_, t)| ("user", t.as_str()))
                            .collect();
                        self.rw.open(&refs)
                    };
                    if !opened {
                        self.note(rewind::NOTHING);
                    }
                }
                Action::Continue
            }
            // Go tui.go KeyBackspace with Alt (opt+delete) deletes the previous word.
            KeyCode::Backspace if alt => {
                self.editor.delete_word();
                self.sugg_sel = 0;
                Action::Continue
            }
            // Cmd+Backspace: terminals send ^U (handled above); kitty CSI-u sends Backspace with Super.
            KeyCode::Backspace if key.modifiers.contains(KeyModifiers::SUPER) => {
                self.editor.kill_to_start();
                self.sugg_sel = 0;
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
            // Go tui.go: the completer owns Up/Down only while open and not browsing a recalled prompt; Up at its top goes to history.
            KeyCode::Up
                if self.sugg_sel > 0 && !self.editor.browsing() && !self.suggest().is_empty() =>
            {
                self.sugg_sel -= 1;
                Action::Continue
            }
            KeyCode::Down if !self.editor.browsing() && !self.suggest().is_empty() => {
                let n = self.suggest().len();
                self.sugg_sel = (self.sugg_sel + 1) % n;
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
                // The input width is the one input_rows wraps at, so Up moves the visual row shown on screen.
                let inner = self.cols.saturating_sub(2).max(1);
                let w = inner.saturating_sub(UnicodeWidthStr::width(PROMPT));
                if !self.editor.move_row(-1, w) {
                    self.editor.history_prev();
                }
                Action::Continue
            }
            KeyCode::Down => {
                let inner = self.cols.saturating_sub(2).max(1);
                let w = inner.saturating_sub(UnicodeWidthStr::width(PROMPT));
                if !self.editor.move_row(1, w) {
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
        let dispatched = {
            let ctx = Ctx {
                model: &self.bar.model,
                last_ctx: self.bar.tokens,
                threshold: self.welcome.threshold,
                rotations: u64::from(self.rot),
                session_id: Some(self.session_id.as_str()),
                mode: commands::parse_mode(&self.bar.mode).unwrap_or(PermMode::Default),
                chaude_version: env!("CARGO_PKG_VERSION"),
                busy: self.busy,
            };
            commands::dispatch(&text, &ctx)
        };
        // Go tui.go /clear: the transcript is emptied first (m.blocks = nil), then the user block is shown.
        // A busy /clear is refused by dispatch, so it never reaches here.
        if matches!(dispatched, Some(Dispatch::Clear)) {
            self.clear_transcript();
        }
        let first = text.split_whitespace().next().unwrap_or("");
        // Go tui.go /view is a local note and echoes nothing into the transcript.
        if first != "/view" {
            self.prompts.push((self.transcript.len(), text.clone()));
            // Go tui.go render(): the user block is grey, "> " on its first row, and is followed by a blank row.
            let at = self.transcript.len();
            for (i, line) in text.split('\n').enumerate() {
                let lead = if i == 0 { PROMPT } else { "  " };
                self.transcript
                    .push(format!("{GREY_ON}{lead}{line}{FG_OFF}"));
            }
            self.link_from(at);
            self.transcript.push(String::new());
            self.follow_tail();
        }
        // Go tui.go /status: answered locally from the TUI state, never sent to the child. /mode and /model are handled here too.
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
            Some(Dispatch::Update) => {
                self.note(commands::UPDATE_START_NOTE);
                return Action::Update;
            }
            Some(Dispatch::OpenView { arg }) => {
                self.open_view(&arg);
                return Action::Continue;
            }
            Some(Dispatch::Import { arg, .. }) => {
                self.import_view(&arg);
                return Action::Continue;
            }
            Some(Dispatch::Resume { args }) => return self.resume(&args),
            Some(Dispatch::Note(s)) => {
                self.note(&s);
                return Action::Continue;
            }
            // Go tui.go /mode: setMode; the engine answers ModeChanged, which prints "mode: <name>".
            Some(Dispatch::SetMode(p)) => return Action::SetMode(p),
            // Go tui.go /model NAME: sent to the engine as the full line, which rotates to the new model.
            _ => {}
        }
        Action::Submit(text)
    }

    // Go /view: the sessions this run and the previous run spawned. The previous run comes from the exit handoff
    // (handoff::prev_owned, Go loadPrevOwned); this run's ids come from UiEvent::SessionIds.
    // /import [id|latest]: the imported Claude Code view replaces the transcript (Go importedMsg), and the
    // transcript is queued for the engine (Go applyImport) so the next prompt starts from it.
    fn import_view(&mut self, arg: &str) {
        match import_claude::load_import_arg(arg, &self.welcome.cwd) {
            Err(e) => self.note(&e),
            Ok((_, imp)) => {
                self.show_import(&imp);
                self.import_install = Some(imp);
            }
        }
    }

    // Empties the transcript and the row indexes that point into it (Go m.blocks = nil).
    fn clear_transcript(&mut self) {
        self.transcript.clear();
        self.wide.clear();
        self.linked.clear();
        self.prompts.clear();
    }

    // Go importedMsg: replaces the transcript with the imported view (importBlocks), then the install note.
    fn show_import(&mut self, imp: &ClaudeImport) {
        self.clear_transcript();
        for (kind, text) in render::import_blocks(&imp.view) {
            self.record_block(kind, &text);
        }
        self.note(IMPORT_INSTALL_NOTE);
        self.follow_tail();
    }

    fn open_view(&mut self, arg: &str) {
        if let Some(mode) = self.session_mode.clone() {
            self.note(&mode);
        }
        self.open_sessions(arg);
    }

    // The /view list itself, also reopened after SessionResumed so the switch is visible.
    fn open_sessions(&mut self, arg: &str) {
        let mut owned: HashSet<String> = handoff::read_exit_handoff(&handoff::handoff_file(
            &chaude_core::record::dir_for(std::path::Path::new(&self.welcome.cwd)),
        ))
        .map(|h| h.prev_owned().into_iter().collect())
        .unwrap_or_default();
        owned.extend(self.this_run.iter().cloned());
        let result = sessview::session_list(&self.welcome.cwd)
            .and_then(|all| sessview::own_sessions(all, &owned))
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

    // Go /resume <args> (resumeTarget): "claude <q>", or any arg no record answers, is a Claude Code session query
    // (group picker). Otherwise the record folder answers: several hits open the picker, one hit or a number loads.
    fn resume(&mut self, args: &[String]) -> Action {
        let arg = args.join(" ");
        let root =
            chaude_core::record::dir_for(&std::env::current_dir().unwrap_or_else(|_| ".".into()));
        let (terms, record) = chaude_core::record_turn::resume_target_in(&root, args);
        if !record {
            return self.open_groups(&terms);
        }
        match records::pick(&root, &arg) {
            Err(e) => self.note(&e),
            Ok(Some(ps)) => self.recs = Some(records::RecPicker::new(ps)),
            Ok(None) => match chaude_core::record_turn::resume(&root, &arg, &records::list_date) {
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
        Action::Continue
    }

    // Go groupList + pick: one matching group imports it (sendImport), several open the picker, an error is the note.
    fn open_groups(&mut self, terms: &[String]) -> Action {
        match resume_pick::groups(&self.welcome.cwd, terms) {
            Err(e) => {
                self.note(&e);
                Action::Continue
            }
            Ok(gs) if gs.len() == 1 => {
                self.import_view(&gs[0].key());
                Action::Continue
            }
            Ok(gs) => {
                self.pick = Some(resume_pick::GroupPicker::new(gs));
                Action::Continue
            }
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

    // One record block in the transcript. User blocks use the prompt echo; asst blocks use the markdown
    // path as live output does (asst_text); other kinds are plain note rows.
    fn record_block(&mut self, kind: &str, text: &str) {
        if kind == "user" {
            let at = self.transcript.len();
            for (i, line) in text.split('\n').enumerate() {
                let lead = if i == 0 { PROMPT } else { "  " };
                self.transcript
                    .push(format!("{GREY_ON}{lead}{line}{FG_OFF}"));
            }
            self.link_from(at);
        } else if kind == "asst" {
            let at = self.transcript.len();
            for (i, line) in text.split('\n').enumerate() {
                let lead = if i == 0 { ASST_FIRST } else { "  " };
                self.transcript.push(format!("{lead}{}", asst_text(line)));
            }
            self.link_from(at);
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
            .map(|(id, desc, _, started_at)| agentview::AgentRec {
                id: id.clone(),
                task: desc.clone(),
                name: desc.clone(),
                started: true,
                started_at: *started_at,
                log: self.agent_logs.get(id).cloned().unwrap_or_default(),
                ..Default::default()
            })
            .collect();
        // Restored rows are finished and interrupted; they open in the view like running agents.
        recs.extend(
            self.restored_rows
                .iter()
                .filter(|r| self.agents.iter().all(|(id, _, _, _)| *id != r.id))
                .map(|r| agentview::AgentRec {
                    id: r.id.clone(),
                    task: r.task.clone(),
                    prompt: r.prompt.clone(),
                    model: r.model.clone(),
                    name: r.name.clone(),
                    done: true,
                    interrupted: true,
                    started: true,
                    started_at: r.started_at * 1000,
                    log: self.agent_logs.get(&r.id).cloned().unwrap_or_default(),
                    ..Default::default()
                }),
        );
        agentview::sort_all(&mut recs);
        recs
    }

    fn overlay_lines(&self, width: usize, h: usize) -> Option<Vec<String>> {
        if self.rw.is_open() {
            let refs: Vec<(&str, &str)> = self
                .prompts
                .iter()
                .map(|(_, t)| ("user", t.as_str()))
                .collect();
            return Some(self.rw.lines(&refs, width, h));
        }
        if let Some(av) = &self.av {
            let recs = self.agent_recs();
            let rec = recs
                .iter()
                .find(|r| r.id == av.id)
                .cloned()
                .unwrap_or_default();
            return Some(av.clone().lines(&rec, &self.bar.model, width, h));
        }
        if let Some(p) = &self.settings {
            return Some(p.lines(width, h));
        }
        if let Some(rp) = &self.recs {
            return Some(rp.lines(width, h));
        }
        if let Some(gp) = &self.pick {
            return Some(gp.lines(width, h));
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
    fn feed_main(&mut self, text: &str) {
        let at = self.transcript.len();
        feed(
            &mut self.pending_main,
            text,
            (ASST_FIRST, "  "),
            &mut self.main_open,
            &mut self.transcript,
            asst_text,
        );
        self.link_from(at);
    }

    fn end_main_block(&mut self) {
        let at = self.transcript.len();
        flush(
            &mut self.pending_main,
            (ASST_FIRST, "  "),
            &mut self.main_open,
            &mut self.transcript,
            asst_text,
        );
        self.link_from(at);
        if self.main_open {
            self.transcript.push(String::new());
        }
        self.main_open = false;
    }

    fn note(&mut self, text: &str) {
        self.transcript.extend(text.split('\n').map(str::to_string));
    }

    // Go finishForeground: on interrupt the foreground panel agents are done at once, not when core reports them.
    // Background agents (bg_agents) keep running.
    fn finish_foreground(&mut self) {
        let fg: Vec<(String, String)> = self
            .agents
            .iter()
            .filter(|(id, _, _, _)| !self.bg_agents.contains(id))
            .map(|(id, desc, _, _)| (id.clone(), desc.clone()))
            .collect();
        for (id, desc) in fg {
            // Core's AgentEnd (model, name) normally arrives first; this fallback has only the start label.
            self.agent_done(&id, "", &desc, &desc);
            self.finished_fg.push(id);
        }
    }

    // One agent's done notice (Go markDoneLocked, doneLine): model, name and what (the agent's last summary
    // line, else its task) come from the AgentEnd.
    fn agent_done(&mut self, id: &str, model: &str, name: &str, what: &str) {
        let failed = self.agent_outcome.remove(id);
        self.agents.retain(|(aid, _, _, _)| aid != id);
        self.bg_agents.retain(|aid| aid != id);
        let notice = bar::done_line(failed, model, name, what);
        self.note(&format!("{GREY_ON}{notice}{FG_OFF}"));
    }

    // Marks the rows pushed since `at` as assistant or user rows, whose links body() styles.
    fn link_from(&mut self, at: usize) {
        self.linked.extend(at..self.transcript.len());
    }

    // A note whose rows wrap at the full width instead of the chat width.
    fn note_wide(&mut self, text: &str) {
        let at = self.transcript.len();
        self.note(text);
        self.wide.extend(at..self.transcript.len());
    }

    // Agent and Task tool calls, which launch a subagent. Checked by the name the ToolStart row was given.
    fn is_agent_tool(&self, id: &str) -> bool {
        matches!(self.tool_rows.get(id), Some((_, name)) if name == "Agent" || name == "Task")
    }

    // Go mainStatusText + the spinner (tui.go:385-390): `✶ 생각중-model-메인(prompt) · Mm SSs`, orange frame, grey text.
    fn busy_row(&self, esc: bool) -> String {
        let mut text = format!("생각중-{}-메인", self.welcome.model);
        if !self.turn_prompt.is_empty() {
            text.push_str(&format!("({})", self.turn_prompt));
        }
        if let Some(t) = self.turn_start {
            text.push_str(&bar::elapsed_suffix(
                self.clock.duration_since(t).as_secs(),
                &self.last_tool,
            ));
        }
        if esc {
            text.push_str(" (esc to interrupt)");
        }
        format!("{ORANGE_ON}{}{FG_OFF}{GREY_ON} {text}{FG_OFF}", self.spin)
    }

    // Lines in the viewport: transcript, then the unfinished text of each role, wrapped to `width`.
    fn body(&self, width: usize) -> Vec<String> {
        let agent = format!("{AGENT_PREFIX}{}", self.pending_agent);
        let status_row = self.busy_row(false);
        let esc_row = self.busy_row(true);
        let mut lines: Vec<&str> = self.transcript.iter().map(String::as_str).collect();
        // The unfinished line gets the lead feed() would give it, so it hangs like the finished rows.
        let live_main = format!(
            "{}{}",
            if self.main_open { "  " } else { ASST_FIRST },
            self.pending_main
        );
        if !self.pending_main.is_empty() {
            lines.push(&live_main);
        }
        if !self.pending_agent.is_empty() {
            lines.push(&agent);
        }
        // Go tui.go View() thinking row (tui.go:383-391), in the transcript flow. Go runningN/residentBusy:
        // an opus Agent or a resident role with a command in flight keeps the row up after text started.
        let fg = self
            .agents
            .iter()
            .any(|(id, _, _, _)| !self.bg_agents.contains(id));
        match bar::thinking_row(
            fg,
            self.opus_busy,
            !self.resident_busy.is_empty(),
            self.got_text,
            self.perm.is_some(),
            self.busy,
        ) {
            bar::ThinkingRow::None => {}
            bar::ThinkingRow::Status => lines.push(&status_row),
            bar::ThinkingRow::StatusWithEsc => lines.push(&esc_row),
        }
        // Text width is one less than the full width, so the welcome box gets the full width back.
        let mut rows = self.welcome_rows(width + 1);
        let mut cache = self.wraps.borrow_mut();
        cache.rows.truncate(self.transcript.len());
        for (i, l) in lines.into_iter().enumerate() {
            let w = if self.wide.contains(&i) {
                width + 1
            } else {
                width
            };
            // Transcript rows come from the cache: rewrapped only when their text, width or style changed.
            if i < self.transcript.len() {
                let linked = self.linked.contains(&i);
                let key = wrap_key(l, w, linked);
                if cache.rows.len() <= i {
                    cache.rows.push((key, wrap_row(l, w, linked)));
                } else if cache.rows[i].0 != key {
                    cache.rows[i] = (key, wrap_row(l, w, linked));
                }
                for row in &cache.rows[i].1 {
                    rows.push(row.clone());
                }
                continue;
            }
            // Go wrapLinked (tui.go:374, 376) styles links in assistant and user rows only; the live row is assistant text.
            let live = i == self.transcript.len() && !self.pending_main.is_empty();
            rows.extend(wrap_row(l, w, live || self.linked.contains(&i)));
        }
        rows
    }

    // The bar, the queue, the input box, the foreground and background agent rows, then the status row, drawn at
    // width `w` on a screen `h` rows tall (Go View order: queue, box, panel, bgPanel, statusLine).
    fn chrome(&self, w: usize, h: usize) -> Vec<String> {
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
        let (fg, bg) = self.panel_lines(w, h);
        let mut chrome = Vec::with_capacity(CHROME + 1);
        if self.shows_bar() {
            chrome.push(bar::clip(&top, width));
        }
        chrome.extend(self.queue_lines(w, h));
        chrome.extend(self.input_box(w));
        chrome.extend(fg);
        chrome.extend(bg);
        chrome.push(format!("{GREY_ON}{}{FG_OFF}", bar::clip(&status, width)));
        chrome
    }

    // Go queueLines (tui.go:929-945): waiting prompts above the input box, oldest first, dim. Past the cap the
    // last row becomes "… +N 대기" with the count still waiting. The transcript keeps MIN_VIEW_ROWS.
    fn queue_lines(&self, w: usize, h: usize) -> Vec<String> {
        let room = h.saturating_sub(self.input_height(w) + 3 + self.popup_h() + MIN_VIEW_ROWS);
        let max_show = MAX_QUEUE_ROWS.min(room);
        if max_show == 0 {
            return Vec::new();
        }
        let width = text_width(w);
        let total = self.queue_texts.len();
        let mut out = Vec::new();
        for (i, q) in self.queue_texts.iter().enumerate() {
            if i + 1 == max_show && total > max_show {
                out.push(format!("{GREY_ON}  … +{} 대기{FG_OFF}", total - i));
                break;
            }
            let text = format!("  대기 {}", q.replace('\n', " "));
            out.push(format!("{GREY_ON}{}{FG_OFF}", bar::clip(&text, width)));
        }
        out
    }

    // Go panelLines / bgLines: the foreground agent rows (below the input box) and the background rows under
    // them. Only running agents show; the panel above takes its rows out of the room the background panel gets.
    // Go runningN (tui.go:1445): running agents plus busy resident sessions.
    fn running_n(&self) -> usize {
        self.agent_rows.iter().filter(|r| !r.done).count() + self.resident_busy.len()
    }

    fn panel_lines(&self, w: usize, h: usize) -> (Vec<String>, Vec<String>) {
        let queue_h = self.queue_lines(w, h).len();
        let fg = self.agent_lines(false, w, h, queue_h, 0);
        let bg = self.agent_lines(true, w, h, queue_h, fg.len());
        (fg, bg)
    }

    // Go rowsA (tui.go:1448-1520): the agent rows of one kind that fit the panel, as (shown records, offset of the
    // first shown record, length of the whole list). The length is 0 when the panel is not drawn. When the list does
    // not fit, the rows start at `pan_off` (shared by both panels, Go panOff) and one indicator row follows.
    fn agent_window(
        &self,
        bg: bool,
        w: usize,
        h: usize,
        queue_h: usize,
        used: usize,
    ) -> (Vec<&AgentRow>, usize, usize) {
        // Go panel order: running agents, then busy residents (foreground only), then sortedRestored (Go rowsA).
        let list: Vec<&AgentRow> = self
            .agent_rows
            .iter()
            .filter(|r| r.background == bg && !r.done)
            .chain(self.resident_busy.iter().filter(|_| !bg))
            .chain(self.restored_rows.iter().filter(|r| r.background == bg))
            .collect();
        if list.is_empty() || w < 10 {
            return (Vec::new(), 0, 0);
        }
        let room = h.saturating_sub(
            self.input_height(w) + 3 + queue_h + self.popup_h() + MIN_VIEW_ROWS + used,
        );
        let limit = MAX_PANEL_ROWS.min(room);
        if limit == 0 {
            return (Vec::new(), 0, 0);
        }
        // limit-1 agent rows and one indicator row when the list does not fit (Go rowsA).
        let total = list.len();
        let (off, shown) = if total > limit {
            let n = limit - 1;
            (self.pan_off.min(total - n), n)
        } else {
            (0, total)
        };
        let window = list.into_iter().skip(off).take(shown).collect();
        (window, off, total)
    }

    // One line per agent row of one kind (agent_window), then the "… a-b/n" indicator when the list does not fit.
    fn agent_lines(
        &self,
        bg: bool,
        w: usize,
        h: usize,
        queue_h: usize,
        used: usize,
    ) -> Vec<String> {
        let (shown, off, total) = self.agent_window(bg, w, h, queue_h, used);
        if total == 0 {
            return Vec::new();
        }
        let width = text_width(w);
        let mut out: Vec<String> = shown
            .iter()
            .map(|r| agent_row_line(r, &self.bar.model, width))
            .collect();
        if total > shown.len() {
            out.push(format!(
                "{GREY_ON}{}{FG_OFF}",
                bar::scroll_range_text(off, shown.len(), total)
            ));
        }
        out
    }

    // Go panelClick (agentview.go:160): the agent on the panel row at screen `row`. Rows are foreground then
    // background, as panel_lines draws them; the scroll indicator row and other rows give None.
    fn panel_agent_at(&self, row: usize) -> Option<String> {
        let (fg, bg) = self.panel_lines(self.cols, self.rows);
        let panel_rows = fg.len() + bg.len();
        let start = self.rows.saturating_sub(panel_rows + 1);
        let i = row.checked_sub(start).filter(|i| *i < panel_rows)?;
        let queue_h = self.queue_lines(self.cols, self.rows).len();
        let mut slots: Vec<Option<&str>> = Vec::new();
        for is_bg in [false, true] {
            let used = if is_bg { fg.len() } else { 0 };
            let (shown, _, total) = self.agent_window(is_bg, self.cols, self.rows, queue_h, used);
            slots.extend(shown.iter().map(|r| Some(r.id.as_str())));
            if total > shown.len() {
                slots.push(None);
            }
        }
        slots.get(i).copied().flatten().map(str::to_string)
    }

    // Go click (agentview.go:172): a left press on an agent panel row opens that agent's view. The open usage,
    // permission, picker, rewind and session overlays keep the press (Go click guard). False when nothing opened.
    fn panel_click(&mut self, row: u16) -> bool {
        if self.usage.is_open()
            || self.perm.is_some()
            || self.pick.is_some()
            || self.recs.is_some()
            || self.rw.is_open()
            || self.sess.is_some()
        {
            return false;
        }
        match self.panel_agent_at(row as usize) {
            Some(id) => {
                self.av = Some(agentview::AgentView::open(&id));
                true
            }
            None => false,
        }
    }

    // Go popH: the slash menu rows (at most maxPop = 8) that take room from the queue and the panels.
    fn popup_h(&self) -> usize {
        self.suggest().len().min(overlay::SLASH_MAX_ROWS)
    }

    // Rows of the transcript viewport on a `w` x `h` screen: what is left after the chrome and the slash menu.
    fn body_h(&self, w: usize, h: usize) -> usize {
        let chrome_h = self.chrome(w, h).len();
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

    // Go tui.go MouseMsg order: the open settings picker, /resume pickers and /view list take the wheel as
    // wheelSel; the agent view scrolls its log by 3. Returns false when none of them is open.
    fn overlay_wheel(&mut self, up: bool, row: usize) -> bool {
        let (sw, sh) = (text_width(self.cols), self.body_h(self.cols, self.rows));
        if let Some(st) = self.settings.as_mut() {
            st.wheel(up);
            return true;
        }
        if let Some(sv) = self.sess.as_mut() {
            sv.wheel(up, sw, sh);
            return true;
        }
        if let Some(rp) = self.recs.as_mut() {
            rp.wheel(up);
            return true;
        }
        if let Some(gp) = self.pick.as_mut() {
            gp.wheel(up);
            return true;
        }
        // Go panelWheel: only with no agent view and no permission prompt open, and only over the panels.
        if self.av.is_none() && self.perm.is_none() && self.panel_wheel(up, row) {
            return true;
        }
        if let Some(av) = self.av.as_mut() {
            av.wheel(up);
            return true;
        }
        false
    }

    // Go panelWheel: moves the panel window one agent row per wheel notch, clamped to the running agents.
    // Returns false when no panel is shown or the row is above the panels.
    fn panel_wheel(&mut self, up: bool, row: usize) -> bool {
        let (fg, bg) = self.panel_lines(self.cols, self.rows);
        let panel_rows = fg.len() + bg.len();
        if panel_rows == 0 || row < self.rows.saturating_sub(panel_rows + 1) {
            return false;
        }
        let total = (self.agent_rows.iter().filter(|r| !r.done).count() + self.restored_rows.len())
            as isize;
        let d: isize = if up { -1 } else { 1 };
        self.pan_off = (self.pan_off as isize + d).clamp(0, total) as usize;
        true
    }

    // Moves the viewport by `delta` rows. Landing on the last page follows the tail again.
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
            .map(|(_, d, t, _)| (d.clone(), self.clock.duration_since(*t).as_secs()))
            .collect();
        b.bg = self
            .agents
            .iter()
            .filter(|(id, _, _, _)| self.bg_agents.contains(id))
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
        self.chrome(self.cols, self.rows).len()
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
                out.push(side(&format!(
                    "{pre}{}",
                    render::input_view_row(row, cursor)
                )));
            }
        }
        out.push(edge("╰", "╯"));
        out
    }
}

// Cache key of one transcript row: its text, the width it wraps at and whether its links are styled.
fn wrap_key(line: &str, width: usize, linked: bool) -> u64 {
    let mut h = DefaultHasher::new();
    (line, width, linked).hash(&mut h);
    h.finish()
}

// Go indent (tui.go:353-378): the text after a block's lead is wrapped at width - hang, and every row after the
// first is indented by the hang, so continuation rows line up under the text. Returns the byte index where the
// text starts and the hang in cells. Plain notes (no lead) get no hang, as Go's note row has none.
// Linked rows are assistant and user blocks, whose continuation lead is "  "; tool and result rows are not linked.
fn hang_split(line: &str, linked: bool) -> (usize, usize) {
    let mut i = 0;
    while let Some(n) = render::csi_len(line.as_bytes(), i) {
        i += n;
    }
    let leads: &[(&str, usize)] = if linked {
        &[("● ", 2), ("> ", 2), ("  ⎿  ", 5), ("  ", 2)]
    } else {
        &[("● ", 2), ("  ⎿  ", 5)]
    };
    for (lead, hang) in leads {
        if line[i..].starts_with(lead) {
            return (i + lead.len(), *hang);
        }
    }
    (0, 0)
}

// One logical row wrapped to `w` cells, split into screen rows. Linked rows get link styles (wrapLinked).
fn wrap_row(line: &str, w: usize, linked: bool) -> Vec<String> {
    let (at, hang) = hang_split(line, linked);
    // A pane too narrow for the lead plus two text cells wraps without the hang, as before.
    let (at, hang) = if w >= hang + 2 { (at, hang) } else { (0, 0) };
    let (head, text) = line.split_at(at);
    let tw = w - hang;
    let wrapped = if linked {
        render::wrap_linked(text, |s| render::wrap_cells(s, tw))
    } else {
        render::wrap_cells(text, tw)
    };
    let pad = " ".repeat(hang);
    wrapped
        .split('\n')
        .enumerate()
        .map(|(i, row)| {
            if i == 0 {
                format!("{head}{row}")
            } else {
                format!("{pad}{row}")
            }
        })
        .collect()
}

// Go stage lines (repl.go stage calls): "· <slot> 처리중" on start, "· <slot> 완료" on success,
// "· <slot> 실패: <err>" on failure. Agent slots use Go's "호출" on start and "agent <label>" as the name,
// where the label is Go subagentLabel (subagent_type, else "agent").
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

// Copies `text` like Go copyToClipboard (selection.go:115-120): the native copy runs first, then OSC 52 is written
// to the terminal (reaches the local clipboard over SSH) even when the native copy failed. The error is the native copy's;
// elsewhere only OSC 52 is written, so success there is unverified. `native` is the injected clipboard write (native_copy in the loop).
fn copy_text(
    out: &mut impl Write,
    text: &str,
    native: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    let result = native(text);
    let osc = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
    let _ = out.write_all(osc.as_bytes()).and_then(|()| out.flush());
    result
}

// Left release over a selection: copies the selected text and records the note (Go selectMouse release + copyNoteMsg).
fn release_copy(
    app: &mut App,
    out: &mut impl Write,
    native: impl FnOnce(&str) -> Result<(), String>,
    x: u16,
    y: u16,
) {
    if let Some(text) = app.select_release(x, y) {
        let result = copy_text(out, &text, native);
        app.copy_done(text.chars().count(), result);
    }
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

// Off macOS, the first clipboard tool found on PATH: wl-copy under WAYLAND_DISPLAY, then xclip, xsel, clip.exe.
#[cfg(not(target_os = "macos"))]
fn native_copy(text: &str) -> Result<(), String> {
    use chaude_core::osenv;
    use std::process::{Command, Stdio};
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
    let dirs = osenv::path_dirs();
    let Some((tool, args)) = clipboard_tool(wayland, |p| osenv::find_on_path(p, &dirs).is_some())
    else {
        return Err("no clipboard tool (wl-copy, xclip, xsel, clip.exe)".to_string());
    };
    let mut child = Command::new(tool)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&clipboard_payload(tool, text))
            .map_err(|e| e.to_string())?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{tool} {status}"))
    }
}

// Tool choice as atotto/clipboard makes it (clipboard_unix.go init).
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn clipboard_tool(
    wayland: bool,
    has: impl Fn(&str) -> bool,
) -> Option<(&'static str, &'static [&'static str])> {
    let mut tools: Vec<(&'static str, &'static [&'static str])> = Vec::new();
    if wayland {
        tools.push(("wl-copy", &[]));
    }
    tools.push(("xclip", &["-in", "-selection", "clipboard"]));
    tools.push(("xsel", &["--input", "--clipboard"]));
    tools.push(("clip.exe", &[]));
    tools.into_iter().find(|(p, _)| has(p))
}

// clip.exe reads UTF-16LE with a BOM, so Korean survives; the other tools take UTF-8.
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn clipboard_payload(tool: &str, text: &str) -> Vec<u8> {
    if tool == "clip.exe" {
        std::iter::once(0xFEFF_u16)
            .chain(text.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect()
    } else {
        text.as_bytes().to_vec()
    }
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

// Go trunc (repl.go:217): the first `n` runes of `s`.
fn trunc_runes(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

// Columns usable for text. The last column stays empty so a full row never auto-wraps.
fn text_width(w: usize) -> usize {
    w.saturating_sub(1).max(1)
}

// One agent panel row (Go rowsA): "  ● 상태-모델-이름(작업)" with the dot in Go's running colour (220). Only running
// agents reach the panel, so the state is always 실행 중. The model falls back to the session model; the summary is
// the task, else the first prompt line.
fn agent_row_line(r: &AgentRow, session_model: &str, width: usize) -> String {
    let summary = if r.task.is_empty() {
        &r.prompt
    } else {
        &r.task
    };
    // Finished rows here are restored ones (AgentsRestored), which were interrupted.
    let state = if r.done {
        bar::AgentState::Interrupted
    } else {
        bar::AgentState::Running
    };
    let text = bar::agent_status_text(state, &r.model, session_model, &r.name, summary);
    let dot = "\x1b[38;5;220m●\x1b[39m";
    format!(
        "  {dot}{GREY_ON} {}{FG_OFF}",
        bar::clip(&text, width.saturating_sub(4))
    )
}

// The whole screen as `h` rows of at most `w` columns. Pure: the same App and size give the same rows.
#[cfg(test)]
pub fn frame(app: &App, w: usize, h: usize) -> Vec<String> {
    layout(app, w, h).0
}

// The rows of `frame`, and how many of the top rows are the body (the part that scrolls with new output).
fn layout(app: &App, w: usize, h: usize) -> (Vec<String>, usize) {
    // Go View before the first window size (tui.go:1229): nothing can be laid out yet.
    if w == 0 || h == 0 {
        return (vec!["starting…".to_string()], 0);
    }
    let width = text_width(w);
    let chrome = app.chrome(w, h);
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
    (rows, body_h)
}

// Terminal cell that holds the cursor: inside the input box (after its left border), on the input row.
fn cursor(app: &App, w: usize, h: usize) -> (u16, u16) {
    let inner = w.saturating_sub(2).max(1);
    let pw = UnicodeWidthStr::width(PROMPT);
    let (rows, cur_row, cur_col) = app.input_rows(inner.saturating_sub(pw));
    let (top, shown) = input_window(rows.len(), cur_row);
    // The input rows sit above the box's bottom border, and the agent rows (below the box) and the status row.
    let (fg, bg) = app.panel_lines(w, h);
    let below = fg.len() + bg.len() + 2;
    let y = h.saturating_sub(below + shown) + (cur_row - top);
    let x = (1 + pw + cur_col) as i64 + app.ime_shift as i64;
    (x.max(0) as u16, y as u16)
}

// imeColShift (tui.go:177): JediTerm draws uncommitted IME text one cell right, so the cursor parks one cell left.
// CHAUDE_IME_SHIFT=<int> overrides; otherwise TERMINAL_EMULATOR=JetBrains-JediTerm gives -1.
fn ime_col_shift(ime: Option<String>, emulator: Option<String>) -> i32 {
    if let Some(n) = ime.and_then(|v| v.trim().parse::<i32>().ok()) {
        return n;
    }
    if emulator.as_deref() == Some("JetBrains-JediTerm") {
        return -1;
    }
    0
}

// How far the body's rows moved since the last draw, so the terminal can scroll them instead of repainting:
// k > 0 means the content moved up k rows (new lines at the tail), k < 0 means it moved down. 0 when no clean shift.
fn body_shift(prev: &[String], rows: &[String], body_h: usize) -> isize {
    const MAX: usize = 8;
    if body_h < 2 || prev.len() != rows.len() || prev[..body_h] == rows[..body_h] {
        return 0;
    }
    for k in 1..=MAX.min(body_h - 1) {
        if prev[k..body_h] == rows[..body_h - k] {
            return k as isize;
        }
        if prev[..body_h - k] == rows[k..body_h] {
            return -(k as isize);
        }
    }
    0
}

// Writes only what changed since `prev` (the rows last drawn), then parks the cursor on the input line.
// A body that only shifted (new output at the tail, or a scroll) is moved with the terminal's scroll region,
// so only the rows that really changed are written. Nothing is cleared on a normal update.
pub fn draw(out: &mut impl Write, app: &App, prev: &mut Vec<String>) -> io::Result<()> {
    let (rows, body_h) = layout(app, app.cols, app.rows);
    let shift = body_shift(prev, &rows, body_h);
    let mut moved;
    let old: &[String] = if shift == 0 {
        prev
    } else {
        // Mirror the terminal after the scroll: rows that scrolled in are blank until repainted.
        write!(out, "\x1b[1;{body_h}r")?;
        if shift > 0 {
            queue!(out, ScrollUp(shift as u16))?;
        } else {
            queue!(out, ScrollDown((-shift) as u16))?;
        }
        write!(out, "\x1b[r")?;
        moved = prev.clone();
        if shift > 0 {
            let k = shift as usize;
            moved[..body_h].rotate_left(k);
            moved[body_h - k..body_h].iter_mut().for_each(String::clear);
        } else {
            let k = (-shift) as usize;
            moved[..body_h].rotate_right(k);
            moved[..k].iter_mut().for_each(String::clear);
        }
        &moved
    };
    for (i, row) in rows.iter().enumerate() {
        if old.get(i) != Some(row) {
            queue!(out, MoveTo(0, i as u16), Clear(ClearType::CurrentLine))?;
            write!(out, "{row}")?;
        }
    }
    let (x, y) = cursor(app, app.cols, app.rows);
    queue!(out, MoveTo(x, y))?;
    if app.hide_cursor {
        queue!(out, Hide)?;
    } else {
        queue!(out, Show)?;
    }
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

// Assistant text as Go shows it: **bold** pairs here; links are styled after wrapping in body() (tui.go:374).
const IMPORT_INSTALL_NOTE: &str =
    "import: the next prompt starts from this transcript (Go applyImport)";

fn asst_text(line: &str) -> String {
    render::md_bold(line)
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
        MouseCellMotion(false),
        DisableBracketedPaste,
        LeaveAlternateScreen,
        Show
    );
    let _ = terminal::disable_raw_mode();
}

// Go tui.go:1394 WithMouseCellMotion: button and drag motion (?1002) with SGR coordinates (?1006) only.
// crossterm's EnableMouseCapture also turns on any-motion (?1003), so the sequences are written here.
struct MouseCellMotion(bool);

impl crossterm::Command for MouseCellMotion {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        let k = if self.0 { 'h' } else { 'l' };
        write!(f, "\x1b[?1002{k}\x1b[?1006{k}")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        if self.0 {
            crossterm::Command::execute_winapi(&crossterm::event::EnableMouseCapture)
        } else {
            crossterm::Command::execute_winapi(&crossterm::event::DisableMouseCapture)
        }
    }
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
            MouseCellMotion(true)
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

// Go repl.go:1364-1368 and 1270-1273: a failed claude start prints one line to stderr and exits 1.
fn start_failed(err: &str) -> ! {
    eprintln!("chaude: cannot start claude: {err}");
    std::process::exit(1)
}

// Entry point for `chaude` with no arguments. Non-TTY stdin falls back to plain line mode.
pub fn run(cfg: Config, startup: Option<ClaudeImport>) -> io::Result<()> {
    // Go repl.go: the TUI needs an interactive stdin and HARNESS_PLAIN unset (empty counts as unset).
    let plain_forced = !std::env::var("HARNESS_PLAIN")
        .unwrap_or_default()
        .is_empty();
    if stdin_is_char_device() && !plain_forced {
        tui(cfg, startup)
    } else {
        plain(cfg, startup)
    }
}

// Go repl.go: interactive when stdin is a character device (st.Mode()&os.ModeCharDevice), so /dev/null counts.
#[cfg(unix)]
fn stdin_is_char_device() -> bool {
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::io::FromRawFd;
    // SAFETY: fd 0 belongs to the process; ManuallyDrop keeps this temporary File from closing it.
    let stdin = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(0) });
    stdin
        .metadata()
        .map(|m| m.file_type().is_char_device())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn stdin_is_char_device() -> bool {
    std::io::IsTerminal::is_terminal(&io::stdin())
}

// Everything the TUI loop waits on: a terminal event, an engine event, or the end of either source.
enum Wake {
    Term(Event),
    Engine(UiEvent),
    // Result of an off-thread job (/update), shown as a note.
    Note(String),
    // SIGINT or SIGTERM (see forward_signals): interrupts a running turn, ignored when idle (Go repl.go).
    Signal,
    Closed,
}

// Go repl.go: SIGINT and SIGTERM are forwarded to the interrupt path instead of ending the process.
// The C handler only writes a byte to a pipe; a thread turns each byte into a Wake::Signal.
#[cfg(unix)]
static SIGNAL_PIPE_W: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

#[cfg(unix)]
extern "C" fn on_signal(_sig: libc::c_int) {
    let byte = 1u8;
    let fd = SIGNAL_PIPE_W.load(std::sync::atomic::Ordering::Relaxed);
    // SAFETY: write(2) is async-signal-safe; fd is the pipe write end set before the handler was installed.
    unsafe {
        libc::write(fd, (&byte as *const u8).cast(), 1);
    }
}

#[cfg(unix)]
fn forward_signals(tx: mpsc::Sender<Wake>) -> io::Result<()> {
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: fds has room for the two descriptors pipe(2) writes.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    for fd in fds {
        // SAFETY: fd is a valid descriptor just returned by pipe(2). Keeps child processes from inheriting it.
        unsafe {
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
        }
    }
    SIGNAL_PIPE_W.store(fds[1], std::sync::atomic::Ordering::Relaxed);
    let handler: extern "C" fn(libc::c_int) = on_signal;
    for sig in [libc::SIGINT, libc::SIGTERM] {
        // SAFETY: on_signal is an extern "C" fn that only calls async-signal-safe functions.
        unsafe {
            libc::signal(sig, handler as libc::sighandler_t);
        }
    }
    let rd = fds[0];
    thread::spawn(move || {
        let mut byte = 0u8;
        loop {
            // SAFETY: reads one byte into a valid stack buffer from the pipe read end.
            let n = unsafe { libc::read(rd, (&mut byte as *mut u8).cast(), 1) };
            if n == 1 {
                if tx.send(Wake::Signal).is_err() {
                    return;
                }
            } else if io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                return;
            }
        }
    });
    Ok(())
}

#[cfg(not(unix))]
fn forward_signals(_tx: mpsc::Sender<Wake>) -> io::Result<()> {
    Ok(())
}

// Blocks until a wake arrives (or `timeout` passes), then takes every queued wake too.
// Returns None when a source has closed and the UI should end.
fn wait_batch(wakes: &Receiver<Wake>, timeout: Option<Duration>) -> Option<Vec<Wake>> {
    let first = match timeout {
        Some(t) => match wakes.recv_timeout(t) {
            Ok(wake) => Some(wake),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return None,
        },
        None => Some(wakes.recv().ok()?),
    };
    let mut batch = Vec::new();
    for wake in first.into_iter().chain(wakes.try_iter()) {
        if matches!(wake, Wake::Closed) {
            return None;
        }
        batch.push(wake);
    }
    Some(batch)
}

// Which mode the previous session loads with, as core decides it at start (handoff::resume_mode, same inputs).
// None when there is no previous run, or when --resume was given explicitly (core takes that one as is).
fn session_mode_note(
    handoff_path: Option<&std::path::Path>,
    model: &str,
    rotate: u64,
    explicit: bool,
) -> Option<String> {
    if explicit {
        return None;
    }
    let (prev, killed) = handoff::previous_run(handoff_path?)?;
    let threshold = chaude_core::rotate::threshold_for(model, rotate);
    Some(
        match handoff::resume_mode(&prev, handoff::MAIN_ROLE, killed, threshold) {
            handoff::Load::Resume(sid) if killed => {
                format!("previous session: resume claude --resume {sid} (last run was killed)")
            }
            handoff::Load::Resume(sid) => {
                format!(
                    "previous session: resume claude --resume {sid} (context below rotation threshold)"
                )
            }
            handoff::Load::Fresh => {
                "previous session: fresh start, handoff note carried over".to_string()
            }
        },
    )
}

fn tui(mut cfg: Config, startup: Option<ClaudeImport>) -> io::Result<()> {
    // Go writes the exit handoff into the record folder on quit; /view reads it back (prev_owned).
    if let (None, Ok(cwd)) = (&cfg.handoff_path, std::env::current_dir()) {
        cfg.handoff_path = Some(handoff::handoff_file(&chaude_core::record::dir_for(&cwd)));
    }
    let model = settings::Settings {
        main_model: cfg.model.clone(),
        ..settings::Settings::default()
    }
    .model_id()
    .to_string();
    let threshold = cfg.context_window;
    let session_mode = session_mode_note(
        cfg.handoff_path.as_deref(),
        &cfg.model,
        cfg.rotate_threshold,
        cfg.resume.is_some(),
    );
    let (engine, rx) = Engine::try_start(cfg).unwrap_or_else(|e| start_failed(&e));
    let _guard = TermGuard::enter()?;
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let mut app = App::new(cols as usize, rows as usize);
    app.set_agent_source(engine.agent_rows());
    app.ime_shift = ime_col_shift(
        std::env::var("CHAUDE_IME_SHIFT").ok(),
        std::env::var("TERMINAL_EMULATOR").ok(),
    );
    app.hide_cursor =
        std::env::var("TERMINAL_EMULATOR").ok().as_deref() == Some("JetBrains-JediTerm");
    if let Some(imp) = startup {
        app.show_import(&imp);
        engine.install_import(imp);
    }
    app.welcome.model = model;
    app.welcome.threshold = threshold;
    if let Some(mode) = &session_mode {
        app.note(mode);
    }
    app.session_mode = session_mode;
    if let Ok(cwd) = std::env::current_dir() {
        app.editor.load_history(&cwd);
    }
    let mut out = io::stdout();
    let mut prev = Vec::new();
    // The guard turned mouse capture on; this tracks whether it is still on.
    let mut captured = true;
    // Terminal input and engine output feed one channel, so the loop blocks on a single receiver.
    let (wake_tx, wakes) = mpsc::channel::<Wake>();
    {
        let tx = wake_tx.clone();
        thread::spawn(move || {
            while let Ok(ev) = event::read() {
                if tx.send(Wake::Term(ev)).is_err() {
                    return;
                }
            }
            let _ = tx.send(Wake::Closed);
        });
    }
    forward_signals(wake_tx.clone())?;
    let update_tx = wake_tx.clone();
    thread::spawn(move || {
        for ev in rx {
            if wake_tx.send(Wake::Engine(ev)).is_err() {
                return;
            }
        }
        let _ = wake_tx.send(Wake::Closed);
    });
    draw(&mut out, &app, &mut prev)?;
    // dirty: state changed since the last frame. A dirty loop waits only until the next frame is due.
    let mut dirty = false;
    let mut last_draw = Instant::now();
    'main: loop {
        // Dirty: wait until the next frame is due (a key or output in between still wakes it). Idle: sleep
        // until input or output arrives. Animating: also wake every TICK for the spinner and clock.
        let timeout = if dirty {
            Some((last_draw + FRAME).saturating_duration_since(Instant::now()))
        } else {
            app.animating().then_some(TICK)
        };
        let Some(batch) = wait_batch(&wakes, timeout) else {
            break;
        };
        dirty = true;
        // Every queued wake is applied before one tick and one draw, so a burst of output costs one frame.
        for wake in batch {
            match wake {
                Wake::Engine(ev) => app.apply(ev),
                Wake::Note(text) => app.note(&text),
                Wake::Signal => {
                    if app.busy {
                        engine.interrupt(InterruptSource::Signal);
                    }
                }
                Wake::Term(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    match app.on_key(key) {
                        Action::Continue => {}
                        Action::Submit(text) => engine.submit(&text),
                        Action::Interrupt(src) => {
                            app.finish_foreground();
                            engine.interrupt(src);
                        }
                        Action::Quit => break 'main,
                        Action::Usage => engine.usage(),
                        Action::Update => {
                            let tx = update_tx.clone();
                            thread::spawn(move || {
                                let _ = tx.send(Wake::Note(chaude_update::text(env!(
                                    "CARGO_PKG_VERSION"
                                ))));
                            });
                        }
                        Action::Answer {
                            id,
                            allow,
                            interrupt,
                        } => {
                            engine.answer_permission(&id, allow);
                            if interrupt {
                                app.finish_foreground();
                                engine.interrupt(InterruptSource::CtrlC);
                            }
                        }
                        Action::CycleMode => engine.set_mode("next"),
                        Action::SetMode(p) => engine.set_mode(p.name()),
                    }
                    if let Some(imp) = app.import_install.take() {
                        engine.install_import(imp);
                    }
                }
                Wake::Term(Event::Paste(text)) => app.editor.insert_str(&text),
                Wake::Term(Event::Mouse(m)) if app.mouse => match m.kind {
                    MouseEventKind::ScrollUp if app.overlay_wheel(true, m.row as usize) => {}
                    MouseEventKind::ScrollDown if app.overlay_wheel(false, m.row as usize) => {}
                    MouseEventKind::ScrollUp => app.scroll(-WHEEL_LINES),
                    MouseEventKind::ScrollDown => app.scroll(WHEEL_LINES),
                    MouseEventKind::Down(MouseButton::Left) => {
                        if !app.panel_click(m.row) {
                            app.select_press(m.column, m.row);
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left) => app.select_drag(m.column, m.row),
                    MouseEventKind::Up(MouseButton::Left) => {
                        release_copy(&mut app, &mut out, native_copy, m.column, m.row);
                    }
                    _ => {}
                },
                Wake::Term(Event::Resize(c, r)) => {
                    app.cols = c as usize;
                    app.rows = r as usize;
                    queue!(out, Clear(ClearType::All))?;
                    prev.clear();
                }
                Wake::Term(_) | Wake::Closed => {}
            }
        }
        let now = Instant::now();
        app.tick(now);
        if app.mouse != captured {
            if app.mouse {
                execute!(out, MouseCellMotion(true))?;
            } else {
                execute!(out, MouseCellMotion(false))?;
            }
            captured = app.mouse;
        }
        // Keys and output above are applied before this check, so a frame never shows stale state.
        if now.saturating_duration_since(last_draw) >= FRAME {
            draw(&mut out, &app, &mut prev)?;
            last_draw = now;
            dirty = false;
        }
    }
    engine.shutdown();
    Ok(())
}

// Go repl.go loop (1200-1262): one line at a time, answered before the next line is read. Local commands run here
// through commands::dispatch; the rest goes to the engine as a turn. "> " + line is echoed when stdin is not a char
// device (Go !interactive); /quit, /rotate and /mode are answered before that echo, as in Go.
fn plain(mut cfg: Config, startup: Option<ClaudeImport>) -> io::Result<()> {
    // Go r.ui == nil: no UI answers permission asks, so idle asks are denied by the engine.
    cfg.no_ui = true;
    let interactive = stdin_is_char_device();
    let model = cfg.model.clone();
    let threshold = cfg.rotate_threshold;
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let (engine, rx) = Engine::try_start(cfg).unwrap_or_else(|e| start_failed(&e));
    let mut app = App::new(80, 24);
    app.set_agent_source(engine.agent_rows());
    if let Some(imp) = startup {
        app.show_import(&imp);
        for l in &app.transcript {
            println!("{l}");
        }
        engine.install_import(imp);
    }
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let quiet =
            line == "/quit" || line == "/exit" || line == "/rotate" || line.starts_with("/mode");
        if !interactive && !quiet {
            println!("> {line}");
        }
        let ctx = Ctx {
            model: &model,
            last_ctx: 0,
            threshold,
            rotations: 0,
            session_id: None,
            mode: PermMode::Default,
            chaude_version: env!("CARGO_PKG_VERSION"),
            busy: false,
        };
        match commands::dispatch(&line, &ctx) {
            Some(Dispatch::Quit) => break,
            Some(Dispatch::Note(s)) => println!("{s}"),
            Some(Dispatch::SetMode(_)) => {
                let arg = line.split_whitespace().nth(1).unwrap_or("");
                engine.set_mode(arg);
                pump(&rx, &mut app, false);
            }
            Some(Dispatch::Import { arg, .. }) => {
                match import_claude::load_import_arg(&arg, &cwd.display().to_string()) {
                    Err(e) => println!("{e}"),
                    Ok((_, imp)) => {
                        app.show_import(&imp);
                        for l in &app.transcript {
                            println!("{l}");
                        }
                        engine.install_import(imp);
                    }
                }
            }
            Some(Dispatch::Resume { args }) => {
                // Go resumeLine: the record path only; the TUI picker for several matches is not in plain mode.
                let arg = args.join(" ");
                let root = chaude_core::record::dir_for(&cwd);
                match chaude_core::record_turn::resume(&root, &arg, &records::list_date) {
                    Err(e) => println!("{e}"),
                    Ok((lines, entries)) => {
                        for l in &lines {
                            println!("{l}");
                        }
                        for e in &entries {
                            println!("{}", e.text);
                        }
                    }
                }
            }
            Some(Dispatch::Update) => {
                println!("{}", chaude_update::text(env!("CARGO_PKG_VERSION")))
            }
            Some(Dispatch::OpenUsage) => {
                engine.usage();
                pump(&rx, &mut app, false);
            }
            // Go plain has no views or mouse: such a line is sent to the engine as a turn (Go handle(line)).
            Some(Dispatch::OpenView { .. } | Dispatch::ToggleMouse | Dispatch::OpenSettings)
            | None => {
                engine.submit(&line);
                pump(&rx, &mut app, true);
            }
            Some(
                Dispatch::Rotate { .. }
                | Dispatch::Clear
                | Dispatch::Model { .. }
                | Dispatch::Passthrough { .. },
            ) => {
                engine.submit(&line);
                pump(&rx, &mut app, true);
            }
        }
    }
    engine.shutdown();
    Ok(())
}

// Prints the engine events of one line. A turn waits for Busy(false); other calls only drain what is queued.
fn pump(rx: &Receiver<UiEvent>, app: &mut App, turn: bool) {
    loop {
        let ev = if turn {
            match rx.recv() {
                Ok(ev) => ev,
                Err(_) => return,
            }
        } else {
            match rx.try_recv() {
                Ok(ev) => ev,
                Err(_) => return,
            }
        };
        let finished = ev == UiEvent::Busy(false);
        let before = app.transcript.len();
        app.apply(ev);
        for l in &app.transcript[before..] {
            println!("{l}");
        }
        if turn && finished {
            return;
        }
    }
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
        assert!(matches!(
            app.on_key(key(KeyCode::Esc, KeyModifiers::NONE)),
            Action::Continue
        ));
        assert!(matches!(
            app.on_key(key(KeyCode::Esc, KeyModifiers::NONE)),
            Action::Continue
        ));
        assert!(app.rw.is_open());
        // The picker starts on the newest prompt; step back to the first one, then confirm.
        app.on_key(key(KeyCode::Up, KeyModifiers::NONE));
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.rw.is_open());
        assert_eq!(app.prompts.len(), 0);
        // Only the rewound note remains; the cut turns are gone.
        assert!(
            app.transcript
                .iter()
                .all(|l| !l.starts_with("> ") && !l.starts_with("answer"))
        );
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

    // JetBrains-JediTerm: the hardware cursor stays hidden but is still parked on the input cell for IME.
    #[test]
    fn draw_hides_the_real_cursor_under_jediterm_and_still_parks_it() {
        let mut app = App::new(30, 10);
        app.hide_cursor = true;
        type_str(&mut app, "ab");
        let mut buf = Vec::new();
        draw(&mut buf, &app, &mut Vec::new()).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.ends_with("\x1b[?25l"), "{out:?}");
        assert!(!out.contains("\x1b[?25h"), "{out:?}");
        assert!(out.contains("H\x1b[?25l"), "{out:?}");
    }

    fn agent_row(id: &str, background: bool, done: bool) -> AgentRow {
        AgentRow {
            id: id.into(),
            task: format!("task {id}"),
            prompt: "first line".into(),
            name: "scout".into(),
            model: "haiku-4".into(),
            background,
            done,
            agent_id: String::new(),
            result_path: String::new(),
            started_at: 0,
        }
    }

    // The frame with SGR codes removed, one String per screen row.
    fn plain_frame(app: &App, w: usize, h: usize) -> Vec<String> {
        frame(app, w, h)
            .iter()
            .map(|r| render::strip_sgr(r))
            .collect()
    }

    // Index of the input box's top border: the last ╭ row, below the welcome box.
    fn box_top(rows: &[String]) -> usize {
        rows.iter().rposition(|r| r.starts_with('╭')).unwrap()
    }

    // Queue rows are the ones that start at the left edge with "  대기" or "  …"; the input placeholder sits in a box.
    fn queue_rows(rows: &[String]) -> Vec<&String> {
        rows.iter()
            .filter(|r| r.starts_with("  대기") || r.starts_with("  … +"))
            .collect()
    }

    #[test]
    fn queue_rows_sit_dimmed_above_the_input_box() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::QueueTexts(vec!["one".into(), "two\nlines".into()]));
        let raw = frame(&app, 60, 20);
        let rows = plain_frame(&app, 60, 20);
        let top = box_top(&rows);
        assert_eq!(rows[top - 2], "  대기 one");
        assert_eq!(rows[top - 1], "  대기 two lines");
        assert!(raw[top - 1].starts_with(GREY_ON), "{:?}", raw[top - 1]);
    }

    #[test]
    fn queue_overflow_row_counts_the_hidden_prompts() {
        let mut app = App::new(60, 30);
        let texts: Vec<String> = (0..7).map(|i| format!("q{i}")).collect();
        app.apply(UiEvent::QueueTexts(texts));
        let rows = plain_frame(&app, 60, 30);
        let top = box_top(&rows);
        // Go queueLines: four prompts, then "… +3" for the three behind them (indices 4..=6).
        assert_eq!(rows[top - 1], "  … +3 대기");
        assert_eq!(rows[top - 2], "  대기 q3");
        assert_eq!(queue_rows(&rows).len(), 5);
    }

    #[test]
    fn foreground_rows_sit_below_the_box_and_list_running_agents_only() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::AgentRows {
            rows: vec![agent_row("a", false, false), agent_row("d", false, true)],
        });
        let rows = plain_frame(&app, 60, 20);
        let top = box_top(&rows);
        let fg = rows.iter().position(|r| r.contains("실행 중-")).unwrap();
        // Bottom border is top+2 and the status row is last, so the agent row sits between them.
        assert_eq!(fg, top + 3, "{rows:?}");
        assert_eq!(rows[fg], "  ● 실행 중-haiku-4-scout(task a)");
        assert_eq!(rows.iter().filter(|r| r.contains("실행 중-")).count(), 1);
        assert!(!rows.iter().any(|r| r.contains("task d")));
    }

    #[test]
    fn restored_agents_follow_running_rows_as_interrupted_until_cleared() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::AgentRows {
            rows: vec![agent_row("a", false, false)],
        });
        app.apply(UiEvent::AgentsRestored {
            rows: vec![agent_row("r", false, true)],
        });
        let rows = plain_frame(&app, 60, 20);
        let run = rows.iter().position(|r| r.contains("task a")).unwrap();
        let rest = rows.iter().position(|r| r.contains("task r")).unwrap();
        assert_eq!(rest, run + 1, "{rows:?}");
        assert!(rows[rest].contains("중단-"), "{rows:?}");
        app.apply(UiEvent::AgentsRestored { rows: Vec::new() });
        let rows = plain_frame(&app, 60, 20);
        assert!(!rows.iter().any(|r| r.contains("task r")), "{rows:?}");
    }

    #[test]
    fn restored_row_opens_in_the_agent_view() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::AgentsRestored {
            rows: vec![agent_row("r", false, true)],
        });
        let recs = app.agent_recs();
        let rec = recs.iter().find(|r| r.id == "r").expect("restored agent");
        assert!(rec.done && rec.interrupted, "{rec:?}");
        let av = agentview::AgentView::open_first(&recs).expect("restored agent opens");
        assert_eq!(av.id, "r");
    }

    // AgentRow.started_at is Unix seconds (api.rs restored_row); AgentRec.started_at is ms.
    #[test]
    fn restored_row_started_at_converts_seconds_to_ms() {
        let mut app = App::new(80, 24);
        let row = AgentRow {
            started_at: 1_700_000_000,
            ..agent_row("r", false, true)
        };
        app.apply(UiEvent::AgentsRestored { rows: vec![row] });
        let recs = app.agent_recs();
        let rec = recs.iter().find(|r| r.id == "r").expect("restored agent");
        assert_eq!(rec.started_at, 1_700_000_000_000, "{rec:?}");
    }

    #[test]
    fn background_rows_follow_the_foreground_rows() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::AgentRows {
            rows: vec![agent_row("bg", true, false), agent_row("fg", false, false)],
        });
        let rows = plain_frame(&app, 60, 20);
        let fg = rows.iter().position(|r| r.contains("task fg")).unwrap();
        let bg = rows.iter().position(|r| r.contains("task bg")).unwrap();
        assert_eq!(bg, fg + 1, "{rows:?}");
        assert_eq!(rows.len() - 1, bg + 1, "status row ends the screen");
    }

    #[test]
    fn panel_shows_a_scroll_row_when_agents_do_not_fit() {
        let mut app = App::new(60, 20);
        let rows_in: Vec<AgentRow> = (0..5)
            .map(|i| agent_row(&i.to_string(), false, false))
            .collect();
        app.apply(UiEvent::AgentRows { rows: rows_in });
        let rows = plain_frame(&app, 60, 20);
        // Go maxPanel is 4: three agent rows and the "1-3/5" indicator row.
        assert_eq!(rows.iter().filter(|r| r.contains("실행 중-")).count(), 3);
        let more = rows.iter().position(|r| r.contains("1-3/5")).unwrap();
        assert_eq!(rows[more], "  … 1-3/5 (마우스 휠로 스크롤)");
    }

    // Port of TestPanelScrollWindow (panel_test.go:367): the window starts at the scroll offset and clamps at the end.
    #[test]
    fn panel_window_starts_at_scroll_offset_and_clamps() {
        let mut app = App::new(60, 30);
        app.apply(UiEvent::AgentRows {
            rows: (0..12)
                .map(|i| agent_row(&format!("{i:02}"), false, false))
                .collect(),
        });
        app.pan_off = 2;
        let rows = plain_frame(&app, 60, 30);
        assert!(rows.iter().any(|r| r.contains("task 02)")), "{rows:?}");
        assert!(!rows.iter().any(|r| r.contains("task 01)")), "{rows:?}");
        assert!(rows.iter().any(|r| r.contains("3-5/12")), "{rows:?}");
        app.pan_off = 99;
        let rows = plain_frame(&app, 60, 30);
        assert!(rows.iter().any(|r| r.contains("task 09)")), "{rows:?}");
        assert!(rows.iter().any(|r| r.contains("10-12/12")), "{rows:?}");
    }

    // Port of the queue half of TestQueueSmallHeightNoOverflow (panel_test.go:289): seven waiting prompts at small heights.
    #[test]
    fn queue_overflow_never_overflows_small_heights() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::QueueTexts(
            (0..7).map(|i| format!("q{i}")).collect(),
        ));
        for h in [8, 10, 12, 20] {
            assert_eq!(frame(&app, 60, h).len(), h, "h={h}");
        }
    }

    // Port of TestSessionPanelAndThinking (panel_test.go:17), panel part; the thinking row is covered in bar.rs.
    #[test]
    fn session_panel_lists_running_agents_and_clears_when_done() {
        let mut app = App::new(60, 20);
        assert_eq!(app.panel_lines(60, 20), (vec![], vec![]));
        let mut row = agent_row("a1", false, false);
        row.task = "fix bug".into();
        row.prompt = "\nFirst line\nsecond".into();
        app.apply(UiEvent::AgentRows {
            rows: vec![row.clone()],
        });
        let (fg, _) = app.panel_lines(60, 20);
        assert_eq!(fg.len(), 1, "{fg:?}");
        assert!(
            fg[0].contains("fix bug") && !fg[0].contains("second"),
            "{fg:?}"
        );
        row.done = true;
        app.apply(UiEvent::AgentRows { rows: vec![row] });
        assert_eq!(app.panel_lines(60, 20), (vec![], vec![]));
    }

    // Port of TestAgentStatusFormat (panel_test.go:340), panel row part: a background row shows the status text.
    #[test]
    fn background_panel_row_shows_status_text() {
        let mut app = App::new(120, 20);
        let mut row = agent_row("a1", true, false);
        row.task = "버그 수정".into();
        row.name = "fixer".into();
        row.model = "haiku".into();
        app.apply(UiEvent::AgentRows { rows: vec![row] });
        let (_, bg) = app.panel_lines(120, 20);
        assert_eq!(bg.len(), 1, "{bg:?}");
        assert!(bg[0].contains("●"), "{bg:?}");
        assert!(
            render::strip_sgr(&bg[0]).contains("실행 중-haiku-fixer(버그 수정)"),
            "{bg:?}"
        );
    }

    // Port of TestInputWrapWide (input_test.go:30), wide-cell wrap and cursor; the Up-key row move is not covered here.
    #[test]
    fn input_wraps_wide_cells_and_places_cursor() {
        let mut app = App::new(60, 20);
        app.editor.insert_str("가나다라마");
        let (rows, r, c) = app.input_rows(6);
        assert_eq!(rows, vec!["가나다".to_string(), "라마".to_string()]);
        assert_eq!((r, c), (1, 4));
        app.editor.insert_str("x");
        assert_eq!(app.input_rows(6).0.len(), 2);
    }

    #[test]
    fn click_on_a_panel_row_opens_that_agent() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::AgentRows {
            rows: vec![agent_row("a", false, false), agent_row("b", false, false)],
        });
        let (fg, bg) = app.panel_lines(60, 20);
        assert_eq!(fg.len() + bg.len(), 2);
        let first = 20 - 1 - 2;
        // A transcript row and the status row do not open an agent.
        assert!(!app.panel_click(0));
        assert!(!app.panel_click(19));
        assert!(app.av.is_none());
        // The second panel row opens agent "b" (Go panelClick maps the row to its record).
        assert!(app.panel_click(first as u16 + 1));
        assert_eq!(app.av.as_ref().map(|v| v.id.as_str()), Some("b"));
    }

    #[test]
    fn panels_and_queue_never_overflow_small_heights() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::QueueTexts(vec!["a".into(), "b".into()]));
        app.apply(UiEvent::AgentRows {
            rows: (0..5)
                .map(|i| agent_row(&i.to_string(), i % 2 == 0, false))
                .collect(),
        });
        for h in [6, 8, 10, 12, 14, 20] {
            assert_eq!(frame(&app, 60, h).len(), h, "h={h}");
        }
        // Too short for any panel: only the input box and the status row are left.
        let tiny = plain_frame(&app, 60, 6);
        assert!(queue_rows(&tiny).is_empty());
        assert!(!tiny.iter().any(|r| r.contains("실행 중-")));
    }

    // Go panelWheel (tui.go:541-567): a wheel notch over the panels moves the shared panel window one row.
    #[test]
    fn wheel_over_the_panels_scrolls_the_agent_window() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::AgentRows {
            rows: (0..5)
                .map(|i| agent_row(&i.to_string(), false, false))
                .collect(),
        });
        let rows = plain_frame(&app, 60, 20);
        let first = rows.iter().position(|r| r.contains("실행 중-")).unwrap();
        assert!(app.overlay_wheel(false, first));
        assert_eq!(app.pan_off, 1);
        let rows = plain_frame(&app, 60, 20);
        assert!(rows.iter().any(|r| r.contains("task 1")), "{rows:?}");
        assert!(!rows.iter().any(|r| r.contains("task 0")), "{rows:?}");
        assert!(rows.iter().any(|r| r.contains("2-4/5")), "{rows:?}");
        // Wheel up stops at the first row.
        app.overlay_wheel(true, first);
        app.overlay_wheel(true, first);
        assert_eq!(app.pan_off, 0);
    }

    // A wheel notch above the panels is not taken by them (Go panelWheel row check).
    #[test]
    fn wheel_above_the_panels_is_not_taken_by_them() {
        let mut app = App::new(60, 20);
        app.apply(UiEvent::AgentRows {
            rows: (0..5)
                .map(|i| agent_row(&i.to_string(), false, false))
                .collect(),
        });
        assert!(!app.overlay_wheel(false, 0));
        assert_eq!(app.pan_off, 0);
    }

    // Go lastTool (tui.go:900-906): the busy row ends with the last tool, cut to 40 runes; a new turn clears it.
    #[test]
    fn busy_row_shows_the_last_tool_and_a_new_turn_clears_it() {
        let mut app = App::new(100, 30);
        app.apply(UiEvent::Busy(true));
        app.apply(UiEvent::ToolStart {
            id: "t1".into(),
            name: "Agent".into(),
            summary: "x".repeat(60),
        });
        assert_eq!(app.last_tool.chars().count(), LAST_TOOL_RUNES);
        let body = app.body(100);
        assert!(
            body.iter()
                .any(|r| r.contains(&format!("0m00s · {}", app.last_tool))),
            "{body:?}"
        );
        app.apply(UiEvent::Busy(true));
        assert!(app.last_tool.is_empty());
    }

    // The slash menu takes room from the queue (Go popH): an open menu leaves fewer queue rows.
    #[test]
    fn slash_menu_takes_room_from_the_queue() {
        let mut app = App::new(60, 14);
        app.apply(UiEvent::QueueTexts(
            (0..5).map(|i| format!("q{i}")).collect(),
        ));
        let before = queue_rows(&plain_frame(&app, 60, 14)).len();
        type_str(&mut app, "/");
        assert!(app.popup_h() > 0);
        let after = queue_rows(&plain_frame(&app, 60, 14)).len();
        assert!(after < before, "{before} -> {after}");
    }

    #[test]
    fn narrow_terminal_hides_agent_rows() {
        let mut app = App::new(9, 20);
        app.apply(UiEvent::AgentRows {
            rows: vec![agent_row("a", false, false)],
        });
        assert!(
            !plain_frame(&app, 9, 20)
                .iter()
                .any(|r| r.contains("실행 중-"))
        );
    }

    #[test]
    fn cursor_stays_on_the_input_line_above_the_agent_rows() {
        let mut app = App::new(60, 20);
        type_str(&mut app, "ab");
        app.apply(UiEvent::AgentRows {
            rows: vec![agent_row("a", false, false), agent_row("b", true, false)],
        });
        let rows = plain_frame(&app, 60, 20);
        let input = rows.iter().position(|r| r.contains("> ab")).unwrap();
        let (_, y) = cursor(&app, 60, 20);
        assert_eq!(y as usize, input, "{rows:?}");
    }

    // Go TestCursorRowMatchesPanelAfterTick (panel_test.go:79): the cursor row follows the panel when a
    // foreground agent starts and again when it ends.
    #[test]
    fn cursor_row_matches_panel_after_agent_rows_change() {
        let mut app = App::new(80, 30);
        type_str(&mut app, "ab");
        let base = cursor(&app, 80, 30).1;
        app.apply(UiEvent::AgentRows {
            rows: vec![agent_row("f1", false, false)],
        });
        let rows = plain_frame(&app, 80, 30);
        let input = rows.iter().position(|r| r.contains("> ab")).unwrap();
        assert_eq!(cursor(&app, 80, 30).1 as usize, input, "{rows:?}");
        assert!(
            (input as u16) < base,
            "panel row moves the input up: {base} {input}"
        );
        assert!(rows.len() <= 30, "{rows:?}");
        app.apply(UiEvent::AgentRows {
            rows: vec![agent_row("f1", false, true)],
        });
        assert_eq!(cursor(&app, 80, 30).1, base);
    }

    // Go TestIdleTickRefreshesWhenBackgroundFinishes (panel_test.go:55-77, :79): the idle tick re-reads the agent
    // list. The finish is written to the shared list only (Go onNotification changes the tracker, no event is sent).
    #[test]
    fn idle_tick_refreshes_when_background_finishes() {
        let list = Arc::new(Mutex::new(vec![agent_row("b1", true, false)]));
        let mut app = App::new(80, 30);
        app.set_agent_source(Arc::clone(&list));
        app.tick(Instant::now());
        let running = plain_frame(&app, 80, 30);
        assert!(
            running.iter().any(|r| r.contains("task b1")),
            "tick shows the running background row: {running:?}"
        );
        list.lock().unwrap()[0].done = true;
        app.tick(Instant::now());
        let after = plain_frame(&app, 80, 30);
        assert_eq!(
            after.len(),
            30,
            "tick relayouts to the full terminal height"
        );
        assert!(
            !after.iter().any(|r| r.contains("task b1")),
            "finished background row leaves the panel on idle tick: {after:?}"
        );
    }

    // Go tick gate (tui.go:454): the idle tick re-reads the list only while busy or when the running count moved
    // since the last refresh (lastRun). Same-count edits are skipped when idle and shown when busy.
    #[test]
    fn idle_tick_gate_skips_unless_busy_or_count_moved() {
        let list = Arc::new(Mutex::new(vec![agent_row("b1", true, false)]));
        let mut app = App::new(80, 30);
        app.set_agent_source(Arc::clone(&list));
        app.tick(Instant::now());
        assert_eq!(app.agent_rows[0].task, "task b1", "count 0 -> 1 fires");

        list.lock().unwrap()[0].task = "edited".into();
        app.tick(Instant::now());
        assert_eq!(
            app.agent_rows[0].task, "task b1",
            "idle, count same: skipped"
        );

        app.apply(UiEvent::Busy(true));
        app.tick(Instant::now());
        assert_eq!(app.agent_rows[0].task, "edited", "busy: fires");

        list.lock().unwrap()[0].task = "edited again".into();
        app.apply(UiEvent::Busy(false));
        app.tick(Instant::now());
        assert_eq!(
            app.agent_rows[0].task, "edited",
            "idle again, count same: skipped"
        );

        list.lock().unwrap()[0].done = true;
        app.tick(Instant::now());
        assert!(app.agent_rows[0].done, "idle, count 1 -> 0: fires");
    }

    #[test]
    fn empty_size_shows_the_starting_frame() {
        let app = App::new(0, 0);
        assert_eq!(frame(&app, 0, 0), vec!["starting…".to_string()]);
    }

    #[test]
    fn welcome_box_fits_narrow_terminals() {
        // Go clamps the box to at least 20 cells and overflows narrower terminals; Rust shrinks it to fit.
        let app = App::new(0, 0);
        for w in 8..=70 {
            for row in app.welcome_rows(w) {
                let plain = render::strip_sgr(&row);
                assert!(render::cell_width(&plain) <= w, "w={w} row={plain:?}");
            }
        }
    }

    #[test]
    fn clipboard_tool_order_matches_atotto() {
        let all = |_: &str| true;
        assert_eq!(clipboard_tool(true, all).map(|t| t.0), Some("wl-copy"));
        assert_eq!(clipboard_tool(false, all).map(|t| t.0), Some("xclip"));
        assert_eq!(
            clipboard_tool(true, |p| p == "xsel").map(|t| t.0),
            Some("xsel")
        );
        assert_eq!(
            clipboard_tool(false, |p| p == "clip.exe").map(|t| t.0),
            Some("clip.exe")
        );
        assert_eq!(clipboard_tool(true, |_| false), None);
    }

    #[test]
    fn clipboard_payload_is_utf16_with_bom_for_clip_exe() {
        assert_eq!(clipboard_payload("xclip", "가"), "가".as_bytes());
        assert_eq!(
            clipboard_payload("clip.exe", "가"),
            vec![0xFF, 0xFE, 0x00, 0xAC]
        );
    }

    #[test]
    fn ime_shift_follows_env_override_and_jediterm() {
        assert_eq!(ime_col_shift(None, None), 0);
        assert_eq!(ime_col_shift(None, Some("JetBrains-JediTerm".into())), -1);
        assert_eq!(
            ime_col_shift(Some(" 0 ".into()), Some("JetBrains-JediTerm".into())),
            0
        );
        assert_eq!(ime_col_shift(Some("-2".into()), None), -2);
        assert_eq!(
            ime_col_shift(Some("".into()), Some("JetBrains-JediTerm".into())),
            -1
        );
        assert_eq!(ime_col_shift(Some("x".into()), None), 0);
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

    // Go TestUpDownHistoryWithSlashCompleter (history_file_test.go:77): with the completer open on "/",
    // Up at its top reaches history, also for a recalled prompt that starts with "/".
    #[test]
    fn up_down_history_with_slash_completer() {
        let mut app = App::new(80, 30);
        for prompt in ["older prompt", "/mo"] {
            app.editor.insert_str(prompt);
            app.editor.take();
        }
        app.editor.insert_str("/");
        assert!(!app.suggest().is_empty(), "completer open for /");
        app.on_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.editor.text(), "/mo");
        assert!(app.editor.browsing());
        app.on_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.editor.text(), "older prompt");
        app.on_key(key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.editor.text(), "/mo");
        app.on_key(key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.editor.text(), "/");
        assert!(!app.editor.browsing());
    }

    // Go TestUpMovesCompleterWhenNotAtTop (history_file_test.go:109): Up above the completer top moves the
    // selection, and Down keeps moving it without touching the editor.
    #[test]
    fn up_moves_completer_when_not_at_top() {
        let mut app = App::new(80, 30);
        app.editor.insert_str("older prompt");
        app.editor.take();
        app.editor.insert_str("/");
        app.sugg_sel = 2;
        app.on_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.sugg_sel, 1);
        assert_eq!(app.editor.text(), "/");
        assert!(!app.editor.browsing());
        app.on_key(key(KeyCode::Down, KeyModifiers::NONE));
        app.on_key(key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.sugg_sel, 3);
        assert_eq!(app.editor.text(), "/");
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

    // Go indent (tui.go:353-378): wrapped rows of a block line up under its text after the lead; a plain note
    // (no lead) wraps at column 0 as Go's note row does.
    #[test]
    fn wrapped_rows_keep_go_hanging_indent() {
        let plain = |rows: Vec<String>| -> Vec<String> {
            rows.iter().map(|r| render::strip_sgr(r)).collect()
        };
        assert_eq!(
            plain(wrap_row(&format!("{ASST_FIRST}aaaaaaaaaaaa"), 8, true)),
            vec!["● aaaaaa", "  aaaaaa"]
        );
        assert_eq!(
            plain(wrap_row(&format!("{GREY_ON}> cccccccc{FG_OFF}"), 6, true)),
            vec!["> cccc", "  cccc"]
        );
        assert_eq!(
            plain(wrap_row(
                &format!("{GREY_ON}  ⎿  bbbbbbbbbbbb{FG_OFF}"),
                10,
                false
            )),
            vec!["  ⎿  bbbbb", "     bbbbb", "     bb"]
        );
        assert_eq!(
            plain(wrap_row(&format!("{TOOL_ON}● dddddddd{FG_OFF}"), 6, false)),
            vec!["● dddd", "  dddd"]
        );
        assert_eq!(
            plain(wrap_row("  eeeeeeee", 6, false)),
            vec!["  eeee", "eeee"]
        );
        // A pane too narrow for the lead plus two cells wraps without the hang.
        assert_eq!(plain(wrap_row("● ffff", 3, true)), vec!["● f", "fff"]);
    }

    // The unfinished assistant line hangs like the finished ones: "● " first, "  " once the block has a line.
    #[test]
    fn live_assistant_row_hangs_under_its_lead() {
        let mut app = App::new(40, 12);
        app.apply(text("xxxxxxxxxxxxxxxx"));
        let rows = app.body(8);
        let tail: Vec<String> = rows[rows.len() - 3..]
            .iter()
            .map(|r| render::strip_sgr(r))
            .collect();
        assert_eq!(tail, vec!["● xxxxxx", "  xxxxxx", "  xxxx"]);

        let mut app = App::new(40, 12);
        app.apply(text("hi\nxxxxxxxxxxxxxxxx"));
        let rows = app.body(8);
        let tail: Vec<String> = rows[rows.len() - 4..]
            .iter()
            .map(|r| render::strip_sgr(r))
            .collect();
        assert_eq!(tail, vec!["● hi", "  xxxxxx", "  xxxxxx", "  xxxx"]);
    }

    // Go input.go: opt+delete (Alt+Backspace, ESC DEL from macOS) deletes a word; cmd+delete arrives as ^U
    // (Ctrl+U) and kills to the line start; kitty CSI-u Cmd+Backspace (Backspace with Super) does the same.
    #[test]
    fn opt_delete_is_delete_word_and_cmd_delete_kills_to_line_start() {
        let mut app = App::new(40, 12);
        for c in "hello, world".chars() {
            app.editor.insert_char(c);
        }
        app.on_key(key(KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(app.editor.text(), "hello, ");
        app.on_key(key(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.editor.text(), "");

        for c in "ab\ncd ef".chars() {
            app.editor.insert_char(c);
        }
        app.on_key(key(KeyCode::Backspace, KeyModifiers::SUPER));
        assert_eq!(app.editor.text(), "ab\n");
        app.on_key(key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.editor.text(), "ab");
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

    // Go TestRunTurnShowsTUIOutputOnly (resident_pipeline_test.go:88): the tui answer is the whole main block.
    #[test]
    fn tui_answer_fills_and_closes_the_main_block() {
        let mut app = app_with_lines(0);
        app.apply(UiEvent::TuiAnswer {
            text: "번역 답변".to_string(),
        });
        assert!(app.transcript.join("\n").contains("번역 답변"));
        assert!(!app.main_open);
        assert!(app.pending_main.is_empty());
    }

    // R03/R08 (tui.go:374): assistant text gets **bold** and link styling, and keeps its visible text.
    #[test]
    fn assistant_link_keeps_style_on_wrapped_rows() {
        let mut app = app_with_lines(0);
        app.apply(UiEvent::TextDelta {
            role: Role::Main,
            text: "see https://example.com/abcdefghijklmnop\n".to_string(),
        });
        let rows = app.body(20);
        let tail = rows
            .iter()
            .find(|r| render::strip_sgr(r).trim() == "mnop")
            .expect("tail row");
        assert!(tail.contains(render::LINK_URL_ON));
        assert!(
            rows.iter().any(
                |r| render::strip_sgr(r).contains("exampl") && r.contains(render::LINK_URL_ON)
            )
        );
    }

    #[test]
    fn only_assistant_and_user_rows_get_link_style() {
        let mut app = app_with_lines(0);
        app.note("log at /var/tmp/some/long/path/file.txt");
        app.apply(UiEvent::TextDelta {
            role: Role::Main,
            text: "see /var/tmp/some/long/path/file.txt\n".to_string(),
        });
        let rows = app.body(60);
        let note = rows
            .iter()
            .find(|r| render::strip_sgr(r).contains("log at"))
            .expect("note row");
        assert!(!note.contains(render::LINK_COLOR_ON));
        let asst = rows
            .iter()
            .find(|r| render::strip_sgr(r).contains("see /var"))
            .expect("assistant row");
        assert!(asst.contains(render::LINK_COLOR_ON));
    }

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
    fn update_notes_and_asks_the_loop_for_the_check() {
        let mut app = App::new(80, 24);
        type_str(&mut app, "/update");
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Update
        );
        assert!(
            app.transcript
                .iter()
                .any(|r| r == commands::UPDATE_START_NOTE)
        );
    }

    #[test]
    fn status_is_answered_locally_not_sent_to_claude() {
        let mut app = App::new(80, 24);
        for c in "/status".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Continue
        );
        let text = app.transcript.join("\n");
        assert!(text.contains("rotations 0"), "{text}");
        assert!(text.contains("session -"), "{text}");
    }

    // Go TestPlainLocalCmdsNoSend (localcmd_test.go:11): /help, /status and /exit are answered in the app and
    // nothing is submitted to the child session.
    #[test]
    fn plain_local_commands_are_answered_without_send() {
        let mut app = App::new(80, 24);
        for line in ["/help", "/status"] {
            type_str(&mut app, line);
            assert_eq!(
                app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
                Action::Continue,
                "{line} must not be submitted"
            );
        }
        let text = app.transcript.join("\n");
        assert!(text.contains("show commands and keys"), "{text}");
        assert!(text.contains("rotations 0"), "{text}");
        type_str(&mut app, "/exit");
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Quit
        );
    }

    #[test]
    fn status_shows_the_main_session_id_once_known() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::SessionIds {
            main: "abcdef0123456789".to_string(),
            residents: std::collections::BTreeMap::new(),
            owned: vec![],
        });
        for c in "/status".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        let text = app.transcript.join("\n");
        assert!(text.contains("session abcdef01"), "{text}");
        assert!(!text.contains("session -"), "{text}");
    }

    #[test]
    fn clear_empties_the_transcript_before_the_clear_echo() {
        let mut app = App::new(80, 24);
        app.transcript.push("old turn text".to_string());
        for c in "/clear".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        let text = app.transcript.join("\n");
        assert!(!text.contains("old turn text"), "{text}");
        assert!(text.contains("/clear"), "{text}");
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
    fn role_context_fills_the_bar_and_session_ids_join_this_run() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::RoleContext {
            role: "tui".to_string(),
            tokens: 12_000,
        });
        let [_, bottom] = bar::rows(&app.bar_input(), ' ', 200);
        assert!(bottom.contains(" T 12.0k/80k"), "{bottom}");
        app.apply(UiEvent::SessionIds {
            main: "m1".to_string(),
            residents: std::collections::BTreeMap::from([(
                "prompter".to_string(),
                "p1".to_string(),
            )]),
            owned: vec!["o1".to_string(), String::new()],
        });
        assert_eq!(
            app.this_run,
            BTreeSet::from(["m1".to_string(), "p1".to_string(), "o1".to_string()])
        );
    }

    #[test]
    fn rotation_notice_names_the_replaced_session() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::RotatedFrom {
            old_session: "abcdef0123456789".to_string(),
        });
        app.apply(UiEvent::Rotated {
            reason: "tokens".to_string(),
        });
        assert!(
            app.transcript
                .iter()
                .any(|r| r == "── context rotated #1 (tokens; abcdef01 → new) ──"),
            "{:?}",
            app.transcript
        );
    }

    #[test]
    fn tool_input_rewrites_the_tool_row_summary() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::ToolStart {
            id: "t1".to_string(),
            name: "Bash".to_string(),
            summary: String::new(),
        });
        app.apply(UiEvent::ToolInput {
            id: "t1".to_string(),
            input: serde_json::json!({"command": "ls"}),
        });
        assert!(
            app.transcript.iter().any(|r| r.contains("● Bash(ls)")),
            "{:?}",
            app.transcript
        );
    }

    #[test]
    fn thinking_row_hides_once_main_text_streams() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::Busy(true));
        assert!(app.body(80).iter().any(|r| r.contains("생각중")));
        app.apply(UiEvent::TextDelta {
            role: Role::Main,
            text: "hi\n".to_string(),
        });
        assert!(!app.body(80).iter().any(|r| r.contains("생각중")));
    }

    // Live AgentStart carries Unix seconds; the running rec must show that start in ms, not 0.
    #[test]
    fn live_agent_started_at_converts_seconds_to_ms() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::AgentStart {
            id: "a1".to_string(),
            description: "Scan repo".to_string(),
            started_at: 1_700_000_000,
        });
        let recs = app.agent_recs();
        let rec = recs.iter().find(|r| r.id == "a1").expect("running agent");
        assert_eq!(rec.started_at, 1_700_000_000_000, "{rec:?}");
    }

    // Go onSubEvent: AgentLog lines land in the running agent's view log, capped, and add no transcript row.
    #[test]
    fn agent_log_feeds_the_running_agent_view() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::AgentStart {
            id: "a1".to_string(),
            description: "Scan repo".to_string(),
            started_at: 0,
        });
        let before = app.transcript.len();
        app.apply(UiEvent::AgentLog {
            id: "a1".to_string(),
            lines: vec!["Looking around".to_string(), "● Bash(ls)".to_string()],
        });
        assert_eq!(app.transcript.len(), before);
        let recs = app.agent_recs();
        let rec = recs.iter().find(|r| r.id == "a1").expect("running agent");
        assert_eq!(rec.log, vec!["Looking around", "● Bash(ls)"]);
    }

    // Go TestAgentViewClickReadOnlyClose (agentview_test.go:21-54): a left press on the "worker" panel row opens the
    // view; typed text and Enter are dropped; esc closes; ctrl+t reopens; q closes without typing.
    #[test]
    fn agentview_click_read_only_close() {
        let mut app = App::new(80, 30);
        // AgentStart feeds the view record; AgentRows feeds the panel row (the engine sends both).
        app.apply(UiEvent::AgentStart {
            id: "w1".to_string(),
            description: "worker".to_string(),
            started_at: 0,
        });
        app.apply(UiEvent::AgentRows {
            rows: vec![AgentRow {
                name: "worker".to_string(),
                ..agent_row("w1", false, false)
            }],
        });
        app.apply(UiEvent::AgentLog {
            id: "w1".to_string(),
            lines: vec!["SUBLINE".to_string()],
        });
        // The panel row carries the "●" dot; the status bar also names the agent but has no dot.
        let rows = plain_frame(&app, 80, 30);
        let row = rows
            .iter()
            .position(|r| r.contains('●') && r.contains("worker"))
            .expect("worker panel row");
        assert!(app.panel_click(row as u16), "{rows:?}");
        assert_eq!(app.av.as_ref().map(|v| v.id.as_str()), Some("w1"));
        assert!(
            plain_frame(&app, 80, 30)
                .iter()
                .any(|r| r.contains("SUBLINE")),
            "view shows the agent log"
        );
        app.on_key(key(KeyCode::Char('h'), KeyModifiers::NONE));
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.editor.is_empty(), "typing is dropped");
        assert!(app.queue_texts.is_empty(), "Enter queues nothing");
        assert!(app.av.is_some(), "typing keeps the view open");
        app.on_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.av.is_none(), "esc closes");
        app.on_key(key(KeyCode::Char('t'), KeyModifiers::CONTROL));
        assert_eq!(
            app.av.as_ref().map(|v| v.id.as_str()),
            Some("w1"),
            "ctrl+t reopens"
        );
        app.on_key(key(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(app.av.is_none(), "q closes");
        assert!(app.editor.is_empty(), "q is not typed");
    }

    // Go TestScrollBottomOnInputAndUpdate (agentview_test.go:130-163): new content and submit follow the tail; typing
    // keeps the scroll position; the agent view keeps its own offset across frames; closing it returns to the bottom.
    #[test]
    fn scroll_bottom_on_input_and_update() {
        let mut app = App::new(80, 24);
        for _ in 0..80 {
            app.note("line");
        }
        assert!(app.follow, "content must follow the tail");
        app.scroll(-10_000);
        assert_eq!(app.top, 0);
        app.on_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        assert!(!app.follow, "typing must keep the scroll position");
        assert_eq!(app.top, 0, "typing must keep the scroll position");
        app.scroll(10_000);
        assert!(app.follow, "scrolled back to the bottom");
        app.apply(text("more"));
        assert!(app.follow, "update must scroll to bottom");
        app.scroll(-10_000);
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.follow, "submit must scroll to bottom");
        // Go keeps the agent view scroll across refresh; Rust frame() reads &App, so it cannot change it.
        app.apply(UiEvent::AgentStart {
            id: "w1".to_string(),
            description: "worker".to_string(),
            started_at: 0,
        });
        app.on_key(key(KeyCode::Char('t'), KeyModifiers::CONTROL));
        app.av.as_mut().expect("view open").off = 2;
        plain_frame(&app, 80, 24);
        assert_eq!(
            app.av.as_ref().map(|v| v.off),
            Some(2),
            "refresh must not reset the view scroll"
        );
        app.scroll(-10_000);
        app.on_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.av.is_none(), "esc closes the view");
        assert!(app.follow, "closing the view puts the body at the bottom");
    }

    #[test]
    fn agent_end_adds_the_done_line() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::AgentStart {
            id: "a1".to_string(),
            description: "Scan repo".to_string(),
            started_at: 0,
        });
        app.apply(UiEvent::AgentEnd {
            id: "a1".to_string(),
            elapsed_ms: 1500,
            model: String::new(),
            name: "haiku-coder".to_string(),
            summary: "Scan repo".to_string(),
        });
        assert!(
            app.transcript
                .iter()
                .any(|r| r.contains("[agent 완료] 상속 haiku-coder · Scan repo")),
            "{:?}",
            app.transcript
        );
        // Go prints no "◆ agent started" or "◆ agent done in" rows.
        assert!(app.transcript.iter().all(|r| !r.contains("◆")));
    }

    // Go repl.go:721-754 and drain.go doneLine: the stage row names the subagent_type label, and the done
    // notice takes the model, name and task from the Agent call.
    #[test]
    fn agent_stage_and_done_follow_go_labels() {
        assert_eq!(
            stage_text("agent:haiku-coder", &StagePhase::Working),
            "· agent haiku-coder 호출"
        );
        assert_eq!(
            stage_text("agent:agent", &StagePhase::Done),
            "· agent agent 완료"
        );
        let mut app = App::new(80, 24);
        app.apply(UiEvent::AgentStart {
            id: "a2".to_string(),
            description: "Scan".to_string(),
            started_at: 0,
        });
        app.apply(UiEvent::AgentEnd {
            id: "a2".to_string(),
            elapsed_ms: 1000,
            model: "haiku".to_string(),
            name: "scan-1".to_string(),
            summary: "Scan repo".to_string(),
        });
        assert!(
            app.transcript
                .iter()
                .any(|r| r.contains("[agent 완료] haiku scan-1 · Scan repo")),
            "{:?}",
            app.transcript
        );
    }

    // E15: an interrupt finishes the foreground panel agents at once (Go finishForeground); a background agent keeps its row.
    #[test]
    fn interrupt_finishes_foreground_agents_only() {
        let mut app = App::new(80, 24);
        for id in ["fg", "bg"] {
            app.apply(UiEvent::AgentStart {
                id: id.to_string(),
                description: id.to_string(),
                started_at: 0,
            });
        }
        app.apply(UiEvent::AgentBackground { id: "bg".into() });
        app.finish_foreground();
        assert_eq!(app.agents.len(), 1);
        assert_eq!(app.agents[0].0, "bg");
        assert!(app.transcript.iter().any(|r| r.contains("[agent 완료]")));
        // Core's late AgentEnd for the finished foreground agent adds no second done line.
        let before = app.transcript.len();
        app.apply(UiEvent::AgentEnd {
            id: "fg".into(),
            elapsed_ms: 900,
            model: String::new(),
            name: "fg".into(),
            summary: "fg".into(),
        });
        assert_eq!(app.transcript.len(), before);
        assert!(app.finished_fg.is_empty());
    }

    // Port of TestEscInterruptClearsForegroundPanel, second half (panel_test.go:50-54): a turn that ends with
    // a foreground agent still listed (no tool_result) clears it; the background agent stays.
    #[test]
    fn turn_end_finishes_foreground_agents_only() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::AgentStart {
            id: "bg".into(),
            description: "bg".into(),
            started_at: 0,
        });
        app.apply(UiEvent::AgentBackground { id: "bg".into() });
        app.apply(UiEvent::AgentStart {
            id: "fg".into(),
            description: "fg".into(),
            started_at: 0,
        });
        app.apply(UiEvent::Busy(false));
        assert_eq!(app.agents.len(), 1);
        assert_eq!(app.agents[0].0, "bg");
    }

    // E23: a background agent's AgentEnd removes its row and adds the done notice (Go onUserEvent path).
    #[test]
    fn background_agent_end_removes_row_and_notes_done() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::AgentStart {
            id: "b1".into(),
            description: "Watch".into(),
            started_at: 0,
        });
        app.apply(UiEvent::AgentBackground { id: "b1".into() });
        app.apply(UiEvent::AgentEnd {
            id: "b1".into(),
            elapsed_ms: 2000,
            model: String::new(),
            name: "b1".into(),
            summary: "Watch".into(),
        });
        assert!(app.agents.is_empty());
        assert!(app.bg_agents.is_empty());
        assert!(
            app.transcript
                .iter()
                .any(|r| r.contains("[agent 완료] 상속 b1 · Watch"))
        );
    }

    #[test]
    fn orphan_notice_names_the_child_and_its_model() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::OrphanNotice {
            role: "main".to_string(),
            pid: 4242,
            model: String::new(),
        });
        assert!(
            app.transcript
                .iter()
                .any(|r| r.starts_with("[orphan] claude pid 4242 (claude-opus-5-5, ")),
            "{:?}",
            app.transcript
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

    // Go tui.go:383-391: after text started, an opus Agent or a busy resident keeps the thinking row up.
    #[test]
    fn opus_or_resident_busy_keeps_the_thinking_row_after_text() {
        let thinking = |app: &App| {
            app.body(100)
                .iter()
                .any(|r| r.contains("생각중-claude-opus-5-5-메인"))
        };
        let mut app = App::new(100, 30);
        type_str(&mut app, "long task");
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        app.apply(UiEvent::Busy(true));
        app.apply(UiEvent::TextDelta {
            role: Role::Main,
            text: "partial".into(),
        });
        assert!(!thinking(&app), "text started and nothing running: hidden");
        app.apply(UiEvent::OpusBusy(true));
        assert!(thinking(&app), "opus running keeps the row");
        app.apply(UiEvent::OpusBusy(false));
        app.apply(UiEvent::ResidentBusy {
            role: "coder".into(),
            busy: true,
            prompt: "rename the helper".into(),
            started_at: 1,
        });
        assert!(thinking(&app), "resident busy keeps the row");
        app.apply(UiEvent::ResidentBusy {
            role: "coder".into(),
            busy: false,
            prompt: String::new(),
            started_at: 0,
        });
        assert!(!thinking(&app), "resident idle again: hidden");
    }

    // Go TestResidentRunningListShowsOnlyBusySessions (resident_busy_test.go:9-35): only a busy resident is a panel
    // row (role and command text), the thinking row shows and runningN counts it; the row drops out when it ends.
    #[test]
    fn resident_running_list_shows_only_busy_sessions() {
        let thinking = |app: &App| app.body(80).iter().any(|r| r.contains("생각중-"));
        let mut app = App::new(80, 30);
        assert!(app.panel_lines(80, 30).0.is_empty(), "idle: no row");
        assert!(!thinking(&app), "idle: no 생각중 indicator");
        assert_eq!(app.running_n(), 0);

        app.apply(UiEvent::ResidentBusy {
            role: "tui".into(),
            busy: true,
            prompt: "review the diff".into(),
            started_at: 1,
        });
        let rows = app.panel_lines(80, 30).0;
        assert!(
            rows.len() == 1 && rows[0].contains("tui") && rows[0].contains("review the diff"),
            "busy resident row: {rows:?}"
        );
        assert!(
            thinking(&app),
            "thinking indicator while a resident handles a command"
        );
        assert_eq!(app.running_n(), 1, "runningN counts the busy resident");

        app.apply(UiEvent::ResidentBusy {
            role: "tui".into(),
            busy: false,
            prompt: String::new(),
            started_at: 0,
        });
        assert!(
            app.panel_lines(80, 30).0.is_empty(),
            "finished resident leaves the list"
        );
        assert!(!thinking(&app), "finished resident: no indicator");
        assert_eq!(app.running_n(), 0, "runningN after finish");
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
    fn ctrl_c_interrupts_when_busy_and_quits_when_idle_even_with_text() {
        let mut app = App::new(80, 24);
        let ctrl_c = || key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        app.on_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(app.on_key(ctrl_c()), Action::Quit);
        app.apply(UiEvent::Busy(true));
        assert_eq!(
            app.on_key(ctrl_c()),
            Action::Interrupt(InterruptSource::CtrlC)
        );
    }

    #[cfg(unix)]
    #[test]
    fn sigterm_is_forwarded_as_wake_signal() {
        let (tx, rx) = mpsc::channel();
        forward_signals(tx).expect("pipe");
        // SAFETY: raise(3) with a signal whose handler is installed by forward_signals above.
        unsafe {
            libc::raise(libc::SIGTERM);
        }
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(2)),
            Ok(Wake::Signal)
        ));
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

    // E14: a process error is a dim note block followed by a blank row (Go note rendering), not a plain "! " line.
    #[test]
    fn error_event_is_a_dim_note_then_blank_row() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::Error("boom".into()));
        let n = app.transcript.len();
        assert_eq!(app.transcript[n - 2], format!("{GREY_ON}boom{FG_OFF}"));
        assert_eq!(app.transcript[n - 1], "");
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
    fn slash_mode_and_model_are_not_sent_to_the_engine_as_text() {
        let mut app = App::new(80, 24);
        for c in "/mode plan".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::SetMode(PermMode::Plan)
        );
        // C07: /model NAME is sent to the engine as the full line (tui.go /model NAME), not handled locally.
        for c in "/model sonnet".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Submit("/model sonnet".to_string())
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
    fn release_copies_selection_once_and_notes_it() {
        // Go TestReleaseCopiesSelection (selection_test.go:50): a drag does not copy, release copies the selected
        // text once and shows the note, a plain click copies nothing.
        let mut app = App::new(30, 6);
        app.transcript.push("alpha".into());
        app.transcript.push("beta gamma".into());
        assert_eq!(screen(&app, 30, 6)[..2], ["alpha", "beta gamma"]);
        let mut copied: Vec<String> = Vec::new();
        let mut osc: Vec<u8> = Vec::new();
        app.select_press(0, 1);
        app.select_drag(4, 1);
        assert!(copied.is_empty(), "{copied:?}");
        release_copy(
            &mut app,
            &mut osc,
            |s| {
                copied.push(s.to_string());
                Ok(())
            },
            4,
            1,
        );
        assert_eq!(copied, ["beta"]);
        assert_eq!(
            String::from_utf8_lossy(&osc),
            format!("\x1b]52;c;{}\x07", base64(b"beta"))
        );
        assert_eq!(
            app.copy_note.as_ref().map(|n| n.0.as_str()),
            Some("복사됨 (4자)")
        );
        app.select_press(2, 0);
        release_copy(
            &mut app,
            &mut osc,
            |s| {
                copied.push(s.to_string());
                Ok(())
            },
            2,
            0,
        );
        assert_eq!(copied, ["beta"], "plain click copied");
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
    fn a_key_press_wakes_the_loop_without_waiting_for_the_tick() {
        let (tx, wakes) = mpsc::channel();
        let sender = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            tx.send(Wake::Term(Event::Key(key(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
            ))))
            .unwrap();
            // Keep the channel open so the wait can only end by the key or the timeout.
            thread::sleep(Duration::from_millis(200));
        });
        let start = Instant::now();
        // A 10 s timeout stands in for "the tick is far away": the key must end the wait first.
        let batch = wait_batch(&wakes, Some(Duration::from_secs(10))).unwrap();
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "{:?}",
            start.elapsed()
        );
        assert!(matches!(batch.as_slice(), [Wake::Term(Event::Key(_))]));
        sender.join().unwrap();
    }

    #[test]
    fn idle_timeout_yields_an_empty_batch_and_closed_ends_the_wait() {
        let (tx, wakes) = mpsc::channel();
        assert!(
            wait_batch(&wakes, Some(Duration::from_millis(1)))
                .unwrap()
                .is_empty()
        );
        tx.send(Wake::Engine(UiEvent::MessageDone { role: Role::Main }))
            .unwrap();
        tx.send(Wake::Closed).unwrap();
        assert!(wait_batch(&wakes, None).is_none());
    }

    #[test]
    fn only_busy_or_a_copy_note_keeps_the_loop_on_a_timer() {
        let mut app = App::new(40, 10);
        assert!(!app.animating());
        app.busy = true;
        assert!(app.animating());
        app.busy = false;
        app.copy_done(3, Ok(()));
        assert!(app.animating());
    }

    #[test]
    fn a_key_press_reaches_the_screen_without_waiting_for_the_tick() {
        let (tx, wakes) = mpsc::channel();
        let sender = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            tx.send(Wake::Term(Event::Key(key(
                KeyCode::Char('z'),
                KeyModifiers::NONE,
            ))))
            .unwrap();
            thread::sleep(Duration::from_millis(200));
        });
        let mut app = App::new(40, 6);
        let mut prev = Vec::new();
        draw(&mut Vec::new(), &app, &mut prev).unwrap();
        let mut buf = Vec::new();
        let start = Instant::now();
        // Same steps as one loop iteration with no timer (idle): wait, apply the keys, tick, draw once.
        let batch = wait_batch(&wakes, Some(Duration::from_secs(10))).unwrap();
        for wake in batch {
            if let Wake::Term(Event::Key(k)) = wake {
                app.on_key(k);
            }
        }
        app.tick(Instant::now());
        draw(&mut buf, &app, &mut prev).unwrap();
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "{:?}",
            start.elapsed()
        );
        assert!(String::from_utf8_lossy(&buf).contains('z'), "{buf:?}");
        sender.join().unwrap();
    }

    #[test]
    fn picked_group_imports_and_never_relaunches() {
        let mut app = App::new(80, 24);
        let info = |id: &str| chaude_core::resume_group::SessionInfo {
            id: id.into(),
            path: std::path::PathBuf::new(),
            modified: std::time::SystemTime::UNIX_EPOCH,
            turns: 1,
            first: String::new(),
            last: String::new(),
            start: None,
            cwd: String::new(),
            name: String::new(),
            cleared: false,
            clear_head: false,
        };
        let group = chaude_core::resume_group::SessionGroup {
            files: vec![info("new"), info("old")],
        };
        app.pick = Some(resume_pick::GroupPicker::new(vec![group]));
        // Enter closes the picker and goes through the import path (Go sendImport), not a relaunch.
        assert_eq!(
            app.on_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Continue
        );
        assert!(app.pick.is_none());
    }

    #[test]
    fn import_replaces_the_transcript_view() {
        let path = std::env::temp_dir().join(format!("chaude-import-{}.jsonl", std::process::id()));
        let lines = concat!(
            r#"{"type":"user","message":{"content":"hello import"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hi there"}]}}"#,
            "\n"
        );
        std::fs::write(&path, lines).unwrap();
        let mut app = App::new(80, 24);
        app.transcript.push("old line before import".to_string());
        app.import_view(path.to_str().unwrap());
        let _ = std::fs::remove_file(&path);
        assert!(app.import_install.is_some());
        assert!(
            !app.transcript
                .iter()
                .any(|l| l.contains("old line before import")),
            "{:?}",
            app.transcript
        );
        assert!(
            app.transcript.iter().any(|l| l.contains("hello import")),
            "{:?}",
            app.transcript
        );
        assert!(
            app.transcript.iter().any(|l| l.contains("next prompt")),
            "{:?}",
            app.transcript
        );
    }

    #[test]
    fn session_resumed_notes_the_switch_and_opens_the_session_list() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::SessionResumed {
            old_session: "old-1".into(),
            session_id: "new-2".into(),
        });
        assert!(
            app.transcript
                .iter()
                .any(|l| l.contains("session resumed: new-2 (was old-1)"))
        );
    }

    #[test]
    fn session_mode_note_follows_core_resume_rule() {
        let dir = std::env::temp_dir().join(format!("chaude-tui-mode-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("exit-handoff.json");
        let with_ctx = |tokens: u64| handoff::ExitHandoff {
            main_session: "sess-1".into(),
            ctx_tokens: [("main".to_string(), tokens)].into_iter().collect(),
            ..Default::default()
        };
        // Clean exit below the threshold: resume.
        handoff::write_exit_handoff(&path, &with_ctx(1_000)).unwrap();
        let note = session_mode_note(Some(&path), "haiku", 80_000, false).unwrap();
        assert!(
            note.contains("claude --resume sess-1") && note.contains("below rotation"),
            "{note}"
        );
        // Clean exit above the threshold: fresh start with the handoff note.
        handoff::write_exit_handoff(&path, &with_ctx(90_000)).unwrap();
        let note = session_mode_note(Some(&path), "haiku", 80_000, false).unwrap();
        assert!(note.starts_with("previous session: fresh start"), "{note}");
        // Killed run (live marker left behind): resume, whatever the context size.
        std::fs::remove_file(&path).unwrap();
        handoff::write_exit_handoff(&handoff::live_handoff_file(&path), &with_ctx(90_000)).unwrap();
        let note = session_mode_note(Some(&path), "haiku", 80_000, false).unwrap();
        assert!(
            note.contains("claude --resume sess-1") && note.contains("killed"),
            "{note}"
        );
        // An explicit --resume and no previous run both give no note.
        assert!(session_mode_note(Some(&path), "haiku", 80_000, true).is_none());
        assert!(session_mode_note(None, "haiku", 80_000, false).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
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

    // Go quiet(): a busy turn with no content for 30 min shows one notice; an open tool call suppresses it.
    #[test]
    fn idle_notice_once_and_not_while_tool_open() {
        let mut app = App::new(80, 24);
        app.apply(UiEvent::Busy(true));
        let later = app.clock + IDLE_NOTICE;
        app.tick(later);
        app.tick(later);
        let rows = app.transcript.join("\n");
        assert_eq!(rows.matches("[no output for 30m0s").count(), 1);

        let mut app = App::new(80, 24);
        app.apply(UiEvent::Busy(true));
        app.apply(UiEvent::ToolStart {
            id: "t1".into(),
            name: "Agent".into(),
            summary: String::new(),
        });
        app.tick(app.clock + IDLE_NOTICE);
        assert!(!app.transcript.join("\n").contains("still waiting"));
    }

    // Minimal terminal model for the escapes draw() emits: cursor moves, line clear, scroll region, SU/SD.
    struct Screen {
        w: usize,
        cells: Vec<Vec<char>>,
        row: usize,
        col: usize,
        top: usize,
        bot: usize,
    }

    impl Screen {
        fn new(w: usize, h: usize) -> Self {
            Screen {
                w,
                cells: vec![vec![' '; w]; h],
                row: 0,
                col: 0,
                top: 0,
                bot: h - 1,
            }
        }

        fn feed(&mut self, s: &str) {
            let mut it = s.chars();
            while let Some(c) = it.next() {
                if c != '\x1b' {
                    if self.col < self.w {
                        self.cells[self.row][self.col] = c;
                    }
                    self.col += 1;
                    continue;
                }
                assert_eq!(it.next(), Some('['), "only CSI is modelled");
                let mut params = String::new();
                let fin = loop {
                    let c = it.next().expect("escape cut short");
                    if c.is_ascii_digit() || c == ';' || c == '?' {
                        params.push(c);
                    } else {
                        break c;
                    }
                };
                let nums: Vec<usize> = params.split(';').filter_map(|p| p.parse().ok()).collect();
                match fin {
                    'm' | 'h' | 'l' => {}
                    'H' => {
                        self.row = nums.first().copied().unwrap_or(1) - 1;
                        self.col = nums.get(1).copied().unwrap_or(1) - 1;
                    }
                    'K' if params == "2" => self.cells[self.row] = vec![' '; self.w],
                    'S' | 'T' => self.scroll(nums.first().copied().unwrap_or(1), fin == 'S'),
                    'r' if params.is_empty() => {
                        self.top = 0;
                        self.bot = self.cells.len() - 1;
                        (self.row, self.col) = (0, 0);
                    }
                    'r' => {
                        self.top = nums[0] - 1;
                        self.bot = nums[1] - 1;
                        (self.row, self.col) = (0, 0);
                    }
                    other => panic!("unmodelled escape {other:?} params {params:?}"),
                }
            }
        }

        fn scroll(&mut self, n: usize, up: bool) {
            let blank = vec![' '; self.w];
            for _ in 0..n {
                if up {
                    for r in self.top..self.bot {
                        self.cells[r] = self.cells[r + 1].clone();
                    }
                    self.cells[self.bot] = blank.clone();
                } else {
                    for r in (self.top + 1..=self.bot).rev() {
                        self.cells[r] = self.cells[r - 1].clone();
                    }
                    self.cells[self.top] = blank.clone();
                }
            }
        }

        fn lines(&self) -> Vec<String> {
            self.cells
                .iter()
                .map(|r| r.iter().collect::<String>().trim_end().to_string())
                .collect()
        }
    }

    fn strip_sgr(s: &str) -> String {
        let mut out = String::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            if c == '\x1b' {
                for c in it.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    // Draws one update and checks the modelled screen equals the frame. Returns the bytes written.
    fn draw_and_check(app: &App, prev: &mut Vec<String>, screen: &mut Screen) -> String {
        let mut buf: Vec<u8> = Vec::new();
        draw(&mut buf, app, prev).unwrap();
        let out = String::from_utf8(buf).unwrap();
        screen.feed(&out);
        let want: Vec<String> = frame(app, app.cols, app.rows)
            .iter()
            .map(|r| strip_sgr(r).trim_end().to_string())
            .collect();
        assert_eq!(screen.lines(), want, "bytes written: {out:?}");
        out
    }

    // Scrolled body moves use the terminal's scroll region; the screen must still match the frame after each update.
    #[test]
    fn draw_scroll_region_leaves_the_screen_equal_to_the_frame() {
        let mut app = App::new(40, 12);
        let mut prev = Vec::new();
        let mut screen = Screen::new(40, 12);
        draw_and_check(&app, &mut prev, &mut screen);
        let mut scrolled = false;
        for i in 0..30 {
            app.apply(text(&format!("line {i}\n")));
            scrolled |= draw_and_check(&app, &mut prev, &mut screen).contains("\x1b[1S");
        }
        assert!(
            scrolled,
            "a streamed line should move the body with a scroll, not a repaint"
        );
        for delta in [-3, -2, 5, 100, -40] {
            app.scroll(delta);
            draw_and_check(&app, &mut prev, &mut screen);
        }
        app.apply(text("tail\n"));
        draw_and_check(&app, &mut prev, &mut screen);
        for i in 0..20 {
            app.apply(text(&format!("burst {i}\n")));
        }
        draw_and_check(&app, &mut prev, &mut screen);
    }

    // Frame bench: 2000-row history, then 200 streamed text events (one frame each) like 200 events/s.
    // Run: cargo test -p chaude-cli --release bench_frame -- --ignored --nocapture
    #[test]
    #[ignore = "bench: timing run only, no Go counterpart; run with --ignored"]
    fn bench_frame_large_history_streaming() {
        let mut app = App::new(120, 40);
        for i in 0..2000 {
            app.transcript.push(format!(
                "**row {i}** see crates/cli/src/tui/mod.rs:{i} and https://example.com/a/{i} then more words to wrap"
            ));
        }
        app.linked.extend(0..2000);
        let mut prev = Vec::new();
        let mut buf: Vec<u8> = Vec::new();
        draw(&mut buf, &app, &mut prev).unwrap();
        let mut ms: Vec<f64> = Vec::new();
        let mut bytes: Vec<usize> = Vec::new();
        let mut cells: Vec<usize> = Vec::new();
        let mut rows_changed: Vec<usize> = Vec::new();
        for i in 0..200 {
            app.apply(text(&format!(
                "streamed line {i} with words to wrap across the width of the pane\n"
            )));
            buf.clear();
            let t = Instant::now();
            draw(&mut buf, &app, &mut prev).unwrap();
            ms.push(t.elapsed().as_secs_f64() * 1000.0);
            bytes.push(buf.len());
            // Rows actually repainted: each repaint starts with a line clear (ESC[2K).
            let repainted = String::from_utf8_lossy(&buf).matches("\x1b[2K").count();
            rows_changed.push(repainted);
            cells.push(repainted * app.cols);
        }
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg = ms.iter().sum::<f64>() / ms.len() as f64;
        println!(
            "BENCH frame ms: avg {avg:.3} p50 {:.3} p95 {:.3} max {:.3} (n={}, history={} rows)",
            ms[ms.len() / 2],
            ms[ms.len() * 95 / 100],
            ms[ms.len() - 1],
            ms.len(),
            app.transcript.len()
        );
        let n = bytes.len();
        println!(
            "BENCH per update: bytes avg {} max {}; rows repainted avg {} max {}; cells repainted avg {} max {} (n={n})",
            bytes.iter().sum::<usize>() / n,
            bytes.iter().max().unwrap(),
            rows_changed.iter().sum::<usize>() / n,
            rows_changed.iter().max().unwrap(),
            cells.iter().sum::<usize>() / n,
            cells.iter().max().unwrap()
        );
    }
}
