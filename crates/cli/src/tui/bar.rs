// Layout of the two status rows above the input line, the coder line, and the agent dot. The words come from core
// bar_text (i18n); this file adds what needs the terminal: clipping to display width, the dot colour palette index,
// and the thinking-row choice. Every row is clipped to the given width so it never wraps.

use chaude_core::api::CoderContext;
use chaude_core::bar_text;
use chaude_core::i18n;

use super::render::{cluster_width, grapheme_clusters};

pub use bar_text::{AgentState, elapsed_suffix, fmt_secs, fmt_tokens, spinner_frame};
pub use chaude_core::ui_state::BarInput;

// xterm-256 index for the coloured ● (agentDot tui.go:1520-1523). Only the dot is coloured; the text stays grey.
pub fn agent_dot_color(state: AgentState) -> u8 {
    match state {
        AgentState::Running => 220,
        AgentState::Done => 42,
        AgentState::Failed => 196,
        AgentState::Queued => 245,
        AgentState::Interrupted => 208,
    }
}

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

// Returns the two status rows: running state and agents, then model/tokens/cost/context/queue.
pub fn rows(b: &BarInput, spinner: char, width: usize) -> [String; 2] {
    let lang = i18n::current();
    let mut top = bar_text::top_text(lang, b, spinner);
    // Each running agent keeps a visible segment. When they do not all fit, every description is cut to an
    // equal share of the room left, so a long first agent cannot push later (e.g. resumed) agents off the row.
    let room = width.saturating_sub(cells(&top));
    let full: usize = b
        .agents
        .iter()
        .map(|(desc, secs)| cells(&bar_text::agent_segment(lang, desc, *secs)))
        .sum();
    let share = room / b.agents.len().max(1);
    for (desc, secs) in &b.agents {
        let desc = if full > room && b.agents.len() > 1 {
            let fixed = cells(&bar_text::agent_segment(lang, "", *secs));
            clip(desc, share.saturating_sub(fixed))
        } else {
            desc.clone()
        };
        top.push_str(&bar_text::agent_segment(lang, &desc, *secs));
    }
    [
        clip(&top, width),
        clip(&bar_text::bottom_text(lang, b), width),
    ]
}

// The coder line under the input box: "C1 12k/80k  C2 3.1k/80k". None when no coder has context tokens yet.
pub fn coder_line(coders: &[CoderContext], width: usize) -> Option<String> {
    bar_text::coder_text(coders).map(|text| clip(&text, width))
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

// Row text after an agent's time, in the current language (see bar_text::agent_status_text).
pub fn agent_status_text(state: AgentState, model: &str, name: &str, summary: &str) -> String {
    bar_text::agent_status_text(i18n::current(), state, model, name, summary)
}

// Completion line of a finished agent, in the current language (see bar_text::done_line).
pub fn done_line(failed: bool, model: &str, name: &str, what: &str) -> String {
    bar_text::done_line(i18n::current(), failed, model, name, what)
}

// Transcript notices, in the current language.
pub fn orphan_notice(pid: u32, model: &str, cwd: &str) -> String {
    bar_text::orphan_notice(i18n::current(), pid, model, cwd)
}

pub fn budget_notice(spent: f64, cap: f64) -> String {
    bar_text::budget_notice(i18n::current(), spent, cap)
}

pub fn rotated_notice(rotations: u32, reason: &str, old_session: &str) -> String {
    bar_text::rotated_notice(i18n::current(), rotations, reason, old_session)
}

pub fn new_session_notice() -> String {
    bar_text::new_session_notice(i18n::current())
}

pub fn quiet_notice(idle: std::time::Duration) -> String {
    bar_text::quiet_notice(i18n::current(), idle)
}

pub fn scroll_range_text(off: usize, shown: usize, total: usize) -> String {
    bar_text::scroll_range_text(i18n::current(), off, shown, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

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
        assert_eq!(agent_dot_color(AgentState::Running), 220);
        assert_eq!(agent_dot_color(AgentState::Done), 42);
        assert_eq!(agent_dot_color(AgentState::Failed), 196);
        assert_eq!(agent_dot_color(AgentState::Queued), 245);
        assert_eq!(agent_dot_color(AgentState::Interrupted), 208);
    }
}
