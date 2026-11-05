status: done
branch: rust-port (not pushed)
truth: deprecated/src/main/intt/internal/harness/

items:
E25: committed d890a8d (resident prompter and tui failures show Go notice lines). Plan row E25 (resident stage notices, plan/tui-parity.md:221) left [ ]; not verified in this batch.
E05: fixed, ccc1adb. Sub-agent text and tool lines feed the agent log. UiEvent::AgentLog. Tests: subagent_stream_lines_reach_agent_log (engine.rs), agent_log_feeds_the_running_agent_view (tui mod.rs).
E21: fixed visible path, 8632bfe. Tests: idle_reply_is_shown_as_main_message_after_agent_ends (pass); idle_reply_is_logged_to_metrics_as_background (#[ignore], Go repl.go:518-575 idleFinish source=background; Rust does not log idle turns).
E24: fixed, 39b6940. Engine startup load("") emits UiEvent::AgentsRestored; panel lists restored rows after running rows as interrupted; next user line (submit) clears them (Go repl.go:1081). Tests: stored_running_agent_is_restored_at_startup_and_cleared_by_next_line (engine.rs), restored_agents_follow_running_rows_as_interrupted_until_cleared (tui mod.rs). No #[ignore] needed for E24.

checks:
cargo fmt --all --check -> PASS (rc=0), after cargo fmt on own lines
cargo clippy --workspace --all-targets -> 0 warning/error lines
scripts/test-isolated.sh --no-fail-fast -> rc=0; sum of "test result" lines: passed=647 failed=0 ignored=65 (doc tests included in the sum)
narrow: scripts/test-isolated.sh restored -> both new E24 tests ok

doubts:
- Restored rows are clickable in the panel (panel_agent_at uses agent_window) but agent_recs only reads agent_rows, so clicking one opens no view. Not tested, not changed.
- panel_wheel scroll total now counts restored rows; not covered by a test.
- E25 plan row not marked; the d890a8d commit covers a different notice.
- Idle-turn metrics record (E21) still missing in Rust; covered only by the #[ignore] test.
