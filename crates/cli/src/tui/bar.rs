// Formatting for the two status rows above the input line. Pure functions only, so they are unit-tested
// without a terminal. Every row is clipped to the given display width so it never wraps.

use unicode_width::UnicodeWidthChar;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BarInput {
    // Seconds since the current turn started; None while idle.
    pub turn_secs: Option<u64>,
    // Running subagents as (description, elapsed seconds).
    pub agents: Vec<(String, u64)>,
    pub has_status: bool,
    pub model: String,
    pub tokens: u64,
    pub cost_usd: f64,
    pub context_pct: u32,
    pub queued: usize,
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
    let mut bottom = if b.has_status {
        format!(
            "{} · tokens {} · ${:.3} · ctx {}%",
            b.model,
            fmt_tokens(b.tokens),
            b.cost_usd,
            b.context_pct
        )
    } else {
        "no status yet".to_string()
    };
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
        let [top, bottom] = rows(&BarInput::default(), '⠋', 80);
        assert_eq!(top, "idle");
        assert_eq!(bottom, "no status yet");
    }

    #[test]
    fn busy_bar_lists_agents_status_and_queue() {
        let b = BarInput {
            turn_secs: Some(12),
            agents: vec![("리뷰".to_string(), 3)],
            has_status: true,
            model: "opus".to_string(),
            tokens: 12_345,
            cost_usd: 0.042,
            context_pct: 6,
            queued: 2,
        };
        let [top, bottom] = rows(&b, '⠙', 80);
        assert_eq!(top, "⠙ running 12s · agent 리뷰 3s");
        assert_eq!(bottom, "opus · tokens 12.3k · $0.042 · ctx 6% · queued 2");
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
}
