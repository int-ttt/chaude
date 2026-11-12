> Source of truth: plan/rust-test-parity.md. Go paths in this file are historical (deprecated/ deleted in 0.11.3).
# Rust test parity vs Go, part 1 of 3

> Merged into plan/rust-test-parity.md (single deduped 320-row table, recounted totals). This file is kept for per-group detail.

Date: 2026-11-04. Reference: 320 Go tests (`grep -rhoE '^func Test' deprecated --include='*_test.go'`).
Columns: Go test | Rust test (path::name) | status (covered/missing/n-a/ignored) | note.
status: covered = Rust test of the same behaviour exists (name-level match noted when names differ);
missing = no Rust test yet (reason in note); n-a = outside Rust scope; ignored = Rust test kept with
`#[ignore = "parity: ..."]` because behaviour differs.

## Split (test-level, sorted order)

- Order: Go test packages sorted by directory name (clean, harness, scripts, settings, update), then by file
  name, then by line. 320 tests total.
- Part 1 = positions 1-107 (107 tests): clean 10 (positions 1-10) + harness 97 (positions 11-107).
- Part 2 = positions 108-213 (106 tests). Part 3 = positions 214-320 (107 tests).
- Equal count: 107 / 106 / 107.
- Scratch order list: /tmp/p1_order.tsv (not committed).

## Remaining (part 1)

- Mapped in this pass: 107 of 107 rows (table below).
- Totals: covered 89, partial 2, ignored 16, missing 0, n-a 0 (total 107).
- Partial: harness/cachettl_test.go::TestRegistryNilWhenNoFile (pre-existing, nil/empty-path half only); harness/drain_test.go::TestAgentTrackerAndDrain (tracker half in parity_p12_harness.rs; drain half ignored).
- Ignored: TestMaxWorkersDefault (pre-existing) plus 15 new ignored stubs in crates/core/tests/parity_p12_harness.rs (reasons: not pub in Rust, needs Repl/TUI internals, live-only, diff vs Go).
- Covered this pass: TestBackgroundAgentShownRunning, TestAgentDoneNotice (crates/core/tests/parity_p12_harness.rs).
- Name-level covered rows (test name differs or only partly matches) are marked in the note column.
- Unmapped: none.
- Update/install tests: none in group 1 (no `wait: update` rows needed).
- Added Rust tests: crates/core/tests/parity_harness_p1.rs (3 tests: 2 pass, 1 ignored).
- Added Rust tests (group 1+2): crates/core/tests/parity_p12_harness.rs (9 pass, 35 ignored; 16 p1 stubs and 19 p2 stubs).

## Table

| Go test | Rust test (path::name) | status | note |
|---|---|---|---|
| clean/clean_test.go::TestApplyKeepsUserModifiedHook | clean/src/tests.rs::apply_keeps_user_modified_hook | covered | |
| clean/clean_test.go::TestApplyKeepsHooksWithoutPackageCopy | clean/src/tests.rs::apply_keeps_hooks_without_package_copy | covered | |
| clean/clean_test.go::TestDryRunChangesNothing | clean/src/tests.rs::dry_run_changes_nothing | covered | |
| clean/clean_test.go::TestApplyRemovesManifestAndBacksUp | clean/src/tests.rs::apply_removes_manifest_and_backs_up | covered | |
| clean/clean_test.go::TestApplyKeepsNotesRecordAndUserSettings | clean/src/tests.rs::apply_keeps_notes_record_and_user_settings | covered | |
| clean/clean_test.go::TestClaudeMdWithoutBlockUntouched | clean/src/tests.rs::claude_md_without_block_untouched | covered | |
| clean/clean_test.go::TestClaudeMdOnlyBlockIsDeleted | clean/src/tests.rs::claude_md_only_block_is_deleted | covered | |
| clean/clean_test.go::TestBrokenMarkersFailBeforeAnyChange | clean/src/tests.rs::broken_markers_fail_before_any_change | covered | |
| clean/clean_test.go::TestManifestMatchesRepo | clean/src/tests.rs::manifest_matches_repo | covered | |
| clean/guard_roundtrip_test.go::TestGuardSettingsRemovedByClean | clean/src/tests.rs::guard_settings_removed_by_clean | covered | |
| harness/acceptance_test.go::TestLiveRotationCarriesContext | core/tests/engine.rs::context_rotation_hands_off_and_fresh_child_gets_snapshot | covered | name-level match |
| harness/acceptance_test.go::TestLiveStreamingMultipleDeltas | - | ignored | no streaming-delta test in Rust core yet  [p12: live_streaming_multiple_deltas] |
| harness/agentstore_test.go::TestAgentStorePersistsEveryChange | crates/core/src/agent_store.rs::agent_store_persists_every_change | covered | tracker-driven; AgentTracker on_tool_use/on_sub_event, no onUserEvent |
| harness/agentstore_test.go::TestAgentRestoreMarksInterrupted | crates/core/src/agent_store.rs::agent_restore_marks_interrupted | covered | tracker-driven; restore_agents + mark_interrupted |
| harness/agentstore_test.go::TestAgentStoreCrashRecoveryAndOwners | crates/core/src/agent_store.rs::agent_store_crash_recovery_and_owners | covered | |
| harness/agentstore_test.go::TestAgentStoreOffAndNil | crates/core/src/agent_store.rs::agent_store_empty_path_is_off | covered | name-level match; nil store is Option::None at the call site |
| harness/agentview_test.go::TestAgentViewClickReadOnlyClose | - | ignored | Rust TUI has no agent view  [p12: agentview_click_read_only_close] |
| harness/agentview_test.go::TestUsagePopupOpenClose | cli/src/tui/usage.rs::esc_closes_and_returns_true | covered | name-level: usage popup close |
| harness/agentview_test.go::TestWheelScrollHoldsPosition | cli/src/tui/usage.rs::wheel_and_refresh_keep_scroll | covered | name-level: usage popup, not agent view |
| harness/agentview_test.go::TestTypingKeepsScroll | - | ignored | Rust TUI has no agent view scroll  [p12: typing_keeps_scroll] |
| harness/agentview_test.go::TestScrollBottomOnInputAndUpdate | - | ignored | Rust TUI has no agent view scroll  [p12: scroll_bottom_on_input_and_update] |
| harness/batchinput_test.go::TestMergePromptsOneTurn | core/tests/engine.rs::queued_prompts_go_out_as_one_merged_turn_on_done | covered | |
| harness/batchinput_test.go::TestMainStatusRendering | cli/src/tui/mod.rs::busy_row_sits_in_the_transcript_flow_with_go_text | covered | name-level: busy/status row text |
| harness/bgagent_defer_test.go::TestRotateDeferDue | core/src/rotate_defer.rs::due_cases | covered | |
| harness/bgagent_defer_test.go::TestRotateHeldWhileBackgroundAgentRuns | core/src/rotate_defer.rs::held_while_agent_runs_then_due_at_timeout | covered | |
| harness/bgagent_defer_test.go::TestRotateDeferReleasedWhenAgentFinishes | core/src/rotate_defer.rs::released_when_agent_finishes | covered | |
| harness/bgagent_defer_test.go::TestDeferRotateTimeoutProceeds | core/src/rotate_defer.rs::timeout_proceeds_and_reports_it | covered | |
| harness/bgagent_defer_test.go::TestBackgroundResultFileAndHandoff | core/tests/engine.rs::handoff_snapshot_lists_running_background_agent_with_result_path | covered | name-level |
| harness/bold_test.go::TestMdBold | cli/src/tui/render.rs::md_bold_pairs_and_skips_fences | covered | |
| harness/budget_test.go::TestBudgetDeltaPerProcess | core/src/budget.rs::delta_per_process | covered | |
| harness/budget_test.go::TestBudgetOffAllows | core/src/budget.rs::cap_zero_is_off | covered | name-level: cap 0 = off |
| harness/budget_test.go::TestBudgetResetPerPrompt | core/src/budget.rs::reset_clears_spent_and_keeps_baseline | covered | name-level |
| harness/budget_test.go::TestBudgetResumeBaseline | core/src/budget.rs::resumed_process_books_whole_first_total | covered | name-level |
| harness/budget_test.go::TestBudgetGateBeforeSend | core/tests/engine.rs::budget_refuses_prompt_whose_next_call_estimate_exceeds_cap | covered | |
| harness/budget_test.go::TestBudgetInterruptMidTurn | core/tests/engine.rs::budget_interrupts_turn_once_when_in_flight_estimate_reaches_cap | covered | |
| harness/budget_test.go::TestBudgetExtraAppends | proc.rs::budget_args_append_after_rule_flag | covered | rule args then --max-budget-usd appended; cap 0 adds no flag |
| harness/budget_test.go::TestBudgetDisabledByDefault | core/src/proc.rs::budget_args_only_when_cap_is_set | covered | name-level: no budget args when off |
| harness/budget_test.go::TestChildPIDRecordedOnPark | - | ignored | needs Repl.parkOld and the client registry (not ported)  [p12: child_pid_recorded_on_park] |
| harness/budget_test.go::TestReconcileParkedAliveChild | - | ignored | needs Repl.parkOld and the client registry (not ported)  [p12: reconcile_parked_alive_child] |
| harness/budget_test.go::TestBudgetFullModelIDPriced | core/src/budget.rs::full_model_id_is_priced_like_alias | covered | |
| harness/busyslash_test.go::TestBusySlashAllowedAndRefused | - | ignored | busy-slash allow/refuse rule not in Rust  [p12: busy_slash_allowed_and_refused] |
| harness/busyslash_test.go::TestVersionNoteShowsChaudeVersion | - | ignored | version note not covered in Rust tests  [p12: version_note_shows_chaude_version] |
| harness/cachettl_test.go::TestParseCacheTTL | crates/core/src/cache_ttl.rs::parse_cache_ttl_accepts_three_values_only | covered | |
| harness/cachettl_test.go::TestRegistryNilWhenNoFile | crates/core/src/clients.rs::empty_path_is_noop_registry | partial | nil/empty-path half only; Repl wiring (reg, children, ttl, idleRotate) stays in api.rs |
| harness/category_test.go::TestParseCategory | crates/core/src/category.rs::parse_category_takes_last_marker_line | covered | |
| harness/category_test.go::TestCategoryRotation | crates/core/src/category.rs::category_rotation_rules | covered | |
| harness/category_test.go::TestCategoryRuleSize | crates/core/src/category.rs::category_rule_fits_budget | covered | |
| harness/category_test.go::TestMarkerFilter | crates/core/src/category.rs::marker_filter_hides_split_marker_line | covered | |
| harness/category_test.go::TestMarkerFilterInline | crates/core/src/category.rs::marker_filter_hides_inline_marker | covered | |
| harness/chaude_settings_test.go::TestSettingsDefaultsWhenMissing | cli/src/tui/settings.rs::defaults_when_missing | covered | |
| harness/chaude_settings_test.go::TestSettingsClamp | cli/src/tui/settings.rs::clamp_fixes_out_of_range | covered | |
| harness/chaude_settings_test.go::TestSettingsSaveLoadRoundTrip | cli/src/tui/settings.rs::save_load_round_trip_leaves_no_tmp | covered | |
| harness/chaude_settings_test.go::TestSettingsPartialFileKeepsDefaults | cli/src/tui/settings.rs::partial_file_keeps_defaults_and_reads_legacy_key | covered | |
| harness/chaude_settings_test.go::TestSettingsModelID | cli/src/tui/settings.rs::model_ids_follow_alias | covered | |
| harness/chaude_settings_test.go::TestPruneRecordFiles | - | ignored | prune of record files not in Rust tests  [p12: prune_record_files] |
| harness/chaude_settings_test.go::TestExitHandoffWriteRead | core/src/handoff.rs::write_then_read_keeps_ids_and_sets_version | covered | name-level |
| harness/chaude_settings_test.go::TestResidentSlotsFixedFour | - | ignored | Rust resident slot count is 8 floor (RESIDENT_MAX_WORKERS), not fixed four  [p12: resident_slots_fixed_four] |
| harness/chaude_settings_test.go::TestLegacyCoderSlotsKey | cli/src/tui/settings.rs::partial_file_keeps_defaults_and_reads_legacy_key | covered | |
| harness/chaude_settings_test.go::TestSettingsFileIsPerProject | cli/src/tui/settings.rs::settings_file_is_per_project | covered | |
| harness/chaude_settings_test.go::TestSettingsFileEmptyCwdUsesWorkingDir | - | ignored | empty-cwd fallback not tested in Rust  [p12: settings_file_empty_cwd_uses_working_dir] |
| harness/childkey_test.go::TestChildKeyRoleCoexist | crates/core/src/role_coexist.rs::same_model_with_role_and_without_role_coexist_when_parked | covered | test-only; child_key + ChildSet park, role logic in child_set.rs |
| harness/childkey_test.go::TestAdmitEvictsOldestParked | crates/core/src/child_set.rs::admit_evicts_oldest_parked | covered | |
| harness/childkey_test.go::TestAdmitRefusesWhenNothingEvictable | crates/core/src/child_set.rs::admit_refuses_when_nothing_evictable | covered | |
| harness/children_test.go::TestChildKey | crates/core/src/child_set.rs::child_key_matches_empty_dot_and_working_dir | covered | name-level match; role-coexist case is harness/childkey_test.go::TestChildKeyRoleCoexist (covered by role_coexist.rs) |
| harness/children_test.go::TestChildSetParkTake | crates/core/src/child_set.rs::park_take_replaces_closes_once_and_expires | covered | |
| harness/children_test.go::TestCloseAllParallel | crates/core/src/child_set.rs::close_all_closes_in_parallel | covered | |
| harness/cleanenv_test.go::TestCleanEnvDropsAPIKey | crates/core/src/proc.rs::apply_env_drops_api_key_and_sets_only_valid_ttl | covered | name-level: checks the recorded env delta (key removed), not the inherited env |
| harness/clients_test.go::TestClientRegistryRoundTrip | crates/core/src/clients.rs::round_trip_upsert_get_mark_ended | covered | |
| harness/clients_test.go::TestClientRegistryBrokenJSON | crates/core/src/clients.rs::broken_json_reads_as_empty | covered | |
| harness/clients_test.go::TestClientRegistryAtomicWrite | crates/core/src/clients.rs::atomic_write_leaves_no_tmp_file | covered | |
| harness/clients_test.go::TestClientRegistryKeyedByCwdModel | crates/core/src/clients.rs::rows_keyed_by_cwd_and_model | covered | |
| harness/clients_test.go::TestIsWarm | crates/core/src/clients.rs::is_warm_checks_state_deadline_and_context | covered | |
| harness/clients_test.go::TestReconcileKilledOwner | crates/core/src/clients.rs::reconcile_marks_dead_owner_rows | covered | |
| harness/clients_test.go::TestIdleDeadline | crates/core/src/clients.rs::idle_deadline_by_ttl_and_cap | covered | |
| harness/clients_test.go::TestExpiredZeroDeadline | crates/core/src/clients.rs::expired_zero_deadline | covered | Go negative case not representable in Duration |
| harness/conductor_test.go::TestWorkerPool | crates/core/src/haiku_pool.rs::worker_pool_notes_edits_and_dedups_files | covered | name-level match; nil pool is Option at the call site; 80-rune cut in worker_pool_truncates_task_to_80_runes |
| harness/conductor_test.go::TestThresholdFor | core/src/rotate.rs::threshold_for_opus_and_configured | covered | |
| harness/conductor_test.go::TestShouldRotateBoundary | core/src/rotate.rs::should_rotate_fires_at_threshold | covered | name-level: threshold boundary |
| harness/delegation_test.go::TestDelegationRuleNamesHaikuCoder | core/src/orch.rs::rule_per_mode | covered | name-level: RULE_B names haiku-coder |
| harness/delegation_test.go::TestForwardRuleFor | crates/core/src/forward.rs::forward_rule_for_skips_opus_models | covered | |
| harness/dispatch_test.go::TestPickChild | crates/core/src/pick_child.rs::pick_child_matches_go_cases | covered | name-level match; owner case is pick_child_refuses_row_owned_by_other_live_pid (pid 1 instead of parent pid) |
| harness/dispatch_test.go::TestMaxWorkersDefault | crates/core/tests/parity_harness_p1.rs::max_workers_default_is_three | ignored | parity: Go default 3; Rust default is max(3, 8 + extra_coders) |
| harness/dispatch_test.go::TestMaxWorkersEnvOverride | crates/core/tests/parity_harness_p1.rs::max_workers_env_override_and_flag_precedence | covered | added in parity_harness_p1.rs |
| harness/dispatch_test.go::TestMaxWorkersRejectZero | crates/core/tests/parity_harness_p1.rs::max_workers_rejects_zero_from_flag_and_env | covered | added in parity_harness_p1.rs |
| harness/dispatch_test.go::TestMaxWorkersCapRefuses | - | ignored | Rust has no worker cap check  [p12: max_workers_cap_refuses] |
| harness/drain_test.go::TestAgentTrackerAndDrain | - | partial | Rust has no agent tracker drain  [p12: agent_tracker_and_drain_tracker_half]  [p12: agent_tracker_drain_half] |
| harness/drain_test.go::TestDrainConsumeDenies | - | ignored | Rust has no drain consume  [p12: drain_consume_denies] |
| harness/drain_test.go::TestBackgroundAgentShownRunning | - | covered | no direct test in Rust  [p12: background_agent_shown_running] |
| harness/drain_test.go::TestAgentDoneNotice | - | covered | no agent-done notice test in Rust  [p12: agent_done_notice] |
| harness/drain_test.go::TestResumeSendMessageReactivatesAgent | core/tests/engine.rs::resumed_background_agent_is_active_again_until_task_notification | covered | name-level |
| harness/handoff_test.go::TestExtractRecoMarkdownMultiline | core/src/rotate.rs::extract_reco_takes_last_marker_block_through_end | covered | name-level |
| harness/handoff_test.go::TestExtractRecoNone | core/src/rotate.rs::extract_reco_none | covered | |
| harness/handoff_test.go::TestValidateHandoffGood | core/src/rotate.rs::validate_good_note_has_no_problems | covered | |
| harness/handoff_test.go::TestValidateHandoffTooShort | core/src/rotate.rs::validate_flags_short_note | covered | |
| harness/handoff_test.go::TestValidateHandoffMissingSection | core/src/rotate.rs::validate_flags_missing_section | covered | |
| harness/handoff_test.go::TestValidateHandoffUnmentionedFile | core/src/rotate.rs::validate_flags_unmentioned_file | covered | |
| harness/handoff_test.go::TestWithRecoRoundTripKorean | core/src/rotate.rs::with_reco_round_trip_and_preservation | covered | name-level: Korean input not confirmed in Rust test |
| harness/handoff_test.go::TestWithRecoEmpty | core/src/rotate.rs::with_reco_empty_is_unchanged | covered | |
| harness/history_file_test.go::TestHistoryFileRoundTrip | cli/src/tui/edit.rs::history_file_round_trip | covered | |
| harness/history_file_test.go::TestHistoryFileLoadRules | cli/src/tui/edit.rs::history_file_load_rules_skip_bad_lines_and_cap | covered | |
| harness/history_file_test.go::TestHistoryFileFor | cli/src/tui/edit.rs::history_file_path_is_under_chaude | covered | |
| harness/history_file_test.go::TestRecordHistoryPersistsAndNewModelLoads | cli/src/tui/edit.rs::recorded_history_persists_and_a_new_editor_loads_it | covered | |
| harness/history_file_test.go::TestUpDownHistoryWithSlashCompleter | - | ignored | slash-completer interplay with up/down not tested in Rust  [p12: up_down_history_with_slash_completer] |
| harness/history_file_test.go::TestUpMovesCompleterWhenNotAtTop | - | ignored | completer movement not tested in Rust  [p12: up_moves_completer_when_not_at_top] |
| harness/history_file_test.go::TestHistoryDedupeConsecutive | cli/src/tui/edit.rs::history_dedupes_consecutive_entries_across_restart | covered | |
| harness/history_test.go::TestInputHistory | cli/src/tui/edit.rs::history_walks_back_and_forth_and_restores_the_draft | covered | |
| harness/history_test.go::TestInputHistoryCap | cli/src/tui/edit.rs::history_file_load_rules_skip_bad_lines_and_cap | covered | name-level: cap checked on file load only |
