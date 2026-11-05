// Parity with Go harness tests listed in plan/rust-test-parity-p1.md and plan/rust-test-parity-p2.md
// (drain_test, idlespend_test, resident_agent_test, resident_coder_test, and the missing rows of
// budget, busyslash, chaude_settings, dispatch, history_file, agentview, acceptance, idle, panel,
// ratestatus). Uses only public core items: agent_store::AgentTracker, turn_spend::TurnSpend,
// resident::agent_body, mcp::{reply, config_json}.
// Rows whose Go side needs Repl, drain, setRate or TUI internals are stubs marked
// #[ignore = "parity: <reason>"]. The stub bodies panic, so --include-ignored cannot pass them.

use chaude_core::agent_store::AgentTracker;
use chaude_core::clients::{ClientRec, ClientRegistry, EXPIRED, PARKED, now_stamp};
use chaude_core::mcp;
use chaude_core::resident::agent_body;
use chaude_core::turn_spend::TurnSpend;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

macro_rules! parity_ignored {
    ($($name:ident = $reason:literal;)*) => {
        $(
            #[test]
            #[ignore = $reason]
            fn $name() {
                panic!(concat!("not ported: ", $reason));
            }
        )*
    };
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chaude-p12-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

// Go drain_test.go TestBackgroundAgentShownRunning.
#[test]
fn background_agent_shown_running() {
    let tr = AgentTracker::new(None, "p", "");
    let now = SystemTime::now();
    tr.on_tool_use("a", "Agent", &json!({"description": "d"}), now);
    tr.on_tool_result_text("a", "Async agent launched successfully. agentId: x");
    let r = tr.running();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].id, "a");

    // A foreground agent finishes with its tool result.
    tr.on_tool_use("b", "Agent", &json!({"description": "fg"}), now);
    tr.on_tool_result_text("b", "done");
    assert_eq!(tr.running().len(), 1);

    // A task notification finishes the background agent.
    let (found, _) =
        tr.on_notification("<task-notification><tool-use-id>a</tool-use-id></task-notification>");
    assert!(found);
    assert!(tr.running().is_empty());
}

// Go drain_test.go TestAgentDoneNotice. The Go notify callback becomes the lines returned by the tracker.
#[test]
fn agent_done_notice() {
    let tr = AgentTracker::new(None, "p", "");
    let now = SystemTime::now();
    tr.on_tool_use(
        "f",
        "Agent",
        &json!({"description": "look up", "model": "haiku", "name": "scout", "prompt": "x"}),
        now,
    );
    tr.on_sub_event(&json!({
        "type": "assistant",
        "parent_tool_use_id": "f",
        "message": {"content": [{"type": "text", "text": "report ready"}]},
    }));
    let first = tr.on_tool_result_text("f", "found it");
    assert_eq!(first.len(), 1, "foreground notice: {first:?}");
    assert!(first[0].contains("haiku"), "{first:?}");
    assert!(first[0].contains("report ready"), "{first:?}");
    // A second result for a finished agent adds no notice.
    assert!(tr.on_tool_result_text("f", "found it").is_empty());

    // A background launch does not notify.
    tr.on_tool_use(
        "b",
        "Agent",
        &json!({"description": "bg job", "run_in_background": true}),
        now,
    );
    assert!(
        tr.on_tool_result_text("b", "Async agent launched successfully")
            .is_empty()
    );
    let note = "<task-notification><tool-use-id>b</tool-use-id></task-notification>";
    assert_eq!(tr.on_notification(note).1.len(), 1);
    // The same notification again adds no notice.
    assert!(tr.on_notification(note).1.is_empty());
}

// Go drain_test.go TestAgentTrackerAndDrain, tracker half only. The drain half is a separate stub below.
#[test]
fn agent_tracker_and_drain_tracker_half() {
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
    // Bash is not tracked, so two agents are known.
    assert_eq!(tr.sorted_all().len(), 2);
    // The background agent's launch result keeps it running. The foreground one finishes.
    tr.on_tool_result("a");
    tr.on_tool_result("b");
    let r = tr.running();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].id, "a");
    tr.on_notification("<task-notification><tool-use-id>a</tool-use-id></task-notification>");
    assert!(tr.running().is_empty());
}

// Go resident_agent_test.go TestAgentPromptLoaded: the agent body with frontmatter dropped.
#[test]
fn agent_prompt_loaded() {
    let cwd = scratch("agent-prompt");
    let dir = cwd.join(".claude").join("agents");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("haiku-prompter.md"),
        "---\nname: haiku-prompter\n---\nBODY LINE\n",
    )
    .unwrap();
    let body = agent_body(&cwd, "haiku-prompter").expect("agent file under cwd");
    assert_eq!(body.trim(), "BODY LINE");
    assert!(!body.contains("name: haiku-prompter"));
}

// Partial: Go prints the missing-agent notice from Repl. Rust agent_body returns the Err that the
// caller turns into that notice, so only the Err case is checked here.
#[test]
fn agent_prompt_missing_is_err() {
    let cwd = scratch("agent-missing");
    assert!(agent_body(&cwd, "haiku-missing").is_err());
}

// Go idlespend_test.go TestIdleSpendRow, partial: sub_output_tokens and spawns only.
// Go result_model_usage is not in TurnSpend (see the ignored stub below).
#[test]
fn idle_spend_row() {
    let mut ts = TurnSpend::new();
    // Subagent message (parent_tool_use_id set): its usage counts toward sub_*.
    ts.on_event(&json!({
        "type": "assistant",
        "parent_tool_use_id": "tu1",
        "message": {"id": "m1", "content": [], "usage": {
            "input_tokens": 3, "output_tokens": 7,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0,
        }},
    }));
    // Main-thread Agent spawn.
    ts.on_event(&json!({
        "type": "assistant",
        "message": {"content": [{"type": "tool_use", "id": "tu1", "name": "Agent",
            "input": {"subagent_type": "Explore"}}]},
    }));
    let f = ts.fields();
    assert_eq!(f["sub_output_tokens"].as_f64(), Some(7.0));
    assert_eq!(f["spawns"].as_u64(), Some(1));
}

// Partial: Go TestIdleSpendLiveReplay replays testdata/bg_turn_live.jsonl and only logs keys.
// Rust replays the same fixture and checks that the spend fields are an object.
#[test]
fn idle_spend_live_replay() {
    let body = fs::read_to_string(manifest().join("tests/fixtures/bg_turn_live.jsonl"))
        .expect("fixture bg_turn_live.jsonl");
    let mut ts = TurnSpend::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let ev: Value = serde_json::from_str(line).expect("fixture line is JSON");
        ts.on_event(&ev);
    }
    assert!(ts.fields().is_object());
}

// Partial: Go TestMCPRoundTripFakeSession. Only the reply-level protocol is ported here. The Go pipe
// and fake coder session (serveMCPConn, spawnCoderSlot) are not pub in Rust.
#[test]
fn mcp_round_trip_reply_level() {
    let run = |task: &str| -> Result<String, String> {
        assert_eq!(task, "add docs");
        Ok("status: done\nchanged: README.md: add usage".to_string())
    };
    let call = |req: &str| -> Value {
        let line = mcp::reply(req, &run).expect("request is answered");
        serde_json::from_str(&line).expect("reply is JSON")
    };

    let init = call(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
    assert!(init["result"].is_object(), "initialize: {init}");

    let list = call(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).to_string();
    assert!(list.contains("resident_coder"), "tools/list: {list}");
    assert!(
        !list.contains("resident_coders_status"),
        "tools/list: {list}"
    );

    let got = call(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"resident_coder","arguments":{"task":"add docs"}}}"#,
    );
    let text = got["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("changed: README.md: add usage"),
        "tools/call: {got}"
    );

    // Notifications get no response.
    assert!(
        mcp::reply(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            &run
        )
        .is_none()
    );
}

// Partial: Go TestResidentMCPConfigWritten. Only the args shape of the config is checked. The
// write-once behaviour of residentMCPConfig is not pub in Rust.
#[test]
fn mcp_config_args() {
    let cfg: Value = serde_json::from_str(&mcp::config_json(
        "/usr/local/bin/chaude",
        "/tmp/x/mcp.sock",
        "tok",
    ))
    .expect("config is JSON");
    let server = &cfg["mcpServers"]["chaude"];
    assert_eq!(server["command"], json!("/usr/local/bin/chaude"));
    assert_eq!(server["args"][0], json!("mcp-resident"));
    assert_eq!(server["args"][1], json!("/tmp/x/mcp.sock"));
    assert_eq!(server["env"][mcp::TOKEN_ENV], json!("tok"));
}

// Go budget_test.go TestReconcileParkedAliveChild: a parked row whose child is alive is expired as
// orphan-alive and its child_pid cleared; a parked row with no child stays parked.
#[test]
fn reconcile_parked_alive_child() {
    let dir = scratch("reconcile-parked");
    let reg = ClientRegistry::new(dir.join("clients.json"));
    let now = now_stamp();
    // owner_pid 0 is a dead owner, so reconcile looks at the child.
    let row = |cwd: &str, child_pid: u32| ClientRec {
        cwd: cwd.into(),
        model: "opus".into(),
        session_id: format!("s-{cwd}"),
        state: PARKED.into(),
        last_api: now,
        child_pid,
        ..Default::default()
    };
    reg.upsert(row("/w/a", std::process::id()));
    reg.upsert(row("/w/b", 0));

    let orphans = reg.reconcile(now, Duration::from_secs(3600));
    assert_eq!(orphans.len(), 1, "{orphans:?}");
    assert_eq!(orphans[0].cwd, "/w/a");
    let a = reg.get("/w/a", "opus").unwrap();
    assert!(
        a.state == EXPIRED && a.ended_reason == "orphan-alive" && a.child_pid == 0,
        "alive-child row {a:?}"
    );
    let b = reg.get("/w/b", "opus").unwrap();
    assert_eq!(b.state, PARKED, "dead-child parked row changed {b:?}");
}

// Stubs for rows that cannot be reached from crates/*/tests/ yet. Each panics when run with
// --include-ignored, so a missing port cannot pass silently.
parity_ignored! {
    // p1 (missing rows)
    live_streaming_multiple_deltas = "parity: Go test is live-only; no fixture with more than 3 main text deltas (text_turn.jsonl has 2, engine.rs:212)";
    agentview_click_read_only_close = "parity: panel-row mouse click opens agent view; panel_click at crates/cli/src/tui/mod.rs:1556 is private (key part ported at agentview.rs:340)";
    scroll_bottom_on_input_and_update = "parity: agent view refresh and close scroll not ported; AgentView scroll state at crates/cli/src/tui/agentview.rs:195 (main body parts at mod.rs:3508)";
    child_pid_recorded_on_park = "parity: parked child pid write is private; upsert_live at crates/core/src/api.rs:3636 has no pub seam";
    version_note_shows_chaude_version = "parity: version note test not ported; version_note at crates/cli/src/tui/commands.rs:295 is pub";
    prune_record_files = "parity: no prune_record_files in crates/core/src (record pruning absent)";
    resident_slots_fixed_four = "parity: diff, Rust uses 8-slot floor RESIDENT_MAX_WORKERS (crates/core/src/api.rs:270)";
    settings_file_empty_cwd_uses_working_dir = "parity: no settings_file helper in crates/core/src (settingsFileFor absent)";
    max_workers_cap_refuses = "parity: diff, Rust child_set::admit evicts oldest instead of refusing (crates/core/src/child_set.rs:147)";
    drain_consume_denies = "parity: drain consume not pub; import_view consume at crates/core/src/import_view.rs:157 is a different path";
    agent_tracker_drain_half = "parity: drain finished and timeout not pub; AgentTracker::running at crates/core/src/agent_store.rs:777";
    up_down_history_with_slash_completer = "parity: history key path private to the TUI; history_prev at crates/cli/src/tui/edit.rs:230";
    up_moves_completer_when_not_at_top = "parity: completer key move private to the TUI; on_key at crates/cli/src/tui/mod.rs:665";

    // p2 (missing rows)
    idle_denies_when_no_prompt = "parity: idle canUse loop is private; idle_deadline at crates/core/src/api.rs:3155";
    idle_background_followup_rendered = "parity: idle followup render is private to the TUI; idle_result at crates/core/src/api.rs:2755";
    idle_tick_refreshes_when_background_finishes = "parity: idle tick is private; finish_oldest_background at crates/core/src/api.rs:2767";
    plain_local_cmds_no_send = "parity: local command no-send gate is private to the TUI; on_key at crates/cli/src/tui/mod.rs:665";
    cursor_row_matches_panel_after_tick = "parity: panel cursor row is private to the TUI; panel_lines at crates/cli/src/tui/mod.rs:1460";
    background_rows_below_input_box = "parity: panel rows are private to the TUI; panel_lines at crates/cli/src/tui/mod.rs:1460";
    background_small_height_no_overflow = "parity: panel overflow is private to the TUI; panel_lines at crates/cli/src/tui/mod.rs:1460";
    stalled_output_file_keeps_agent_running = "parity: file-based drain finish not pub; AgentTracker::running at crates/core/src/agent_store.rs:777";
    queue_lines_dim_above_input = "parity: queue render is private to the TUI; queue_lines at crates/cli/src/tui/mod.rs:1438";
    queue_small_height_no_overflow = "parity: queue overflow is private to the TUI; queue_lines at crates/cli/src/tui/mod.rs:1438";
    cursor_row_includes_background_rows = "parity: panel cursor row is private to the TUI; panel_lines at crates/cli/src/tui/mod.rs:1460";
    panel_scroll_window = "parity: panel scroll window is private to the TUI; panel_lines at crates/cli/src/tui/mod.rs:1460";
    user_event_notification_finishes_background = "parity: user-event path is private; idle_result at crates/core/src/api.rs:2755";
    set_rate_logs_on_five_hour_change = "parity: setRate has no Rust counterpart (no set_rate in crates/core/src)";
    set_rate_top_level_fallback = "parity: setRate has no Rust counterpart (no set_rate in crates/core/src)";
    resident_running_list_shows_only_busy_sessions = "parity: residentRepl running list has no pub Rust seam; AgentTracker::running at crates/core/src/agent_store.rs:777";
    resident_coder_slot_reuse = "parity: coder slot reuse is private; coder_matches at crates/core/src/api.rs:1714";
    coder_permission_only_when_busy = "parity: coder permission path is private; coder_idle_left at crates/core/src/api.rs:1873";
    resident_coder_busy_refuses = "parity: diff, Go refuses busy single coder, Rust pool errors (crates/core/src/api.rs:1046)";
}
