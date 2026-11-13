use super::*;
use crate::clients::idle_deadline;
use crate::testutil::{TempDir, isolated};

#[test]
fn pure_ack_skips_prompter_but_short_instructions_do_not() {
    for ack in [
        "ㅇㅇ",
        "ㅇㅋ",
        "OK",
        " 응 ",
        "좋아.",
        "계속!",
        "thanks",
        "yes",
        "네",
        "넵",
        "감사합니다",
    ] {
        assert!(is_pure_ack(ack), "{ack:?} should be an ack");
    }
    for not_ack in [
        "go",
        "hello",
        "plain",
        "one",
        "ok, fix it",
        "응 그래서 어떻게?",
        "고쳐",
        "다시",
        "ㅇ",
        "고마워",
        "",
    ] {
        assert!(
            !is_pure_ack(not_ack),
            "{not_ack:?} should reach the prompter"
        );
    }
}

// A restored row keeps its saved final status: done stays done, failed shows failed, and only interrupted or
// still-running (at shutdown) rows are marked interrupted. The saved model is kept; an empty one stays empty.
#[test]
fn restored_row_keeps_saved_status_and_model() {
    let saved = |status: &str, model: &str| -> SavedAgent {
        serde_json::from_value(serde_json::json!({
            "id": "a",
            "model": model,
            "name": "scout",
            "status": status,
        }))
        .expect("saved agent")
    };
    let done = restored_row(&saved(STO_DONE, "haiku-4"));
    assert!(done.done && !done.failed && !done.interrupted, "{done:?}");
    assert_eq!(done.model, "haiku-4");
    let failed = restored_row(&saved(STO_FAILED, "haiku-4"));
    assert!(
        failed.done && failed.failed && !failed.interrupted,
        "{failed:?}"
    );
    let interrupted = restored_row(&saved(STO_INTERRUPTED, "haiku-4"));
    assert!(
        interrupted.done && !interrupted.failed && interrupted.interrupted,
        "{interrupted:?}"
    );
    let running = restored_row(&saved(STO_RUNNING, "haiku-4"));
    assert!(
        running.done && running.interrupted && !running.failed,
        "{running:?}"
    );
    assert_eq!(running.model, "haiku-4");
    let unknown = restored_row(&saved(STO_DONE, ""));
    assert!(unknown.model.is_empty(), "{unknown:?}");
}

// A restored row's duration runs from its start to its stored last change (Unix seconds on both ends).
#[test]
fn restored_row_last_active_is_the_stored_update() {
    let mut s: SavedAgent = serde_json::from_value(serde_json::json!({
        "id": "a",
        "status": STO_INTERRUPTED,
    }))
    .expect("saved agent");
    s.started = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    s.updated = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_125);
    let row = restored_row(&s);
    assert_eq!(row.started_at, 1_700_000_000, "{row:?}");
    assert_eq!(row.last_active, 1_700_000_125, "{row:?}");
}

// With stored log lines, the last activity is the newest line's arrival, not the later interrupt write.
#[test]
fn restored_row_last_active_is_the_last_log_line() {
    let mut s: SavedAgent = serde_json::from_value(serde_json::json!({
        "id": "a",
        "status": STO_INTERRUPTED,
    }))
    .expect("saved agent");
    s.started = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    s.updated = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_500);
    s.log = vec![
        SavedLine {
            at: 1_700_000_010,
            text: "first".to_string(),
        },
        SavedLine {
            at: 1_700_000_090,
            text: "last".to_string(),
        },
    ];
    let row = restored_row(&s);
    assert_eq!(row.last_active, 1_700_000_090, "{row:?}");
}

// /resume lists only the imported session's stored agents; a session with none empties the list. The other
// sessions' records stay in the store (on disk), they are not loaded into the list.
#[test]
fn resume_lists_only_its_session_agents() {
    let tmp = TempDir::new("agent-list-resume");
    let path = tmp.path().join("agents.json");
    let store = AgentStore::new(&path);
    let now = SystemTime::now();
    let mut done = SavedAgent::new("a1", STO_DONE, now);
    done.parent = "s1".to_string();
    done.summary = "report".to_string();
    let mut other = SavedAgent::new("b1", STO_INTERRUPTED, now);
    other.parent = "s2".to_string();
    store.put(&[done, other]);
    let ids = |rows: &[AgentRow]| rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
    let (rows, events) = restored_events(&AgentStore::new(&path).load("s1"));
    assert_eq!(ids(&rows), ["a1"]);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, UiEvent::AgentLog { id, .. } if id == "a1")),
        "summary line of the restored agent"
    );
    let (rows, _) = restored_events(&AgentStore::new(&path).load("s3"));
    assert!(rows.is_empty(), "{rows:?}");
    assert_eq!(
        AgentStore::new(&path).load("s2").len(),
        1,
        "record kept on disk"
    );
}

// An agent's model is its own: the stream model of its assistant lines, else its name prefix. Other lines and
// unprefixed names give none, so the row omits the model instead of showing the main session's.
#[test]
fn agent_model_comes_from_its_own_stream_or_name_prefix() {
    let asst = serde_json::json!({
        "type": "assistant",
        "parent_tool_use_id": "t1",
        "message": {"model": "claude-haiku-5-5", "content": []},
    });
    assert_eq!(stream_model(&asst).as_deref(), Some("claude-haiku-5-5"));
    let user = serde_json::json!({
        "type": "user",
        "parent_tool_use_id": "t1",
        "message": {"model": "claude-opus-5-5"},
    });
    assert_eq!(stream_model(&user), None);
    assert_eq!(agent_type_model("haiku-scout").as_deref(), Some("haiku"));
    assert_eq!(agent_type_model("opus-runner").as_deref(), Some("opus"));
    assert_eq!(agent_type_model("sonnet-x").as_deref(), Some("sonnet"));
    assert_eq!(agent_type_model("general-purpose"), None);
    assert_eq!(agent_type_model("haiku"), None);
}

// A finished agent whose tool result errored (a failed run or a hook block) is saved as failed; a clean one
// stays done, and the other statuses are kept.
#[test]
fn errored_agent_is_saved_as_failed() {
    assert_eq!(saved_status(STO_DONE, true), STO_FAILED);
    assert_eq!(saved_status(STO_DONE, false), STO_DONE);
    assert_eq!(saved_status(STO_RUNNING, true), STO_RUNNING);
    assert_eq!(saved_status(STO_INTERRUPTED, false), STO_INTERRUPTED);
}

// Unset cwd resolves to an absolute path (not "."), so the record folder is not "misc".
#[test]
fn unset_cwd_resolves_to_absolute_dir() {
    let got = resolve_cwd(None);
    assert!(got.is_absolute(), "cwd must be absolute: {got:?}");
    assert_ne!(crate::record_folder::folder_name(&got), "misc");
    assert_eq!(
        resolve_cwd(Some(PathBuf::from("/src/app"))),
        PathBuf::from("/src/app")
    );
}

// Go TestCoderPermissionOnlyWhenBusy: idle denied, busy allowed inside cwd, outside cwd denied, Bash idle denied.
#[test]
fn coder_permission_only_when_busy() {
    let cwd = Path::new("/work/repo");
    let edit = |p: &str| json!({"file_path": p});
    assert!(!coder_may_use(
        false,
        Some(cwd),
        "Edit",
        &edit("/work/repo/a.go")
    ));
    assert!(coder_may_use(
        true,
        Some(cwd),
        "Edit",
        &edit("/work/repo/a.go")
    ));
    assert!(coder_may_use(true, Some(cwd), "Edit", &edit("a.go")));
    assert!(coder_may_use(true, Some(cwd), "Read", &edit("sub/../b.go")));
    assert!(!coder_may_use(true, Some(cwd), "Edit", &edit("/etc/hosts")));
    assert!(!coder_may_use(true, Some(cwd), "Write", &edit("../x.go")));
    assert!(!coder_may_use(true, Some(cwd), "Edit", &edit("")));
    assert!(!coder_may_use(true, None, "Edit", &edit("/work/repo/a.go")));
    assert!(!coder_may_use(false, Some(cwd), "Bash", &Value::Null));
    assert!(coder_may_use(true, Some(cwd), "Bash", &Value::Null));
    assert!(!coder_may_use(
        true,
        Some(cwd),
        "Grep",
        &edit("/work/repo/a.go")
    ));
}

// Go switchChild snapshot rule (resumeshift_test.go TestRotateKeepsPendingSnapshot: no handoff when a snapshot is pending).
#[test]
fn switch_handoff_only_for_fresh_child_without_pending_snapshot() {
    assert!(switch_needs_handoff(
        "",
        2,
        true,
        ChildAction::StartNew,
        false
    ));
    assert!(!switch_needs_handoff(
        "IMPORT SNAPSHOT",
        2,
        true,
        ChildAction::StartNew,
        false
    ));
    assert!(!switch_needs_handoff(
        "",
        0,
        true,
        ChildAction::StartNew,
        false
    ));
    assert!(!switch_needs_handoff(
        "",
        2,
        false,
        ChildAction::StartNew,
        false
    ));
    assert!(!switch_needs_handoff(
        "",
        2,
        true,
        ChildAction::UseLive,
        false
    ));
    assert!(!switch_needs_handoff(
        "",
        2,
        true,
        ChildAction::Resume,
        false
    ));
    assert!(!switch_needs_handoff(
        "",
        2,
        true,
        ChildAction::StartNew,
        true
    ));
}

#[test]
fn observe_ttl_reads_assistant_usage_only() {
    let up = r#"{"type":"assistant","message":{"usage":{"cache_creation":{"ephemeral_1h_input_tokens":5}}}}"#;
    assert_eq!(observe_ttl(up, "5m").as_deref(), Some("1h"));
    let user =
        r#"{"type":"user","message":{"usage":{"cache_creation":{"ephemeral_1h_input_tokens":5}}}}"#;
    assert_eq!(observe_ttl(user, "5m"), None);
    let no_tier = r#"{"type":"assistant","message":{"usage":{"input_tokens":2}}}"#;
    assert_eq!(observe_ttl(no_tier, "auto").as_deref(), Some("auto"));
    assert_eq!(observe_ttl("not json cache_creation", "5m"), None);
}

#[test]
fn effective_ttl_prefers_observed() {
    assert_eq!(effective_ttl("", "5m"), "5m");
    assert_eq!(effective_ttl("1h", "5m"), "1h");
}

#[test]
fn with_spend_merges_subagent_fields_into_turn_row() {
    let mut spend = TurnSpend::new();
    spend.on_event(&json!({"type": "assistant", "parent_tool_use_id": "t1", "message": {"id": "m1", "usage": {"output_tokens": 7.0}}}));
    let row = with_spend(json!({"stage": "turn", "delegations": 0}), &spend);
    assert_eq!(row["stage"], "turn");
    assert_eq!(row["delegations"], 0);
    assert!(
        row.as_object()
            .unwrap()
            .keys()
            .any(|k| k.starts_with("sub_"))
    );
}

#[test]
fn usage_poll_throttles_to_one_per_interval() {
    let t0 = Instant::now();
    let every = Duration::from_secs(USAGE_POLL_EVERY_SECS);
    assert!(usage_poll_due(None, t0, false));
    assert!(!usage_poll_due(
        Some(t0),
        t0 + every - Duration::from_secs(1),
        false
    ));
    assert!(usage_poll_due(Some(t0), t0 + every, false));
    assert!(!usage_poll_due(None, t0, true));
}

#[test]
fn stamp_time_maps_go_zero_to_epoch_and_keeps_instant() {
    let zero = crate::clients::Stamp::default();
    assert_eq!(stamp_time(zero), UNIX_EPOCH);
    let s = crate::clients::Stamp {
        secs: 1_700_000_000,
        nanos: 5,
        off: 0,
    };
    assert_eq!(stamp_time(s), UNIX_EPOCH + Duration::new(1_700_000_000, 5));
}

#[test]
fn max_workers_flag_wins_and_rejects_zero() {
    assert_eq!(resolve_max_workers(Some(5), "9", 4), Ok(5));
    assert!(resolve_max_workers(Some(0), "", 0).is_err());
}

#[test]
fn max_workers_env_when_flag_unset() {
    assert_eq!(resolve_max_workers(None, " 12 ", 0), Ok(12));
    assert!(resolve_max_workers(None, "0", 0).is_err());
    assert!(resolve_max_workers(None, "many", 0).is_err());
}

#[test]
fn max_workers_default_is_three() {
    assert_eq!(resolve_max_workers(None, "", 0), Ok(3));
    assert_eq!(resolve_max_workers(None, "  ", 4), Ok(3));
    let tmp = TempDir::new("max-workers-default");
    assert_eq!(isolated(tmp.path()).max_workers, 3);
}

// Go start(): the main child's rule carries the forward rule unless the model is Opus.
#[test]
fn main_rule_forwards_for_non_opus_and_adds_category() {
    let tmp = TempDir::new("main-rule-forward");
    let mut cfg = Config {
        model: "haiku".into(),
        ..isolated(tmp.path())
    };
    assert_eq!(
        main_rule_with(&cfg, ""),
        join_rules(&[DELEGATION_RULE, crate::forward::FORWARD_RULE, CATEGORY_RULE])
    );
    cfg.model = "claude-OPUS-4".into();
    assert_eq!(
        main_rule_with(&cfg, ""),
        join_rules(&[DELEGATION_RULE, CATEGORY_RULE])
    );
    cfg.no_category_rotate = true;
    cfg.no_delegation_rule = true;
    assert_eq!(main_rule_with(&cfg, ""), "");
}

// Go start(): delegation rule first, orch rule last; both can be switched off.
#[test]
fn main_rule_order_delegation_forward_category_orch() {
    let tmp = TempDir::new("main-rule-order");
    let mut cfg = Config {
        model: "haiku".into(),
        ..isolated(tmp.path())
    };
    assert_eq!(
        main_rule_with(&cfg, "b"),
        join_rules(&[
            DELEGATION_RULE,
            crate::forward::FORWARD_RULE,
            CATEGORY_RULE,
            &orch::rule("b")
        ])
    );
    cfg.no_delegation_rule = true;
    cfg.no_category_rotate = true;
    assert_eq!(main_rule_with(&cfg, "m0"), crate::forward::FORWARD_RULE);
}

#[test]
fn idle_deadline_matches_go_cases() {
    let h = Duration::from_secs(3570);
    assert_eq!(idle_deadline(h, "1h"), h);
    assert_eq!(idle_deadline(h, "5m"), Duration::from_secs(290));
    // Unobserved auto keeps the base 1 h; only an observed 5m cache shortens it.
    assert_eq!(idle_deadline(h, "auto"), h);
    assert_eq!(
        idle_deadline(Duration::from_secs(100), "auto"),
        Duration::from_secs(100)
    );
    assert_eq!(idle_deadline(Duration::ZERO, "1h"), Duration::ZERO);
    assert_eq!(idle_deadline(Duration::ZERO, "5m"), Duration::ZERO);
}

// User rule 261109: the pickChild warm window is idle_rotate (3570 s) even when the cache showed 5m.
#[test]
fn pick_warm_window_ignores_observed_5m_ttl() {
    let h = Duration::from_secs(3570);
    let cfg = Config {
        idle_rotate: h,
        cache_ttl: "5m".to_string(),
        ..Config::default()
    };
    assert_eq!(pick_warm_window(&cfg), h);
    // The 5m window still applies to the cache-cost estimate, not to the warm window.
    assert_eq!(
        idle_deadline(h, effective_ttl("5m", "1h")),
        Duration::from_secs(290)
    );
    assert_eq!(
        pick_warm_window(&Config {
            idle_rotate: Duration::ZERO,
            ..cfg
        }),
        NEVER_COLD
    );
}

// User rule: the main session's idle rotation stays at idle_rotate (3570 s) even with a 5m cache.
#[test]
fn main_idle_rotation_stays_at_idle_rotate_with_5m_cache() {
    let h = Duration::from_secs(3570);
    let cfg = Config {
        idle_rotate: h,
        cache_ttl: "5m".to_string(),
        ..Config::default()
    };
    assert_eq!(main_idle_rotation(&cfg), h);
    let off = Config {
        idle_rotate: Duration::ZERO,
        cache_ttl: "5m".to_string(),
        ..Config::default()
    };
    assert_eq!(main_idle_rotation(&off), Duration::ZERO);
}

// Go repl.go rotate: the old child drains only with agents running and max_drain > 0.
#[test]
fn drain_only_with_running_agents_and_max_drain() {
    assert!(drain_wanted(3, 1));
    assert!(!drain_wanted(0, 1));
    assert!(!drain_wanted(3, 0));
}

// Go drain.finished: done when the shared tracker is empty, timed out at drain_timeout.
#[test]
fn drain_finished_follows_shared_tracker_then_timeout() {
    let t = Duration::from_secs(10);
    assert_eq!(drain_finished(0, Duration::ZERO, t), Some(false));
    assert_eq!(drain_finished(2, Duration::from_secs(9), t), None);
    assert_eq!(drain_finished(2, t, t), Some(true));
    assert_eq!(drain_finished(0, t, t), Some(false));
}

// Go TestAgentTrackerAndDrain (drain_test.go:11-59): running follows the launch ack, the foreground result and the
// completion notification. Go drain.finished(now) (drain.go:305-313) reads only d.tr.running() and d.since/d.timeout,
// never the drain file or a quiet period, so the Go file checks (drain_test.go:33-48) only pin "not done while the
// tracker still runs", which the running() checks below cover. go_finished feeds a real AgentTracker's
// running count to finished_drains, the seam reap_drains uses, and returns Go's (done, timedOut).
#[test]
fn agent_tracker_drain_half() {
    use crate::agent_store::AgentTracker;
    fn go_finished(
        tr: &AgentTracker,
        since: SystemTime,
        now: SystemTime,
        timeout: Duration,
    ) -> (bool, bool) {
        let elapsed = now.duration_since(since).unwrap_or(Duration::ZERO);
        match finished_drains(&[(1, elapsed)], tr.running().len(), timeout).first() {
            Some(&(_, timed_out)) => (true, timed_out),
            None => (false, false),
        }
    }
    let tr = AgentTracker::new(None, "p", "");
    let now = SystemTime::now();
    let t = Duration::from_secs(3600);
    tr.on_tool_use(
        "a",
        "Agent",
        &json!({"description": "d", "prompt": "write notes/runs/x.md", "run_in_background": true}),
        now,
    );
    tr.on_tool_use("b", "Task", &json!({"description": "fg"}), now);
    tr.on_tool_use("c", "Bash", &json!({}), now);
    assert_eq!(tr.sorted_all().len(), 2, "Bash is not tracked");
    tr.on_tool_result("a"); // background: launch ack, still running
    tr.on_tool_result("b"); // foreground: finishes
    let running = tr.running();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].id, "a");
    // Go d.finished: not done before the tracker empties (drain_test.go:31, 36, 41, 46).
    assert_eq!(go_finished(&tr, now, now, t), (false, false));
    assert_eq!(
        go_finished(&tr, now, now + Duration::from_secs(60), t),
        (false, false)
    );
    // Go drain_test.go:49-52: the completion notification empties the tracker, done without timeout.
    tr.on_notification("<task-notification><tool-use-id>a</tool-use-id></task-notification>");
    assert_eq!(go_finished(&tr, now, now, t), (true, false));

    // Go d2.finished past the timeout (drain_test.go:53-58) with an agent still running: done and timed out.
    let tr2 = AgentTracker::new(None, "p", "");
    tr2.on_tool_use("z", "Agent", &json!({"run_in_background": true}), now);
    assert_eq!(
        go_finished(
            &tr2,
            now,
            now + Duration::from_secs(120),
            Duration::from_secs(60)
        ),
        (true, true)
    );
}

// Go drain.consume: only can_use_tool control_requests are denied while draining.
#[test]
fn drain_denies_only_permission_requests() {
    let ask = r#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash"}}"#;
    assert_eq!(drain_permission_id(ask), Some("r1".to_string()));
    let other = r#"{"type":"control_request","request_id":"r2","request":{"subtype":"interrupt"}}"#;
    assert_eq!(drain_permission_id(other), None);
    let text = r#"{"type":"assistant","message":{"content":[]}}"#;
    assert_eq!(drain_permission_id(text), None);
    assert_eq!(drain_permission_id("not json"), None);
}

// Go pollDrains notice text and repl.go takeNotices prefix (notices, a blank line, then the text).
#[test]
fn out_path_matches_go_regex() {
    assert_eq!(
        out_path_in("Write the summary to notes/runs/a-b.md, then stop"),
        "notes/runs/a-b.md"
    );
    assert_eq!(out_path_in("save to `notes/runs/x.md`."), "notes/runs/x.md");
    assert_eq!(out_path_in("Write notes/runs/x.md.bak"), "notes/runs/x.md");
    assert_eq!(out_path_in("no path here"), "");
    assert_eq!(out_path_in("Write notes/runs/.md"), "");
}

// Item 1: a path the prompt only reads or cites is not the result file.
#[test]
fn out_path_ignores_read_only_citations() {
    assert_eq!(
        out_path_in("Read notes/runs/old.md for context.\nWrite the result to notes/runs/new.md."),
        "notes/runs/new.md"
    );
    assert_eq!(out_path_in("Read notes/runs/old.md, then stop"), "");
}

#[test]
fn announce_once_until_the_agent_is_resumed() {
    let mut set = HashSet::new();
    assert!(announce_once(&mut set, "toolu_a"));
    assert!(!announce_once(&mut set, "toolu_a"));
    assert!(announce_once(&mut set, "toolu_b"));
    set.remove("toolu_a");
    assert!(announce_once(&mut set, "toolu_a"));
}

#[test]
fn out_path_skips_glob_patterns() {
    assert_eq!(
        out_path_in("Read notes/runs/261109-x-*.md and write notes/runs/y.md"),
        "notes/runs/y.md"
    );
    assert_eq!(out_path_in("Write notes/runs/z-*.md"), "");
}

#[test]
fn pick_child_row_runs_while_busy_and_is_done_after() {
    let p = picked_agent_row("sess-1", "sonnet", 100);
    let running = with_picked(Vec::new(), Some(&p), true);
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].id, "pick:sess-1");
    assert_eq!(running[0].model, "sonnet");
    assert!(!running[0].done);
    let idle = with_picked(Vec::new(), Some(&p), false);
    assert_eq!(idle.len(), 1);
    assert!(idle[0].done);
    assert!(with_picked(Vec::new(), None, true).is_empty());
}

// Each carried agent brief keeps its first 1500 chars and says it was cut; a short brief is unchanged.
#[test]
fn carried_brief_is_capped_and_marked() {
    assert_eq!(cap_brief("look around"), "look around");
    let exact = "b".repeat(CARRIED_BRIEF_CHARS);
    assert_eq!(cap_brief(&exact), exact);
    let cut = cap_brief(&"a".repeat(1_600));
    assert!(cut.starts_with(&"a".repeat(1_500)), "{cut}");
    assert!(!cut.starts_with(&"a".repeat(1_501)), "{cut}");
    assert!(
        cut.ends_with("[brief truncated: first 1500 chars shown]"),
        "{cut}"
    );
}

#[test]
fn drain_notice_text_and_prefix() {
    assert_eq!(
        drain_done_notice("a1", "lint", "notes/runs/x.md", true, "", ""),
        "[agent done] a1 · lint · notes/runs/x.md · 완료"
    );
    assert_eq!(
        drain_done_notice("a2", "fix", "", false, "", ""),
        "[agent done] a2 · fix ·  · 타임아웃"
    );
    assert_eq!(
        drain_done_notice("a3", "fix", "", true, "", "DONE-ROT"),
        "[agent done] a3 · fix ·  · 완료\nDONE-ROT"
    );
    assert_eq!(
        drain_done_notice("a5", "fix", "", true, "look\n around", "DONE-ROT"),
        "[agent done] a5 · fix ·  · 완료\nbrief: look around\nDONE-ROT"
    );
    let long = "#".repeat(NOTICE_REPORT_CHARS + 5);
    let cut = drain_done_notice("a4", "fix", "", true, "", &long);
    assert!(cut.ends_with(&format!("\n[cut at {NOTICE_REPORT_CHARS} chars]")));
    assert_eq!(cut.matches('#').count(), NOTICE_REPORT_CHARS);
    assert_eq!(with_notices(&[], "hi"), "hi");
    let ns = vec!["[agent done] a1 · t · o · 완료".to_string()];
    assert_eq!(
        with_notices(&ns, "hi"),
        "[agent done] a1 · t · o · 완료\n\nhi"
    );
}

// Go warmDeadline: idle expiry off still keeps a parked child warm for neverCold (pickChild is never cold).
#[test]
fn warm_deadline_uses_never_cold_when_idle_expiry_off() {
    assert_eq!(warm_deadline(Duration::ZERO), NEVER_COLD);
    assert_eq!(
        warm_deadline(Duration::from_secs(290)),
        Duration::from_secs(290)
    );
    let now = SystemTime::now();
    let day_old = now - Duration::from_secs(86_400);
    assert_eq!(
        pick_child::pick_child(
            Some((day_old, 0)),
            None,
            80_000,
            now,
            warm_deadline(Duration::ZERO)
        ),
        pick_child::ChildAction::UseLive
    );
    assert_eq!(
        pick_child::pick_child(Some((day_old, 0)), None, 80_000, now, Duration::ZERO),
        pick_child::ChildAction::StartNew
    );
}

#[test]
fn worker_floor_applies_resident_plus_extra_coders() {
    assert_eq!(worker_floor(3, 0), 8);
    assert_eq!(worker_floor(3, 4), 12);
    assert_eq!(worker_floor(20, 4), 20);
}

// With residents, the text a main reply writes before a tool call is shown when that call arrives, ahead of its
// row; the final reply after the tools is held and comes from the tui answer. Drives the real actor with a fake
// main that streams text, tool_use, tool_result, then text.
#[cfg(unix)]
#[test]
fn pretool_text_shows_before_its_tool_and_final_reply_after() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new("pretool-text");
    let turn = dir.path().join("turn.jsonl");
    let lines = [
        r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"plan first"}},"session_id":"s1","parent_tool_use_id":null}"#,
        r#"{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1","parent_tool_use_id":null}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"plan first"},{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"ls"}}]},"session_id":"s1","parent_tool_use_id":null}"#,
        r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"ok"}]},"session_id":"s1","parent_tool_use_id":null}"#,
        r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"final reply"}},"session_id":"s1","parent_tool_use_id":null}"#,
        r#"{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1","parent_tool_use_id":null}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"final reply"}]},"session_id":"s1","parent_tool_use_id":null}"#,
        r#"{"type":"result","session_id":"s1","result":"final reply"}"#,
    ];
    std::fs::write(&turn, lines.join("\n") + "\n").expect("write turn");
    // Role by argv as in the parked test: main replays the turn, the prompter and tui answer with a result.
    let script = r#"#!/bin/sh
case "$*" in
  *--mcp-config*) ROLE=main ;;
  *"input summarizer"*) ROLE=prompter ;;
  *) ROLE=tui ;;
esac
while IFS= read -r line; do
  case "$line" in
*'"type":"user"'*)
  if [ "$ROLE" = main ]; then
    cat 'TURN'
  else
    printf '%s\n' '{"type":"result","session_id":"s-'"$ROLE"'","result":"ok"}'
  fi
  ;;
  esac
done
"#
    .replace("TURN", &turn.display().to_string());
    let bin = dir.path().join("claude");
    std::fs::write(&bin, script).expect("write fake claude");
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let cfg = Config {
        claude_bin: bin.display().to_string(),
        model: "haiku".into(),
        residents: true,
        record_dir: Some(dir.path().join("record")),
        ..isolated(dir.path())
    };
    let (h, rx) = Engine::try_start(cfg).expect("engine boots");
    h.submit("hello");
    let mut evs = Vec::new();
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(60))
            .expect("engine went silent");
        if ev == UiEvent::Busy(false) {
            break;
        }
        evs.push(ev);
    }
    h.shutdown();
    let plan = evs
        .iter()
        .position(
            |e| matches!(e, UiEvent::TextDelta { role: Role::Main, text } if text == "plan first"),
        )
        .unwrap_or_else(|| panic!("pre-tool text not shown: {evs:?}"));
    let tool = evs
        .iter()
        .position(|e| matches!(e, UiEvent::ToolStart { .. }))
        .expect("tool row");
    let answer = evs
        .iter()
        .position(|e| matches!(e, UiEvent::TuiAnswer { .. }))
        .expect("tui answer");
    assert!(plan < tool && tool < answer, "order: {evs:?}");
    assert!(
        !evs[..answer].iter().any(
            |e| matches!(e, UiEvent::TextDelta { role: Role::Main, text } if text.contains("final"))
        ),
        "final reply shown before the tui answer: {evs:?}"
    );
}

// Engine whose main child replays turn one (an Agent call with the given result, then a final reply) and turn two
// (the continuation) on the next prompt. Roles by argv as in the pretool test.
#[cfg(unix)]
fn agent_queue_engine(
    dir: &TempDir,
    agent_wait: bool,
    wait: Duration,
    result: &str,
    is_error: bool,
) -> (EngineHandle, Receiver<UiEvent>) {
    agent_queue_engine_with(dir, agent_wait, wait, result, is_error, None, "", None)
}

// extra: a child-initiated turn (file content, delay in seconds) that the main child writes on its own after turn one.
#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn agent_queue_engine_with(
    dir: &TempDir,
    agent_wait: bool,
    wait: Duration,
    result: &str,
    is_error: bool,
    extra: Option<(&str, &str)>,
    inline: &str,
    resume: Option<&str>,
) -> (EngineHandle, Receiver<UiEvent>) {
    use std::os::unix::fs::PermissionsExt;
    let turn1 = r#"{"type":"system","subtype":"init","session_id":"s1"}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"waiting"}},"session_id":"s1","parent_tool_use_id":null}
{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1","parent_tool_use_id":null}
{"type":"assistant","message":{"content":[{"type":"text","text":"waiting"},{"type":"tool_use","id":"toolu_a","name":"Agent","input":{"description":"scout","prompt":"look"}}]},"session_id":"s1","parent_tool_use_id":null}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_a","content":"RESULT","is_error":IS_ERROR}]},"session_id":"s1","parent_tool_use_id":null}
INLINE{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"first final"}},"session_id":"s1","parent_tool_use_id":null}
{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1","parent_tool_use_id":null}
{"type":"assistant","message":{"content":[{"type":"text","text":"first final"}]},"session_id":"s1","parent_tool_use_id":null}
{"type":"result","session_id":"s1","result":"first final"}
"#
    .replace("RESULT", result)
    .replace("IS_ERROR", if is_error { "true" } else { "false" })
    .replace("INLINE", inline);
    let turn2 = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"all done"}},"session_id":"s1","parent_tool_use_id":null}
{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1","parent_tool_use_id":null}
{"type":"assistant","message":{"content":[{"type":"text","text":"all done"}]},"session_id":"s1","parent_tool_use_id":null}
{"type":"result","session_id":"s1","result":"all done"}
"#;
    let t1 = dir.path().join("turn1.jsonl");
    let t2 = dir.path().join("turn2.jsonl");
    std::fs::write(&t1, turn1).expect("write turn 1");
    std::fs::write(&t2, turn2).expect("write turn 2");
    let extra_cmd = match extra {
        Some((body, delay)) => {
            let t3 = dir.path().join("extra.jsonl");
            std::fs::write(&t3, body).expect("write extra turn");
            format!("(sleep {delay}; cat '{}') &", t3.display())
        }
        None => String::new(),
    };
    let script = r#"#!/bin/sh
case "$*" in
  *--mcp-config*) ROLE=main ;;
  *"input summarizer"*) ROLE=prompter ;;
  *) ROLE=tui ;;
esac
N=0
while IFS= read -r line; do
  case "$line" in
*'"type":"user"'*)
  if [ "$ROLE" = main ]; then
    N=$((N+1))
    if [ "$N" -eq 1 ]; then cat 'TURN1'; EXTRA_CMD else cat 'TURN2'; fi
  else
    printf '%s\n' '{"type":"result","session_id":"s-'"$ROLE"'","result":"ok"}'
  fi
  ;;
  esac
done
"#
    .replace("EXTRA_CMD", &extra_cmd)
    .replace("TURN1", &t1.display().to_string())
    .replace("TURN2", &t2.display().to_string());
    let bin = dir.path().join("claude");
    std::fs::write(&bin, script).expect("write fake claude");
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let cfg = Config {
        claude_bin: bin.display().to_string(),
        model: "haiku".into(),
        residents: true,
        record_dir: Some(dir.path().join("record")),
        cwd: Some(dir.path().to_path_buf()),
        agent_wait_timeout: wait,
        agent_wait,
        resume: resume.map(String::from),
        ..isolated(dir.path())
    };
    let (h, rx) = Engine::try_start(cfg).expect("engine boots");
    h.submit("hello");
    (h, rx)
}

// Events up to Busy(false). With cancel set, a held reply is cancelled as soon as it is announced.
#[cfg(unix)]
fn agent_queue_events(h: &EngineHandle, rx: &Receiver<UiEvent>, cancel: bool) -> Vec<UiEvent> {
    let mut evs = Vec::new();
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(60))
            .expect("engine went silent");
        if ev == UiEvent::Busy(false) {
            break;
        }
        if cancel && matches!(ev, UiEvent::ReplyHeld { .. }) {
            h.cancel_wait();
        }
        evs.push(ev);
    }
    evs
}

// A background Agent holds the final reply (ReplyHeld, no TurnEnd) until its wait times out; then the timeout
// releases one continuation and the reply ends. The task list is saved under the cwd for /resume.
#[cfg(unix)]
#[test]
fn agent_queue_holds_reply_until_timeout_then_releases() {
    let dir = TempDir::new("agent-queue-timeout");
    let (h, rx) = agent_queue_engine(
        &dir,
        true,
        Duration::from_secs(1),
        "Async agent launched. agentId: x1",
        false,
    );
    let evs = agent_queue_events(&h, &rx, false);
    h.shutdown();
    let queued = evs
        .iter()
        .position(|e| {
            *e == UiEvent::TaskQueued {
                id: "toolu_a".into(),
                desc: "scout".into(),
            }
        })
        .expect("task queued");
    let held = evs
        .iter()
        .position(|e| *e == UiEvent::ReplyHeld { pending: 1 })
        .expect("reply held");
    let timed = evs
        .iter()
        .position(|e| {
            *e == UiEvent::TaskResolved {
                id: "toolu_a".into(),
                status: TaskStatus::Timeout,
            }
        })
        .expect("task timed out");
    let released = evs
        .iter()
        .position(|e| *e == UiEvent::ReplyReleased)
        .expect("reply released");
    assert!(
        queued < held && held < timed && timed < released,
        "order: {evs:?}"
    );
    assert!(
        !evs[..released]
            .iter()
            .any(|e| matches!(e, UiEvent::TuiAnswer { .. } | UiEvent::TurnEnd { .. })),
        "held turn ended early: {evs:?}"
    );
    assert!(
        evs[released..]
            .iter()
            .any(|e| matches!(e, UiEvent::TurnEnd { .. })),
        "continuation did not end: {evs:?}"
    );
    let saved =
        std::fs::read_to_string(crate::tmp_layout::long_dir(dir.path()).join("tasks-s1.json"))
            .expect("task list saved");
    assert!(
        saved.contains("toolu_a") && saved.contains("timeout"),
        "{saved}"
    );
}

// A foreground Agent that fails settles as Failed in its own turn: no hold, the turn ends normally.
#[cfg(unix)]
#[test]
fn agent_queue_failed_agent_settles_without_hold() {
    let dir = TempDir::new("agent-queue-failure");
    let (h, rx) = agent_queue_engine(&dir, true, Duration::from_secs(600), "agent crashed", true);
    let evs = agent_queue_events(&h, &rx, false);
    h.shutdown();
    assert!(
        evs.contains(&UiEvent::TaskResolved {
            id: "toolu_a".into(),
            status: TaskStatus::Failed,
        }),
        "{evs:?}"
    );
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::ReplyHeld { .. })),
        "{evs:?}"
    );
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::TurnEnd { .. })),
        "{evs:?}"
    );
}

// agent_wait off keeps the old semantics: the turn ends at once while its Agent call runs, with no hold.
#[cfg(unix)]
#[test]
fn agent_queue_agent_wait_off_does_not_hold_reply() {
    let dir = TempDir::new("agent-queue-wait-off");
    let (h, rx) = agent_queue_engine(
        &dir,
        false,
        Duration::from_secs(600),
        "Async agent launched. agentId: x1",
        false,
    );
    let evs = agent_queue_events(&h, &rx, false);
    h.shutdown();
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::ReplyHeld { .. } | UiEvent::ReplyReleased)),
        "{evs:?}"
    );
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::TurnEnd { .. })),
        "{evs:?}"
    );
}

// cancel_wait on a held reply settles the running task as Timeout and ends the turn through the usual path.
#[cfg(unix)]
#[test]
fn agent_queue_cancel_wait_ends_held_turn() {
    let dir = TempDir::new("agent-queue-cancel");
    let (h, rx) = agent_queue_engine(
        &dir,
        true,
        Duration::from_secs(600),
        "Async agent launched. agentId: x1",
        false,
    );
    let evs = agent_queue_events(&h, &rx, true);
    h.shutdown();
    assert!(evs.contains(&UiEvent::ReplyHeld { pending: 1 }), "{evs:?}");
    assert!(
        evs.contains(&UiEvent::TaskResolved {
            id: "toolu_a".into(),
            status: TaskStatus::Timeout,
        }),
        "{evs:?}"
    );
    assert!(!evs.contains(&UiEvent::ReplyReleased), "{evs:?}");
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::TuiAnswer { .. })),
        "{evs:?}"
    );
}

// Ends the first main turn with the agent already settled (its completion arrives before the Result): no hold,
// and the one reply is shown when the turn ends. A timed run needs the agent to outlive the turn (live run1 was not).
#[cfg(unix)]
#[test]
fn agent_queue_settled_before_result_is_not_held() {
    let dir = TempDir::new("agent-queue-settled");
    let inline = "{\"type\":\"user\",\"message\":{\"content\":\"<task-notification><tool-use-id>toolu_a</tool-use-id><status>completed</status></task-notification>\"},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n";
    let (h, rx) = agent_queue_engine_with(
        &dir,
        true,
        Duration::from_secs(30),
        "Async agent launched. agentId: x1",
        false,
        None,
        inline,
        None,
    );
    let evs = agent_queue_events(&h, &rx, false);
    h.shutdown();
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::ReplyHeld { .. } | UiEvent::ReplyReleased)),
        "settled agent was held: {evs:?}"
    );
    // The user sees the tui resident's answer to the main text: one prompt, the whole main reply.
    let shown: Vec<&str> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::ResidentBusy {
                role,
                busy: true,
                prompt,
                ..
            } if role.as_str() == "tui" => Some(prompt.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(shown, vec!["waitingfirst final"], "events: {evs:?}");
}

// A background hand-back that arrives while the reply is held starts a child turn of its own. Its text is not a
// reply, and the release waits for that turn to end, so the continuation is the only reply (no leak, no second).
#[cfg(unix)]
#[test]
fn agent_queue_early_report_while_held_does_not_leak_or_double_reply() {
    let dir = TempDir::new("agent-queue-early");
    let early = r#"{"type":"user","message":{"content":"<task-notification><tool-use-id>toolu_a</tool-use-id><status>completed</status></task-notification>"},"session_id":"s1","parent_tool_use_id":null}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"early report"}},"session_id":"s1","parent_tool_use_id":null}
{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1","parent_tool_use_id":null}
{"type":"assistant","message":{"content":[{"type":"text","text":"early report"}]},"session_id":"s1","parent_tool_use_id":null}
{"type":"result","session_id":"s1","result":"early report"}
"#;
    let (h, rx) = agent_queue_engine_with(
        &dir,
        true,
        Duration::from_secs(30),
        "Async agent launched. agentId: x1",
        false,
        Some((early, "1")),
        "",
        None,
    );
    let evs = agent_queue_events(&h, &rx, false);
    h.shutdown();
    let held = evs
        .iter()
        .position(|e| *e == UiEvent::ReplyHeld { pending: 1 })
        .expect("reply held");
    let released = evs
        .iter()
        .position(|e| *e == UiEvent::ReplyReleased)
        .expect("reply released");
    assert!(held < released, "order: {evs:?}");
    // Only the continuation reaches the tui resident; the early report text never does.
    let shown: Vec<&str> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::ResidentBusy {
                role,
                busy: true,
                prompt,
                ..
            } if role.as_str() == "tui" => Some(prompt.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(shown, vec!["all done"], "events: {evs:?}");
    let turn_ended_while_held = evs[..released]
        .iter()
        .any(|e| matches!(e, UiEvent::TurnEnd { .. }));
    assert!(!turn_ended_while_held, "a turn ended while held: {evs:?}");
}

// A report that lands after the wait timed out and the continuation ended is not a new reply: its notice waits
// for the next prompt, and no turn starts by itself.
#[cfg(unix)]
#[test]
fn agent_queue_late_report_after_timeout_is_not_a_second_reply() {
    let dir = TempDir::new("agent-queue-late");
    let late = r#"{"type":"user","message":{"content":"<task-notification><tool-use-id>toolu_a</tool-use-id><status>completed</status></task-notification>"},"session_id":"s1","parent_tool_use_id":null}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"late report"}},"session_id":"s1","parent_tool_use_id":null}
{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1","parent_tool_use_id":null}
{"type":"assistant","message":{"content":[{"type":"text","text":"late report"}]},"session_id":"s1","parent_tool_use_id":null}
{"type":"result","session_id":"s1","result":"late report"}
"#;
    let (h, rx) = agent_queue_engine_with(
        &dir,
        true,
        Duration::from_secs(1),
        "Async agent launched. agentId: x1",
        false,
        Some((late, "4")),
        "",
        None,
    );
    let mut evs = agent_queue_events(&h, &rx, false);
    // Keep listening past the late report: a second reply would show up here.
    while let Ok(ev) = rx.recv_timeout(Duration::from_secs(6)) {
        evs.push(ev);
    }
    h.shutdown();
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::ReplyReleased)),
        "not released: {evs:?}"
    );
    let shown: Vec<&str> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::ResidentBusy {
                role,
                busy: true,
                prompt,
                ..
            } if role.as_str() == "tui" => Some(prompt.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(shown, vec!["all done"], "events: {evs:?}");
}

// A startup resume (cfg.resume) restores the saved task list at boot, before the first turn's events.
#[cfg(unix)]
#[test]
fn agent_queue_startup_resume_shows_saved_tasks() {
    let dir = TempDir::new("agent-queue-startup-resume");
    let long = crate::tmp_layout::long_dir(dir.path());
    std::fs::create_dir_all(&long).expect("long dir");
    std::fs::write(
        long.join("tasks-s9.json"),
        r#"[{"id":"toolu_saved","desc":"saved","status":"done","tokens":2100,"duration_ms":9000}]"#,
    )
    .expect("write saved tasks");
    let (h, rx) = agent_queue_engine_with(
        &dir,
        true,
        Duration::from_secs(1),
        "Async agent launched. agentId: x1",
        false,
        None,
        "",
        Some("s9"),
    );
    let evs = agent_queue_events(&h, &rx, false);
    h.shutdown();
    assert!(
        evs.contains(&UiEvent::TaskQueued {
            id: "toolu_saved".into(),
            desc: "saved".into(),
        }),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::TaskStats {
            id: "toolu_saved".into(),
            tokens: Some(2_100),
            duration_ms: Some(9_000),
        }),
        "{evs:?}"
    );
}

// /resume reads the session's saved task list back as display rows (TaskQueued + TaskResolved), before SessionResumed.
#[cfg(unix)]
#[test]
fn agent_queue_resume_shows_saved_tasks() {
    let dir = TempDir::new("agent-queue-resume");
    let (h, rx) = agent_queue_engine(
        &dir,
        true,
        Duration::from_secs(1),
        "Async agent launched. agentId: x1",
        false,
    );
    let _ = agent_queue_events(&h, &rx, false);
    let long = crate::tmp_layout::long_dir(dir.path());
    std::fs::create_dir_all(&long).expect("long dir");
    std::fs::write(
        long.join("tasks-s9.json"),
        r#"[{"id":"toolu_old","desc":"legacy","status":"done"},{"id":"toolu_new","desc":"fresh","status":"done","tokens":12300,"duration_ms":45000}]"#,
    )
    .expect("write saved tasks");
    h.resume_session("s9");
    let mut evs = Vec::new();
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(60))
            .expect("engine went silent");
        let resumed = matches!(ev, UiEvent::SessionResumed { .. });
        evs.push(ev);
        if resumed {
            break;
        }
    }
    h.shutdown();
    assert!(
        evs.contains(&UiEvent::TaskQueued {
            id: "toolu_old".into(),
            desc: "legacy".into(),
        }),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::TaskResolved {
            id: "toolu_old".into(),
            status: TaskStatus::Done,
        }),
        "{evs:?}"
    );
    // Saved stats come back on resume; the legacy row (no keys) gets none.
    assert!(
        evs.contains(&UiEvent::TaskStats {
            id: "toolu_new".into(),
            tokens: Some(12_300),
            duration_ms: Some(45_000),
        }),
        "{evs:?}"
    );
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::TaskStats { id, .. } if id == "toolu_old")),
        "{evs:?}"
    );
}

// Old task files (no tokens / duration_ms keys) load with no stats; new ones keep them. Missing keys never panic.
#[test]
fn saved_task_stats_read_optional_keys() {
    let old: Value = serde_json::from_str(r#"{"id":"t","desc":"d","status":"done"}"#).unwrap();
    assert_eq!(saved_stats(&old), None);
    let new: Value = serde_json::from_str(
        r#"{"id":"t","desc":"d","status":"done","tokens":12300,"duration_ms":45000}"#,
    )
    .unwrap();
    assert_eq!(
        saved_stats(&new),
        Some(ToolStats {
            tokens: Some(12300),
            duration_ms: Some(45000),
        })
    );
    let partial: Value = serde_json::from_str(r#"{"tokens":null,"duration_ms":7}"#).unwrap();
    assert_eq!(
        saved_stats(&partial),
        Some(ToolStats {
            tokens: None,
            duration_ms: Some(7),
        })
    );
}

// Turn tokens fall back to the per-message sum: one message id counted once, and cache is read + creation.
#[test]
fn turn_tokens_fall_back_to_message_sum() {
    let mut per_msg: HashMap<String, Usage> = HashMap::new();
    let streamed = Usage {
        input_tokens: Some(12),
        output_tokens: Some(1),
        cache_read_input_tokens: Some(80_000),
        cache_creation_input_tokens: Some(0),
    };
    // The same message id repeats on each content block: it must count once.
    for _ in 0..3 {
        per_msg
            .entry("msg_1".into())
            .or_default()
            .max_with(&streamed);
    }
    per_msg.entry("msg_2".into()).or_default().max_with(&Usage {
        input_tokens: Some(3),
        output_tokens: Some(40),
        cache_creation_input_tokens: Some(5),
        cache_read_input_tokens: None,
    });
    assert_eq!(sum_turn_counts(per_msg.values()), Some((15, 41, 80_005)));
    assert_eq!(sum_turn_counts(std::iter::empty()), None);
}

// Go parkedIdleEvent (livechild.go:42-49): a parked non-coder child's can_use_tool gets a deny, never silence.
// Drives the real actor: the prompter and tui children are fake claude scripts that ask can_use_tool and
// block on the answer, so a missing deny fails the test (engine silent) instead of hanging it.
#[cfg(unix)]
#[test]
fn parked_non_coder_can_use_tool_is_denied_on_prompter_and_tui() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new("item3-deny");
    let log = dir.path().join("deny.log");
    let turn = dir.path().join("turn.jsonl");
    std::fs::write(
        &turn,
        "{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n\
         {\"type\":\"result\",\"session_id\":\"s1\",\"result\":\"ok\"}\n",
    )
    .expect("write turn");
    // Role by argv: main has --mcp-config, the prompter rule is "input summarizer", tui is the rest.
    let script = r#"#!/bin/sh
case "$*" in
  *--mcp-config*) ROLE=main ;;
  *"input summarizer"*) ROLE=prompter ;;
  *) ROLE=tui ;;
esac
while IFS= read -r line; do
  case "$line" in
*'"type":"user"'*)
  if [ "$ROLE" = main ]; then
    cat 'TURN'
  else
    printf '%s\n' '{"type":"control_request","request_id":"req-'"$ROLE"'","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{}}}'
    read -r reply
    printf '%s\n' "$reply" >> 'LOG'
    printf '%s\n' '{"type":"result","session_id":"s-'"$ROLE"'","result":"ok"}'
  fi
  ;;
  esac
done
"#
    .replace("TURN", &turn.display().to_string())
    .replace("LOG", &log.display().to_string());
    let bin = dir.path().join("claude");
    std::fs::write(&bin, script).expect("write fake claude");
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let cfg = Config {
        claude_bin: bin.display().to_string(),
        model: "haiku".into(),
        residents: true,
        record_dir: Some(dir.path().join("record")),
        ..isolated(dir.path())
    };
    let (h, rx) = Engine::try_start(cfg).expect("engine boots");
    h.submit("hello");
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(60))
            .expect("engine went silent: a parked child's can_use_tool got no answer");
        if ev == UiEvent::Busy(false) {
            break;
        }
    }
    h.shutdown();
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    let frames: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).expect("answer is json"))
        .collect();
    assert_eq!(
        frames.len(),
        2,
        "prompter and tui each answered once: {text}"
    );
    for id in ["req-prompter", "req-tui"] {
        let f = frames
            .iter()
            .find(|v| v["response"]["request_id"] == id)
            .unwrap_or_else(|| panic!("no answer for {id}: {text}"));
        assert_eq!(f["type"], "control_response");
        assert_eq!(f["response"]["subtype"], "success");
        assert_eq!(f["response"]["response"]["behavior"], "deny");
        assert_eq!(
            f["response"]["response"]["message"],
            "denied: session parked (another model is active)"
        );
    }
}

// S14: dropping the handle stops the actor and kills its main child; no claude child outlives the engine.
#[cfg(unix)]
#[test]
fn dropping_engine_handle_kills_main_child() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new("drop-kills");
    let pid_file = dir.path().join("main.pid");
    // Main child (argv has --mcp-config) records its own pid, then holds stdin open until the engine closes it.
    let script = format!(
        "#!/bin/sh\ncase \"$*\" in\n  *--mcp-config*) echo $$ > '{}' ;;\nesac\ncat > /dev/null\n",
        pid_file.display()
    );
    let bin = dir.path().join("claude");
    std::fs::write(&bin, script).expect("write fake claude");
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let cfg = Config {
        claude_bin: bin.display().to_string(),
        model: "haiku".into(),
        ..isolated(dir.path())
    };
    let (h, _rx) = Engine::try_start(cfg).expect("engine boots");
    let mut pid = None;
    for _ in 0..200 {
        if let Some(p) = std::fs::read_to_string(&pid_file)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            pid = Some(p);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let pid = pid.expect("main child wrote its pid");
    assert!(crate::handoff::pid_alive(pid), "main child not running");
    drop(h);
    assert!(
        !crate::handoff::pid_alive(pid),
        "main child {pid} still alive after the engine handle dropped"
    );
}

// S15: a turn goes Idle -> Busy -> Idle; the turn clock lives only while busy and is dropped by the clear or the idle step.
#[test]
fn phase_turn_cycle_and_clock() {
    let t0 = Instant::now();
    let busy = Phase::Idle.set_busy(true);
    assert_eq!(busy, Phase::Busy { started: None });
    let running = busy.start_clock(t0);
    assert_eq!(running, Phase::Busy { started: Some(t0) });
    assert_eq!(
        running.set_busy(true),
        running,
        "going busy keeps the clock"
    );
    let ended = running.clear_clock();
    assert_eq!(
        ended,
        Phase::Busy { started: None },
        "the turn ends its clock, stays busy"
    );
    assert_eq!(ended.set_busy(false), Phase::Idle);
    assert_eq!(
        running.set_busy(false),
        Phase::Idle,
        "going idle drops the clock"
    );
    assert_eq!(Phase::Idle.set_busy(false), Phase::Idle);
}

// S15 (release_held from idle): a continuation sent while idle starts no clock, so the phase stays Idle as the old busy=false did.
#[test]
fn idle_send_clock_keeps_phase_idle() {
    let now = Instant::now();
    assert_eq!(Phase::Idle.start_clock(now), Phase::Idle);
    assert_eq!(Phase::Idle.clear_clock(), Phase::Idle);
}
