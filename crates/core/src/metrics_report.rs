//! Metrics summary and per-turn CSV (Go metricscmd.go, `chaude metrics [--dir D] [--csv]`).
//! Pure text over the `<YYYY-MM-DD>.jsonl` files of one directory; printing is left to the caller.

use crate::metrics::num;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Header of the CSV output (Go metricscmd.go).
pub const CSV_HEADER: &str = "ts,stage,session,model,ctx,input,output,cache_read,cache_write,cost_usd,duration_ms,rotations,delegations,handoff_valid";

/// Totals over every record of one metrics directory.
#[derive(Debug, Default, PartialEq)]
pub struct Totals {
    pub files: usize,
    pub turns: i64,
    pub rotations: i64,
    pub handoff_valid: i64,
    pub handoff_invalid: i64,
    pub imports: i64,
    pub delegations: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub cost_usd: f64,
    pub duration_ms: i64,
}

/// Totals over all records in `dir` (Go MetricsMain summary loop).
pub fn totals(dir: &Path) -> Totals {
    let mut t = Totals::default();
    let files = for_each_record(dir, |d| match text(d.get("stage")) {
        "turn" => {
            t.turns += 1;
            t.input += tok(d, "input_tokens");
            t.output += tok(d, "output_tokens");
            t.cache_read += tok(d, "cache_read_input_tokens");
            t.cache_write += tok(d, "cache_creation_input_tokens");
            t.cost_usd += d.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0);
            t.duration_ms += turn_ms(d);
            t.delegations += num(d.get("delegations"));
        }
        "rotate" => {
            t.rotations += 1;
            if d.get("handoff_valid").and_then(Value::as_bool) == Some(true) {
                t.handoff_valid += 1;
            } else {
                t.handoff_invalid += 1;
            }
        }
        "import" => t.imports += 1,
        _ => {}
    });
    t.files = files;
    t
}

/// The `chaude metrics` summary text: two printf-style lines, each ending in "\n".
pub fn summary(dir: &Path) -> String {
    let t = totals(dir);
    format!(
        "metrics: {} file(s) in {}\nturns={} rotations={} (handoff valid={} invalid={}) imports={} delegations={}\ntokens: input={} output={} cache_read={} cache_write={}\ncost_usd={:.4} total_duration_s={:.1}\n",
        t.files,
        dir.display(),
        t.turns,
        t.rotations,
        t.handoff_valid,
        t.handoff_invalid,
        t.imports,
        t.delegations,
        t.input,
        t.output,
        t.cache_read,
        t.cache_write,
        t.cost_usd,
        t.duration_ms as f64 / 1000.0,
    )
}

/// The `chaude metrics --csv` text: the header, then one row per turn and per rotate record, in file order.
pub fn csv(dir: &Path) -> String {
    let mut out = format!("{CSV_HEADER}\n");
    for_each_record(dir, |d| match text(d.get("stage")) {
        "turn" => {
            let _ = writeln!(
                out,
                "{ts},turn,{session},{model},{ctx},{input},{output},{cr},{cc},{cost},{ms},{rot},{deleg},",
                ts = text(d.get("ts")),
                session = text(d.get("session")),
                model = text(d.get("model")),
                ctx = num(d.get("ctx")),
                input = tok(d, "input_tokens"),
                output = tok(d, "output_tokens"),
                cr = tok(d, "cache_read_input_tokens"),
                cc = tok(d, "cache_creation_input_tokens"),
                cost = cost_text(d.get("cost_usd")),
                ms = turn_ms(d),
                rot = num(d.get("rotations")),
                deleg = num(d.get("delegations")),
            );
        }
        "rotate" => {
            let _ = writeln!(
                out,
                "{ts},rotate,{old},,{at},,,,,,,,,{valid}",
                ts = text(d.get("ts")),
                old = text(d.get("old_session")),
                at = num(d.get("rotate_at_tokens")),
                valid = flag_text(d.get("handoff_valid")),
            );
        }
        _ => {}
    });
    out
}

// Calls `f` for every JSON object line of every *.jsonl file in `dir`, files in name order.
// Lines that are not objects are skipped. Returns the number of files found.
fn for_each_record(dir: &Path, mut f: impl FnMut(&Map<String, Value>)) -> usize {
    let files = jsonl_files(dir);
    for path in &files {
        let Ok(fh) = File::open(path) else {
            continue;
        };
        let mut reader = BufReader::with_capacity(1 << 20, fh);
        let mut line = Vec::new();
        loop {
            line.clear();
            let n = reader.read_until(b'\n', &mut line).unwrap_or(0);
            if let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(&line) {
                f(&map);
            }
            if n == 0 {
                break;
            }
        }
    }
    files.len()
}

fn jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("jsonl"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

// Go str: a JSON string, or "" for anything else.
fn text(v: Option<&Value>) -> &str {
    v.and_then(Value::as_str).unwrap_or("")
}

// Go tok: the turn total ("result_" + key) when the record has it, else the older last-call value.
fn tok(d: &Map<String, Value>, key: &str) -> i64 {
    match d.get(&format!("result_{key}")) {
        Some(v) => num(Some(v)),
        None => num(d.get(key)),
    }
}

// Go turn duration: duration_ms, or end_ms - start_ms when duration_ms is 0.
fn turn_ms(d: &Map<String, Value>) -> i64 {
    match num(d.get("duration_ms")) {
        0 => num(d.get("end_ms")) - num(d.get("start_ms")),
        ms => ms,
    }
}

// Go costStr: %.6f for a numeric cost, empty otherwise.
fn cost_text(v: Option<&Value>) -> String {
    v.and_then(Value::as_f64)
        .map_or_else(String::new, |f| format!("{f:.6}"))
}

// Go %v of handoff_valid: true or false, "<nil>" when absent or null.
fn flag_text(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "<nil>".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    const RECORDS: &str = r#"{"stage":"turn","ts":"2026-11-03T00:00:00Z","session":"s1","model":"opus","ctx":1200,"result_input_tokens":10,"input_tokens":3,"output_tokens":5,"cache_read_input_tokens":7,"cache_creation_input_tokens":2,"cost_usd":0.5,"duration_ms":2000,"rotations":0,"delegations":1}
{"stage":"turn","ts":"2026-11-03T00:01:00Z","session":"s1","model":"opus","ctx":2000,"input_tokens":4,"output_tokens":1,"start_ms":1000,"end_ms":1500,"cost_usd":0.25}
{"stage":"rotate","ts":"2026-11-03T00:02:00Z","old_session":"s1","rotate_at_tokens":150000,"handoff_valid":true}
{"stage":"rotate","ts":"2026-11-03T00:03:00Z","old_session":"s2","rotate_at_tokens":160000,"handoff_valid":false}
{"stage":"import","ts":"2026-11-03T00:04:00Z"}
not json
[1,2]
"#;

    fn tmp_with_records(tag: &str) -> TempDir {
        let tmp = TempDir::new(tag);
        fs::write(tmp.path().join("2026-11-03.jsonl"), RECORDS).unwrap();
        tmp
    }

    #[test]
    fn totals_and_summary_follow_go_counts() {
        let tmp = tmp_with_records("metrics-summary");
        let t = totals(tmp.path());
        assert_eq!(t.files, 1);
        assert_eq!((t.turns, t.rotations, t.imports), (2, 2, 1));
        assert_eq!((t.handoff_valid, t.handoff_invalid), (1, 1));
        assert_eq!(t.delegations, 1);
        // result_input_tokens wins for turn 1; turn 2 falls back to input_tokens.
        assert_eq!((t.input, t.output), (14, 6));
        assert_eq!((t.cache_read, t.cache_write), (7, 2));
        assert_eq!(t.cost_usd, 0.75);
        // Turn 2 has no duration_ms, so end_ms - start_ms = 500.
        assert_eq!(t.duration_ms, 2500);
        assert_eq!(
            summary(tmp.path()),
            format!(
                "metrics: 1 file(s) in {}\nturns=2 rotations=2 (handoff valid=1 invalid=1) imports=1 delegations=1\ntokens: input=14 output=6 cache_read=7 cache_write=2\ncost_usd=0.7500 total_duration_s=2.5\n",
                tmp.path().display()
            )
        );
    }

    #[test]
    fn csv_rows_match_go_format_with_fourteen_columns() {
        let tmp = tmp_with_records("metrics-csv");
        let text = csv(tmp.path());
        let rows: Vec<&str> = text.lines().collect();
        assert_eq!(rows[0], CSV_HEADER);
        assert_eq!(
            &rows[1..],
            [
                "2026-11-03T00:00:00Z,turn,s1,opus,1200,10,5,7,2,0.500000,2000,0,1,",
                "2026-11-03T00:01:00Z,turn,s1,opus,2000,4,1,0,0,0.250000,500,0,0,",
                "2026-11-03T00:02:00Z,rotate,s1,,150000,,,,,,,,,true",
                "2026-11-03T00:03:00Z,rotate,s2,,160000,,,,,,,,,false",
            ]
        );
        for row in &rows {
            assert_eq!(row.split(',').count(), 14, "bad column count: {row}");
        }
    }

    #[test]
    fn empty_or_missing_dir_gives_zero_totals_and_header_only_csv() {
        let tmp = TempDir::new("metrics-empty");
        assert_eq!(totals(tmp.path()), Totals::default());
        assert_eq!(csv(tmp.path()), format!("{CSV_HEADER}\n"));
        let missing = tmp.path().join("nope");
        assert_eq!(totals(&missing), Totals::default());
        assert!(summary(&missing).starts_with("metrics: 0 file(s) in "));
    }
}
