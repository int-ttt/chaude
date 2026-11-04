# Rust test parity vs Go, part 3 of 3 (deprecated/src/main/intt)

Date: 2026-11-04. Reference: 320 Go tests (`grep -c '^func Test'` per `*_test.go` under deprecated/).
Columns: Go test | Rust test (path::name) | status | note.
status: covered = Rust test of the same behaviour exists (name-level or body-level, see note);
missing = no Rust test yet (feature or helper not found, or not body-checked); n-a = Go behaviour outside Rust scope (reason in note);
wait: update = Go update/install test, waiting for the crates/update port.
Shared table: plan/rust-test-parity.md (not edited here).

## Split (per brief: sort Go test files by path, 3 groups, roughly equal test counts)

Cumulative count over `find deprecated -name '*_test.go' | sort` (320 tests).

- Group 1: from first file through `harness/history_file_test.go` = 105 tests (ends at cumulative 105).
- Group 2: `harness/history_test.go` through `harness/resident_coder_test.go` = 112 tests (cumulative 106-217).
- Group 3 (this file): `harness/resident_extra_test.go` through `update/update_test.go` = 103 tests (cumulative 218-320).

Group 3 breakdown: harness 75 (cumulative 218-292), scripts/charcount 2 (cumulative 293-294) and settings 10 (295-304) are
already mapped in plan/rust-test-parity.md (not repeated here), update 16 (305-320, wait: update).

Note on balance: group 3 has 16 update/install tests that are blocked on the update port, so only 75 harness rows here are real mapping work.

## Remaining (top of file)

- Missing rows (26): resident_pipeline 2, resident_respawn 2, resident_route 1, resident_test 2, resume 1,
  resumeshift 1, selection 1, sessview 1, turn_wait 1, turnspend 4, unit 6, usagepoll 4. Need Rust code or a port first.
- Update/install rows (16): wait: update.
- Not verified by body: resident_test (EagerSpawn, SlotsFour), resident_respawn, SpawnSlotParksReadingChild were only
  name-grepped; their status may change after a body read.

## Commits (this part)

- see git log for `test(core): parity_harness_p3` commit (plan + crates/core/tests/parity_harness_p3.rs).

## harness/resident_extra_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestExtraCoderPoolFromSetting | crates/cli/src/tui/settings.rs::extra_coders_wins_over_legacy_key; crates/core/tests/parity_harness_p3.rs::default_extra_pool_and_idle_ttl_match_go | covered | default pool 4 asserted in p3 (new); the setting parse is name-level |
| TestExtraCoderExpiresAt270s | crates/core/tests/engine.rs::idle_extra_coder_expires_and_base_coder_stays; crates/core/tests/parity_harness_p3.rs::default_extra_pool_and_idle_ttl_match_go | covered | engine test uses 300 ms TTL; p3 asserts the default 270 s |

## harness/resident_pipeline_test.go (4 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestViaPrompterFailureNotice | crates/core/tests/engine.rs::prompter_failure_falls_back_to_raw_prompt | covered | name-level |
| TestViaTUITranslates | crates/core/tests/engine.rs::tui_answer_replaces_main_reply_after_turn | covered | name-level |
| TestViaTUIFailureNotice | none | missing | no Rust test for the TUI translator failure notice |
| TestRunTurnShowsTUIOutputOnly | none | missing | closest is engine.rs::tui_answer_replaces_main_reply_after_turn, not the same check |

## harness/resident_respawn_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestSendResidentRespawnsDeadSlot | none | missing | respawn of a dead resident slot not found by name grep |
| TestViaPrompterRawWhenRespawnFails | none | missing | raw fallback exists (engine.rs::prompter_failure_falls_back_to_raw_prompt) but respawn-failure path not covered |

## harness/resident_route_test.go (10 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestSendResidentBusyUntilResult | crates/core/tests/engine.rs::coder_task_emits_resident_busy_bracket_before_reply | covered | name-level |
| TestSpawnSlotParksReadingChild | none | missing | not found by name grep |
| TestViaPrompterSummaryOnly | crates/core/src/resident.rs::summary_is_composed_with_original_and_stage_done | covered | name-level |
| TestViaPrompterOKNoOriginal | crates/core/src/resident.rs::empty_summary_passes_input_through_without_stage | covered | name-level |
| TestViaPrompterNeedOriginalFlag | crates/core/src/resident.rs::need_original_flags_exact_inputs | covered | |
| TestViaPrompterNoFlagHeuristic | crates/core/src/resident.rs::need_original_flags_exact_inputs | covered | same test covers the no-flag path |
| TestViaPrompterEmptySummaryReturnsText | crates/core/src/resident.rs::empty_summary_passes_input_through_without_stage | covered | |
| TestNeedOriginal | crates/core/src/resident.rs::need_original_flags_exact_inputs | covered | |
| TestViaPrompterFallsBack | crates/core/tests/engine.rs::prompter_failure_falls_back_to_raw_prompt | covered | |
| TestResidentUsageFillsRoleSlot | crates/cli/src/tui/bar.rs::role_tokens_fill_the_prompter_tui_coder_slots | covered | |

## harness/resident_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestResidentSpawnsEager | none | missing | not body-checked; no name match |
| TestResidentSlotsFour | none | missing | not body-checked; default pool of 4 is in parity_harness_p3.rs (extra_coders) but slot count check not added |

## harness/resume_group_test.go (7 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestGroupSessions | crates/core/src/resume_group.rs::group_sessions_joins_or_splits_by_clear_gap_and_overlap | covered | 7 cases as Go: gap, clear at tail/head, day gap, overlap, TUI clear mark |
| TestGroupOrderAndKey | crates/core/src/resume_group.rs::group_order_and_key | covered | |
| TestMatchGroups | crates/core/src/resume_group.rs::match_groups_terms_typos_and_cwd | covered | Levenshtein <=1 for 4+ char terms |
| TestLoadImportArgMergesInTimeOrder | crates/core/src/import_claude.rs::merge_orders_files_by_start_and_returns_newest_path | covered | |
| TestReadClaudeTranscriptClearFlags | crates/core/src/import_claude.rs::transcript_extracts_turns_files_name_and_clear_markers | covered | name-level; head-clear flag not body-compared |
| TestClearMarks | crates/core/src/resume_group.rs::clear_marks_are_per_cwd_and_missing_file_is_empty | covered | |
| TestResumeTarget | crates/core/src/record_turn.rs::resume_target_picks_record_claude_or_sessions | covered | record_turn::resume_target_in; Go cwd arg maps to record dir |

## harness/resume_picker_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestRecordPickCandidates | crates/cli/src/tui/records.rs::pick_candidates_follow_go | covered | |
| TestRecordPickerKeysWheelEnter | crates/cli/src/tui/records.rs::picker_keys_wheel_enter_follow_go | covered | |

## harness/resume_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestListClaudeSessions | crates/cli/src/tui/sessview.rs::list_sessions (fn, used by list_draws_sessions_and_esc_closes) | covered | name differs (list_sessions), behaviour same |
| TestViewHelpers | none | missing | helper set not found by name grep |

## harness/resumeshift_test.go (3 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestJediTermIMEColumnShift | crates/cli/src/tui/mod.rs::ime_shift_follows_env_override_and_jediterm | covered | |
| TestImportShowsTurnsInTUI | crates/cli/src/tui/mod.rs::import_replaces_the_transcript_view | covered | |
| TestRotateKeepsPendingSnapshot | none | missing | model switch must not replace the pending import snapshot; no Rust test (engine.rs::installed_import_snapshot_goes_in_front_of_next_prompt covers only the snapshot placement) |

## harness/selection_test.go (3 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestSelectionAnchorStaysOnContentAfterAppend | crates/cli/src/tui/render.rs::selection_anchor_stays_on_content_after_append | covered | |
| TestSelectionMultiLineAndReversedOrder | crates/cli/src/tui/render.rs::selection_text_multi_line_and_reversed | covered | |
| TestReleaseCopiesSelection | none | missing | clipboard copy on release; K5 clipboard commit has no release-path test found |

## harness/sessview_test.go (6 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestViewEscBacksOutToNormalScreen | crates/cli/src/tui/sessview.rs::list_draws_sessions_and_esc_closes | covered | |
| TestViewSessionSelectionAndPromptNavigation | crates/cli/src/tui/sessview.rs::selection_and_prompt_navigation_follow_go | covered | |
| TestViewArgOpensSessionDirectly | crates/cli/src/tui/sessview.rs::view_arg_opens_session_directly | covered | |
| TestViewListsOnlyOwnSessions | crates/cli/src/tui/sessview.rs::view_lists_only_own_sessions | covered | |
| TestViewWheelMovesSessionCursor | crates/cli/src/tui/sessview.rs::wheel_moves_session_cursor_and_wraps | covered | |
| TestViewPromptScrollsInsideTranscript | none | missing | prompt scroll inside transcript not found by name grep |

## harness/specter_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestSpecterIdleNoElapsed | crates/cli/src/tui/bar.rs::idle_bar_without_status_says_so | covered | closest match; not body-compared |
| TestSpecterRowRenders | crates/cli/src/tui/bar.rs::busy_bar_lists_agents_status_and_queue | covered | closest match; not body-compared |

## harness/tui_caveman_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestIsCavemanBlock | crates/cli/src/tui/render.rs::caveman_blocks_are_detected_and_hidden | covered | |
| TestToolResultCavemanHidden | crates/cli/src/tui/mod.rs::caveman_tool_result_is_hidden_from_the_transcript | covered | |

## harness/turn_wait_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestTurnWaitsForOpenAgentToolResult | none | missing | engine.rs::silent_turn_emits_still_running_notice_and_never_interrupts has no open tool_use; open-Agent wait not covered |
| TestTurnQuietNoticeKeepsWaiting | crates/core/tests/engine.rs::silent_turn_emits_still_running_notice_and_never_interrupts | covered | |

## harness/turnspend_test.go (4 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestTurnSpendSubagentUsageMaxPerMessage | none | missing | turn spend (subagent usage) not found by name grep |
| TestTurnSpendMainUsageIgnored | none | missing | same feature |
| TestTurnSpendSpawnCounting | none | missing | same feature |
| TestTurnSpendEmptyAndNil | none | missing | same feature; nearest is engine.rs::turn_stats_reports_turn_cost_delegations_and_context (not the same check) |

## harness/unit_test.go (8 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestParseSamples | none | missing | not found by name grep |
| TestCtxTokens | crates/core/src/event.rs::usage_context_tokens_sums_input_and_cache | covered | |
| TestObservedTTL | crates/core/src/cache_ttl.rs::observed_ttl_reads_written_tier_else_prev | covered | helper only; api.rs does not call observed_ttl yet (Go livechild.go touchMain) |
| TestSnapshot | crates/core/src/import_claude.rs::snapshot_keeps_tail_turns_and_files | covered | name-level |
| TestCleanEnv | crates/core/src/proc.rs::apply_env_drops_api_key_and_sets_only_valid_ttl | covered | name-level: env delta (ANTHROPIC_API_KEY removed, exact window value), not inherited env |
| TestWithCacheTTL | crates/core/src/cache_ttl.rs::with_cache_ttl_pins_single_entry_last | covered | name-level: pure helper; proc apply_env sets the TTL entry, not with_cache_ttl |
| TestFmtK | none | missing | bar.rs fmt_k (line 90) has no test |
| TestCtxBar | none | missing | not found by name grep |

## harness/usagebar_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestRenderBar | crates/cli/src/tui/render.rs::render_bar_cases_and_width | covered | |
| TestDecorateUsage | crates/cli/src/tui/render.rs::decorate_usage_adds_bars | covered | |

## harness/usagebusy_test.go (3 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestUsageAllowedWhileBusy | crates/cli/src/tui/commands.rs::usage_is_accepted_while_busy | covered | |
| TestUsageSlashAcceptedWhileBusy | crates/cli/src/tui/commands.rs::usage_is_accepted_while_busy | covered | |
| TestUsageMsgWhileBusyOpensPopup | crates/cli/src/tui/usage.rs::usage_event_fills_window_and_error_goes_to_window_while_pending | covered | name-level |

## harness/usagepoll_test.go (5 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestParseUsage | none | missing | usage text parser (session/week pct, resets) not found by name grep |
| TestPollUsageLogsRow | none | missing | usage metric row not found in Rust |
| TestPollUsageError | none | missing | same feature |
| TestTurnCostUSDDelta | none | missing | per-turn cost delta not found by name grep |
| TestUsageLive | none | n-a | Go hits the real claude CLI, skipped unless HARNESS_LIVE_USAGE=1; Rust usage path is covered by engine.rs::usage_answers_with_result_text_from_one_shot_run |

## harness/usageview_test.go (4 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestUsageViewScrollBounds | crates/cli/src/tui/usage.rs::scroll_bounds_match_go | covered | |
| TestUsageViewWheelAndRefreshKeepScroll | crates/cli/src/tui/usage.rs::wheel_and_refresh_keep_scroll | covered | |
| TestUsageViewEscCloses | crates/cli/src/tui/usage.rs::esc_closes_and_returns_true | covered | |
| TestUsageOpenNotInline | crates/cli/src/tui/usage.rs::usage_command_opens_window_without_transcript_block | covered | |

## update/update_test.go and update/install_path_test.go (16 rows, wait: update)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestCompareVersion | crates/update/tests/parity_update.rs::compare_version | covered | added; Version parse+Ord |
| TestUpToDateDownloadsNothing | crates/update/tests/parity_update.rs::up_to_date_downloads_nothing | covered | added |
| TestUpdateUnixReplacesBinaryAtomically | crates/update/tests/parity_update.rs::update_unix_replaces_binary_atomically | covered | added; mode 0755, no leftovers |
| TestUpdateWindowsRenamesOldAndCleanupRemovesIt | crates/update/tests/parity_update.rs::update_windows_renames_old_and_cleanup_removes_it | covered | added; uses cleanup::remove_old seam |
| TestChecksumMismatchInstallsNothing | crates/update/tests/parity_update.rs::checksum_mismatch_installs_nothing | covered | added |
| TestChecksumsRequired | crates/update/tests/parity_update.rs::checksums_required | covered | added |
| TestNoAssetForPlatform | crates/update/tests/parity_update.rs::no_asset_for_platform | covered | added |
| TestOfflineIsClearError | crates/update/tests/parity_update.rs::offline_is_clear_error | covered | added; Fake returns Error::Offline (no httptest) |
| TestRejectedTokenIsDroppedAndRetried | crates/update/tests/parity_update.rs::rejected_token_is_dropped_and_retried | covered | added; via run() |
| TestUnknownCurrentVersionIsRefused | crates/update/tests/parity_update.rs::unknown_current_version_is_refused | covered | added; Rust has no ErrUnknownVersion, Error::Failed with same text |
| TestMainUsage | crates/update/tests/parity_update.rs::main_usage | covered | added; run_cli |
| TestAssetNamePlatforms | crates/update/tests/parity_update.rs::asset_name_platforms | covered | added; asset_name is pub(crate), checked via run() error text |
| TestNotWritableChangesNothing | crates/update/tests/parity_update.rs::not_writable_changes_nothing | covered | added; unix; skips if probe write succeeds (root) |
| TestMigrateLegacyRemovesOldInstallAndPathLine | crates/update/tests/parity_update.rs::migrate_legacy_removes_old_install_and_path_line | covered | added |
| TestMigrateLegacySkipsWhenRunningLegacyBinary | crates/update/tests/parity_update.rs::migrate_legacy_skips_when_running_legacy_binary | covered | added |
| TestMigrateLegacyNoopWithoutLegacyFiles | crates/update/tests/parity_update.rs::migrate_legacy_noop_without_legacy_files | covered | added |

## Totals (group 3)

- Group 3 tests: 103 = harness 75 + update 16 + scripts 2 + settings 10 (last 10 and 2 are mapped in the shared file).
- Harness 75 rows: covered 43 (of which 2 use the new p3 test: TestExtraCoderPoolFromSetting, TestExtraCoderExpiresAt270s),
  missing 31, n-a 1 (TestUsageLive).
- Update 16 rows: wait: update.
- Added tests: 1 test in crates/core/tests/parity_harness_p3.rs (default_extra_pool_and_idle_ttl_match_go); passes.
- Ignored diffs (#[ignore = "parity: ..."]): 0. Missing rows are features or crate-private helpers, so no ignored stub was added.

## Top behaviour differences / gaps

- Turn spend and per-turn cost delta, plus usage poll row logging, are not found in Rust (9 rows).
- Clean env and cache-TTL setting in proc.rs are not tested and proc is not pub (2 rows).
- Model switch with a pending import snapshot (TestRotateKeepsPendingSnapshot) has no Rust test.
- Open-Agent turn wait (TestTurnWaitsForOpenAgentToolResult) has no Rust test.
