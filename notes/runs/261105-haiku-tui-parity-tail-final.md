status: partial
workstream: tui-parity tail (branch rust-port)
disk: du -sh . = 2.2G (before final checks)

done:
- X13: 605b9ff (main.rs harness prefix, exit 1)
- X02 + X08: e6999b7 (stdin char-device check; MouseCellMotion) — committed together, not per item
- X01: f89ad2e (failed first claude spawn exits 1 with stderr line)
- X05: ae2090e (metrics header carries Go flag keys); test turn_is_logged_to_metrics_jsonl
- X09: already in 028dc58; test running_agent_is_stored_interrupted_on_shutdown passes; no code change
- X12: verdict same (Go refuses /rotate and /clear while busy, tui.go:804-810; Rust matches); no code change
- A02: 900a982 plain() routes each line through commands::dispatch; pump helper; "> " echo when not char device
    checks: cargo build -p chaude-cli OK; cargo clippy -p chaude-cli --all-targets 0 warnings;
    scripts/test-isolated.sh -p chaude-cli: all result lines ok (0 failed);
    smoke: printf '/help\n/rotate\n' | chaude repl -> "> /help" echoed, help printed, no hang
- E25: verified d890a8d; tests tui_exit_keeps_main_reply_and_reports_notice, prompter_failure_falls_back_to_raw_prompt PASS;
    plan/tui-parity.md row 221 marked [x]: 09cab81

remaining:
- E27: drain old child on rotate when agents busy and max_drain > 0 (Go drain.go; repl.go:985-1001);
    api.rs restart_child/rotate_child (~2978-3260); "No engine effect yet" at api.rs:325-328. Commit separately.
- E24 follow-up: agent_recs in TUI reads only agent_rows; restored rows must be openable. Commit separately.
- final: scripts/test-isolated.sh (full), cargo clippy --workspace --all-targets (0 warnings), cargo fmt --all --check, du -sh .

doubts:
- A02 pump(turn=true) waits for Busy(false); /rotate with no engine turn was not confirmed to emit Busy(false).
  Smoke run exited without hang, but "· main 처리중" status line printed in plain mode is unverified against Go.
- Risk: /dev/null stdin routes to tui() (char-device check); TermGuard raw mode failure path untested.
- X02/X08 in one commit (per-item rule not followed).
- X01 vs X13: start_failed prints "chaude: cannot start claude:" and exits 1 (not the main "harness:" prefix).
- .claude/settings.json shows modified in working tree; not touched by this run (excluded per brief).

## tail continuation (E27, E24 follow-up, final checks)
status: done
commits:
- E27: 84e45c5 fix(core): rotation keeps the old child draining while agents run (crates/core/src/api.rs only)
- E24 follow-up: 3052d8c fix(tui): restored agent rows open in the agent view (crates/cli/src/tui/mod.rs only)
- plan: 8701892 docs(plan): mark E24 follow-up and E27 drain fix (plan/tui-parity.md)
tests added:
- api::tests::drain_only_with_running_agents_and_max_drain: PASS
- api::tests::tool_result_ids_mark_async_launches: PASS
- tui::tests::restored_row_opens_in_the_agent_view: PASS
checks:
- scripts/test-isolated.sh run1: passed=651 failed=0 ignored=65 (log /tmp/haiku-run1.log) EXIT 0
- scripts/test-isolated.sh run2: passed=651 failed=0 ignored=65 (log /tmp/haiku-run2.log) EXIT 0
- cargo clippy --workspace --all-targets: exit 0, 0 warnings, 0 errors (run twice; second run cached and replayed)
- cargo fmt --all --check: exit 0
- du -sh .: 2.4G (cap 15GB)
doubts:
- E27 gaps: background agents of a drained child end only by drain timeout (Go: task-notification); old child can_use_tool not auto-denied (Go does); drain completion notice not ported.
- E24 follow-up: restored started_at converted seconds to ms (x1000); unit assumed from AgentRow comment, not checked against a real restore.
- E24 follow-up: restored rows use the restored name field, running rows use description; not compared to Go sortedRestored.
- untracked crates/cli/src/tui/sessview.rs-E left alone (pre-existing stray file).
- excluded files untouched: .claude/hooks/main-thread-guard.py, .claude/settings*.json, plan/caveman-structure.md, README.ko.md, CHANGELOG.md.

## Results (haiku-tui-parity-tail, E27 + E24)
- commit E27 b: 3b4cd68 · pending can_use_tool of draining child auto-denied ("denied: session is draining", Go drain.go:317-322)
- commit E27 a: defc5b9 · drained child's task-notification finishes its bg agents (Go drain.go:182-199; unit drain_test.go:49-52)
- commit E27 c: 3ce1423 · drain end queues "[agent done]" notice, prefixed to next prompt (Go repl.go:1039-1062, 1088-1089)
- commit E24: 2911fae · test only, restored started_at seconds->ms pinned (no code change; verified api.rs restored_row seconds, mod.rs:1198 x1000)
- test: scripts/test-isolated.sh --no-fail-fast -> exit 0; passed=655 failed=0 ignored=65 (sum of 26 test binaries)
- flake: run 1 failed mcp_config_args_point_at_unix_socket (parity_harness_p2.rs:326, start_owned AlreadyExists). Cause: mcp.rs:219-220 dir name pid+nanos collides for parallel tests. Rerun passed 3/3 alone and full run 2 passed. Not fixed (out of scope).
- clippy: cargo clippy --workspace --all-targets after touch of crates/core/src/api.rs, crates/cli/src/tui/mod.rs -> exit 0, 0 warnings
- fmt: cargo fmt --all --check -> exit 0
- du -sh .: 2.6G (limit 15G; cargo clean not needed)
- doubt: E27 a is Go unit contract only; Go production drains end by timeout (no #[ignore] added).
- doubt: live agent recs (mod.rs ~1171-1180) leave started_at 0, so started_at sort is degraded for live agents.
- doubt: E27 c shows notice only via next user message (no TUI note); uses result_path where Go uses prompt OutPath.
