# parity loop4 log

## Round 1
- I1 (flaky main_spawned_once_with_strict_mcp): commit 4fdfd66 (test(core)).
  root cause: crates/core/tests/parity_final3_resident.rs fake claude (pre-fix :55) logged every child argv to shared args.log; resident minimal children (prompter, --system-prompt) raced main, printf split long main line, spliced line had --disable-slash-commands -> assert at pre-fix :145 "main must keep slash commands" failed. Substring match was the symptom; shared log was the cause.
  fix: log only argv with --mcp-config (fixed file :57). Go filter resident_agent_test.go:199 containsArg(o.Extra,"--mcp-config"); engine.rs:101 logs role=main only.
  before: full suite 9/10 pass, target failed 1/10 (run 1; 687 passed, 1 failed, 22 ignored). isolated binary 9/10.
  after: full suite 3/3 green (688 passed, 0 failed, 22 ignored each); isolated binary 10/10 pass.
  checks: scripts/test-isolated.sh 3/3 rc=0; cargo clippy --workspace --all-targets 0 warnings; cargo fmt --all --check PASS.
- I2 (stray scope line in notes/runs/261105-parity-seams-design.md): commit 7050037 (docs(parity)). Removed "this round scope: ..." plus its blank separator; file now 1 line (D1-D6 design line). Checks on same tree as I1 (doc-only): test-isolated 3/3 green, clippy 0 warnings, fmt PASS.
- disk: repo 3.5G (target 3.4G); /System/Volumes/Data 58Gi free. Under 15GB.
- DOUBT: none for I1 fix. Full-suite "10x" was done as 10 baseline runs (9 pass, 1 fail) before; after 3 runs per brief.
- DOUBT: fix relies on only main child logging; if a future test needs resident argv, it needs its own log file.
- NOT DONE: none in Round 1. Not pushed.

## Round 2
- L30 (go_release_copies_selection, selection_test.go:50-80): ported. commit 94d27bb (test(cli)). cli copy_text takes injectable native fn param; release_copy helper; test release_copies_selection_once_and_notes_it in cli mod tests.
- L16 (go_spawn_slot_parks_reading_child, resident_route_test.go:55): ported as behavior proxy. commit 9c029d2 (test(core)). engine.rs spawned_resident_is_read_while_idle: fake prompter emits unsolicited result line, Rust reads it while idle.
  DOUBT: no park flag in Rust; this checks reader keeps running, not a literal parkedIdleEvent port (livechild.go:31-100).
- L23 (go_view_helpers, resume_test.go:42, Go resume.go:174-175): N-A. formatTranscript has no production caller (only resume_test.go:61). Reason text updated; commit 86febb8 (docs-only stub reason). list/match/prompts covered by cli sessview::list_match_and_prompts_follow_go_view_helpers.
- L9 (go_run_turn_shows_tui_output_only, resident_pipeline_test.go:88; repl.go:636-700 routed; resident.go:354-377 viaTUI): ported. commit 029c7c8 (feat(core,cli)).
  core api.rs: routed_text holds filtered main text when residents; not emitted. finish_tui Err emits raw as TextDelta Main + MessageDone Main, then notice. spawn-fail path now goes through finish_tui.
  cli tui/mod.rs: TuiAnswer arm = feed_main + end_main_block; got_text set for it.
  tests: engine.rs residents_pane_shows_only_the_tui_answer (new); tui_exit_keeps_main_reply_and_reports_notice (+ raw "Hello" fallback assert); cli tui_answer_fills_and_closes_the_main_block.
  DOUBT: fallback TextDelta is emitted after MessageDone of the stream (empty block); cli shows raw under its own block after TurnEnd.
- checks (final tree at 029c7c8, before commit): scripts/test-isolated.sh rc=0, all binaries 0 failed; remaining ignored: 1 in chaude_cli (bench, mod.rs, not touched) and 1 in parity_harness_p3_ignored (go_view_helpers, N-A). cargo clippy --workspace --all-targets 0 warnings/errors. cargo fmt --all --check PASS (after rustfmt on engine.rs; only my added lines).
- disk: repo 3.6G (du -sh, after L9). Under 15GB.
- NOT DONE: none. Not pushed. Not tagged.

## Round 3
- P12 prune_record_files (chaude_settings_test.go TestPruneRecordFiles; Go exit_handoff.go:116 pruneRecordFiles): ported. commit 6452fb1 (feat(core)). record_folder.rs prune_record_files; test in parity_p12_harness.rs. Partial: Go call site repl.go:1357 (SessionRetentionDays) not wired, Rust has no session_retention setting.
- P12 resident_slots_fixed_four (resident_test.go:44 TestResidentSlotsFour; Go resident.go:91 residentSlots): ported as mirror. commit d2b8c75 (feat(core)). resident.rs resident_slots table; test in parity_p12_harness.rs. Proxy: engine still hardcodes roles (api.rs prompter, tui, coder); table not consumed. Old stub reason (8-slot floor, RESIDENT_MAX_WORKERS) was a different concept and is dropped.
- P12 max_workers_cap_refuses (dispatch_test.go:82; Go dispatch.go:35 checkWorkerCap): N-A. commit 70e5831 (test(core), stub reason only). checkWorkerCap has no production caller (only dispatch_test.go:87,91). Row stays ignored. Refusal proxy: child_set.rs admit_refuses_when_nothing_evictable (child_set.rs:339).
- P12 drain_consume_denies (drain_test.go:65 TestDrainConsumeDenies; Go drain.go:317 consume): ported as proxy. commit 48917e9 (test(core)). Test drain_deny_frame_reaches_child_stdin in proc.rs (owner of write_permission; needs ClaudeProc stdin, runs `cat`). Not in api.rs as the brief suggested, because the write path is in proc.rs. Proxy: drain_deny/drain_event kinds log not ported, only the response half is covered. Request-id parse already covered by api.rs drain_denies_only_permission_requests.
- P12 agent_tracker_drain_half (drain_test.go:11 TestAgentTrackerAndDrain; Go drain.go:305 finished): ported. commit 3291ac7 (test(core)). api.rs agent_tracker_drain_half next to drain_finished. Proxy: Go file/quiet-period assertions are trivially met because Go finished() only reads tracker and timeout.
- checks (each commit): scripts/test-isolated.sh rc=0, no failed result blocks; cargo clippy --workspace --all-targets 0 warnings/errors; cargo fmt --all --check PASS (rustfmt --edition 2024 on touched files only).
- remaining ignored in parity_p12_harness: max_workers_cap_refuses (N-A, stub kept with evidence) plus the other p1/p2 rows (not in this round).
- DOUBT: item 2 table and item 4/5 are proxies; not engine-consumed or only partially matched to Go.
- DOUBT: a background loop agent (ad3cdb0f66601f465) was running on the same tree; my files were disk-modified once mid-round and only my hunks were staged.
- disk: repo du -sh shown above. Under 15GB.
- NOT DONE: none in the 5 items. Not pushed. Not tagged. No reset/stash.
- merged from 261105-haiku-parity-loop4-round3.md (deleted): status done, 5 of 5 handled, 4 un-ignored, 1 N-A kept ignored. Pushed: no. Tagged: no. Stash/reset: no.
  - prune_record_files 6452fb1 feat(core), partial (repl.go:1357 call site not wired)
  - resident_slots_fixed_four d2b8c75 feat(core), mirror table, engine not consumed
  - max_workers_cap_refuses 70e5831 test(core), N-A (dispatch.go:35 no production caller), stays ignored
  - drain_consume_denies 48917e9 test(core), proxy in proc.rs (response half only)
  - agent_tracker_drain_half 3291ac7 test(core), proxy in api.rs next to drain_finished
  - DOUBT: resident table and drain/tracker items are proxies, not full ports.
  - DOUBT: item 4 test location is proc.rs, not api.rs as brief said.

## Round 4
agent: haiku (parity loop4 round 4)
scope: 5 p12 ignores in order; one commit per item; no push, reset, stash, tag.
checks (each commit): scripts/test-isolated.sh rc=0; cargo clippy --workspace --all-targets 0 warnings; cargo fmt --all --check rc=0.
- set_rate_logs_on_five_hour_change: b1c3441 feat(core) (RateLog + set_rate in metrics.rs, RateLimitEvent arm in api.rs). Go: ratestatus_test.go:12-58, repl.go:321-342. Real port. Suite: 28 ok, 0 failed (/tmp/r4_item1_suite.log). PASS.
- set_rate_top_level_fallback: 6f87206 test(core) (five_hour_util top-level fallback). Go: ratestatus_test.go:60-83, ratestatus.go:12-30. Real port. Suite: 28 ok, 0 failed (/tmp/r4_item2_suite.log). PASS.
  - PROCESS ERROR: committed 6f87206 before scripts/test-isolated.sh ran. Ran it after the commit on HEAD: rc=0, 28 ok, 0 failed.
- user_event_notification_finishes_background: 7a2adcd test(core) (engine test + fixture bg_string_notice_turn.jsonl). Go: panel_test.go:384-390, drain.go:213-231. Production path already existed; this adds the Go-named direct test. Suite: 28 ok, 0 failed (/tmp/r4_item3_suite.log). PASS.
- idle_tick_refreshes_when_background_finishes: c17da5f test(core,cli). Go: panel_test.go:55-77, :79. PROXY, stub kept ignored.
  - Added: engine check (idle_reply_without_notice_finishes_oldest_background_agent now also waits for AgentRows with toolu_idle1 done); cli check idle_tick_drops_background_row_after_it_finishes (tui/mod.rs tests).
  - MISSING PIECE: Go tick re-reads agent list and relayouts (lastRun, bgH, vp.Height). Rust tick (cli tui/mod.rs tick) has no row refresh; rows only change via UiEvent::AgentRows.
  - DOUBT: the cli check passes on AgentRows alone, before tick runs, so it does not prove the tick path. Stub reason left unchanged.
  - Suite: 28 ok, 0 failed (/tmp/r4_item4_suite.log). Clippy 0, fmt rc=0.
- resident_coder_slot_reuse: f1387d9 test(core). Go: resident_coder_test.go:98-111. p2 test resident_coder_slot_reuse_spawns_no_extra now asserts both base lines share one pid; p12 stub removed (behavior observable via public Engine). Suite: 28 ok, 0 failed (/tmp/r4_item5_suite.log). PASS.
- totals: 4 un-ignored with real or Go-named tests (items 1, 2, 3, 5); 1 proxy (item 4, stub kept ignored). Item 4 is not a real port.
- not pushed, not tagged, no reset/stash. Only my files staged per commit.
- background loop agent ad3cdb0f66601f465 was running on the same tree during this round; do not treat its changes as mine.

## Round 5
- item 1 idle_tick_refreshes_when_background_finishes: ported (real tick pull of shared agent rows). Go panel_test.go:55-79; tui.go:450-456. commit ecfc01f. PASS. Doubt: Go lastRun gate not ported (refresh is unconditional per tick).
- item 2 coder_permission_only_when_busy: un-ignored. Go resident_coder_test.go:129-160; resident_coder.go:85 coderMayUse; livechild.go:31 parkedIdleEvent. Rust had no coder can_use_tool handling (idle coder requests were dropped). Added coder_may_use + coder_permission (api.rs). Test is unit test api::tests::coder_permission_only_when_busy, not an engine fixture. commit 4f3969c. PASS. Doubt: the Go non-coder-role case is not modeled; idle request routing is not covered end-to-end.
- item 3 resident_coder_busy_refuses: un-ignored. Go resident_coder_test.go:115-127. Rust already refuses; p2 test extended with first-task CoderReply check. commit 6d7f6cc. PASS. The p12 stub had a stale "Rust pool errors" claim.
- checks per commit: scripts/test-isolated.sh rc=0; cargo clippy --workspace --all-targets 0 warnings; cargo fmt --all --check rc=0.
- optional items 4-5: not attempted (budget).
- git log -10: all loop commits; no ad3cdb0f66601f465 commit seen.
- merged from notes/runs/261105-haiku-parity-loop4-round5.md (deleted): status done (items 1-3), items 4-5 not attempted; checks rc=0 / 0 warnings; doubts: item 2 is a unit test, not an engine fixture; item 1 tick refresh unconditional.

## Round 6
- item 1 live_streaming_multiple_deltas: ported (30-delta fixture count_deltas_turn.jsonl; asserts delta count). Go acceptance_test.go:81-96. commit 53687b2. PASS: isolated suite green at commit.
- item 2 child_pid_recorded_on_park: ported via real park path (CHAUDE_ORCH=f, tagged line, fake claude records pid; asserts PARKED row child_pid == opus pid). Go budget_test.go:145-160. commit b328577. PASS: test-isolated rc=0, 702 passed, 0 failed, 6 ignored; fmt rc=0; clippy 0 warnings.
- item 3 agentview_click_read_only_close: ported as cli unit test tui::tests (tui/mod.rs). panel_click on the worker row opens AgentView with its log line; read-only close path. Go agentview_test.go:21-54. commit 59ffb94. PASS: targeted 1 passed; test-isolated rc=0, 703 passed, 0 failed, 5 ignored; fmt rc=0; clippy 0. Doubt: test sends AgentStart (view record) and AgentRows (panel row) together, as the engine does; not checked against a live engine run.
- item 4 scroll_bottom_on_input_and_update: ported as cli unit test tui::tests (tui/mod.rs). Go agentview_test.go:130-163. commit 5cadbfe. PASS: targeted 1 passed; test-isolated rc=0, 704 passed, 0 failed, 4 ignored; fmt rc=0; clippy 0. Doubt: "refresh keeps view scroll" check is weak (frame takes &App, so it only shows the offset is unchanged).
- item 5 resident_running_list_shows_only_busy_sessions: NOT PORTED, no commit. Go resident_busy_test.go:9-35 needs a busy-resident panel row (role + command text "review the diff"), thinking indicator, runningN count, and removal on finish. Rust gap: UiEvent::ResidentBusy{role,busy} has no command text (crates/core/src/api.rs:128, emit fn api.rs:1490); CLI panel_lines (mod.rs:1485) has no resident rows; no runningN. Indicator alone exists (mod.rs:1395, test mod.rs:4287) but a partial port would be a proxy, so the stub stays ignored (parity_p12_harness.rs:424).
- non-bench #[ignore] left: 3. go_view_helpers (crates/core/tests/parity_harness_p3_ignored.rs:9, N-A: formatTranscript has no production caller); max_workers_cap_refuses (parity_p12_harness.rs:421, N-A: checkWorkerCap has no production caller); resident_running_list_shows_only_busy_sessions (parity_p12_harness.rs:424, open, item 5). Bench excluded: crates/cli/src/tui/mod.rs:5003 (moved from 4867 by new tests).
- open proxies from earlier rounds, not touched this round: drain_consume_denies, agent_tracker_drain_half, resident_slots_fixed_four, prune_record_files (partial), R5 item 2 non-coder role doubt.
- process: no push, tag, reset, or stash. Only own files staged per commit (mod.rs and parity_p12_harness.rs for items 3-4; engine.rs and p12 for items 1-2). No new report file written; this section is the report.

## Round 7
- item 5 resident_running_list_shows_only_busy_sessions: ported as cli unit test tui::tests::resident_running_list_shows_only_busy_sessions (tui/mod.rs). Go resident_busy_test.go:9-35. Asserts: idle gives no panel row, no thinking indicator, running_n 0; busy "tui" with prompt "review the diff" gives one row with role and text, thinking row shown, running_n 1; finish removes row and indicator, running_n 0. commit 1ea0da5. PASS: targeted 1 passed.
- payload change: UiEvent::ResidentBusy gains prompt (first line of command, Go firstLine) and started_at (Unix secs, Go Started) in crates/core/src/api.rs. engine.rs busy-bracket asserts match role and busy only.
- row source: App.resident_busy is Vec<AgentRow> (was BTreeSet<String>), listed after running agents and before restored rows (Go rowsA order, foreground only). App::running_n is #[cfg(test)] (Go runningN, tui.go:1445).
- stub: removed from crates/core/tests/parity_p12_harness.rs (p12 ignored now: max_workers_cap_refuses only).
- design: D6 (test in owning cli module; stub removed). Busy-state owner = core (existing deadline-driven emit); one extended event, not a second one.
- checks: scripts/test-isolated.sh rc=0, 705 passed, 0 failed, 3 ignored (bench tui::tests::bench_frame_large_history_streaming, go_view_helpers, max_workers_cap_refuses); cargo clippy --workspace --all-targets rc=0 0 warnings; cargo fmt --all --check rc=0.
- non-bench #[ignore] left: 2. go_view_helpers (N-A), max_workers_cap_refuses (N-A). Bench excluded: crates/cli/src/tui/mod.rs:~5003.
- doubts: resident rows use session model (Go shows child Model; payload has none). Clicking a resident row opens AgentView by role id, not checked against a live run.

## Round 9
- item 1 startup prune of old records: ported. tui/settings.rs prune_records_at_startup (prune_record_files over dir_for(cwd) with loaded session_retention_days) called from main.rs run_tui. Go repl.go:1357. Test tui::settings::startup_prune_removes_old_records_by_retention (retention window and 0). commit 0e83099. PASS.
- item 2 engine consumes resident slot table: State.resident_slots (api.rs) from resident::resident_slots; spawn_residents and resident_model read it. Go repl.go:1354 residentSlots. Test engine.rs resident_slots_fixed_four drives the fake claude (4 roles, models). p12 proxy resident_slots_fixed_four removed. commit 81ae19c. PASS.
- item 3 drain_deny/drain_event log and return on stdout close: api.rs on_drain_msg writes metrics rows stage drain_deny (can_use_tool, deny control_response) or drain_event, with type and session; Eof returns without end_drain (Go drain.go:317-325). Drain gets session field. Test engine.rs drained_child_prompt_is_denied_and_lines_are_logged. proc.rs drain_deny_frame_reaches_child_stdin proxy kept, comment corrected. commit 6aec6dc. PASS.
- item 3 gap: Go log sink is tracker tr.add (event list). No Rust tracker sink; metrics JSONL used instead. Not directly tested: Eof no longer ends drain (only reap_drains path is covered).
- not touched: agent_tracker_drain_half (per brief).
- checks at 6aec6dc: scripts/test-isolated.sh rc=0, 707 passed, 0 failed, 3 ignored; cargo clippy --workspace --all-targets rc=0, 0 warnings; cargo fmt --all --check rc=0.
- non-bench #[ignore] left: 2. go_view_helpers (parity_harness_p3_ignored.rs:9, N-A), max_workers_cap_refuses (parity_p12_harness.rs:408, N-A). Bench excluded: crates/cli/src/tui/mod.rs:5071.
- doubt D5: drain struct is production (Go repl.go:987-998 builds drain per rotation), so D5's "test-only" premise does not hold; drain kept, not ignored.
