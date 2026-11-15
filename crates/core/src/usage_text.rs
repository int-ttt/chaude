//! /usage text decoration shared by the TUI and the GUI: a bar after each percentage and a context bar (Go usagebar.go
//! renderBar, pctRe, decorateUsage). The text itself comes from one `claude -p /usage` run (api, UiEvent::Usage).

use crate::i18n::{Msg, t, tf};
use crate::slash::commas;

const BAR_CELLS: usize = 10;
// R15/R16 ctxBarMax (usagebar.go:9).
pub const CTX_BAR_MAX: i64 = 150_000;

// R15 renderBar (usagebar.go:18): "[████░░░░░░] 45%", fill clamped, " !" above 100%.
pub fn render_bar(pct: f64) -> String {
    let pct = if pct < 0.0 { 0.0 } else { pct };
    let n = ((pct / 100.0 * BAR_CELLS as f64 + 0.5) as usize).min(BAR_CELLS);
    let bar = format!("{}{}", "█".repeat(n), "░".repeat(BAR_CELLS - n));
    let mut s = format!("[{bar}] {}%", (pct + 0.5) as i64);
    if pct > 100.0 {
        s.push_str(" !");
    }
    s
}

// R17 pctRe (usagebar.go:37) `(\d+(?:\.\d+)?)%`: the value of the first percentage on a line.
pub fn pct_value(line: &str) -> Option<f64> {
    let b = line.as_bytes();
    for p in 0..b.len() {
        if !b[p].is_ascii_digit() {
            continue;
        }
        let mut q = digits_end(b, p);
        if q + 1 < b.len() && b[q] == b'.' && b[q + 1].is_ascii_digit() {
            q = digits_end(b, q + 1);
        }
        if b.get(q) == Some(&b'%') {
            return line[p..q].parse().ok();
        }
    }
    None
}

// Go usageFetch: the one-shot run failed; the error text follows. Core reports a result-less answer as
// "usage: no result", which Go shows as the empty text.
pub fn usage_failed(err: &str) -> String {
    if err == "usage: no result" {
        return t(Msg::UsageEmpty).to_string();
    }
    tf(Msg::UsageFailed, &[&err])
}

// The end of the run of ASCII digits that starts at j.
fn digits_end(b: &[u8], mut j: usize) -> usize {
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
    }
    j
}

// R16 decorateUsage (usagebar.go:41): a bar after each line's percentage, then a context bar against CTX_BAR_MAX.
pub fn decorate_usage(text: &str, ctx: i64) -> String {
    let lines: Vec<String> = text
        .split('\n')
        .map(|l| match pct_value(l) {
            Some(v) => format!("{l}  {}", render_bar(v)),
            None => l.to_string(),
        })
        .collect();
    let pct = ctx as f64 / CTX_BAR_MAX as f64 * 100.0;
    format!(
        "{}\n\nContext (now): {}  ({} / {})",
        lines.join("\n"),
        render_bar(pct),
        commas(ctx.max(0) as u64),
        commas(CTX_BAR_MAX as u64)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorate_adds_bars_and_context_line() {
        let out = decorate_usage("Current session: 45% used\nother", 75_000);
        assert!(out.starts_with("Current session: 45% used  ["), "{out}");
        assert!(
            out.contains("Context (now): [█████░░░░░] 50%  (75,000 / 150,000)"),
            "{out}"
        );
    }

    #[test]
    fn pct_value_and_bar_cases() {
        assert_eq!(pct_value("usage 12.5%"), Some(12.5));
        assert_eq!(pct_value("a1.2.3%"), Some(2.3));
        assert_eq!(pct_value("no number"), None);
        assert_eq!(render_bar(130.0), "[██████████] 130% !");
    }
}
