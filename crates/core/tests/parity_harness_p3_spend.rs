// Parity with Go harness group 3 (see plan/rust-test-parity-p3.md): turnspend_test.go, usagepoll_test.go
// (parseUsage, pollUsageSync rows, turnCostUSD) and unit_test.go TestParseSamples.
// Public core API only: turn_spend, usage_poll, metrics, import_view.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use chaude_core::import_view::parse_event;
use chaude_core::metrics::{Metrics, usage_fields};
use chaude_core::turn_spend::{TurnCost, TurnSpend};
use chaude_core::usage_poll::{log_usage, parse_usage};
use serde_json::{Value, json};

// Empty directory under the system temp dir (the core testutil is cfg(test)-only, not visible here).
fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chaude-p3-{}-{tag}", process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

// Go metricRows: every *.jsonl row under dir.
fn metric_rows(dir: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .expect("read metrics dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    files.sort();
    for f in files {
        for line in fs::read_to_string(&f).expect("read jsonl").lines() {
            out.push(serde_json::from_str(line).expect("json row"));
        }
    }
    out
}

// Go subAssist: an assistant event with a message id and usage, optionally under a parent tool id.
fn sub_assist(parent: Option<&str>, msg_id: &str, usage: Value) -> Value {
    let mut ev = json!({"type": "assistant", "message": {"id": msg_id, "usage": usage}});
    if let Some(p) = parent {
        ev["parent_tool_use_id"] = json!(p);
    }
    ev
}

// Go agentUse: an assistant event holding one Agent/Task tool_use block.
fn agent_use(tool_id: &str, name: &str, sub_type: &str) -> Value {
    let mut input = json!({});
    if !sub_type.is_empty() {
        input["subagent_type"] = json!(sub_type);
    }
    json!({"type": "assistant", "parent_tool_use_id": null, "message": {"content": [
        {"type": "tool_use", "id": tool_id, "name": name, "input": input}
    ]}})
}

// Go TestTurnSpendSubagentUsageMaxPerMessage
#[test]
fn go_turnspend_subagent_usage_max_per_message() {
    let mut s = TurnSpend::new();
    s.on_event(&sub_assist(
        Some("t1"),
        "m1",
        json!({"output_tokens": 5.0, "cache_creation_input_tokens": 100.0}),
    ));
    s.on_event(&sub_assist(
        Some("t1"),
        "m1",
        json!({"output_tokens": 9.0, "cache_creation_input_tokens": 100.0}),
    ));
    let f = s.fields();
    assert_eq!(f["sub_msgs"], 1);
    assert_eq!(f["sub_output_tokens"].as_f64(), Some(9.0));
    assert_eq!(f["sub_cache_creation_input_tokens"].as_f64(), Some(100.0));
}

// Go TestTurnSpendMainUsageIgnored
#[test]
fn go_turnspend_main_usage_ignored() {
    let mut s = TurnSpend::new();
    s.on_event(&sub_assist(None, "m1", json!({"output_tokens": 7.0})));
    let f = s.fields();
    let keys = f.as_object().expect("fields is an object");
    assert!(
        keys.keys().all(|k| !k.starts_with("sub_")),
        "unexpected sub_ key from main event: {keys:?}"
    );
}

// Go TestTurnSpendSpawnCounting: a duplicate tool id is not counted again; unset type is general-purpose.
#[test]
fn go_turnspend_spawn_counting() {
    let mut s = TurnSpend::new();
    s.on_event(&agent_use("a1", "Agent", "haiku-coder"));
    s.on_event(&agent_use("a1", "Agent", "haiku-coder"));
    s.on_event(&agent_use("a2", "Task", "haiku-scout"));
    s.on_event(&agent_use("a3", "Agent", ""));
    let f = s.fields();
    assert_eq!(f["spawns"], 3);
    assert_eq!(
        f["spawn_types"],
        json!({"haiku-coder": 1, "haiku-scout": 1, "general-purpose": 1})
    );
}

// Go TestTurnSpendEmptyAndNil. Go's nil receiver has no Rust counterpart (no nil values), so only the
// empty case and the no-panic path for a subagent event are checked here.
#[test]
fn go_turnspend_empty_and_no_panic() {
    let mut s = TurnSpend::new();
    assert_eq!(s.fields(), json!({}));
    s.on_event(&sub_assist(Some("t1"), "m1", json!({"output_tokens": 1.0})));
    assert_eq!(s.fields()["sub_msgs"], 1);
}

// Go TestParseUsage
#[test]
fn go_parse_usage_session_and_week() {
    const LIVE: &str = "Current session: 18% used · resets Oct 30 at 12:49pm (Asia/Seoul)\n\
Current week (all models): 27% used · resets Nov 1 at 4:59pm (Asia/Seoul)";
    let rec = parse_usage(LIVE).expect("live sample is ok");
    assert_eq!(rec["session_pct"], 18.0);
    assert_eq!(rec["session_resets"], "Oct 30 at 12:49pm (Asia/Seoul)");
    assert_eq!(rec["week_pct"], 27.0);
    assert_eq!(rec["week_resets"], "Nov 1 at 4:59pm (Asia/Seoul)");

    assert!(
        parse_usage("Current week (all models): 27% used").is_none(),
        "no session line: want None"
    );
    let rec = parse_usage("Current session: 5% used").expect("no resets part is ok");
    assert_eq!(rec["session_pct"], 5.0);
    assert!(!rec.contains_key("session_resets"), "{rec:?}");
}

// Go TestPollUsageLogsRow: one stage=usage row with the session percentage.
#[test]
fn go_poll_usage_logs_row() {
    const LIVE: &str = "Current session: 18% used · resets Oct 30 at 12:49pm (Asia/Seoul)\n\
Current week (all models): 27% used · resets Nov 1 at 4:59pm (Asia/Seoul)";
    let dir = fresh_dir("usage-row");
    let metrics = Metrics::new(Some(dir.clone()), json!({}));
    log_usage(&metrics, Ok(LIVE)).expect("log usage row");
    let rows = metric_rows(&dir);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["stage"], "usage");
    assert_eq!(rows[0]["session_pct"], 18.0);
    let _ = fs::remove_dir_all(&dir);
}

// Go TestPollUsageError: a failed run logs one usage error row with the run error text.
#[test]
fn go_poll_usage_error_row() {
    let dir = fresh_dir("usage-err");
    let metrics = Metrics::new(Some(dir.clone()), json!({}));
    log_usage(&metrics, Err("claude missing")).expect("log usage error row");
    let rows = metric_rows(&dir);
    assert!(
        rows.len() == 1 && rows[0]["stage"] == "usage" && rows[0]["error"] == "claude missing",
        "want one usage error row, got {rows:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

// Go TestTurnCostUSDDelta
#[test]
fn go_turn_cost_usd_delta() {
    let mut c = TurnCost::new();
    let mut check = |sess: &str, cost: Option<f64>, want: f64| {
        let got = c.delta(sess, cost).expect("numeric cost");
        assert!(
            (got - want).abs() < 1e-9,
            "delta({sess}) = {got}, want {want}"
        );
    };
    check("a", Some(0.438), 0.438); // first turn of a session
    check("a", Some(0.963), 0.525); // cumulative grows: this turn's share
    check("b", Some(0.100), 0.100); // new session starts from 0
    check("b", Some(0.050), 0.050); // cumulative went down: restart from 0
    assert_eq!(c.delta("b", None), None, "nil cost: want None");
}

// Go TestParseSamples: probe1.jsonl parses with no _unparsed line, has text deltas and results with ctx > 0.
#[test]
fn go_parse_samples_probe1() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/probe1.jsonl");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let (mut deltas, mut results) = (0usize, 0usize);
    for line in text.lines() {
        let ev = parse_event(line);
        match ev["type"].as_str() {
            Some("_unparsed") => panic!("unparsed: {:.80}", line),
            Some("stream_event") => {
                if ev["event"]["type"] == "content_block_delta"
                    && ev["event"]["delta"]["type"] == "text_delta"
                {
                    deltas += 1;
                }
            }
            Some("result") => {
                results += 1;
                let ctx = usage_fields(&ev["usage"])["ctx"].as_i64().unwrap_or(0);
                assert!(ctx > 0, "ctx tokens zero in result line");
            }
            _ => {}
        }
    }
    assert!(
        deltas > 0 && results > 0,
        "deltas={deltas} results={results}"
    );
    let bad = parse_event("not json");
    assert_eq!(bad["type"], "_unparsed");
    assert_eq!(bad["raw"], "not json");
}
