# Rust test parity vs Go (deprecated/src/main/intt)

Date: 2026-11-04. Reference: 320 Go tests (`grep -rhoE '^func Test' deprecated --include='*_test.go'`).
This file is the single deduped table. Per-group detail stays in plan/rust-test-parity-p1.md, -p2.md, -p3.md (kept).
Columns: Go test | Go file | Rust test (path::name or name) | status | note.
status: covered = Rust test of the same behaviour exists; partial = part of the Go body covered, rest needs an
ignored stub; ignored = #[ignore = "parity: ..."] stub exists, reason in note; missing = no Rust test yet;
n-a = Go behaviour lives outside the Rust scope (reason in note).
Dedupe key: Go test name (320 unique names, no collisions across packages). Precedence when sources disagree: p3 > p2 > p1 > shared.

## Totals (recounted from this table)

| status | count |
|---|---|
| covered | 237 |
| partial | 10 |
| ignored | 48 |
| missing | 0 |
| n-a | 25 |
| **total** | **320** |

Ignored by reason (sum 48):

| reason | count |
|---|---|
| core seam/API not wired | 15 |
| TUI behaviour not in Rust yet | 23 |
| no Rust test written yet | 2 |
| Rust differs by design | 2 |
| bin-only/cli-private helper | 6 |

## Remaining

- partial (10): Rust test covers the part of the Go body that is ported; the rest has an ignored stub in
  crates/core/tests/parity_final3_ignored.rs (or the stub named in the note). Stubs panic when run with --ignored.
- ignored (48): stubs exist; each needs its seam wired (core Repl seam, TUI, or cli test seam) before #[ignore] can be removed.
- design (2): TestResidentSlotsFixedFour, TestResidentCoderBusyRefuses. Go fixes four resident roles and refuses a busy single coder;
  Rust uses a coder pool (floor 8, api.rs RESIDENT_MAX_WORKERS) and errors "all resident coders are busy".
- TestMaxWorkersDefault: Rust default 3 equals Go DefaultMaxWorkers 3 (dispatch.go:12); the Go value 4 in the brief is
  chaude_settings.go:20 (extra coders), not the max-workers default. Expected value stays 3; row covered.
- n-a (25): parked-child pool (24, p2) and live usage CLI (1, needs HARNESS_LIVE_USAGE=1; Rust covered by engine.rs).
- Update/install rows: covered via crates/update/tests/parity_update.rs (p3).

## Platform and N-A listing (round 3, item 6a)

Checked by grep of `#[ignore`, `cfg(unix|windows|target_os|not(unix))` under crates/ (2026-11-05).

Platform-gated test sites (not `#[ignore]`d; skipped on non-unix builds, not counted in ignored):
- crates/core/tests/engine.rs:1 `#![cfg(unix)]` (fake claude shell script; no Windows variant).
- crates/core/tests/parity_harness_p2.rs:1 `#![cfg(unix)]`.
- crates/core/tests/parity_final3.rs:44 and :111 `#[cfg(unix)]` (mcp over unix socket; Go serveMCPConn over net.Pipe).
- Other `cfg(unix)` sites are src-level unit tests and branches (handoff.rs, mcp.rs, osenv.rs, cli tui mod.rs/settings.rs/sessview.rs, update crate); not parity rows.

Ignores with a platform reason: none. No `#[ignore]` in crates/ is platform-specific.

N-A ignores and rows kept with reason:
- bench_frame_large_history_streaming (crates/cli/src/tui/mod.rs:4585): `#[ignore = "bench: timing run only, no Go counterpart; run with --ignored"]`. Not a parity row.
- TestUsageLive (usagepoll_test.go): n-a; live CLI, needs HARNESS_LIVE_USAGE=1. No stub; Rust usage covered by engine.rs::usage_answers_with_result_text_from_one_shot_run.
- 24 multiproc_s_test.go rows (TestTouchMainOnly ... TestQuitCloseAllWithNilSess, table rows above): n-a; Go parked-child pool, Rust uses the resident pool (api.rs). No stub.

## Table

| Go test | Go file | Rust test | status | note |
|---|---|---|---|---|
| TestApplyKeepsUserModifiedHook | internal/clean/clean_test.go | clean/src/tests.rs::apply_keeps_user_modified_hook | covered |  |
| TestApplyKeepsHooksWithoutPackageCopy | internal/clean/clean_test.go | clean/src/tests.rs::apply_keeps_hooks_without_package_copy | covered |  |
| TestDryRunChangesNothing | internal/clean/clean_test.go | clean/src/tests.rs::dry_run_changes_nothing | covered |  |
| TestApplyRemovesManifestAndBacksUp | internal/clean/clean_test.go | clean/src/tests.rs::apply_removes_manifest_and_backs_up | covered |  |
| TestApplyKeepsNotesRecordAndUserSettings | internal/clean/clean_test.go | clean/src/tests.rs::apply_keeps_notes_record_and_user_settings | covered |  |
| TestClaudeMdWithoutBlockUntouched | internal/clean/clean_test.go | clean/src/tests.rs::claude_md_without_block_untouched | covered |  |
| TestClaudeMdOnlyBlockIsDeleted | internal/clean/clean_test.go | clean/src/tests.rs::claude_md_only_block_is_deleted | covered |  |
| TestBrokenMarkersFailBeforeAnyChange | internal/clean/clean_test.go | clean/src/tests.rs::broken_markers_fail_before_any_change | covered |  |
| TestManifestMatchesRepo | internal/clean/clean_test.go | clean/src/tests.rs::manifest_matches_repo | covered |  |
| TestGuardSettingsRemovedByClean | internal/clean/guard_roundtrip_test.go | clean/src/tests.rs::guard_settings_removed_by_clean | covered |  |
| TestLiveRotationCarriesContext | internal/harness/acceptance_test.go | core/tests/engine.rs::context_rotation_hands_off_and_fresh_child_gets_snapshot | covered | name-level match |
| TestLiveStreamingMultipleDeltas | internal/harness/acceptance_test.go | - | ignored | ignored (core seam/API not wired): no streaming-delta test in Rust core yet  [p12: live_streaming_multiple_deltas] |
| TestAgentStorePersistsEveryChange | internal/harness/agentstore_test.go | crates/core/src/agent_store.rs::agent_store_persists_every_change | covered | tracker-driven; AgentTracker on_tool_use/on_sub_event, no onUserEvent |
| TestAgentRestoreMarksInterrupted | internal/harness/agentstore_test.go | crates/core/src/agent_store.rs::agent_restore_marks_interrupted | covered | tracker-driven; restore_agents + mark_interrupted |
| TestAgentStoreCrashRecoveryAndOwners | internal/harness/agentstore_test.go | crates/core/src/agent_store.rs::agent_store_crash_recovery_and_owners | covered |  |
| TestAgentStoreOffAndNil | internal/harness/agentstore_test.go | crates/core/src/agent_store.rs::agent_store_empty_path_is_off | covered | name-level match; nil store is Option::None at the call site |
| TestAgentViewClickReadOnlyClose | internal/harness/agentview_test.go | - | ignored | ignored (TUI behaviour not in Rust yet): Rust TUI has no agent view  [p12: agentview_click_read_only_close] |
| TestUsagePopupOpenClose | internal/harness/agentview_test.go | cli/src/tui/usage.rs::esc_closes_and_returns_true | covered | name-level: usage popup close |
| TestWheelScrollHoldsPosition | internal/harness/agentview_test.go | cli/src/tui/usage.rs::wheel_and_refresh_keep_scroll | covered | name-level: usage popup, not agent view |
| TestTypingKeepsScroll | internal/harness/agentview_test.go | - | ignored | ignored (TUI behaviour not in Rust yet): Rust TUI has no agent view scroll  [p12: typing_keeps_scroll] |
| TestScrollBottomOnInputAndUpdate | internal/harness/agentview_test.go | - | ignored | ignored (TUI behaviour not in Rust yet): Rust TUI has no agent view scroll  [p12: scroll_bottom_on_input_and_update] |
| TestMergePromptsOneTurn | internal/harness/batchinput_test.go | core/tests/engine.rs::queued_prompts_go_out_as_one_merged_turn_on_done | covered |  |
| TestMainStatusRendering | internal/harness/batchinput_test.go | cli/src/tui/mod.rs::busy_row_sits_in_the_transcript_flow_with_go_text | covered | name-level: busy/status row text |
| TestRotateDeferDue | internal/harness/bgagent_defer_test.go | core/src/rotate_defer.rs::due_cases | covered |  |
| TestRotateHeldWhileBackgroundAgentRuns | internal/harness/bgagent_defer_test.go | core/src/rotate_defer.rs::held_while_agent_runs_then_due_at_timeout | covered |  |
| TestRotateDeferReleasedWhenAgentFinishes | internal/harness/bgagent_defer_test.go | core/src/rotate_defer.rs::released_when_agent_finishes | covered |  |
| TestDeferRotateTimeoutProceeds | internal/harness/bgagent_defer_test.go | core/src/rotate_defer.rs::timeout_proceeds_and_reports_it | covered |  |
| TestBackgroundResultFileAndHandoff | internal/harness/bgagent_defer_test.go | core/tests/engine.rs::handoff_snapshot_lists_running_background_agent_with_result_path | covered | name-level |
| TestMdBold | internal/harness/bold_test.go | cli/src/tui/render.rs::md_bold_pairs_and_skips_fences | covered |  |
| TestBudgetDeltaPerProcess | internal/harness/budget_test.go | core/src/budget.rs::delta_per_process | covered |  |
| TestBudgetOffAllows | internal/harness/budget_test.go | core/src/budget.rs::cap_zero_is_off | covered | name-level: cap 0 = off |
| TestBudgetResetPerPrompt | internal/harness/budget_test.go | core/src/budget.rs::reset_clears_spent_and_keeps_baseline | covered | name-level |
| TestBudgetResumeBaseline | internal/harness/budget_test.go | core/src/budget.rs::resumed_process_books_whole_first_total | covered | name-level |
| TestBudgetGateBeforeSend | internal/harness/budget_test.go | core/tests/engine.rs::budget_refuses_prompt_whose_next_call_estimate_exceeds_cap | covered |  |
| TestBudgetInterruptMidTurn | internal/harness/budget_test.go | core/tests/engine.rs::budget_interrupts_turn_once_when_in_flight_estimate_reaches_cap | covered |  |
| TestBudgetExtraAppends | internal/harness/budget_test.go | proc.rs::budget_args_append_after_rule_flag | covered | rule args then --max-budget-usd appended; cap 0 adds no flag |
| TestBudgetDisabledByDefault | internal/harness/budget_test.go | core/src/proc.rs::budget_args_only_when_cap_is_set | covered | name-level: no budget args when off |
| TestChildPIDRecordedOnPark | internal/harness/budget_test.go | - | ignored | ignored (core seam/API not wired): needs Repl.parkOld and the client registry (not ported)  [p12: child_pid_recorded_on_park] |
| TestReconcileParkedAliveChild | internal/harness/budget_test.go | - | ignored | ignored (core seam/API not wired): needs Repl.parkOld and the client registry (not ported)  [p12: reconcile_parked_alive_child] |
| TestBudgetFullModelIDPriced | internal/harness/budget_test.go | core/src/budget.rs::full_model_id_is_priced_like_alias | covered |  |
| TestBusySlashAllowedAndRefused | internal/harness/busyslash_test.go | - | ignored | ignored (core seam/API not wired): busy-slash allow/refuse rule not in Rust  [p12: busy_slash_allowed_and_refused] |
| TestVersionNoteShowsChaudeVersion | internal/harness/busyslash_test.go | - | ignored | ignored (no Rust test written yet): version note not covered in Rust tests  [p12: version_note_shows_chaude_version] |
| TestParseCacheTTL | internal/harness/cachettl_test.go | crates/core/src/cache_ttl.rs::parse_cache_ttl_accepts_three_values_only | covered |  |
| TestRegistryNilWhenNoFile | internal/harness/cachettl_test.go | crates/core/src/clients.rs::empty_path_is_noop_registry | partial | nil/empty-path half only; Repl wiring stays in api.rs; stub: crates/core/tests/parity_final3_ignored.rs::go_registry_nil_when_no_file_wiring |
| TestParseCategory | internal/harness/category_test.go | crates/core/src/category.rs::parse_category_takes_last_marker_line | covered |  |
| TestCategoryRotation | internal/harness/category_test.go | crates/core/src/category.rs::category_rotation_rules | covered |  |
| TestCategoryRuleSize | internal/harness/category_test.go | crates/core/src/category.rs::category_rule_fits_budget | covered |  |
| TestMarkerFilter | internal/harness/category_test.go | crates/core/src/category.rs::marker_filter_hides_split_marker_line | covered |  |
| TestMarkerFilterInline | internal/harness/category_test.go | crates/core/src/category.rs::marker_filter_hides_inline_marker | covered |  |
| TestSettingsDefaultsWhenMissing | internal/harness/chaude_settings_test.go | cli/src/tui/settings.rs::defaults_when_missing | covered |  |
| TestSettingsClamp | internal/harness/chaude_settings_test.go | cli/src/tui/settings.rs::clamp_fixes_out_of_range | covered |  |
| TestSettingsSaveLoadRoundTrip | internal/harness/chaude_settings_test.go | cli/src/tui/settings.rs::save_load_round_trip_leaves_no_tmp | covered |  |
| TestSettingsPartialFileKeepsDefaults | internal/harness/chaude_settings_test.go | cli/src/tui/settings.rs::partial_file_keeps_defaults_and_reads_legacy_key | covered |  |
| TestSettingsModelID | internal/harness/chaude_settings_test.go | cli/src/tui/settings.rs::model_ids_follow_alias | covered |  |
| TestPruneRecordFiles | internal/harness/chaude_settings_test.go | - | ignored | ignored (core seam/API not wired): prune of record files not in Rust tests  [p12: prune_record_files] |
| TestExitHandoffWriteRead | internal/harness/chaude_settings_test.go | core/src/handoff.rs::write_then_read_keeps_ids_and_sets_version | covered | name-level |
| TestResidentSlotsFixedFour | internal/harness/chaude_settings_test.go | - | ignored | ignored (Rust differs by design): Rust resident slot count is 8 floor (RESIDENT_MAX_WORKERS), not fixed four  [p12: resident_slots_fixed_four] |
| TestLegacyCoderSlotsKey | internal/harness/chaude_settings_test.go | cli/src/tui/settings.rs::partial_file_keeps_defaults_and_reads_legacy_key | covered |  |
| TestSettingsFileIsPerProject | internal/harness/chaude_settings_test.go | cli/src/tui/settings.rs::settings_file_is_per_project | covered |  |
| TestSettingsFileEmptyCwdUsesWorkingDir | internal/harness/chaude_settings_test.go | - | ignored | ignored (no Rust test written yet): empty-cwd fallback not tested in Rust  [p12: settings_file_empty_cwd_uses_working_dir] |
| TestChildKeyRoleCoexist | internal/harness/childkey_test.go | crates/core/src/role_coexist.rs::same_model_with_role_and_without_role_coexist_when_parked | covered | test-only; child_key + ChildSet park, role logic in child_set.rs |
| TestAdmitEvictsOldestParked | internal/harness/childkey_test.go | crates/core/src/child_set.rs::admit_evicts_oldest_parked | covered |  |
| TestAdmitRefusesWhenNothingEvictable | internal/harness/childkey_test.go | crates/core/src/child_set.rs::admit_refuses_when_nothing_evictable | covered |  |
| TestChildKey | internal/harness/children_test.go | crates/core/src/child_set.rs::child_key_matches_empty_dot_and_working_dir | covered | name-level match; role-coexist case is harness/childkey_test.go::TestChildKeyRoleCoexist (covered by role_coexist.rs) |
| TestChildSetParkTake | internal/harness/children_test.go | crates/core/src/child_set.rs::park_take_replaces_closes_once_and_expires | covered |  |
| TestCloseAllParallel | internal/harness/children_test.go | crates/core/src/child_set.rs::close_all_closes_in_parallel | covered |  |
| TestCleanEnvDropsAPIKey | internal/harness/cleanenv_test.go | crates/core/src/proc.rs::apply_env_drops_api_key_and_sets_only_valid_ttl | covered | name-level: checks the recorded env delta (key removed), not the inherited env |
| TestClientRegistryRoundTrip | internal/harness/clients_test.go | crates/core/src/clients.rs::round_trip_upsert_get_mark_ended | covered |  |
| TestClientRegistryBrokenJSON | internal/harness/clients_test.go | crates/core/src/clients.rs::broken_json_reads_as_empty | covered |  |
| TestClientRegistryAtomicWrite | internal/harness/clients_test.go | crates/core/src/clients.rs::atomic_write_leaves_no_tmp_file | covered |  |
| TestClientRegistryKeyedByCwdModel | internal/harness/clients_test.go | crates/core/src/clients.rs::rows_keyed_by_cwd_and_model | covered |  |
| TestIsWarm | internal/harness/clients_test.go | crates/core/src/clients.rs::is_warm_checks_state_deadline_and_context | covered |  |
| TestReconcileKilledOwner | internal/harness/clients_test.go | crates/core/src/clients.rs::reconcile_marks_dead_owner_rows | covered |  |
| TestIdleDeadline | internal/harness/clients_test.go | crates/core/src/clients.rs::idle_deadline_by_ttl_and_cap | covered |  |
| TestExpiredZeroDeadline | internal/harness/clients_test.go | crates/core/src/clients.rs::expired_zero_deadline | covered | Go negative case not representable in Duration |
| TestWorkerPool | internal/harness/conductor_test.go | crates/core/src/haiku_pool.rs::worker_pool_notes_edits_and_dedups_files | covered | name-level match; nil pool is Option at the call site; 80-rune cut in worker_pool_truncates_task_to_80_runes |
| TestThresholdFor | internal/harness/conductor_test.go | core/src/rotate.rs::threshold_for_opus_and_configured | covered |  |
| TestShouldRotateBoundary | internal/harness/conductor_test.go | core/src/rotate.rs::should_rotate_fires_at_threshold | covered | name-level: threshold boundary |
| TestDelegationRuleNamesHaikuCoder | internal/harness/delegation_test.go | core/src/orch.rs::rule_per_mode | covered | name-level: RULE_B names haiku-coder |
| TestForwardRuleFor | internal/harness/delegation_test.go | crates/core/src/forward.rs::forward_rule_for_skips_opus_models | covered |  |
| TestPickChild | internal/harness/dispatch_test.go | crates/core/src/pick_child.rs::pick_child_matches_go_cases | covered | name-level match; owner case is pick_child_refuses_row_owned_by_other_live_pid (pid 1 instead of parent pid) |
| TestMaxWorkersDefault | internal/harness/dispatch_test.go | crates/core/tests/parity_harness_p1.rs::max_workers_default_is_three | covered | Rust default 3 matches Go DefaultMaxWorkers=3 (dispatch.go:12); brief premise of 4 not Go (chaude_settings.go:20 is extra coders); expected stays 3 |
| TestMaxWorkersEnvOverride | internal/harness/dispatch_test.go | crates/core/tests/parity_harness_p1.rs::max_workers_env_override_and_flag_precedence | covered | added in parity_harness_p1.rs |
| TestMaxWorkersRejectZero | internal/harness/dispatch_test.go | crates/core/tests/parity_harness_p1.rs::max_workers_rejects_zero_from_flag_and_env | covered | added in parity_harness_p1.rs |
| TestMaxWorkersCapRefuses | internal/harness/dispatch_test.go | - | ignored | ignored (core seam/API not wired): Rust has no worker cap check  [p12: max_workers_cap_refuses] |
| TestAgentTrackerAndDrain | internal/harness/drain_test.go | crates/core/tests/parity_final3.rs::agent_tracker_tracks_and_finishes | partial | tracker half covered (final3); drain.finished stub kept ignored: Go drain_test feeds onNotification, which Go production never does (drain.go:317-327); stub: crates/core/tests/parity_final3_ignored.rs::go_agent_tracker_drain_finished  [p12: agent_tracker_and_drain_tracker_half]  [p12: agent_tracker_drain_half] |
| TestDrainConsumeDenies | internal/harness/drain_test.go | - | ignored | ignored (core seam/API not wired): Rust has no drain consume  [p12: drain_consume_denies] |
| TestBackgroundAgentShownRunning | internal/harness/drain_test.go | - | covered | no direct test in Rust  [p12: background_agent_shown_running] |
| TestAgentDoneNotice | internal/harness/drain_test.go | - | covered | no agent-done notice test in Rust  [p12: agent_done_notice] |
| TestResumeSendMessageReactivatesAgent | internal/harness/drain_test.go | core/tests/engine.rs::resumed_background_agent_is_active_again_until_task_notification | covered | name-level |
| TestExtractRecoMarkdownMultiline | internal/harness/handoff_test.go | core/src/rotate.rs::extract_reco_takes_last_marker_block_through_end | covered | name-level |
| TestExtractRecoNone | internal/harness/handoff_test.go | core/src/rotate.rs::extract_reco_none | covered |  |
| TestValidateHandoffGood | internal/harness/handoff_test.go | core/src/rotate.rs::validate_good_note_has_no_problems | covered |  |
| TestValidateHandoffTooShort | internal/harness/handoff_test.go | core/src/rotate.rs::validate_flags_short_note | covered |  |
| TestValidateHandoffMissingSection | internal/harness/handoff_test.go | core/src/rotate.rs::validate_flags_missing_section | covered |  |
| TestValidateHandoffUnmentionedFile | internal/harness/handoff_test.go | core/src/rotate.rs::validate_flags_unmentioned_file | covered |  |
| TestWithRecoRoundTripKorean | internal/harness/handoff_test.go | core/src/rotate.rs::with_reco_round_trip_and_preservation | covered | name-level: Korean input not confirmed in Rust test |
| TestWithRecoEmpty | internal/harness/handoff_test.go | core/src/rotate.rs::with_reco_empty_is_unchanged | covered |  |
| TestHistoryFileRoundTrip | internal/harness/history_file_test.go | cli/src/tui/edit.rs::history_file_round_trip | covered |  |
| TestHistoryFileLoadRules | internal/harness/history_file_test.go | cli/src/tui/edit.rs::history_file_load_rules_skip_bad_lines_and_cap | covered |  |
| TestHistoryFileFor | internal/harness/history_file_test.go | cli/src/tui/edit.rs::history_file_path_is_under_chaude | covered |  |
| TestRecordHistoryPersistsAndNewModelLoads | internal/harness/history_file_test.go | cli/src/tui/edit.rs::recorded_history_persists_and_a_new_editor_loads_it | covered |  |
| TestUpDownHistoryWithSlashCompleter | internal/harness/history_file_test.go | - | ignored | ignored (TUI behaviour not in Rust yet): slash-completer interplay with up/down not tested in Rust  [p12: up_down_history_with_slash_completer] |
| TestUpMovesCompleterWhenNotAtTop | internal/harness/history_file_test.go | - | ignored | ignored (TUI behaviour not in Rust yet): completer movement not tested in Rust  [p12: up_moves_completer_when_not_at_top] |
| TestHistoryDedupeConsecutive | internal/harness/history_file_test.go | cli/src/tui/edit.rs::history_dedupes_consecutive_entries_across_restart | covered |  |
| TestInputHistory | internal/harness/history_test.go | cli/src/tui/edit.rs::history_walks_back_and_forth_and_restores_the_draft | covered |  |
| TestInputHistoryCap | internal/harness/history_test.go | cli/src/tui/edit.rs::history_file_load_rules_skip_bad_lines_and_cap | covered | name-level: cap checked on file load only |
| TestRewindState | internal/harness/history_test.go | crates/cli/src/tui/rewind.rs::rewind_state_follows_go | covered | body read: rewind open/move/wrap/confirm/cancel match Go (rewind.rs:146) |
| TestAgentCounts | internal/harness/history_test.go | crates/core/tests/parity_final2.rs::agent_counts | covered | fg/bg counts via running() filtered by background |
| TestIdleControlRequestAnswered | internal/harness/idle_test.go | crates/core/tests/engine.rs::can_use_tool_in_default_mode_asks_ui_and_sends_answer | partial | ask/answer half covered; remaining assertions stub: crates/core/tests/parity_final3_ignored.rs::go_idle_control_request_answered_rest |
| TestIdleDeniesWhenNoPrompt | internal/harness/idle_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): idle can_use_tool with no UI prompt must deny  [p12: idle_denies_when_no_prompt] |
| TestStrayLateResultSkipped | internal/harness/idle_test.go | crates/core/tests/engine.rs::stray_result_of_abandoned_turn_does_not_end_next_turn | covered | name-level match |
| TestIdleBackgroundFollowupRendered | internal/harness/idle_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): task-notification between turns renders "에이전트 완료" and clears tracker  [p12: idle_background_followup_rendered] |
| TestIdleSpendRow | internal/harness/idlespend_test.go | crates/core/tests/parity_final3.rs::idle_spend_row_sub_and_spawns | partial | background row sub tokens and spawns covered (final3); result_model_usage stub: crates/core/tests/parity_final3_ignored.rs::go_idle_spend_row_background_row  [p12: idle_spend_row] |
| TestIdleSpendLiveReplay | internal/harness/idlespend_test.go | crates/core/tests/parity_final3.rs::idle_spend_live_replay | covered | reads crates/core/tests/fixtures/bg_turn_live.jsonl (final3) |
| TestReadClaudeTranscriptFullView | internal/harness/importview_test.go | crates/core/src/import_view.rs::tool_use_result_and_thinking_are_rendered_in_order, crates/core/src/import_view.rs::subagent_renders_after_its_agent_call, crates/core/tests/parity_final3.rs::read_claude_transcript_full_view | covered | full view read (final3) plus import_view unit tests |
| TestInputWidthMixed | internal/harness/input_test.go | crates/cli/src/tui/render.rs::mixed_width_measure; crates/cli/src/tui/edit.rs::mixed_width_cursor_and_backspace_follow_cells | covered | round 3: cursor cells, left moves and backspace over a wide char ported (Editor has no width; cell column via cells()) |
| TestInputWrapWide | internal/harness/input_test.go | crates/cli/src/tui/render.rs::wrap_cells_breaks_by_cell_width | partial | wide-cell wrap covered; remaining cases stub: crates/core/tests/parity_final3_ignored.rs::go_input_wrap_wide_rest |
| TestInputNewline | internal/harness/input_test.go | crates/cli/src/tui/edit.rs::newline_is_a_marker_column_and_home_end_stay_on_the_line | covered | name-level match |
| TestInputKillToStart | internal/harness/input_test.go | crates/cli/src/tui/edit.rs::kill_to_start_cases | covered | name-level match |
| TestInputDeleteWordPunct | internal/harness/input_test.go | crates/cli/src/tui/edit.rs::delete_word_is_punctuation_aware | covered | name-level match |
| TestTUIAltBackspaceAndCtrlU | internal/harness/input_test.go | crates/cli/src/tui/edit.rs::alt_backspace_then_ctrl_u_on_a_single_line | covered | name-level match |
| TestLinkSpansDetects | internal/harness/links_test.go | crates/cli/src/tui/render.rs::link_spans_detects_urls_paths_and_trims_tail | covered | name-level match |
| TestLinkSpansProseNegatives | internal/harness/links_test.go | crates/cli/src/tui/render.rs::link_spans_prose_negatives | covered | name-level match |
| TestLinkSpansOverlapAndBoundary | internal/harness/links_test.go | crates/cli/src/tui/render.rs::link_spans_overlap_and_boundary | covered | name-level match |
| TestLinkifyPreservesVisibleText | internal/harness/links_test.go | crates/cli/src/tui/render.rs::linkify_keeps_visible_text_and_width | covered | name-level match |
| TestWrapLinkedSkipsFence | internal/harness/links_test.go | crates/cli/src/tui/render.rs::wrap_linked_skips_fenced_code | covered | name-level match |
| TestWrapLinkedColorsWholeTokens | internal/harness/links_wrap_test.go | crates/cli/src/tui/render.rs::wrap_linked_colors_whole_tokens | covered | name-level match |
| TestPlainLocalCmdsNoSend | internal/harness/localcmd_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): plain local commands must not send a prompt; crates/cli/src/tui/commands.rs::plain_text_is_not_a_command is not the same case  [p12: plain_local_cmds_no_send] |
| TestMetricsLog | internal/harness/metrics_test.go | crates/core/src/metrics.rs::log_appends_one_line_per_record_with_ts_and_flags | covered | name-level match |
| TestMetricsNilAndDisabled | internal/harness/metrics_test.go | crates/core/src/metrics.rs::disabled_log_is_noop | covered | name-level match (nil case not separately checked) |
| TestUsageFields | internal/harness/metrics_test.go | crates/core/src/metrics.rs::usage_fields_sums_ctx_and_defaults_missing_to_zero | covered | name-level match |
| TestModeCycleAndParse | internal/harness/mode_test.go | crates/core/src/mode.rs::cycle_and_parse | covered | name-level match |
| TestDecideMode | internal/harness/mode_test.go | crates/core/src/mode.rs::decide_mode_table | covered | name-level match |
| TestReplModeLine | internal/harness/mode_test.go | crates/cli/src/tui/commands.rs::mode_no_args_set_and_unknown | covered | body read: mode line text checked by mode_no_args_set_and_unknown |
| TestTouchMainOnly | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestIdleEventFromParkedChild | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestIdleEventParkedNoRender | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestIdleEventParkedPermDenied | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestParkedReplyNoUnseen | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestBindUnbindRoundTrip | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestBindAddsParkedDelta | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestBindNoDeltaKeepsEmpty | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestTrackerNeverNil | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestUnbindParkedRace | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestMakeSnapshotStates | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestParkReplaceClosesOnce | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestRouteSameModelWorkerUsesOldPath | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestIdleDue | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestIdleDueDisabled | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestSweepIdleClosesParkedOnly | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestSweepDrainsBusyParked | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestSweepSkippedWhileBusy | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestSweepSkippedDuringPermPrompt | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestIdleRotateKeepsPendingSnapshot | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestIdleNoticeKeepsHeldOffset | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestQuitCallsCloseAll | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestPlainLoopCallsCloseAll | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestQuitCloseAllWithNilSess | internal/harness/multiproc_s_test.go | - | n-a | Go parked-child pool (p2 multiproc_s_test.go); Rust uses resident pool in crates/core/src/api.rs |
| TestOrchParseMode | internal/harness/orch_test.go | crates/core/src/orch.rs::parse_mode_normalizes_and_rejects | covered | name-level match |
| TestOrchDefaultModel | internal/harness/orch_test.go | crates/core/src/orch.rs::default_model_per_mode | covered | name-level match |
| TestOrchRule | internal/harness/orch_test.go | crates/core/src/orch.rs::rule_per_mode | covered | name-level match |
| TestParseWSCases | internal/harness/orchws_test.go | crates/core/src/orch.rs::parse_ws_cases | covered | name-level match |
| TestWSNeedsRotateCases | internal/harness/orchws_test.go | crates/core/src/orch.rs::needs_rotate_cases | covered | name-level match |
| TestWSHandoffRuneCap | internal/harness/orchws_test.go | crates/core/src/orch.rs::handoff_is_rune_capped_with_prefix | covered | name-level match |
| TestRouteOrchBReturnsLine | internal/harness/orchws_test.go | crates/core/src/orch.rs::route_non_f_mode_keeps_line | covered | body read: route_non_f_mode_keeps_line checks the returned line |
| TestSessionPanelAndThinking | internal/harness/panel_test.go | crates/cli/src/tui/bar.rs::thinking_row_shows_only_for_opus_resident_or_pre_text | partial | thinking row covered; panel assertions stub: crates/core/tests/parity_final3_ignored.rs::go_session_panel_rest |
| TestEscInterruptClearsForegroundPanel | internal/harness/panel_test.go | crates/cli/src/tui/mod.rs::a_single_escape_interrupts_only_while_busy; crates/cli/src/tui/mod.rs::interrupt_finishes_foreground_agents_only; crates/cli/src/tui/mod.rs::turn_end_finishes_foreground_agents_only | covered | round 3: turn end (Busy(false)) finishes foreground agents like Go doneMsg (tui.go:517); fixed in mod.rs Busy handler |
| TestIdleTickRefreshesWhenBackgroundFinishes | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): idle tick refresh of panel after background finishes  [p12: idle_tick_refreshes_when_background_finishes] |
| TestCursorRowMatchesPanelAfterTick | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): cursor row vs panel after tick  [p12: cursor_row_matches_panel_after_tick] |
| TestPanelMoreRowAndSmallHeight | internal/harness/panel_test.go | crates/cli/src/tui/bar.rs::every_running_agent_stays_visible_when_row_is_narrow | ignored | bin-only (cli has no lib target): more-row and small-height panel; stub: crates/core/tests/parity_final3_ignored.rs::go_panel_more_row_small_height |
| TestThinkingOnlyForForeground | internal/harness/panel_test.go | crates/cli/src/tui/bar.rs::thinking_row_shows_only_for_opus_resident_or_pre_text | covered | body read: thinking row shown only for foreground agent |
| TestBackgroundRowsBelowInputBox | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): background rows layout below input  [p12: background_rows_below_input_box] |
| TestBackgroundSmallHeightNoOverflow | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): background rows, small height  [p12: background_small_height_no_overflow] |
| TestStalledOutputFileKeepsAgentRunningUntilDoneSignal | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): stalled output file keeps agent running until done  [p12: stalled_output_file_keeps_agent_running] |
| TestQueueWhileBusyNotSent | internal/harness/panel_test.go | crates/core/tests/engine.rs::submit_while_busy_queues_and_never_interrupts | covered | name-level match |
| TestQueueSentOnDoneInOrder | internal/harness/panel_test.go | crates/core/tests/engine.rs::queued_prompts_go_out_as_one_merged_turn_on_done | covered | body read: Go sends "second\n\nthird" after first done; Rust merges queued prompts into one turn on done. Same order and merge |
| TestQueueLinesDimAboveInput | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): queued lines dim above input (render)  [p12: queue_lines_dim_above_input] |
| TestBackgroundOnlyDoesNotQueue | internal/harness/panel_test.go | crates/core/tests/engine.rs::background_only_agent_does_not_queue_next_prompt | covered | name-level match |
| TestQueueSmallHeightNoOverflow | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): queue rows, small height  [p12: queue_small_height_no_overflow] |
| TestCursorRowIncludesBackgroundRows | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): cursor row counts background rows  [p12: cursor_row_includes_background_rows] |
| TestAgentStatusTextPrefixedName | internal/harness/panel_test.go | crates/cli/src/tui/bar.rs::agent_name_with_model_prefix_skips_model_segment | partial | model-prefix skip covered; remaining cases stub: crates/core/tests/parity_final3_ignored.rs::go_agent_status_prefixed_rest |
| TestAgentStatusFormat | internal/harness/panel_test.go | crates/cli/src/tui/bar.rs::agent_status_text_uses_state_model_and_name | partial | status text covered; remaining format cases stub: crates/core/tests/parity_final3_ignored.rs::go_agent_status_format_rest |
| TestPanelScrollWindow | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): panel scroll window  [p12: panel_scroll_window] |
| TestUserEventNotificationFinishesBackground | internal/harness/panel_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): user-event notification finishes background row  [p12: user_event_notification_finishes_background] |
| TestSetRateLogsOnFiveHourChange | internal/harness/ratestatus_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): rate log row (stage "rate") on five-hour change; Rust metrics has no rate stage test  [p12: set_rate_logs_on_five_hour_change] |
| TestSetRateTopLevelFallback | internal/harness/ratestatus_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): rate top-level fallback  [p12: set_rate_top_level_fallback] |
| TestFmtRateStatus | internal/harness/ratestatus_test.go | crates/cli/src/tui/mod.rs::rate_status_follows_go_fmt_rate_status | covered | name-level match |
| TestRecordMidnightBoundary | internal/harness/record_test.go | crates/core/tests/parity_record.rs::midnight_boundary_keeps_newest_file_and_stamps_new_day | covered | new this pass |
| TestRecordSplitOnClearStartsNewFile | internal/harness/record_test.go | crates/core/src/record.rs::append_fills_one_file_then_splits_on_request | covered | split flag path |
| TestRecordSplitByCaps | internal/harness/record_test.go | crates/core/tests/parity_record.rs::split_by_caps_starts_new_file_at_101st_answer | covered | new; 100 answers cap, Rust MAX_ANSWERS = 100 |
| TestRecordSlugKorean | internal/harness/record_test.go | crates/core/src/record.rs::slugify_keeps_korean_and_bounds_length | covered |  |
| TestRecordProjectName | internal/harness/record_test.go | crates/core/src/record_folder.rs::project_name_is_the_slugged_basename | covered |  |
| TestRecordRoundTripParse | internal/harness/record_test.go | crates/core/src/record.rs::render_then_parse_round_trips_blocks | covered |  |
| TestRecordNumberContinuesAfterRestart | internal/harness/record_test.go | crates/core/src/record.rs::last_number_reads_the_tail_of_a_big_file | covered | name-level match, body not compared |
| TestRecordEscapeHeaderInBody | internal/harness/record_test.go | crates/core/src/record.rs::body_line_that_looks_like_a_header_survives | covered |  |
| TestRecordNumberIgnoresMtime | internal/harness/record_test.go | crates/core/tests/parity_record.rs::number_ignores_mtime | covered | new this pass |
| TestRecordTopicParse | internal/harness/record_test.go | crates/core/src/record.rs::topic_prefers_last_switch_then_last_cat_then_prev | covered |  |
| TestRecordFolderListing | internal/harness/record_test.go | crates/core/src/record_folder.rs::listing_skips_hidden_and_empty_folders_newest_first | covered |  |
| TestRecordFolderQueryFilter | internal/harness/record_test.go | crates/core/src/record_folder.rs::filter_matches_name_topic_or_text_case_insensitively | covered |  |
| TestRecordFolderLoadAllInTimeOrder | internal/harness/record_test.go | crates/core/src/record_folder.rs::load_project_gives_all_files_in_rec_order | covered |  |
| TestRecordMigrateLegacyRoot | internal/harness/record_test.go | crates/core/src/record_folder.rs::migrate_root_moves_legacy_files_into_owner_folder | covered |  |
| TestRecordSkip | internal/harness/record_turn_test.go | crates/core/src/record_turn.rs::skip_slash_commands_and_empty_prompts | covered | body read: slash commands and empty prompts skipped |
| TestMeasureLine | internal/harness/record_turn_test.go | crates/core/src/record_turn.rs::measure_line_fixed_then_result_keys | covered | body read: measure line key order matches |
| TestAgentPromptLoaded | internal/harness/resident_agent_test.go | none | covered | agent prompt file loaded (crates/core/src/resident.rs::strip_frontmatter_drops_header_block covers frontmatter only)  [p12: agent_prompt_loaded] |
| TestAgentPromptMissingNotice | internal/harness/resident_agent_test.go | none | partial | missing agent prompt shows notice (p12: agent_prompt_missing_is_err); notice and agent-less spawn stub: crates/core/tests/parity_final3_ignored.rs::go_agent_prompt_missing_notice  [p12: agent_prompt_missing_is_err] |
| TestStageLinesOrder | internal/harness/resident_agent_test.go | crates/core/tests/engine.rs::resident_stage_rows_bracket_main_turn_and_agent_calls | partial | stage row bracket covered; full order stub: crates/core/tests/parity_final3_ignored.rs::go_stage_lines_full_order |
| TestSessionArgsMinimal | internal/harness/resident_agent_test.go | crates/core/src/proc.rs::args_match_go_session_defaults | covered | body read: Rust args checked against Go session defaults |
| TestSessionArgsCoder | internal/harness/resident_agent_test.go | crates/core/src/proc.rs::resident_shape_args_match_go_session_args | covered | body read: coder session args |
| TestMainStrictMCP | internal/harness/resident_agent_test.go | crates/core/src/proc.rs::mcp_config_flag_only_when_set, crates/core/tests/parity_final3.rs::main_strict_mcp_args, crates/core/tests/parity_final3_resident.rs::main_spawned_once_with_strict_mcp | covered | strict-MCP args (final3) plus one main spawn across a turn (engine, final3 resident) |
| TestResidentRunningListShowsOnlyBusySessions | internal/harness/resident_busy_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): running list shows only busy residents  [p12: resident_running_list_shows_only_busy_sessions] |
| TestResidentCoderReply | internal/harness/resident_coder_test.go | crates/core/tests/engine.rs::coder_task_emits_working_done_rows_and_reply | covered | body read: coder reply rows |
| TestResidentCoderSlotReuse | internal/harness/resident_coder_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): coder slot reuse  [p12: resident_coder_slot_reuse] |
| TestResidentCoderBusyRefuses | internal/harness/resident_coder_test.go | none | ignored | ignored (Rust differs by design): Go refuses a busy single coder; Rust pool errors "all resident coders are busy" (crates/core/src/api.rs:1046). Planned #[ignore = "parity: single-coder busy refusal vs pool error"]  [p12: resident_coder_busy_refuses] |
| TestCoderPermissionOnlyWhenBusy | internal/harness/resident_coder_test.go | none | ignored | ignored (TUI behaviour not in Rust yet): coder permission prompt only when busy  [p12: coder_permission_only_when_busy] |
| TestMCPRoundTripFakeSession | internal/harness/resident_coder_test.go | crates/core/tests/parity_final3.rs::mcp_round_trip_fake_session | covered | fake session MCP round trip (final3) |
| TestResidentMCPConfigWritten | internal/harness/resident_coder_test.go | crates/core/tests/parity_final3.rs::resident_mcp_config_written, crates/core/tests/parity_final3_resident.rs::mcp_config_path_reused_after_respawn | covered | config written with 0600 mode (final3) plus same mcp.json path after respawn (engine, final3 resident)  [p12: mcp_config_args] |
| TestExtraCoderPoolFromSetting | internal/harness/resident_extra_test.go | crates/cli/src/tui/settings.rs::extra_coders_wins_over_legacy_key; crates/core/tests/parity_harness_p3.rs::default_extra_pool_and_idle_ttl_match_go | covered | default pool 4 asserted in p3 (new); the setting parse is name-level |
| TestExtraCoderExpiresAt270s | internal/harness/resident_extra_test.go | crates/core/tests/engine.rs::idle_extra_coder_expires_and_base_coder_stays; crates/core/tests/parity_harness_p3.rs::default_extra_pool_and_idle_ttl_match_go | covered | engine test uses 300 ms TTL; p3 asserts the default 270 s |
| TestViaPrompterFailureNotice | internal/harness/resident_pipeline_test.go | crates/core/tests/engine.rs::prompter_failure_falls_back_to_raw_prompt | covered | name-level |
| TestViaTUITranslates | internal/harness/resident_pipeline_test.go | crates/core/tests/engine.rs::tui_answer_replaces_main_reply_after_turn | covered | name-level |
| TestViaTUIFailureNotice | internal/harness/resident_pipeline_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_via_t_u_i_failure_notice | ignored | ignored (core seam/API not wired): #[ignore] stub: resident TUI translator failure notice seam not wired |
| TestRunTurnShowsTUIOutputOnly | internal/harness/resident_pipeline_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_run_turn_shows_t_u_i_output_only | ignored | ignored (core seam/API not wired): #[ignore] stub: run_turn TUI-only output seam not wired |
| TestSendResidentRespawnsDeadSlot | internal/harness/resident_respawn_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_send_resident_respawns_dead_slot | ignored | ignored (core seam/API not wired): #[ignore] stub: resident dead-slot respawn not wired |
| TestViaPrompterRawWhenRespawnFails | internal/harness/resident_respawn_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_via_prompter_raw_when_respawn_fails | ignored | ignored (core seam/API not wired): #[ignore] stub: respawn-failure raw fallback not wired |
| TestSendResidentBusyUntilResult | internal/harness/resident_route_test.go | crates/core/tests/engine.rs::coder_task_emits_resident_busy_bracket_before_reply | covered | name-level |
| TestSpawnSlotParksReadingChild | internal/harness/resident_route_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_spawn_slot_parks_reading_child | ignored | ignored (core seam/API not wired): #[ignore] stub: spawnSlot seam not wired |
| TestViaPrompterSummaryOnly | internal/harness/resident_route_test.go | crates/core/src/resident.rs::summary_is_composed_with_original_and_stage_done | covered | name-level |
| TestViaPrompterOKNoOriginal | internal/harness/resident_route_test.go | crates/core/src/resident.rs::empty_summary_passes_input_through_without_stage | covered | name-level |
| TestViaPrompterNeedOriginalFlag | internal/harness/resident_route_test.go | crates/core/src/resident.rs::need_original_flags_exact_inputs | covered |  |
| TestViaPrompterNoFlagHeuristic | internal/harness/resident_route_test.go | crates/core/src/resident.rs::need_original_flags_exact_inputs | covered | same test covers the no-flag path |
| TestViaPrompterEmptySummaryReturnsText | internal/harness/resident_route_test.go | crates/core/src/resident.rs::empty_summary_passes_input_through_without_stage | covered |  |
| TestNeedOriginal | internal/harness/resident_route_test.go | crates/core/src/resident.rs::need_original_flags_exact_inputs | covered |  |
| TestViaPrompterFallsBack | internal/harness/resident_route_test.go | crates/core/tests/engine.rs::prompter_failure_falls_back_to_raw_prompt | covered |  |
| TestResidentUsageFillsRoleSlot | internal/harness/resident_route_test.go | crates/cli/src/tui/bar.rs::role_tokens_fill_the_prompter_tui_coder_slots | covered |  |
| TestResidentSpawnsEager | internal/harness/resident_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_resident_spawns_eager | ignored | ignored (core seam/API not wired): #[ignore] stub: eager resident spawn not on a pub core API |
| TestResidentSlotsFour | internal/harness/resident_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_resident_slots_four | ignored | ignored (core seam/API not wired): #[ignore] stub: resident slot count not on a pub core API |
| TestGroupSessions | internal/harness/resume_group_test.go | crates/core/src/resume_group.rs::group_sessions_joins_or_splits_by_clear_gap_and_overlap | covered | 7 cases as Go: gap, clear at tail/head, day gap, overlap, TUI clear mark |
| TestGroupOrderAndKey | internal/harness/resume_group_test.go | crates/core/src/resume_group.rs::group_order_and_key | covered |  |
| TestMatchGroups | internal/harness/resume_group_test.go | crates/core/src/resume_group.rs::match_groups_terms_typos_and_cwd | covered | Levenshtein <=1 for 4+ char terms |
| TestLoadImportArgMergesInTimeOrder | internal/harness/resume_group_test.go | crates/core/src/import_claude.rs::merge_orders_files_by_start_and_returns_newest_path | covered |  |
| TestReadClaudeTranscriptClearFlags | internal/harness/resume_group_test.go | crates/core/src/import_claude.rs::transcript_extracts_turns_files_name_and_clear_markers | covered | name-level; head-clear flag not body-compared |
| TestClearMarks | internal/harness/resume_group_test.go | crates/core/src/resume_group.rs::clear_marks_are_per_cwd_and_missing_file_is_empty | covered |  |
| TestResumeTarget | internal/harness/resume_group_test.go | crates/core/src/record_turn.rs::resume_target_picks_record_claude_or_sessions | covered | record_turn::resume_target_in; Go cwd arg maps to record dir |
| TestRecordPickCandidates | internal/harness/resume_picker_test.go | crates/cli/src/tui/records.rs::pick_candidates_follow_go | covered |  |
| TestRecordPickerKeysWheelEnter | internal/harness/resume_picker_test.go | crates/cli/src/tui/records.rs::picker_keys_wheel_enter_follow_go | covered |  |
| TestListClaudeSessions | internal/harness/resume_test.go | crates/cli/src/tui/sessview.rs::list_sessions (fn, used by list_draws_sessions_and_esc_closes) | covered | name differs (list_sessions), behaviour same |
| TestViewHelpers | internal/harness/resume_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_view_helpers | ignored | ignored (bin-only/cli-private helper): #[ignore] stub: cli-private sessview helpers; bin-only crate |
| TestJediTermIMEColumnShift | internal/harness/resumeshift_test.go | crates/cli/src/tui/mod.rs::ime_shift_follows_env_override_and_jediterm | covered |  |
| TestImportShowsTurnsInTUI | internal/harness/resumeshift_test.go | crates/cli/src/tui/mod.rs::import_replaces_the_transcript_view | covered |  |
| TestRotateKeepsPendingSnapshot | internal/harness/resumeshift_test.go | crates/core/tests/parity_final3_ignored.rs::go_rotate_keeps_pending_snapshot | ignored | core seam: Rust keeps pending snapshot (api.rs rotate_keep_snapshot); Repl rotate needs fake newSessionFn seam; not a design difference |
| TestSelectionAnchorStaysOnContentAfterAppend | internal/harness/selection_test.go | crates/cli/src/tui/render.rs::selection_anchor_stays_on_content_after_append | covered |  |
| TestSelectionMultiLineAndReversedOrder | internal/harness/selection_test.go | crates/cli/src/tui/render.rs::selection_text_multi_line_and_reversed | covered |  |
| TestReleaseCopiesSelection | internal/harness/selection_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_release_copies_selection | ignored | ignored (bin-only/cli-private helper): #[ignore] stub: cli-private clipboard release; bin-only crate |
| TestViewEscBacksOutToNormalScreen | internal/harness/sessview_test.go | crates/cli/src/tui/sessview.rs::list_draws_sessions_and_esc_closes | covered |  |
| TestViewSessionSelectionAndPromptNavigation | internal/harness/sessview_test.go | crates/cli/src/tui/sessview.rs::selection_and_prompt_navigation_follow_go | covered |  |
| TestViewArgOpensSessionDirectly | internal/harness/sessview_test.go | crates/cli/src/tui/sessview.rs::view_arg_opens_session_directly | covered |  |
| TestViewListsOnlyOwnSessions | internal/harness/sessview_test.go | crates/cli/src/tui/sessview.rs::view_lists_only_own_sessions | covered |  |
| TestViewWheelMovesSessionCursor | internal/harness/sessview_test.go | crates/cli/src/tui/sessview.rs::wheel_moves_session_cursor_and_wraps | covered |  |
| TestViewPromptScrollsInsideTranscript | internal/harness/sessview_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_view_prompt_scrolls_inside_transcript | ignored | ignored (bin-only/cli-private helper): #[ignore] stub: cli-private sessview prompt scroll; bin-only crate |
| TestSpecterIdleNoElapsed | internal/harness/specter_test.go | crates/cli/src/tui/bar.rs::idle_bar_without_status_says_so | covered | closest match; not body-compared |
| TestSpecterRowRenders | internal/harness/specter_test.go | crates/cli/src/tui/bar.rs::busy_bar_lists_agents_status_and_queue | covered | closest match; not body-compared |
| TestIsCavemanBlock | internal/harness/tui_caveman_test.go | crates/cli/src/tui/render.rs::caveman_blocks_are_detected_and_hidden | covered |  |
| TestToolResultCavemanHidden | internal/harness/tui_caveman_test.go | crates/cli/src/tui/mod.rs::caveman_tool_result_is_hidden_from_the_transcript | covered |  |
| TestTurnWaitsForOpenAgentToolResult | internal/harness/turn_wait_test.go | crates/core/tests/engine.rs::slow_agent_result_is_delivered_and_timer_is_paused_without_interrupt | covered | open Agent tool result is waited for; timer paused; no interrupt |
| TestTurnQuietNoticeKeepsWaiting | internal/harness/turn_wait_test.go | crates/core/tests/engine.rs::silent_turn_emits_still_running_notice_and_never_interrupts | covered |  |
| TestTurnSpendSubagentUsageMaxPerMessage | internal/harness/turnspend_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_turnspend_subagent_usage_max_per_message | covered | TurnSpend::on_event/fields; body-level |
| TestTurnSpendMainUsageIgnored | internal/harness/turnspend_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_turnspend_main_usage_ignored | covered | body-level |
| TestTurnSpendSpawnCounting | internal/harness/turnspend_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_turnspend_spawn_counting | covered | body-level; duplicate tool id counted once |
| TestTurnSpendEmptyAndNil | internal/harness/turnspend_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_turnspend_empty_and_no_panic | covered | Go nil receiver has no Rust counterpart; empty and no-panic checked |
| TestParseSamples | internal/harness/unit_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_parse_samples_probe1 | covered | samples/probe1.jsonl via import_view::parse_event; ctx from usage_fields |
| TestCtxTokens | internal/harness/unit_test.go | crates/core/src/event.rs::usage_context_tokens_sums_input_and_cache | covered |  |
| TestObservedTTL | internal/harness/unit_test.go | crates/core/src/cache_ttl.rs::observed_ttl_reads_written_tier_else_prev | covered | helper only; api.rs does not call observed_ttl yet (Go livechild.go touchMain) |
| TestSnapshot | internal/harness/unit_test.go | crates/core/src/import_claude.rs::snapshot_keeps_tail_turns_and_files | covered | name-level |
| TestCleanEnv | internal/harness/unit_test.go | crates/core/src/proc.rs::apply_env_drops_api_key_and_sets_only_valid_ttl | covered | name-level: env delta (ANTHROPIC_API_KEY removed, exact window value), not inherited env |
| TestWithCacheTTL | internal/harness/unit_test.go | crates/core/src/cache_ttl.rs::with_cache_ttl_pins_single_entry_last | covered | name-level: pure helper; proc apply_env sets the TTL entry, not with_cache_ttl |
| TestFmtK | internal/harness/unit_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_fmt_k | ignored | ignored (bin-only/cli-private helper): #[ignore] stub: fmt_k private in cli bar.rs; bin-only crate |
| TestCtxBar | internal/harness/unit_test.go | crates/core/tests/parity_harness_p3_ignored.rs::go_ctx_bar | ignored | ignored (bin-only/cli-private helper): #[ignore] stub: cli-private ctx bar; bin-only crate |
| TestRenderBar | internal/harness/usagebar_test.go | crates/cli/src/tui/render.rs::render_bar_cases_and_width | covered |  |
| TestDecorateUsage | internal/harness/usagebar_test.go | crates/cli/src/tui/render.rs::decorate_usage_adds_bars | covered |  |
| TestUsageAllowedWhileBusy | internal/harness/usagebusy_test.go | crates/cli/src/tui/commands.rs::usage_is_accepted_while_busy | covered |  |
| TestUsageSlashAcceptedWhileBusy | internal/harness/usagebusy_test.go | crates/cli/src/tui/commands.rs::usage_is_accepted_while_busy | covered |  |
| TestUsageMsgWhileBusyOpensPopup | internal/harness/usagebusy_test.go | crates/cli/src/tui/usage.rs::usage_event_fills_window_and_error_goes_to_window_while_pending | covered | name-level |
| TestParseUsage | internal/harness/usagepoll_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_parse_usage_session_and_week | covered | usage_poll::parse_usage; body-level |
| TestPollUsageLogsRow | internal/harness/usagepoll_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_poll_usage_logs_row | covered | usage_poll::log_usage row; Go Repl.pollUsageSync seam not in Rust |
| TestPollUsageError | internal/harness/usagepoll_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_poll_usage_error_row | covered | usage_poll::log_usage error row |
| TestTurnCostUSDDelta | internal/harness/usagepoll_test.go | crates/core/tests/parity_harness_p3_spend.rs::go_turn_cost_usd_delta | covered | TurnCost::delta; Go Repl.turnCostUSD seam not in Rust |
| TestUsageLive | internal/harness/usagepoll_test.go | none | n-a | Go hits the real claude CLI, skipped unless HARNESS_LIVE_USAGE=1; Rust usage path is covered by engine.rs::usage_answers_with_result_text_from_one_shot_run |
| TestUsageViewScrollBounds | internal/harness/usageview_test.go | crates/cli/src/tui/usage.rs::scroll_bounds_match_go | covered |  |
| TestUsageViewWheelAndRefreshKeepScroll | internal/harness/usageview_test.go | crates/cli/src/tui/usage.rs::wheel_and_refresh_keep_scroll | covered |  |
| TestUsageViewEscCloses | internal/harness/usageview_test.go | crates/cli/src/tui/usage.rs::esc_closes_and_returns_true | covered |  |
| TestUsageOpenNotInline | internal/harness/usageview_test.go | crates/cli/src/tui/usage.rs::usage_command_opens_window_without_transcript_block | covered |  |
| TestCharcountBOMAndCRLFMatchPlain | internal/scripts/charcount_test.go | crates/scripts/tests/charcount.rs::bom_and_crlf_match_plain_lf | covered |  |
| TestCharcountBOMOnlyStripped | internal/scripts/charcount_test.go | crates/scripts/tests/charcount.rs::bom_only_is_stripped | covered |  |
| TestEnsureProjectSettingsCreatesGuard | internal/settings/project_test.go | crates/core/src/settings/project.rs::creates_guard | covered |  |
| TestEnsureProjectAgentsKeepExisting | internal/settings/project_test.go | crates/core/src/settings/project.rs::agents_written_and_existing_kept | covered |  |
| TestEnsureProjectSettingsMergesExisting | internal/settings/project_test.go | crates/core/src/settings/project.rs::merges_existing_settings | covered |  |
| TestMergeGuardRejectsBadJSON | internal/settings/project_test.go | crates/core/src/settings/project.rs::rejects_non_object_settings | covered | name-level match |
| TestEnsureProjectSettingsSkipsHome | internal/settings/project_test.go | crates/core/src/settings/project.rs::skips_home | covered |  |
| TestEmbeddedHooksMatchRepo | internal/settings/project_test.go | crates/core/src/settings/project.rs::embedded_hooks_match_repo | covered |  |
| TestEmbeddedAgentsMatchRepo | internal/settings/project_test.go | crates/core/src/settings/project.rs::embedded_agents_match_repo | covered |  |
| TestHookPythonOrder | internal/settings/project_test.go | crates/core/src/settings/project.rs::hook_python_order | covered |  |
| TestDefaultSrcPrecedence | internal/settings/settings_test.go | crates/core/src/settings/mod.rs::default_src_precedence | covered |  |
| TestGenSettingsCreatesSettingsFolder | internal/settings/settings_test.go | crates/core/src/settings/mod.rs::gen_settings_creates_settings_folder | covered |  |
| TestNotWritableChangesNothing | internal/update/install_path_test.go | crates/update/tests/parity_update.rs::not_writable_changes_nothing | covered | added; unix; skips if probe write succeeds (root) |
| TestMigrateLegacyRemovesOldInstallAndPathLine | internal/update/install_path_test.go | crates/update/tests/parity_update.rs::migrate_legacy_removes_old_install_and_path_line | covered | added |
| TestMigrateLegacySkipsWhenRunningLegacyBinary | internal/update/install_path_test.go | crates/update/tests/parity_update.rs::migrate_legacy_skips_when_running_legacy_binary | covered | added |
| TestMigrateLegacyNoopWithoutLegacyFiles | internal/update/install_path_test.go | crates/update/tests/parity_update.rs::migrate_legacy_noop_without_legacy_files | covered | added |
| TestCompareVersion | internal/update/update_test.go | crates/update/tests/parity_update.rs::compare_version | covered | added; Version parse+Ord |
| TestUpToDateDownloadsNothing | internal/update/update_test.go | crates/update/tests/parity_update.rs::up_to_date_downloads_nothing | covered | added |
| TestUpdateUnixReplacesBinaryAtomically | internal/update/update_test.go | crates/update/tests/parity_update.rs::update_unix_replaces_binary_atomically | covered | added; mode 0755, no leftovers |
| TestUpdateWindowsRenamesOldAndCleanupRemovesIt | internal/update/update_test.go | crates/update/tests/parity_update.rs::update_windows_renames_old_and_cleanup_removes_it | covered | added; uses cleanup::remove_old seam |
| TestChecksumMismatchInstallsNothing | internal/update/update_test.go | crates/update/tests/parity_update.rs::checksum_mismatch_installs_nothing | covered | added |
| TestChecksumsRequired | internal/update/update_test.go | crates/update/tests/parity_update.rs::checksums_required | covered | added |
| TestNoAssetForPlatform | internal/update/update_test.go | crates/update/tests/parity_update.rs::no_asset_for_platform | covered | added |
| TestOfflineIsClearError | internal/update/update_test.go | crates/update/tests/parity_update.rs::offline_is_clear_error | covered | added; Fake returns Error::Offline (no httptest) |
| TestRejectedTokenIsDroppedAndRetried | internal/update/update_test.go | crates/update/tests/parity_update.rs::rejected_token_is_dropped_and_retried | covered | added; via run() |
| TestUnknownCurrentVersionIsRefused | internal/update/update_test.go | crates/update/tests/parity_update.rs::unknown_current_version_is_refused | covered | added; Rust has no ErrUnknownVersion, Error::Failed with same text |
| TestMainUsage | internal/update/update_test.go | crates/update/tests/parity_update.rs::main_usage | covered | added; run_cli |
| TestAssetNamePlatforms | internal/update/update_test.go | crates/update/tests/parity_update.rs::asset_name_platforms | covered | added; asset_name is pub(crate), checked via run() error text |
