//! Live /usage poll row (Go harness usagepoll.go: parseUsage, pollUsageSync). The claude run itself is
//! `proc::one_shot_usage`; the throttle (usagePollEvery, usagePolling) belongs to the caller. This module
//! only turns the /usage text, or its error, into the stage=usage metrics row.

use serde_json::{Map, Value, json};
use std::io;

use crate::metrics::Metrics;

const SESSION_LABEL: &str = "Current session:";
const WEEK_LABEL: &str = "Current week (all models):";
// Go usagePollEvery: at most one /usage poll per this many seconds.
pub const USAGE_POLL_EVERY_SECS: u64 = 300;

// Go regex `\s*` in these patterns: ASCII whitespace.
fn skip_ws(s: &str) -> &str {
    s.trim_start_matches(|c: char| c.is_ascii_whitespace())
}

// One `<label>\s*<pct>%\s*used[\s*·\s*resets\s*<rest of line>]` match at one label occurrence.
// Returns (pct text, resets) where resets is trimmed and None when empty or absent. The pct text is
// parsed by the caller: Go matches the regex first, then fails the whole parse on a bad number.
fn match_at<'a>(text: &'a str, label_at: usize, label: &str) -> Option<(&'a str, Option<String>)> {
    let rest = skip_ws(&text[label_at + label.len()..]);
    let digits = rest
        .bytes()
        .take_while(|b| b.is_ascii_digit() || *b == b'.')
        .count();
    if digits == 0 {
        return None;
    }
    let pct = &rest[..digits];
    let rest = skip_ws(rest[digits..].strip_prefix('%')?);
    let rest = skip_ws(rest.strip_prefix("used")?);
    let resets = rest
        .strip_prefix('·')
        .map(skip_ws)
        .and_then(|r| r.strip_prefix("resets"))
        .map(|r| r.split('\n').next().unwrap_or("").trim().to_string())
        .filter(|r| !r.is_empty());
    Some((pct, resets))
}

// First label occurrence whose pattern matches, like Go FindStringSubmatch.
fn find_usage<'a>(text: &'a str, label: &str) -> Option<(&'a str, Option<String>)> {
    text.match_indices(label)
        .find_map(|(at, _)| match_at(text, at, label))
}

// Go parseUsage: the session line is required; the week line is optional. Keys: session_pct,
// session_resets, week_pct, week_resets (resets omitted when empty).
pub fn parse_usage(text: &str) -> Option<Map<String, Value>> {
    let (session_pct, session_resets) = find_usage(text, SESSION_LABEL)?;
    let session_pct: f64 = session_pct.parse().ok()?;
    let mut rec = Map::new();
    rec.insert("session_pct".into(), json!(session_pct));
    if let Some(r) = session_resets {
        rec.insert("session_resets".into(), json!(r));
    }
    if let Some((week_pct, week_resets)) = find_usage(text, WEEK_LABEL) {
        let Ok(week_pct) = week_pct.parse::<f64>() else {
            return Some(rec);
        };
        rec.insert("week_pct".into(), json!(week_pct));
        if let Some(r) = week_resets {
            rec.insert("week_resets".into(), json!(r));
        }
    }
    Some(rec)
}

// The stage=usage row for one poll (Go pollUsageSync). `result` is the /usage text or the run error.
pub fn usage_row(result: Result<&str, &str>) -> Value {
    let text = match result {
        Err(e) => return json!({"stage": "usage", "error": e}),
        Ok(t) => t,
    };
    match parse_usage(text) {
        Some(mut rec) => {
            rec.insert("stage".into(), json!("usage"));
            rec.insert("source".into(), json!("cli"));
            Value::Object(rec)
        }
        None => {
            // Go keeps the first 120 runes of the text for the parse-failure row.
            let head: String = text.chars().take(120).collect();
            json!({"stage": "usage", "error": "parse", "head": head})
        }
    }
}

// Appends the poll row to the metrics log. Errors are the caller's to ignore (Go discards them).
pub fn log_usage(metrics: &Metrics, result: Result<&str, &str>) -> io::Result<()> {
    metrics.log(usage_row(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::path::PathBuf;

    const LIVE: &str = "Current session: 18% used · resets Oct 30 at 12:49pm (Asia/Seoul)\n\
Current week (all models): 27% used · resets Nov 1 at 4:59pm (Asia/Seoul)";

    // Metrics rows (one JSON object per line) under dir.
    fn rows(dir: &std::path::Path) -> Vec<Value> {
        let mut out = Vec::new();
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .expect("read metrics dir")
            .map(|e| e.expect("entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
            .collect();
        files.sort();
        for f in files {
            for line in std::fs::read_to_string(f).expect("read jsonl").lines() {
                out.push(serde_json::from_str(line).expect("json row"));
            }
        }
        out
    }

    // Go TestParseUsage
    #[test]
    fn parse_usage_reads_session_and_week() {
        let rec = parse_usage(LIVE).expect("live sample is ok");
        assert_eq!(rec["session_pct"], 18.0);
        assert_eq!(rec["session_resets"], "Oct 30 at 12:49pm (Asia/Seoul)");
        assert_eq!(rec["week_pct"], 27.0);
        assert_eq!(rec["week_resets"], "Nov 1 at 4:59pm (Asia/Seoul)");

        assert!(parse_usage("Current week (all models): 27% used").is_none());
        let rec = parse_usage("Current session: 5% used").expect("no resets part is ok");
        assert_eq!(rec["session_pct"], 5.0);
        assert!(!rec.contains_key("session_resets"));
    }

    #[test]
    fn parse_usage_rejects_bad_number() {
        assert!(parse_usage("Current session: 1.2.3% used").is_none());
        assert!(parse_usage("Current session: %% used").is_none());
    }

    // Go TestPollUsageLogsRow
    // Go regex edge cases: optional resets part, whitespace variants, first label occurrence.
    #[test]
    fn parse_usage_regex_edges() {
        let rec = parse_usage("Current session:5%used").expect("no spaces is ok");
        assert_eq!(rec["session_pct"], 5.0);
        let rec = parse_usage("Current session:  7.5%  used  ·  resets   Nov 2  \n").expect("ok");
        assert_eq!(rec["session_pct"], 7.5);
        assert_eq!(rec["session_resets"], "Nov 2");
        // "·" without "resets" is not part of the match; the match still succeeds without resets.
        let rec = parse_usage("Current session: 3% used · later").expect("ok");
        assert!(!rec.contains_key("session_resets"));
        // Bad session number fails the whole parse, even when a later session line is valid (Go FindStringSubmatch).
        assert!(parse_usage("Current session: 1.2.3% used\nCurrent session: 4% used").is_none());
        // Week line with a bad number is dropped, session kept.
        let rec = parse_usage("Current session: 2% used\nCurrent week (all models): 1.2.3% used").expect("ok");
        assert_eq!(rec["session_pct"], 2.0);
        assert!(!rec.contains_key("week_pct"));
        // Label without "used" or "%" does not match.
        assert!(parse_usage("Current session: 9 percent").is_none());
        assert!(parse_usage("Current session: 9 used").is_none());
    }

    #[test]
    fn poll_logs_one_usage_row() {
        let dir = TempDir::new("usage-poll-row");
        let m = Metrics::new(Some(dir.path().to_path_buf()), json!({}));
        log_usage(&m, Ok(LIVE)).expect("log");
        let got = rows(dir.path());
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0]["stage"], "usage");
        assert_eq!(got[0]["source"], "cli");
        assert_eq!(got[0]["session_pct"], 18.0);
    }

    // Go TestPollUsageError
    #[test]
    fn poll_error_logs_one_usage_error_row() {
        let dir = TempDir::new("usage-poll-err");
        let m = Metrics::new(Some(dir.path().to_path_buf()), json!({}));
        log_usage(&m, Err("claude missing")).expect("log");
        let got = rows(dir.path());
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0]["stage"], "usage");
        assert_eq!(got[0]["error"], "claude missing");
    }

    #[test]
    fn unparsable_text_logs_parse_error_with_head() {
        let long = "x".repeat(200);
        let row = usage_row(Ok(&long));
        assert_eq!(row["error"], "parse");
        assert_eq!(row["head"].as_str().map(str::len), Some(120));
    }
}
