//! Text of the status bar, the agent rows and the transcript notices, shared by the TUI (cli tui/bar.rs) and the GUI.
//! Pure and width-free: callers clip to their own display width. Every word comes from core i18n for an explicit
//! language, so the TUI wrappers pass i18n::current() and tests can pin a language.

use std::time::Duration;

use crate::api::CoderContext;
use crate::i18n::{self, Lang, Msg};
use crate::ui_state::BarInput;

// Spinner frames and step from tui.go:259-260 (spinner.Frames, FPS = 1/8 s).
const SPIN_FRAMES: [char; 6] = ['✻', '✽', '✶', '✲', '✢', '·'];
const SPIN_STEP: Duration = Duration::from_millis(125);

// Per-role context limit shown in the Go ctx bar (haikuThreshold in repl.go).
const ROLE_LIMIT: &str = "80k";

// Agent state (tui.go:1498-1501). The dot colour lives in the TUI because it is a terminal palette index.
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
}

// Template fill for an explicit language (i18n::tf reads the process language).
fn fill_in(lang: Lang, msg: Msg, args: &[&dyn std::fmt::Display]) -> String {
    i18n::fill(i18n::text(lang, msg), args)
}

// Go fmtK (repl.go:142-162): "0" to "999", then k or M with 2, 1 or 0 decimals by magnitude.
pub fn fmt_k(n: u64) -> String {
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
pub fn fmt_limit(n: u64) -> String {
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
fn status_suffix(lang: Lang, rot: u32, mode: Option<&str>, fg: usize, bg: usize) -> String {
    let mut s = format!(" · {}", fill_in(lang, Msg::BarRot, &[&rot]));
    if let Some(m) = mode {
        s.push_str(&format!(" · {}", fill_in(lang, Msg::BarMode, &[&m])));
    }
    if fg + bg > 0 {
        s.push_str(&format!(
            " · {}",
            fill_in(lang, Msg::BarAgentsRunning, &[&(fg + bg)])
        ));
        if bg > 0 {
            s.push_str(&format!(" {}", fill_in(lang, Msg::BarBg, &[&bg])));
        }
    }
    s
}

// One running agent segment of the top row: " · agent <desc> <time>". An empty description leaves a double space,
// which the TUI uses to measure the fixed part of the segment.
pub fn agent_segment(lang: Lang, desc: &str, secs: u64) -> String {
    format!(
        " · {}",
        fill_in(lang, Msg::BarAgent, &[&desc, &fmt_secs(secs)])
    )
}

// Top status row before the agent segments: the running state, or held replies, else the idle row with the last
// turn's tokens. The spinner glyph leads the running forms.
pub fn top_text(lang: Lang, b: &BarInput, spinner: char) -> String {
    match (b.turn_secs, b.waiting) {
        (Some(s), Some(n)) => format!(
            "{spinner} {} {}",
            fill_in(lang, Msg::AgentsWaiting, &[&n]),
            fmt_secs(s)
        ),
        (None, Some(n)) => fill_in(lang, Msg::AgentsWaiting, &[&n]),
        (Some(s), None) => format!(
            "{spinner} {}",
            fill_in(lang, Msg::BarRunning, &[&fmt_secs(s)])
        ),
        (None, None) => match b.turn_tokens {
            Some((input, output, cache)) => format!(
                "{} · {}",
                i18n::text(lang, Msg::BarIdle),
                fill_in(
                    lang,
                    Msg::TurnTokens,
                    &[&fmt_tokens(input), &fmt_tokens(output), &fmt_tokens(cache)]
                )
            ),
            None => i18n::text(lang, Msg::BarIdle).to_string(),
        },
    }
}

// Bottom status row: M (main context), P/T/C (role contexts), the rate window, then the status suffix and the queue.
// Go statusLine + ctxBar (tui.go:1193); M from UiEvent::Status, P/T/C from UiEvent::RoleContext.
pub fn bottom_text(lang: Lang, b: &BarInput) -> String {
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
        status_suffix(lang, b.rot, mode, b.fg, b.bg),
        r = ROLE_LIMIT
    );
    if b.queued > 0 {
        bottom.push_str(&format!(
            " · {}",
            fill_in(lang, Msg::BarQueued, &[&b.queued])
        ));
    }
    bottom
}

// The coder line under the input box: "C1 12k/80k  C2 3.1k/80k", one entry per live coder in spawn order (used
// context tokens over the rotation limit). A coder with no context tokens yet (the resident one, idle at start)
// is not listed; with none listed the line is None.
pub fn coder_text(coders: &[CoderContext]) -> Option<String> {
    let shown: Vec<&CoderContext> = coders.iter().filter(|c| c.tokens > 0).collect();
    if shown.is_empty() {
        return None;
    }
    Some(
        shown
            .iter()
            .map(|c| format!("C{} {}/{}", c.slot, fmt_limit(c.tokens), fmt_limit(c.limit)))
            .collect::<Vec<_>>()
            .join("  "),
    )
}

// Single-line status for the GUI: model, context percent, tokens, cost, then mode (when not default), rotations and
// the rate window.
pub fn status_line(lang: Lang, b: &BarInput) -> String {
    let mut parts = vec![
        b.model.clone(),
        fill_in(lang, Msg::BarCtx, &[&b.context_pct]),
        fill_in(lang, Msg::BarTok, &[&fmt_tokens(b.tokens)]),
        format!("${:.2}", b.cost_usd),
    ];
    if !b.mode.is_empty() && b.mode != "default" {
        parts.push(fill_in(lang, Msg::BarMode, &[&b.mode]));
    }
    if b.rot > 0 {
        parts.push(fill_in(lang, Msg::BarRot, &[&b.rot]));
    }
    if let Some(rate) = &b.rate {
        parts.push(rate.clone());
    }
    parts.join(" · ")
}

// Single-line activity for the GUI: turn timer, agents, queue, held reply, per-role context and the last turn's
// tokens. Empty when idle.
pub fn activity_line(lang: Lang, b: &BarInput) -> String {
    let mut parts = Vec::new();
    if let Some(secs) = b.turn_secs {
        parts.push(fill_in(lang, Msg::BarTurn, &[&secs]));
    }
    if b.fg + b.bg > 0 {
        parts.push(fill_in(lang, Msg::BarAgentsFgBg, &[&b.fg, &b.bg]));
    }
    if b.queued > 0 {
        parts.push(fill_in(lang, Msg::BarQueued, &[&b.queued]));
    }
    if let Some(n) = b.waiting {
        parts.push(fill_in(lang, Msg::BarHeld, &[&n]));
    }
    for (role, tokens) in &b.role_tokens {
        parts.push(format!("{role} {}", fmt_tokens(*tokens)));
    }
    if let Some((input, output, cache)) = b.turn_tokens {
        parts.push(fill_in(
            lang,
            Msg::BarLastTokens,
            &[&fmt_tokens(input), &fmt_tokens(output), &fmt_tokens(cache)],
        ));
    }
    parts.join(" · ")
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
pub fn agent_status_text(
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
pub fn done_line(lang: Lang, failed: bool, model: &str, name: &str, what: &str) -> String {
    let kind = if failed {
        i18n::text(lang, Msg::AgentFailedTag)
    } else {
        i18n::text(lang, Msg::AgentDoneTag)
    };
    let model = if model.is_empty() {
        i18n::text(lang, Msg::Inherit)
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
pub fn orphan_notice(lang: Lang, pid: u32, model: &str, cwd: &str) -> String {
    fill_in(lang, Msg::NoticeOrphan, &[&pid, &model, &cwd])
}

// Budget stop line (repl.go:839). Both amounts use four decimals, as in Go's %.4f.
pub fn budget_notice(lang: Lang, spent: f64, cap: f64) -> String {
    fill_in(
        lang,
        Msg::NoticeBudget,
        &[&format!("{spent:.4}"), &format!("{cap:.4}")],
    )
}

// Context rotation line (repl.go:1019). The old session id is cut to 8 characters, or shown as "None".
pub fn rotated_notice(lang: Lang, rotations: u32, reason: &str, old_session: &str) -> String {
    let old: String = if old_session.is_empty() {
        "None".to_string()
    } else {
        old_session.chars().take(8).collect()
    };
    fill_in(lang, Msg::NoticeRotated, &[&rotations, &reason, &old])
}

// Shown after /clear starts a new session (repl.go:1412).
pub fn new_session_notice(lang: Lang) -> String {
    i18n::text(lang, Msg::NoticeNewSession).to_string()
}

// Quiet-wait notice (repl.go:788). The duration is written the way Go's time.Duration prints it.
pub fn quiet_notice(lang: Lang, idle: Duration) -> String {
    fill_in(lang, Msg::NoticeQuiet, &[&go_duration(idle)])
}

// Agent list scroll indicator (tui.go:1490). `off` is the 0-based first shown row, `shown` the rows in the
// window, `total` all agents. Shows 1-based bounds, e.g. "  … 3-5/12 (마우스 휠로 스크롤)".
pub fn scroll_range_text(lang: Lang, off: usize, shown: usize, total: usize) -> String {
    fill_in(lang, Msg::ScrollHint, &[&(off + 1), &(off + shown), &total])
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

    // Moved from gui app.rs (deleted in b41d445) so the status bar text keeps its test in core.
    #[test]
    fn fmt_k_keeps_small_and_shortens_large() {
        assert_eq!(fmt_k(999), "999");
        assert_eq!(fmt_k(1500), "1.50k");
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
        assert_eq!(
            status_line(Lang::En, &bar),
            "opus · ctx 12% · 2.5k tok · $0.50"
        );
        let plan = BarInput {
            mode: "plan".into(),
            ..bar
        };
        assert!(status_line(Lang::En, &plan).ends_with("· mode plan"));
    }

    #[test]
    fn activity_line_is_empty_when_idle() {
        assert_eq!(activity_line(Lang::En, &BarInput::default()), "");
        let busy = BarInput {
            turn_secs: Some(7),
            queued: 2,
            ..Default::default()
        };
        assert_eq!(activity_line(Lang::En, &busy), "turn 7s · queued 2");
    }

    #[test]
    fn scroll_range_text_is_one_based() {
        assert_eq!(
            scroll_range_text(Lang::Ko, 2, 3, 12),
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

    // Row text: "<state> · <agent type> · <short model> (<summary>)"; the model is the agent's own, else omitted.
    #[test]
    fn agent_status_text_uses_state_model_and_name() {
        let running = AgentState::Running;
        assert_eq!(
            agent_status_text(Lang::Ko, running, "claude-haiku-5-5", "haiku-scout", "scan"),
            "실행 중 · haiku-scout · haiku 5.5 (scan)"
        );
        assert_eq!(
            agent_status_text(Lang::Ko, running, "", "fixer", "버그 수정"),
            "실행 중 · fixer (버그 수정)"
        );
        assert_eq!(
            agent_status_text(Lang::Ko, AgentState::Done, "", "", "t"),
            "완료 · agent (t)"
        );
        assert_eq!(
            agent_status_text(Lang::En, running, "claude-haiku-5-5", "haiku-scout", "scan"),
            "running · haiku-scout · haiku 5.5 (scan)"
        );
        assert_eq!(
            agent_status_text(Lang::En, AgentState::Interrupted, "", "fixer", "stop"),
            "interrupted · fixer (stop)"
        );
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
                Lang::Ko,
                AgentState::Interrupted,
                "claude-opus-5-5",
                "haiku-runner",
                "t"
            ),
            "중단 · haiku-runner · opus 5.5 (t)"
        );
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
        assert_eq!(status_suffix(Lang::En, 0, None, 0, 0), " · rot 0");
        assert_eq!(
            status_suffix(Lang::En, 2, Some("plan"), 1, 2),
            " · rot 2 · mode plan · agents 3 running (bg 2)"
        );
        assert_eq!(
            status_suffix(Lang::En, 1, None, 2, 0),
            " · rot 1 · agents 2 running"
        );
    }

    #[test]
    fn done_line_collapses_whitespace_and_caps_length() {
        assert_eq!(
            done_line(Lang::Ko, false, "", "fixer", "  버그\n수정 "),
            "[agent 완료] 상속 fixer · 버그 수정"
        );
        assert_eq!(
            done_line(Lang::Ko, true, "haiku", "x", "y"),
            "[agent 실패] haiku x · y"
        );
        let long = "가".repeat(200);
        assert_eq!(
            done_line(Lang::Ko, false, "m", "n", &long).chars().count(),
            120
        );
    }

    #[test]
    fn stage_and_fixed_notices_have_go_text() {
        assert_eq!(new_session_notice(Lang::En), "── new session ──");
    }

    #[test]
    fn orphan_budget_and_rotation_notices() {
        assert_eq!(
            orphan_notice(Lang::En, 4242, "opus", "/w"),
            "[orphan] claude pid 4242 (opus, /w) still running after its harness died; left running"
        );
        assert_eq!(
            budget_notice(Lang::En, 1.5, 2.0),
            "[budget] $1.5000 / $2.0000 reached, task stopped"
        );
        assert_eq!(
            rotated_notice(Lang::En, 3, "tokens", "abcdefghijkl"),
            "── context rotated #3 (tokens; abcdefgh → new) ──"
        );
        assert_eq!(
            rotated_notice(Lang::En, 1, "manual", ""),
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
            quiet_notice(Lang::En, Duration::from_secs(30 * 60)),
            "[no output for 30m0s; still waiting, Esc/Ctrl+C to interrupt]"
        );
    }
}
