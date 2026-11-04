# 261104 haiku test-parity p1+p2 final

task: test parity groups 1+2 (plan/rust-test-parity-p1.md, plan/rust-test-parity-p2.md)
branch: rust-port (no push)
scope: new file crates/core/tests/parity_p12_harness.rs; plan p1/p2 rows and tallies only; no src edits

test: cargo test -p chaude-core --test parity_p12_harness
result: 9 passed, 0 failed, 35 ignored

p1 (107 rows, HEAD: covered 87, missing 18, partial 1, ignored 1)
- now: covered 89, partial 2, ignored 16, missing 0, n-a 0
- partial: harness/drain_test.go::TestAgentTrackerAndDrain (tracker half active, drain half ignored); harness/cachettl_test.go::TestRegistryNilWhenNoFile (pre-existing)
- ignored reasons: not pub in Rust (Repl/drain/settings/prune internals), cli bin-only (TUI rows), live-only (LiveStreaming), diff vs Go (MaxWorkersCap, ResidentSlotsFixedFour)
- ported covered: TestBackgroundAgentShownRunning, TestAgentDoneNotice

p2 (table 70 rows, HEAD: covered 29, unverified 16, missing 24, diff 1)
- now: covered 30, partial 5, unverified 16, ignored 19, missing 0, diff 0
- header total 94 = 70 table rows + 24 n-a; the 24 n-a are not table rows and were carried over, not re-checked
- partial: TestIdleSpendRow, TestIdleSpendLiveReplay, TestAgentPromptMissingNotice, TestMCPRoundTripFakeSession, TestResidentMCPConfigWritten
- covered: TestAgentPromptLoaded
- ignored: 19 stubs (idle, cursor/panel, queue, set_rate, resident coder)

unverified (body check still open, 16): crates/cli/src/tui/bar.rs::every_running_agent_stays_visible_when_row_is_narrow; crates/cli/src/tui/bar.rs::thinking_row_shows_only_for_opus_resident_or_pre_text; crates/cli/src/tui/bar.rs::agent_name_with_model_prefix_skips_model_segment; crates/cli/src/tui/bar.rs::agent_status_text_uses_state_model_and_name; crates/core/src/import_view.rs (2 tests, TestReadClaudeTranscriptFullView); crates/core/tests/engine.rs::resident_stage_rows_bracket_main_turn_and_agent_calls; plus the remaining unverified rows listed in plan/rust-test-parity-p2.md

doubts:
- p1 header previously read covered 58 / missing 48; real HEAD table was 87 / 18. Fixed to the real table.
- p2 first edit pass merged 14 rows onto single lines (regex ate newlines); repaired and blank separators restored. git diff shows only status, tag, legend, and tally changes.
- n-a 24 in the p2 header is not reproducible from table rows.

commits: see git log (one commit, Co-Authored-By trailer)
