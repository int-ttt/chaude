# 261105 haiku parity loop, round 1 (E1 items 1-4)

item4: mcp_config_args flake (crates/core/src/mcp.rs)
  status: done
  commit: f5a8fce (amended msg; no Go counterpart, Rust-only race)
  fix: start_owned dir name adds AtomicU64 seq; test owned_configs_started_together_use_distinct_dirs
  check: mcp_config_args 1 passed; mcp:: 9 passed

item3: live agent recs started_at 0 (crates/cli/src/tui/mod.rs agent_recs)
  status: done
  commit: 7ad5bb4
  fix: agents tuple keeps AgentStart/AgentResumed started_at (Unix s -> ms); agent_recs uses it
  test: live_agent_started_at_converts_seconds_to_ms (new)
  check: agent filter 28 passed

item2: drain notice Go Task + OutPath (crates/core/src/api.rs end_drain)
  status: done
  commit: 41568dd
  go: repl.go:1040-1063 prints a.Task, a.OutPath (drain.go outPathRe)
  fix: AgentInfo.out_path from prompt (out_path_in, Go regex semantics); notice uses task + out_path (was name + result_path)
  test: out_path_matches_go_regex (new); drain_notice_text_and_prefix unchanged
  check: drain 3+1 passed

item1: E27 bg agents end on drain in production
  status: NOT DONE (remaining)
  go: drain.go:317-327 consume only logs+denies; tracker not fed in prod. drain_test.go:49-52 expects task-notification finishes agent.
  rust: E27 a (defc5b9) follows unit contract.
  todo: decide; engine-level test in crates/core/tests/engine.rs with custom fake claude (old child prints task-notification after drain starts); log Go divergence.

items 5-6: not in this round.

final checks:
  test-isolated.sh: exit 0, 658 passed, 0 failed, 65 ignored (before: 655 passed, 65 ignored)
  cargo clippy --workspace --all-targets: 0 warnings/errors
  cargo fmt --all --check: exit 0
  du -sh .: 2.7G (no cargo clean)

# 261105 haiku parity loop, round 2 (E1 items 1, 5, 6)

item1: E27 bg agents of drained child end on drain timeout, not on its notices
  status: done
  commit: 89634f1
  go: drain.go:317-327 consume only denies can_use_tool; drain.go:305-313 finished on empty tracker or timeout; repl.go:1040-1063 pollDrains
  fix: on_drain_msg no longer feeds notification/tool_result ids into drain tracker (removed notification_ids, tool_result_ids); end_drain calls finish_agent for drained agents still in self.agents
  test: drained_background_agent_ends_at_drain_timeout_not_on_notice (engine.rs, new; fake_drain helper)
  deviation: end_drain emits AgentEnd at drain close (Go UI does not); Eof ends drain at once (Go waits for timeout)
  check: engine drained_background 1 passed

item5: E21 idle reply metrics row (idle_result / log_turn)
  status: done
  commit: 1ab51ab
  go: repl.go:518-575 idleFinish logs turn with source=background
  fix: IdleTurn gains t0 (idle_started); idle_result logs background row with idle start time; log_turn takes source: Option<&str>
  test: idle_reply_is_logged_to_metrics_as_background (#[ignore] removed)
  check: engine idle_reply_is_logged 1 passed; lib 238 passed

item6: parity ignore reasons (sub-items A and B only)
  status: done
  commit: 9dd69a7
  A: parity_final3_ignored.rs:31 stale drain reason (api.rs:322 "No engine effect yet" was wrong) -> cites drain.go:317-327; stays ignored
  B: cli/src/tui/mod.rs:4585 bare #[ignore] -> reason "bench: timing run only, no Go counterpart"
  docs: plan/tui-parity.md E21 and E27 rows updated; plan/rust-test-parity.md TestAgentTrackerAndDrain row updated
  not done: platform/N-A ignore listing; other partial rows in rust-test-parity.md (only the drain row reviewed)
  check: cargo test -p chaude-core --test parity_final3_ignored --no-run OK; cargo test -p chaude-cli bench_frame_large_history_streaming --no-run OK

final checks (round 2, after 9dd69a7):
  test-isolated.sh: exit 0, 658 passed, 0 failed, 64 ignored (round 1: 658 passed, 65 ignored)
  doubt: expected passed +2 (new drain test, un-ignored idle test); observed passed +0, ignored -1. Not resolved.
  cargo clippy --workspace --all-targets: exit 0, 0 warnings
  cargo fmt --all --check: exit 0
  du -sh .: 2.7G (no cargo clean)

## round 3

count mismatch (round 2 explained):
  expected passed +2 (new drain test, un-ignored idle test); observed passed +0, ignored -1
  cause: commit 89634f1 removed api.rs unit tests tool_result_ids_mark_async_launches and notification_ids_read_string_and_text_blocks (-2 passed) with their functions; new drain test and un-ignored idle test (+2)
  verified: targeted run of engine drained_background and idle_reply_is_logged passes by name
  status: resolved (explained, not a test failure)

item6a: platform/N-A ignore listing
  status: done
  commit: 91320e3
  docs: plan/rust-test-parity.md "Platform and N-A listing (round 3, item 6a)" section; commit 9dd69a7 gave every ignore a reason
  totals in plan/rust-test-parity.md: covered 235 -> 237, partial 12 -> 10

item6b: partial rows in plan/rust-test-parity.md (2 rows reviewed)
  status: done
  commit: fe0120a
  TestInputWidthMixed: partial -> covered (render.rs::mixed_width_measure; edit.rs::mixed_width_cursor_and_backspace_follow_cells, new)
    go: input_test.go mixed-width cases
  TestEscInterruptClearsForegroundPanel: partial -> covered (mod.rs::a_single_escape_interrupts_only_while_busy; mod.rs::interrupt_finishes_foreground_agents_only; mod.rs::turn_end_finishes_foreground_agents_only, new)
    go: panel_test.go:50-54; tui.go:517 doneMsg calls finishForeground
  fix: cli/src/tui/mod.rs Busy(false) now calls finish_foreground() (Go doneMsg tui.go:517); before, a foreground agent with no tool_result stayed listed after turn end
  stub removal: parity_final3_ignored.rs go_input_width_mixed_rest, go_esc_interrupt_clears_foreground_panel (ignored 64 -> 62)
  not done: tui-parity non-[x] rows E09 E10 E11 E12 E13 E16 E17 E22 (not triaged this round, out of sub-item limit)

unresolved gaps found (not fixed this round):
  input wrap: Editor move_row is logical-line, not soft-wrap aware; Go inputBox.MoveRow input.go:185 is row-based
  resident missing-prompt notice: Rust errors with no notice; Go prints notice (resident_agent_test.go:37-59); parity_harness_p2.rs ~380
  drain.finished stub, cachettl wiring, idlespend result_model_usage, opusRunning is core OpusBusy in Rust

final checks (round 3, after fe0120a):
  test-isolated.sh: exit 0, 660 passed, 0 failed, 62 ignored (round 2: 658 passed, 64 ignored)
  stub count: grep -c '#\[ignore' parity_final3_ignored.rs = 13; one is a header comment (line 2), so 12 real ignores (matches cargo summary)
  cargo clippy --workspace --all-targets: exit 0, 0 warnings
  cargo fmt --all --check: exit 0
  du -sh .: 2.7G (no cargo clean)
