// Formatting for the two status rows above the input line. Pure functions only, so they are unit-tested
// without a terminal. Every row is clipped to the given display width so it never wraps.
// Helpers that need core data or hooks not wired yet carry a targeted #[allow(dead_code)] with the reason.

use std::time::Duration;

use chaude_core::api::CoderContext;
use chaude_core::i18n::{self, Lang, Msg};

use super::render::{cluster_width, grapheme_clusters};

// Spinner frames and step from tui.go:259-260 (spinner.Frames, FPS = 1/8 s).
const SPIN_FRAMES: [char; 6] = ['✻', '✽', '✶', '✲', '✢', '·'];
const SPIN_STEP: Duration = Duration::from_millis(125);

// Agent state labels (tui.go:1498-1501).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Running,
    Done,
    Failed,
    Queued,
    Interrupted,
}

impl AgentState {
    pub fn label(self) -> &'static str {
        self.label_in(i18n::current())
    }

    pub fn label_in(self, lang: Lang) -> &'static str {
        let msg = match self {
            AgentState::Running => Msg::StateRunning,
            AgentState::Done => Msg::StateDone,
            AgentState::Failed => Msg::StateFailed,
            AgentState::Queued => Msg::StateQueued,
            AgentState::Interrupted => Msg::StateInterrupted,
        };
        i18n::text(lang, msg)
    }

    // xterm-256 index for the colored ● (agentDot tui.go:1520-1523).
    pub fn dot_color(self) -> u8 {
        match self {
            AgentState::Running => 220,
            AgentState::Done => 42,
            AgentState::Failed => 196,
            AgentState::Queued => 245,
            AgentState::Interrupted => 208,
        }
    }
}

pub use chaude_core::ui_state::BarInput;

// Per-role context limit shown in the Go ctx bar (haikuThreshold in repl.go).
const ROLE_LIMIT: &str = "80k";

// Go %.4g for the measurement floats (record_turn.go measureLine). Fixed notation only; exponent form is not handled.
pub fn fmt_g4(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    let e = x.abs().log10().floor() as i32;
    let decimals = (3 - e).max(0) as usize;
    let s = format!("{x:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

// Go fmtK (repl.go:142-162): "0" to "999", then k or M with 2, 1 or 0 decimals by magnitude.
fn fmt_k(n: u64) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    let (unit, div) = if n >= 1_000_000 {
        ("M", 1_000_000)
    } else {
        ("k", 1_000)
    };
    let x = n / div;
    if x >= 100 {
        format!("{x}{unit}")
    } else if x >= 10 {
        format!("{x}.{}{unit}", (n / (div / 10)) % 10)
    } else {
        format!("{x}.{:02}{unit}", (n / (div / 100)) % 100)
    }
}

// Go fmtLimit (repl.go:164-169): whole thousands as "200k", otherwise fmtK.
fn fmt_limit(n: u64) -> String {
    if n >= 1_000 && n.is_multiple_of(1_000) {
        format!("{}k", n / 1_000)
    } else {
        fmt_k(n)
    }
}

pub fn fmt_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m{:02}s", secs / 60, secs % 60)
    }
}

// Token count for the turn and task lines: "999", "12.3k", "1.2M".
pub fn fmt_tokens(n: u64) -> String {
    if n < 1_000 {
        n.to_string()
    } else if n < 1_000_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

// Returns the two status rows: running state and agents, then model/tokens/cost/context/queue.
pub fn rows(b: &BarInput, spinner: char, width: usize) -> [String; 2] {
    // While a reply is held, the top row says how many agents it waits for (Go-style "N agents waiting").
    let mut top = match (b.turn_secs, b.waiting) {
        (Some(s), Some(n)) => format!(
            "{spinner} {} {}",
            i18n::tf(Msg::AgentsWaiting, &[&n]),
            fmt_secs(s)
        ),
        (None, Some(n)) => i18n::tf(Msg::AgentsWaiting, &[&n]),
        (Some(s), None) => format!("{spinner} running {}", fmt_secs(s)),
        (None, None) => match b.turn_tokens {
            Some((input, output, cache)) => format!(
                "idle · {}",
                i18n::tf(
                    Msg::TurnTokens,
                    &[&fmt_tokens(input), &fmt_tokens(output), &fmt_tokens(cache)]
                )
            ),
            None => "idle".to_string(),
        },
    };
    // Each running agent keeps a visible segment. When they do not all fit, every description is cut to an
    // equal share of the room left, so a long first agent cannot push later (e.g. resumed) agents off the row.
    let room = width.saturating_sub(cells(&top));
    let full: usize = b
        .agents
        .iter()
        .map(|(desc, secs)| cells(&format!(" · agent {desc} {}", fmt_secs(*secs))))
        .sum();
    let share = room / b.agents.len().max(1);
    for (desc, secs) in &b.agents {
        let secs = fmt_secs(*secs);
        let desc = if full > room && b.agents.len() > 1 {
            let fixed = cells(&format!(" · agent  {secs}"));
            clip(desc, share.saturating_sub(fixed))
        } else {
            desc.clone()
        };
        top.push_str(&format!(" · agent {desc} {secs}"));
    }
    // Go statusLine + ctxBar (tui.go:1193). M is the main context from UiEvent::Status; P/T/C come from
    // UiEvent::RoleContext (prompter, tui, coder).
    let mode = Some(b.mode.as_str()).filter(|m| !m.is_empty() && *m != "default");
    let rate = b
        .rate
        .as_ref()
        .map(|r| format!(" · {r}"))
        .unwrap_or_default();
    let role = |k: &str| fmt_k(b.role_tokens.get(k).copied().unwrap_or(0));
    let mut bottom = format!(
        " M {}/{} P {}/{r} T {}/{r} C {}/{r}{rate}{}",
        fmt_k(b.tokens),
        fmt_limit(b.limit),
        role("prompter"),
        role("tui"),
        role("coder"),
        status_suffix(b.rot, mode, b.fg, b.bg),
        r = ROLE_LIMIT
    );
    if b.queued > 0 {
        bottom.push_str(&format!(" · queued {}", b.queued));
    }
    [clip(&top, width), clip(&bottom, width)]
}

// The coder line under the input box: "C1 12k/80k  C2 3.1k/80k", one entry per live coder in spawn order (used
// context tokens over the rotation limit). A coder with no context tokens yet (the resident one, idle at start)
// is not listed; with none listed the line is hidden.
pub fn coder_line(coders: &[CoderContext], width: usize) -> Option<String> {
    let shown: Vec<&CoderContext> = coders.iter().filter(|c| c.tokens > 0).collect();
    if shown.is_empty() {
        return None;
    }
    let text = shown
        .iter()
        .map(|c| format!("C{} {}/{}", c.slot, fmt_limit(c.tokens), fmt_limit(c.limit)))
        .collect::<Vec<_>>()
        .join("  ");
    Some(clip(&text, width))
}

// Display columns of `s` (the width clip uses).
fn cells(s: &str) -> usize {
    grapheme_clusters(s).into_iter().map(cluster_width).sum()
}

// Cuts `s` to at most `max` display columns. Control characters (newlines, tabs) become spaces.
pub fn clip(s: &str, max: usize) -> String {
    let flat: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let mut out = String::new();
    let mut used = 0;
    for g in grapheme_clusters(&flat) {
        let w = cluster_width(g);
        if used + w > max {
            break;
        }
        out.push_str(g);
        used += w;
    }
    out
}

// Spinner glyph for the time since the spinner started. Frames advance every 125 ms.
pub fn spinner_frame(elapsed: Duration) -> char {
    let step = (elapsed.as_millis() / SPIN_STEP.as_millis()) as usize;
    SPIN_FRAMES[step % SPIN_FRAMES.len()]
}

// Main row suffix while a turn runs: " · Mm SSs", then " · <last tool>" when known (mainStatusText tui.go:888-898).
// Unlike fmt_secs, the minutes field is always present.
pub fn elapsed_suffix(secs: u64, last_tool: &str) -> String {
    let mut s = format!(" · {}m{:02}s", secs / 60, secs % 60);
    if !last_tool.is_empty() {
        s.push_str(" · ");
        s.push_str(last_tool);
    }
    s
}

// Status line suffix after the context bar (statusLine tui.go:1193-1210): " · rot N", optional " · mode X"
// (None for the default mode), then " · agents N running" with " (bg M)" when some run in the background.
pub fn status_suffix(rot: u32, mode: Option<&str>, fg: usize, bg: usize) -> String {
    let mut s = format!(" · rot {rot}");
    if let Some(m) = mode {
        s.push_str(&format!(" · mode {m}"));
    }
    if fg + bg > 0 {
        s.push_str(&format!(" · agents {} running", fg + bg));
        if bg > 0 {
            s.push_str(&format!(" (bg {bg})"));
        }
    }
    s
}

// agentState (tui.go:1505-1518): failed, then interrupted, then done; never started means queued.
pub fn agent_state(failed: bool, interrupted: bool, done: bool, started: bool) -> AgentState {
    if failed {
        AgentState::Failed
    } else if interrupted {
        AgentState::Interrupted
    } else if done {
        AgentState::Done
    } else if !started {
        AgentState::Queued
    } else {
        AgentState::Running
    }
}

// Which thinking row to draw under the transcript (tui.go:383-391). A running foreground agent alone is not
// enough: the row shows only when Opus or a resident session is busy, or no text has streamed yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingRow {
    None,
    Status,
    StatusWithEsc,
}

pub fn thinking_row(
    fg_running: bool,
    opus_running: bool,
    resident_busy: bool,
    got_text: bool,
    perm_pending: bool,
    busy: bool,
) -> ThinkingRow {
    if perm_pending {
        return ThinkingRow::None;
    }
    if (fg_running || opus_running || resident_busy) && (opus_running || resident_busy || !got_text)
    {
        ThinkingRow::Status
    } else if busy && !got_text {
        ThinkingRow::StatusWithEsc
    } else {
        ThinkingRow::None
    }
}

// The agent's model as the row shows it: "claude-haiku-5-5" → "haiku 5.5", "claude-3-5-sonnet-20241022" →
// "sonnet 3.5", "haiku" → "haiku". Empty stays empty, so the caller omits the segment (no session-model fallback).
pub fn short_model(model: &str) -> String {
    let parts: Vec<&str> = model
        .trim_start_matches("claude-")
        .split('-')
        .filter(|p| !p.is_empty())
        .collect();
    let Some(family) = parts
        .iter()
        .find(|p| p.chars().all(|c| c.is_ascii_alphabetic()))
    else {
        return String::new();
    };
    let versions: Vec<&str> = parts
        .iter()
        .filter(|p| p.len() <= 2 && p.chars().all(|c| c.is_ascii_digit()))
        .take(2)
        .copied()
        .collect();
    if versions.is_empty() {
        family.to_string()
    } else {
        format!("{family} {}", versions.join("."))
    }
}

// Agent row text after its time: "<state> · <agent type> · <short model> (<summary>)". Empty name becomes "agent";
// an empty model drops its segment. The caller passes the prompt line as summary when no task is set.
pub fn agent_status_text(state: AgentState, model: &str, name: &str, summary: &str) -> String {
    agent_status_text_in(i18n::current(), state, model, name, summary)
}

// The same row in an explicit language (the state label is the only translated part).
pub fn agent_status_text_in(
    lang: Lang,
    state: AgentState,
    model: &str,
    name: &str,
    summary: &str,
) -> String {
    let name = if name.is_empty() { "agent" } else { name };
    let label = state.label_in(lang);
    let short = short_model(model);
    if short.is_empty() {
        format!("{label} · {name} ({summary})")
    } else {
        format!("{label} · {name} · {short} ({summary})")
    }
}

// doneLine (drain.go:39-55): the one-line completion notice of a finished agent. Whitespace is collapsed and
// the result is capped at 120 characters (trunc counts runes in Go).
pub fn done_line(failed: bool, model: &str, name: &str, what: &str) -> String {
    let kind = if failed {
        i18n::t(Msg::AgentFailedTag)
    } else {
        i18n::t(Msg::AgentDoneTag)
    };
    let model = if model.is_empty() {
        i18n::t(Msg::Inherit)
    } else {
        model
    };
    let joined = format!("{kind} {model} {name} · {what}");
    joined
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect()
}

// Notice for a claude process left over from a dead harness (repl.go:1435).
pub fn orphan_notice(pid: u32, model: &str, cwd: &str) -> String {
    format!(
        "[orphan] claude pid {pid} ({model}, {cwd}) still running after its harness died; left running"
    )
}

// Budget stop line (repl.go:839). Both amounts use four decimals, as in Go's %.4f.
pub fn budget_notice(spent: f64, cap: f64) -> String {
    format!("[budget] ${spent:.4} / ${cap:.4} reached, task stopped")
}

// Context rotation line (repl.go:1019). The old session id is cut to 8 characters, or shown as "None".
pub fn rotated_notice(rotations: u32, reason: &str, old_session: &str) -> String {
    let old = if old_session.is_empty() {
        "None".to_string()
    } else {
        old_session.chars().take(8).collect()
    };
    format!("── context rotated #{rotations} ({reason}; {old} → new) ──")
}

// Shown after /clear starts a new session (repl.go:1412).
pub const NEW_SESSION_NOTICE: &str = "── new session ──";

// Quiet-wait notice (repl.go:788). The duration is written the way Go's time.Duration prints it.
pub fn quiet_notice(idle: Duration) -> String {
    format!(
        "[no output for {}; still waiting, Esc/Ctrl+C to interrupt]",
        go_duration(idle)
    )
}

// Agent list scroll indicator (tui.go:1490). `off` is the 0-based first shown row, `shown` the rows in the
// window, `total` all agents. Shows 1-based bounds, e.g. "  … 3-5/12 (마우스 휠로 스크롤)".
pub fn scroll_range_text(off: usize, shown: usize, total: usize) -> String {
    i18n::tf(Msg::ScrollHint, &[&(off + 1), &(off + shown), &total])
}

// Go time.Duration.String for whole seconds: "30s", "1m0s", "1h0m0s". Sub-second parts are dropped.
pub fn go_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m{}s", s / 60, s % 60)
    } else {
        format!("{}h{}m{}s", s / 3600, (s % 3600) / 60, s % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[test]
    fn scroll_range_text_is_one_based() {
        assert_eq!(
            scroll_range_text(2, 3, 12),
            "  … 3-5/12 (마우스 휠로 스크롤)"
        );
    }

    #[test]
    fn durations_and_token_counts_are_short() {
        assert_eq!(fmt_secs(5), "5s");
        assert_eq!(fmt_secs(125), "2m05s");
        assert_eq!(fmt_tokens(950), "950");
        assert_eq!(fmt_tokens(12_345), "12.3k");
        assert_eq!(fmt_tokens(2_500_000), "2.5M");
    }

    #[test]
    fn fmt_k_matches_go_fmt_k() {
        // Go TestFmtK (unit_test.go:133); fmtLimit cases included.
        let cases = [
            (0, "0"),
            (999, "999"),
            (1_000, "1.00k"),
            (1_234, "1.23k"),
            (1_999, "1.99k"),
            (12_345, "12.3k"),
            (123_456, "123k"),
            (999_999, "999k"),
            (1_234_567, "1.23M"),
            (200_000, "200k"),
            (80_000, "80.0k"),
        ];
        for (n, want) in cases {
            assert_eq!(fmt_k(n), want, "fmt_k({n})");
        }
        assert_eq!(fmt_limit(200_000), "200k");
        assert_eq!(fmt_limit(80_000), "80k");
    }

    #[test]
    fn ctx_bar_row_matches_go_ctx_bar() {
        // Go TestCtxBar (unit_test.go:149): the M/P/T/C context part of the bottom status row.
        let empty = BarInput {
            limit: 200_000,
            ..BarInput::default()
        };
        assert!(rows(&empty, '⠋', 200)[1].starts_with(" M 0/200k P 0/80k T 0/80k C 0/80k"));
        let coder = BarInput {
            tokens: 12_345,
            limit: 200_000,
            role_tokens: BTreeMap::from([
                ("prompter".to_string(), 1_000),
                ("coder".to_string(), 999),
            ]),
            ..BarInput::default()
        };
        assert!(
            rows(&coder, '⠋', 200)[1].starts_with(" M 12.3k/200k P 1.00k/80k T 0/80k C 999/80k")
        );
    }

    #[test]
    fn idle_bar_without_status_says_so() {
        let b = BarInput {
            limit: 200_000,
            mode: "auto".to_string(),
            ..BarInput::default()
        };
        let [top, bottom] = rows(&b, '⠋', 80);
        assert_eq!(top, "idle");
        assert_eq!(
            bottom,
            " M 0/200k P 0/80k T 0/80k C 0/80k · rot 0 · mode auto"
        );
    }

    // The idle top row keeps the last turn's counts (k/M one decimal) and says "idle" when there are none.
    #[test]
    fn idle_top_row_shows_last_turn_tokens() {
        let b = BarInput {
            turn_tokens: Some((12_300, 1_200, 80_000)),
            ..BarInput::default()
        };
        let [top, _] = rows(&b, '⠋', 80);
        assert!(top.starts_with("idle · "), "{top}");
        for want in ["12.3k", "1.2k", "80.0k"] {
            assert!(top.contains(want), "{top}");
        }
        assert_eq!(fmt_tokens(999), "999");
        assert_eq!(fmt_tokens(1_250_000), "1.2M");
    }

    #[test]
    fn busy_bar_lists_agents_status_and_queue() {
        let b = BarInput {
            turn_secs: Some(12),
            agents: vec![("리뷰".to_string(), 3)],
            model: "opus".to_string(),
            tokens: 12_345,
            cost_usd: 0.042,
            context_pct: 6,
            queued: 2,
            limit: 200_000,
            mode: "auto".to_string(),
            fg: 1,
            ..BarInput::default()
        };
        let [top, bottom] = rows(&b, '⠙', 100);
        assert_eq!(top, "⠙ running 12s · agent 리뷰 3s");
        assert_eq!(
            bottom,
            " M 12.3k/200k P 0/80k T 0/80k C 0/80k · rot 0 · mode auto · agents 1 running · queued 2"
        );
    }

    #[test]
    fn role_tokens_fill_the_prompter_tui_coder_slots() {
        let role_tokens =
            BTreeMap::from([("prompter".to_string(), 1_500), ("tui".to_string(), 12_000)]);
        let b = BarInput {
            role_tokens,
            ..BarInput::default()
        };
        let [_, bottom] = rows(&b, ' ', 200);
        assert!(
            bottom.contains(" P 1.50k/80k T 12.0k/80k C 0/80k"),
            "{bottom}"
        );
    }

    #[test]
    fn rows_are_clipped_by_display_width_not_char_count() {
        assert_eq!(clip("가나다라", 5), "가나");
        assert_eq!(clip("abc\ndef", 10), "abc def");
        let b = BarInput {
            turn_secs: Some(1),
            agents: vec![("가".repeat(30), 1)],
            ..BarInput::default()
        };
        let [top, _] = rows(&b, ' ', 20);
        assert!(
            unicode_width::UnicodeWidthStr::width(top.as_str()) <= 20,
            "{top}"
        );
    }

    // A long first agent must not push a later (resumed) agent off the top row: each keeps a share.
    #[test]
    fn every_running_agent_stays_visible_when_row_is_narrow() {
        let b = BarInput {
            turn_secs: Some(60),
            agents: vec![
                (
                    "Resume TUI A workstream with a very long description".to_string(),
                    120,
                ),
                ("Resume engine workstream".to_string(), 5),
            ],
            ..BarInput::default()
        };
        let [top, _] = rows(&b, '⠙', 80);
        assert!(cells(&top) <= 80, "{top}");
        assert!(top.contains("agent Resume TUI A"), "{top}");
        assert!(top.contains("agent Resume engine"), "{top}");
        assert!(top.contains(" 5s"), "{top}");
    }

    // Row text: "<state> · <agent type> · <short model> (<summary>)"; the model is the agent's own, else omitted.
    #[test]
    fn agent_status_text_uses_state_model_and_name() {
        let running = AgentState::Running;
        assert_eq!(
            agent_status_text(running, "claude-haiku-5-5", "haiku-scout", "scan"),
            "실행 중 · haiku-scout · haiku 5.5 (scan)"
        );
        assert_eq!(
            agent_status_text(running, "", "fixer", "버그 수정"),
            "실행 중 · fixer (버그 수정)"
        );
        assert_eq!(
            agent_status_text(AgentState::Done, "", "", "t"),
            "완료 · agent (t)"
        );
        assert_eq!(
            agent_status_text_in(Lang::En, running, "claude-haiku-5-5", "haiku-scout", "scan"),
            "running · haiku-scout · haiku 5.5 (scan)"
        );
        assert_eq!(
            agent_status_text_in(Lang::En, AgentState::Interrupted, "", "fixer", "stop"),
            "interrupted · fixer (stop)"
        );
        assert_eq!(agent_state(true, false, true, true), AgentState::Failed);
        assert_eq!(agent_state(false, false, false, false), AgentState::Queued);
        assert_eq!(
            agent_state(false, true, true, true),
            AgentState::Interrupted
        );
        assert_eq!(agent_state(false, false, false, true), AgentState::Running);
    }

    // The model segment is the agent's own model, shortened to family and version.
    #[test]
    fn short_model_reads_family_and_version() {
        assert_eq!(short_model("claude-haiku-5-5"), "haiku 5.5");
        assert_eq!(short_model("claude-opus-5-5"), "opus 5.5");
        assert_eq!(short_model("claude-3-5-sonnet-20241022"), "sonnet 3.5");
        assert_eq!(short_model("haiku"), "haiku");
        assert_eq!(short_model("claude-haiku-test"), "haiku");
        assert_eq!(short_model(""), "");
        // A restored agent that ran on opus keeps opus even when its name says haiku.
        assert_eq!(
            agent_status_text(
                AgentState::Interrupted,
                "claude-opus-5-5",
                "haiku-runner",
                "t"
            ),
            "중단 · haiku-runner · opus 5.5 (t)"
        );
    }

    // The coder line lists every live resident coder in spawn order and hides when none.
    #[test]
    fn coder_line_lists_live_coders_and_hides_when_empty() {
        assert_eq!(coder_line(&[], 80), None);
        let coders = [
            CoderContext {
                slot: 1,
                tokens: 12_000,
                limit: 80_000,
            },
            CoderContext {
                slot: 2,
                tokens: 3_100,
                limit: 80_000,
            },
        ];
        assert_eq!(
            coder_line(&coders, 80).as_deref(),
            Some("C1 12k/80k  C2 3.10k/80k")
        );
        assert_eq!(coder_line(&coders, 10).as_deref(), Some("C1 12k/80k"));
        // An idle resident coder with no tokens yet does not show a 0/80k line.
        let idle = [CoderContext {
            slot: 1,
            tokens: 0,
            limit: 80_000,
        }];
        assert_eq!(coder_line(&idle, 80), None);
    }

    // Port of the thinking-row gate in tuiModel.view (tui.go:383-391).
    #[test]
    fn thinking_row_shows_only_for_opus_resident_or_pre_text() {
        use ThinkingRow::*;
        // foreground agent alone after text started: hidden
        assert_eq!(thinking_row(true, false, false, true, false, true), None);
        // foreground agent before any text: shown
        assert_eq!(thinking_row(true, false, false, false, false, true), Status);
        // opus running after text: shown
        assert_eq!(thinking_row(false, true, false, true, false, true), Status);
        // resident busy after text: shown
        assert_eq!(thinking_row(false, false, true, true, false, true), Status);
        // busy, no text, nothing running: esc hint
        assert_eq!(
            thinking_row(false, false, false, false, false, true),
            StatusWithEsc
        );
        // permission prompt pending hides everything
        assert_eq!(thinking_row(false, true, true, false, true, true), None);
        // idle
        assert_eq!(thinking_row(false, false, false, false, false, false), None);
    }

    #[test]
    fn agent_dot_colors_match_go() {
        assert_eq!(AgentState::Running.dot_color(), 220);
        assert_eq!(AgentState::Done.dot_color(), 42);
        assert_eq!(AgentState::Failed.dot_color(), 196);
        assert_eq!(AgentState::Queued.dot_color(), 245);
        assert_eq!(AgentState::Interrupted.dot_color(), 208);
    }

    #[test]
    fn spinner_steps_every_125_ms() {
        assert_eq!(spinner_frame(Duration::from_millis(0)), '✻');
        assert_eq!(spinner_frame(Duration::from_millis(124)), '✻');
        assert_eq!(spinner_frame(Duration::from_millis(125)), '✽');
        assert_eq!(spinner_frame(Duration::from_millis(625)), '·');
        assert_eq!(spinner_frame(Duration::from_millis(750)), '✻');
    }

    #[test]
    fn main_elapsed_suffix_always_shows_minutes() {
        assert_eq!(elapsed_suffix(5, ""), " · 0m05s");
        assert_eq!(elapsed_suffix(125, "Bash"), " · 2m05s · Bash");
    }

    #[test]
    fn status_suffix_lists_rotation_mode_and_agents() {
        assert_eq!(status_suffix(0, None, 0, 0), " · rot 0");
        assert_eq!(
            status_suffix(2, Some("plan"), 1, 2),
            " · rot 2 · mode plan · agents 3 running (bg 2)"
        );
        assert_eq!(status_suffix(1, None, 2, 0), " · rot 1 · agents 2 running");
    }

    #[test]
    fn done_line_collapses_whitespace_and_caps_length() {
        assert_eq!(
            done_line(false, "", "fixer", "  버그\n수정 "),
            "[agent 완료] 상속 fixer · 버그 수정"
        );
        assert_eq!(
            done_line(true, "haiku", "x", "y"),
            "[agent 실패] haiku x · y"
        );
        let long = "가".repeat(200);
        assert_eq!(done_line(false, "m", "n", &long).chars().count(), 120);
    }

    #[test]
    fn stage_and_fixed_notices_have_go_text() {
        assert_eq!(NEW_SESSION_NOTICE, "── new session ──");
    }

    #[test]
    fn orphan_budget_and_rotation_notices() {
        assert_eq!(
            orphan_notice(4242, "opus", "/w"),
            "[orphan] claude pid 4242 (opus, /w) still running after its harness died; left running"
        );
        assert_eq!(
            budget_notice(1.5, 2.0),
            "[budget] $1.5000 / $2.0000 reached, task stopped"
        );
        assert_eq!(
            rotated_notice(3, "tokens", "abcdefghijkl"),
            "── context rotated #3 (tokens; abcdefgh → new) ──"
        );
        assert_eq!(
            rotated_notice(1, "manual", ""),
            "── context rotated #1 (manual; None → new) ──"
        );
    }

    #[test]
    fn quiet_notice_uses_go_duration_text() {
        assert_eq!(go_duration(Duration::from_secs(30)), "30s");
        assert_eq!(go_duration(Duration::from_secs(60)), "1m0s");
        assert_eq!(go_duration(Duration::from_secs(65)), "1m5s");
        assert_eq!(go_duration(Duration::from_secs(30 * 60)), "30m0s");
        assert_eq!(go_duration(Duration::from_secs(3661)), "1h1m1s");
        assert_eq!(
            quiet_notice(Duration::from_secs(30 * 60)),
            "[no output for 30m0s; still waiting, Esc/Ctrl+C to interrupt]"
        );
    }
}
