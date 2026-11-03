// Formatting for the two status rows above the input line. Pure functions only, so they are unit-tested
// without a terminal. Every row is clipped to the given display width so it never wraps.
// The Go parity helpers below (status suffix, spinner, agent rows, notices) take their data as arguments;
// the callers that feed them from UiEvent are not wired yet, hence the module-level allow.
#![allow(dead_code)]

use std::time::Duration;

use unicode_width::UnicodeWidthChar;

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
        match self {
            AgentState::Running => "실행 중",
            AgentState::Done => "완료",
            AgentState::Failed => "실패",
            AgentState::Queued => "대기",
            AgentState::Interrupted => "중단",
        }
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

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BarInput {
    // Seconds since the current turn started; None while idle.
    pub turn_secs: Option<u64>,
    // Running subagents as (description, elapsed seconds).
    pub agents: Vec<(String, u64)>,
    pub model: String,
    pub tokens: u64,
    pub cost_usd: f64,
    pub context_pct: u32,
    pub queued: usize,
    // Main-context limit in tokens (the welcome threshold); used by the idle Go-style status row.
    pub limit: u64,
    // Status line extras (Go statusLine): rate window text, rotations, permission mode ("default" or empty is
    // hidden), and foreground / background agent counts.
    pub rate: Option<String>,
    pub rot: u32,
    pub mode: String,
    pub fg: usize,
    pub bg: usize,
}

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

// fmtK-style token count for the ctx bar: "0" below 1000, otherwise thousands with a "k" suffix.
fn fmt_k(n: u64) -> String {
    if n < 1_000 {
        n.to_string()
    } else {
        format!("{}k", n / 1_000)
    }
}

pub fn fmt_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m{:02}s", secs / 60, secs % 60)
    }
}

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
    let mut top = match b.turn_secs {
        Some(s) => format!("{spinner} running {}", fmt_secs(s)),
        None => "idle".to_string(),
    };
    for (desc, secs) in &b.agents {
        top.push_str(&format!(" · agent {desc} {}", fmt_secs(*secs)));
    }
    // Go statusLine + ctxBar (tui.go:1193). M is the main context from UiEvent::Status; P/T/C stay 0 because
    // no UiEvent carries per-role context yet.
    let mode = Some(b.mode.as_str()).filter(|m| !m.is_empty() && *m != "default");
    let rate = b
        .rate
        .as_ref()
        .map(|r| format!(" · {r}"))
        .unwrap_or_default();
    let mut bottom = format!(
        " M {}/{} P 0/{r} T 0/{r} C 0/{r}{rate}{}",
        fmt_k(b.tokens),
        fmt_k(b.limit),
        status_suffix(b.rot, mode, b.fg, b.bg),
        r = ROLE_LIMIT
    );
    if b.queued > 0 {
        bottom.push_str(&format!(" · queued {}", b.queued));
    }
    [clip(&top, width), clip(&bottom, width)]
}

// Cuts `s` to at most `max` display columns. Control characters (newlines, tabs) become spaces.
pub fn clip(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for c in s.chars() {
        let c = if c.is_control() { ' ' } else { c };
        let w = c.width().unwrap_or(0);
        if used + w > max {
            break;
        }
        out.push(c);
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

// hasModelPrefix (tui.go:1546-1554): the name already starts with a model id, so the model segment is skipped.
pub fn has_model_prefix(name: &str) -> bool {
    ["haiku-", "opus-", "sonnet-", "fable-"]
        .iter()
        .any(|p| name.starts_with(p))
}

// agentStatusText (tui.go:1527-1543): "<state>-<model>-<name>(<summary>)". Empty model falls back to the
// session model, empty name becomes "agent". The caller passes the prompt line as summary when no task is set.
pub fn agent_status_text(
    state: AgentState,
    model: &str,
    session_model: &str,
    name: &str,
    summary: &str,
) -> String {
    let model = if model.is_empty() {
        session_model
    } else {
        model
    };
    let name = if name.is_empty() { "agent" } else { name };
    let label = state.label();
    if has_model_prefix(name) {
        format!("{label}-{name}({summary})")
    } else {
        format!("{label}-{model}-{name}({summary})")
    }
}

// doneLine (drain.go:39-55): the one-line completion notice of a finished agent. Whitespace is collapsed and
// the result is capped at 120 characters (trunc counts runes in Go).
pub fn done_line(failed: bool, model: &str, name: &str, what: &str) -> String {
    let kind = if failed {
        "[agent 실패]"
    } else {
        "[agent 완료]"
    };
    let model = if model.is_empty() { "상속" } else { model };
    let joined = format!("{kind} {model} {name} · {what}");
    joined
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect()
}

// Stage notices shown as dim lines (repl.go:649, 722, 752, 804).
pub const STAGE_MAIN_START: &str = "· main 처리중";
pub const STAGE_MAIN_DONE: &str = "· main 완료";

pub fn stage_agent_call(agent_type: &str) -> String {
    format!("· agent {agent_type} 호출")
}

pub fn stage_agent_done(agent_type: &str) -> String {
    format!("· agent {agent_type} 완료")
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

    #[test]
    fn durations_and_token_counts_are_short() {
        assert_eq!(fmt_secs(5), "5s");
        assert_eq!(fmt_secs(125), "2m05s");
        assert_eq!(fmt_tokens(950), "950");
        assert_eq!(fmt_tokens(12_345), "12.3k");
        assert_eq!(fmt_tokens(2_500_000), "2.5M");
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
            " M 12k/200k P 0/80k T 0/80k C 0/80k · rot 0 · mode auto · agents 1 running · queued 2"
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

    // Port of TestAgentStatusFormat (panel_test.go:340), text parts only.
    #[test]
    fn agent_status_text_uses_state_model_and_name() {
        let running = AgentState::Running;
        assert_eq!(
            agent_status_text(running, "", "opus", "", "t"),
            "실행 중-opus-agent(t)"
        );
        assert_eq!(
            agent_status_text(running, "haiku", "opus", "fixer", "버그 수정"),
            "실행 중-haiku-fixer(버그 수정)"
        );
        assert!(agent_status_text(AgentState::Done, "", "m", "", "t").starts_with("완료"));
        assert_eq!(agent_state(true, false, true, true), AgentState::Failed);
        assert_eq!(agent_state(false, false, false, false), AgentState::Queued);
        assert_eq!(
            agent_state(false, true, true, true),
            AgentState::Interrupted
        );
        assert_eq!(agent_state(false, false, false, true), AgentState::Running);
    }

    // Port of TestAgentStatusTextPrefixedName (panel_test.go:314).
    #[test]
    fn agent_name_with_model_prefix_skips_model_segment() {
        assert!(has_model_prefix("haiku-scout"));
        assert!(!has_model_prefix("scout-haiku"));
        assert_eq!(
            agent_status_text(AgentState::Running, "", "haiku", "haiku-scout", "t"),
            "실행 중-haiku-scout(t)"
        );
        assert_eq!(
            agent_status_text(AgentState::Running, "", "opus", "opus-architect", "t"),
            "실행 중-opus-architect(t)"
        );
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
        assert_eq!(STAGE_MAIN_START, "· main 처리중");
        assert_eq!(STAGE_MAIN_DONE, "· main 완료");
        assert_eq!(stage_agent_call("Explore"), "· agent Explore 호출");
        assert_eq!(stage_agent_done("Explore"), "· agent Explore 완료");
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
