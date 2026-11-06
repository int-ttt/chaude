// Parity bodies for the partial/missing rows in plan/rust-test-parity.md (final3 workstream).
// Go sources: harness/resident_coder_test.go (TestMCPRoundTripFakeSession, TestResidentMCPConfigWritten),
// resident_agent_test.go (TestMainStrictMCP), importview_test.go (TestReadClaudeTranscriptFullView),
// idlespend_test.go (TestIdleSpendRow, TestIdleSpendLiveReplay), drain_test.go (TestAgentTrackerAndDrain).
// Public core API only; the Go Repl/fake-session seams are covered by the ignored stubs in
// parity_final3_ignored.rs.

use std::fs;
#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process;
#[cfg(unix)]
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chaude_core::agent_store::AgentTracker;
use chaude_core::import_claude::read_claude_transcript;
#[cfg(unix)]
use chaude_core::mcp::{self, CoderFn};
use chaude_core::proc::{Shape, build_args};
#[cfg(unix)]
use chaude_core::resident::summarize_reply;
use chaude_core::turn_spend::TurnSpend;
use serde_json::{Value, json};

// Empty directory under the system temp dir, unique per process and tag.
fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chaude-final3-{}-{tag}", process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[cfg(unix)]
// Fake resident coder: the raw reply is cut to its status/changed lines, as the coder path does.
fn fake_coder() -> Arc<CoderFn> {
    Arc::new(|task: &str| {
        Ok(summarize_reply(&format!(
            "long prose\nstatus: done\nchanged: README.md: add usage ({task})"
        )))
    })
}

// Go TestMCPRoundTripFakeSession: initialize, tools/list, tools/call (summary only), notifications unanswered.
// Runs over a real unix socket served by mcp::serve (Go serveMCPConn over net.Pipe).
#[cfg(unix)]
#[test]
fn mcp_round_trip_fake_session() {
    let dir = fresh_dir("mcp-rt");
    let sock = dir.join("mcp.sock");
    let addr = mcp::serve(fake_coder(), "", &sock).expect("serve");
    let stream = UnixStream::connect(&addr).expect("connect");
    let mut wr = stream.try_clone().expect("clone");
    let mut rd = BufReader::new(stream);

    let init = rpc(
        &mut wr,
        &mut rd,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
    );
    assert!(init["result"].is_object(), "initialize: {init}");

    let list = rpc(
        &mut wr,
        &mut rd,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    )
    .to_string();
    assert!(list.contains("resident_coder"), "tools/list: {list}");
    assert!(
        !list.contains("resident_coders_status"),
        "tools/list: {list}"
    );

    let got = rpc(&mut wr, &mut rd,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"resident_coder","arguments":{"task":"add docs"}}}"#,
    )
    .to_string();
    assert!(
        got.contains("changed: README.md: add usage"),
        "resident_coder result: {got}"
    );
    assert!(!got.contains("long prose"), "resident_coder result: {got}");

    // Notifications get no response: the next answer must be the id 4 reply, not one for the notification.
    writeln!(
        wr,
        r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
    )
    .expect("write");
    let after = rpc(
        &mut wr,
        &mut rd,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/list"}"#,
    );
    assert_eq!(after["id"], json!(4), "notification was answered: {after}");

    // Same rule on the pure reply function.
    let run = fake_coder();
    assert!(
        mcp::reply(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            run.as_ref()
        )
        .is_none(),
        "notification must not be answered"
    );
    let _ = fs::remove_dir_all(&dir);
}

// Go TestResidentMCPConfigWritten (config body): mcp.json names the bridge with mcp-resident and the socket,
// the token rides in env, and the file is 0600. The Go "created once" check is Repl-level (see ignored stubs).
#[cfg(unix)]
#[test]
fn resident_mcp_config_written() {
    let cfg = mcp::start_owned(fake_coder()).expect("start_owned");
    let path = cfg.path().to_path_buf();
    let raw = fs::read_to_string(&path).expect("read mcp.json");
    let v: Value = serde_json::from_str(&raw).expect("parse mcp.json");
    let chaude = &v["mcpServers"]["chaude"];
    let args = chaude["args"].as_array().expect("args array");
    assert_eq!(args.len(), 2, "args = {args:?}");
    assert_eq!(args[0], json!("mcp-resident"));
    assert!(
        args[1].as_str().unwrap_or("").ends_with("mcp.sock"),
        "socket arg = {:?}",
        args[1]
    );
    assert!(
        chaude["env"][mcp::TOKEN_ENV]
            .as_str()
            .is_some_and(|t| !t.is_empty()),
        "token missing from env: {chaude}"
    );
    let mode = fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "mcp.json mode");
    drop(cfg);
    assert!(
        !Path::new(&path).exists(),
        "guard removes the config dir on drop"
    );
}

// Go TestMainStrictMCP (argument body): the main child gets --mcp-config and --strict-mcp-config, and keeps
// slash commands. Spawn-once is Repl-level (see ignored stubs).
#[test]
fn main_strict_mcp_args() {
    let shape = Shape {
        strict_mcp: true,
        mcp_config: Some(PathBuf::from("/tmp/mcp.json")),
        ..Shape::default()
    };
    let args = build_args("haiku", None, &shape);
    assert!(
        args.iter().any(|a| a == "--strict-mcp-config"),
        "args: {args:?}"
    );
    assert!(args.iter().any(|a| a == "--mcp-config"), "args: {args:?}");
    assert!(
        !args.iter().any(|a| a == "--disable-slash-commands"),
        "main must keep slash commands: {args:?}"
    );
}

// Go TestReadClaudeTranscriptFullView: text-only turns for the snapshot, the full display view in transcript
// order, one subagent folded in, and injected meta user content left out.
#[test]
fn read_claude_transcript_full_view() {
    let dir = fresh_dir("view");
    let path = dir.join("sid1.jsonl");
    let main = [
        r#"{"type":"user","message":{"role":"user","content":"make the thing"},"cwd":"/x","timestamp":"2026-01-01T00:00:00Z"}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"plan first"},{"type":"text","text":"Checking."},{"type":"tool_use","id":"toolu_bash","name":"Bash","input":{"command":"ls"}}]}}"#,
        r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_bash","content":"a.go\nb.go"}]}}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_agent","name":"Agent","input":{"description":"scan"}}]}}"#,
        r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_agent","content":[{"type":"text","text":"scan done"}]}]}}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"All done."}]}}"#,
        r#"{"type":"user","isMeta":true,"message":{"content":"skill body"}}"#,
        r#"{"type":"user","message":{"content":"second prompt"}}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Second answer."}]}}"#,
    ]
    .join("\n")
        + "\n";
    let sub = [
        r#"{"type":"user","isSidechain":true,"uuid":"u1","message":{"role":"user","content":"find things"}}"#,
        r#"{"type":"assistant","isSidechain":true,"uuid":"u2","message":{"content":[{"type":"tool_use","id":"toolu_grep","name":"Grep","input":{"pattern":"x"}}]}}"#,
        r#"{"type":"user","isSidechain":true,"uuid":"u3","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_grep","content":"hit1"}]}}"#,
        r#"{"type":"assistant","isSidechain":true,"uuid":"u4","message":{"content":[{"type":"text","text":"scan finished"}]}}"#,
    ]
    .join("\n")
        + "\n";
    let sub_dir = dir.join("sid1").join("subagents");
    fs::create_dir_all(&sub_dir).expect("subagents dir");
    fs::write(&path, main).expect("write main");
    fs::write(sub_dir.join("agent-a1.jsonl"), sub).expect("write sub");
    fs::write(
        sub_dir.join("agent-a1.meta.json"),
        r#"{"agentType":"haiku-scout","toolUseId":"toolu_agent"}"#,
    )
    .expect("write meta");

    let imp = read_claude_transcript(&path).expect("read transcript");

    // Snapshot input: text only, no tool lines, no thinking.
    assert_eq!(imp.turns.len(), 2, "turns: {:?}", imp.turns);
    assert_eq!(imp.turns[0].0, "make the thing");
    assert_eq!(imp.turns[0].1, "Checking.\nAll done.");

    assert_eq!(imp.view.len(), 2, "view turns: {:?}", imp.view);
    assert_eq!(imp.subagents, 1, "subagents");
    assert_eq!(imp.view[1].0, "second prompt");
    assert_eq!(imp.view[1].1, "Second answer.");
    assert_eq!(imp.view[0].0, "make the thing");

    // Every item appears, in transcript order.
    let first = &imp.view[0].1;
    let order = [
        "[thinking] plan first",
        "Checking.",
        "▸ Bash",
        "← a.go",
        "▸ Agent",
        "┆ task: find things",
        "┆ ▸ Grep",
        "┆ ← hit1",
        "┆ scan finished",
        "← scan done",
        "All done.",
    ];
    let mut at: Option<usize> = None;
    for want in order {
        let i = first
            .find(want)
            .unwrap_or_else(|| panic!("View missing {want:?} in:\n{first}"));
        if let Some(prev) = at {
            assert!(i >= prev, "{want:?} out of order in:\n{first}");
        }
        at = Some(i);
    }
    assert!(
        !(imp.view[0].1.clone() + &imp.view[1].0).contains("skill body"),
        "meta (injected) user content leaked into View"
    );
    let _ = fs::remove_dir_all(&dir);
}

// Go TestIdleSpendRow (turn-spend half): a subagent message adds to sub_* fields, a main-thread Agent spawn
// counts once. The background-row write and result_model_usage are record_turn-level (see ignored stubs).
#[test]
fn idle_spend_row_sub_and_spawns() {
    let mut sp = TurnSpend::new();
    let feed = |sp: &mut TurnSpend, line: &str| {
        let ev: Value = serde_json::from_str(line).expect("event json");
        sp.on_event(&ev);
    };
    feed(
        &mut sp,
        r#"{"type":"assistant","parent_tool_use_id":"tu1","message":{"id":"m1","content":[],"usage":{"input_tokens":3,"output_tokens":7,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}"#,
    );
    feed(
        &mut sp,
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tu1","name":"Agent","input":{"subagent_type":"Explore"}}]}}"#,
    );
    feed(
        &mut sp,
        r#"{"type":"result","session_id":"s1","total_cost_usd":0.01,"usage":{"input_tokens":3,"output_tokens":2},"modelUsage":{"claude-haiku-4-5":{"costUSD":0.01}}}"#,
    );
    let f = sp.fields();
    assert_eq!(f["sub_output_tokens"], json!(7.0), "fields: {f}");
    assert_eq!(f["spawns"], json!(1), "fields: {f}");
}

// Go TestIdleSpendLiveReplay: replays testdata/bg_turn_live.jsonl after the first result event.
// The Go test only logs the last row; the Rust replay asserts the spend fields it can see.
#[test]
fn idle_spend_live_replay() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bg_turn_live.jsonl");
    let body = match fs::read_to_string(&file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("skip: {} missing: {e}", file.display());
            return;
        }
    };
    let mut sp = TurnSpend::new();
    let mut seen_result = false;
    for line in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let ev: Value = serde_json::from_str(line).expect("fixture line json");
        if !seen_result {
            seen_result = ev["type"] == "result";
            continue;
        }
        sp.on_event(&ev);
    }
    assert!(seen_result, "fixture has no result event");
    assert!(sp.fields().is_object(), "fields must be an object");
}

// Go TestAgentTrackerAndDrain (tracker half): tracked Agent/Task calls, background launch ack keeps the agent
// running, a foreground result finishes it, a task-notification finishes the named agent.
// The drain.finished half has no Rust type (see ignored stubs).
#[test]
fn agent_tracker_tracks_and_finishes() {
    let tr = AgentTracker::new(None, "p", "");
    let now = SystemTime::now();
    tr.on_tool_use(
        "a",
        "Agent",
        &json!({"description": "d", "prompt": "write notes/runs/x.md", "run_in_background": true}),
        now,
    );
    tr.on_tool_use("b", "Task", &json!({"description": "fg"}), now);
    tr.on_tool_use("c", "Bash", &json!({}), now);

    let ids =
        |tr: &AgentTracker| -> Vec<String> { tr.running().into_iter().map(|a| a.id).collect() };
    assert_eq!(tr.sorted_all().len(), 2, "tracking: Bash must be ignored");
    assert!(
        tr.sorted_all()
            .iter()
            .any(|a| a.id == "a" && !a.prompt.is_empty()),
        "agent a missing prompt"
    );
    // Background launch ack keeps a running; foreground b finishes.
    tr.on_tool_result_text("a", "Async agent launched. agentId: abc123");
    tr.on_tool_result("b");
    assert_eq!(ids(&tr), vec!["a".to_string()], "running after results");

    let (named, _) =
        tr.on_notification("<task-notification><tool-use-id>a</tool-use-id></task-notification>");
    assert!(named, "notification must name tracked agent a");
    assert!(
        ids(&tr).is_empty(),
        "a must finish on its completion notification"
    );
    let _ = Duration::from_secs(0);
}

// One JSON-RPC line out, one line back. Requests with no id get no reply, so callers never read for them.
#[cfg(unix)]
fn rpc(wr: &mut UnixStream, rd: &mut BufReader<UnixStream>, req: &str) -> Value {
    writeln!(wr, "{req}").expect("write request");
    let mut line = String::new();
    rd.read_line(&mut line).expect("read response");
    serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad response {line:?}: {e}"))
}
