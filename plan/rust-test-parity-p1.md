# Rust test parity vs Go, part 1 of 3

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
- Totals: covered 58 (clean 10, harness 48; 2 of the harness covered rows are new in parity_harness_p1.rs),
  ignored 1 (TestMaxWorkersDefault), missing 48 (harness only), n-a 0.
- Name-level covered rows (test name differs or only partly matches) are marked in the note column.
- Unmapped: none.
- Update/install tests: none in group 1 (no `wait: update` rows needed).
- Added Rust tests: crates/core/tests/parity_harness_p1.rs (3 tests: 2 pass, 1 ignored).

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
| harness/acceptance_test.go::TestLiveStreamingMultipleDeltas | - | missing | no streaming-delta test in Rust core yet |
| harness/agentstore_test.go::TestAgentStorePersistsEveryChange | - | missing | Rust has no agent store |
| harness/agentstore_test.go::TestAgentRestoreMarksInterrupted | - | missing | Rust has no agent store |
| harness/agentstore_test.go::TestAgentStoreCrashRecoveryAndOwners | - | missing | Rust has no agent store |
| harness/agentstore_test.go::TestAgentStoreOffAndNil | - | missing | Rust has no agent store |
| harness/agentview_test.go::TestAgentViewClickReadOnlyClose | - | missing | Rust TUI has no agent view |
| harness/agentview_test.go::TestUsagePopupOpenClose | cli/src/tui/usage.rs::esc_closes_and_returns_true | covered | name-level: usage popup close |
| harness/agentview_test.go::TestWheelScrollHoldsPosition | cli/src/tui/usage.rs::wheel_and_refresh_keep_scroll | covered | name-level: usage popup, not agent view |
| harness/agentview_test.go::TestTypingKeepsScroll | - | missing | Rust TUI has no agent view scroll |
| harness/agentview_test.go::TestScrollBottomOnInputAndUpdate | - | missing | Rust TUI has no agent view scroll |
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
| harness/budget_test.go::TestBudgetExtraAppends | - | missing | Rust budget_args test covers only set/unset, not extra args |
| harness/budget_test.go::TestBudgetDisabledByDefault | core/src/proc.rs::budget_args_only_when_cap_is_set | covered | name-level: no budget args when off |
| harness/budget_test.go::TestChildPIDRecordedOnPark | - | missing | Rust has no child set/park |
| harness/budget_test.go::TestReconcileParkedAliveChild | - | missing | Rust has no parked-child reconcile |
| harness/budget_test.go::TestBudgetFullModelIDPriced | core/src/budget.rs::full_model_id_is_priced_like_alias | covered | |
| harness/busyslash_test.go::TestBusySlashAllowedAndRefused | - | missing | busy-slash allow/refuse rule not in Rust |
| harness/busyslash_test.go::TestVersionNoteShowsChaudeVersion | - | missing | version note not covered in Rust tests |
| harness/cachettl_test.go::TestParseCacheTTL | - | missing | Rust has no cache-TTL registry |
| harness/cachettl_test.go::TestRegistryNilWhenNoFile | - | missing | Rust has no cache-TTL registry |
| harness/category_test.go::TestParseCategory | - | missing | Rust has no category rotation |
| harness/category_test.go::TestCategoryRotation | - | missing | Rust has no category rotation |
| harness/category_test.go::TestCategoryRuleSize | - | missing | Rust has no category rotation |
| harness/category_test.go::TestMarkerFilter | - | missing | Rust has no marker filter |
| harness/category_test.go::TestMarkerFilterInline | - | missing | Rust has no marker filter |
| harness/chaude_settings_test.go::TestSettingsDefaultsWhenMissing | cli/src/tui/settings.rs::defaults_when_missing | covered | |
| harness/chaude_settings_test.go::TestSettingsClamp | cli/src/tui/settings.rs::clamp_fixes_out_of_range | covered | |
| harness/chaude_settings_test.go::TestSettingsSaveLoadRoundTrip | cli/src/tui/settings.rs::save_load_round_trip_leaves_no_tmp | covered | |
| harness/chaude_settings_test.go::TestSettingsPartialFileKeepsDefaults | cli/src/tui/settings.rs::partial_file_keeps_defaults_and_reads_legacy_key | covered | |
| harness/chaude_settings_test.go::TestSettingsModelID | cli/src/tui/settings.rs::model_ids_follow_alias | covered | |
| harness/chaude_settings_test.go::TestPruneRecordFiles | - | missing | prune of record files not in Rust tests |
| harness/chaude_settings_test.go::TestExitHandoffWriteRead | core/src/handoff.rs::write_then_read_keeps_ids_and_sets_version | covered | name-level |
| harness/chaude_settings_test.go::TestResidentSlotsFixedFour | - | missing | Rust resident slot count is 8 floor (RESIDENT_MAX_WORKERS), not fixed four |
| harness/chaude_settings_test.go::TestLegacyCoderSlotsKey | cli/src/tui/settings.rs::partial_file_keeps_defaults_and_reads_legacy_key | covered | |
| harness/chaude_settings_test.go::TestSettingsFileIsPerProject | cli/src/tui/settings.rs::settings_file_is_per_project | covered | |
| harness/chaude_settings_test.go::TestSettingsFileEmptyCwdUsesWorkingDir | - | missing | empty-cwd fallback not tested in Rust |
| harness/childkey_test.go::TestChildKeyRoleCoexist | - | missing | Rust has no child key registry |
| harness/childkey_test.go::TestAdmitEvictsOldestParked | - | missing | Rust has no admit/evict of parked children |
| harness/childkey_test.go::TestAdmitRefusesWhenNothingEvictable | - | missing | Rust has no admit/evict of parked children |
| harness/children_test.go::TestChildKey | - | missing | Rust has no child key registry |
| harness/children_test.go::TestChildSetParkTake | - | missing | Rust has no child set/park |
| harness/children_test.go::TestCloseAllParallel | - | missing | Rust has no child set |
| harness/cleanenv_test.go::TestCleanEnvDropsAPIKey | - | missing | apply_env is private in core/src/proc.rs; no public test seam |
| harness/clients_test.go::TestClientRegistryRoundTrip | - | missing | Rust has no client registry |
| harness/clients_test.go::TestClientRegistryBrokenJSON | - | missing | Rust has no client registry |
| harness/clients_test.go::TestClientRegistryAtomicWrite | - | missing | Rust has no client registry |
| harness/clients_test.go::TestClientRegistryKeyedByCwdModel | - | missing | Rust has no client registry |
| harness/clients_test.go::TestIsWarm | - | missing | Rust has no warm-client check |
| harness/clients_test.go::TestReconcileKilledOwner | - | missing | Rust has no client reconcile (handoff orphans test is different) |
| harness/clients_test.go::TestIdleDeadline | - | missing | idle_deadline is private in core/src/api.rs |
| harness/clients_test.go::TestExpiredZeroDeadline | - | missing | idle deadline is private in core/src/api.rs |
| harness/conductor_test.go::TestWorkerPool | - | missing | Rust engine has no worker pool |
| harness/conductor_test.go::TestThresholdFor | core/src/rotate.rs::threshold_for_opus_and_configured | covered | |
| harness/conductor_test.go::TestShouldRotateBoundary | core/src/rotate.rs::should_rotate_fires_at_threshold | covered | name-level: threshold boundary |
| harness/delegation_test.go::TestDelegationRuleNamesHaikuCoder | core/src/orch.rs::rule_per_mode | covered | name-level: RULE_B names haiku-coder |
| harness/delegation_test.go::TestForwardRuleFor | - | missing | Rust has no forward rule |
| harness/dispatch_test.go::TestPickChild | - | missing | Rust has no pick-child |
| harness/dispatch_test.go::TestMaxWorkersDefault | crates/core/tests/parity_harness_p1.rs::max_workers_default_is_three | ignored | parity: Go default 3; Rust default is max(3, 8 + extra_coders) |
| harness/dispatch_test.go::TestMaxWorkersEnvOverride | crates/core/tests/parity_harness_p1.rs::max_workers_env_override_and_flag_precedence | covered | added in parity_harness_p1.rs |
| harness/dispatch_test.go::TestMaxWorkersRejectZero | crates/core/tests/parity_harness_p1.rs::max_workers_rejects_zero_from_flag_and_env | covered | added in parity_harness_p1.rs |
| harness/dispatch_test.go::TestMaxWorkersCapRefuses | - | missing | Rust has no worker cap check |
| harness/drain_test.go::TestAgentTrackerAndDrain | - | missing | Rust has no agent tracker drain |
| harness/drain_test.go::TestDrainConsumeDenies | - | missing | Rust has no drain consume |
| harness/drain_test.go::TestBackgroundAgentShownRunning | - | missing | no direct test in Rust |
| harness/drain_test.go::TestAgentDoneNotice | - | missing | no agent-done notice test in Rust |
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
| harness/history_file_test.go::TestUpDownHistoryWithSlashCompleter | - | missing | slash-completer interplay with up/down not tested in Rust |
| harness/history_file_test.go::TestUpMovesCompleterWhenNotAtTop | - | missing | completer movement not tested in Rust |
| harness/history_file_test.go::TestHistoryDedupeConsecutive | cli/src/tui/edit.rs::history_dedupes_consecutive_entries_across_restart | covered | |
| harness/history_test.go::TestInputHistory | cli/src/tui/edit.rs::history_walks_back_and_forth_and_restores_the_draft | covered | |
| harness/history_test.go::TestInputHistoryCap | cli/src/tui/edit.rs::history_file_load_rules_skip_bad_lines_and_cap | covered | name-level: cap checked on file load only |
