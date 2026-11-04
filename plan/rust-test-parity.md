# Rust test parity vs Go (deprecated/src/main/intt)

Date: 2026-11-04. Reference: 320 Go tests (`grep -rhoE '^func Test' deprecated --include='*_test.go'`).
Columns: Go test | Rust test (path::name) | status (covered/missing/n-a) | note.
status: covered = Rust test of the same behaviour exists; missing = no Rust test yet; n-a = Go behaviour
lives outside the Rust scope (with reason).

## Remaining (top of file, per brief)

- Done in this pass: 52 of 320 Go rows mapped (below). 268 rows not yet mapped.
- Not yet mapped, by Go file: harness/{panel,multiproc,idle,drain,resident_*,usage*,input,links,selection,
  sessview,agentview,history_file,history,resume,resume_group,resume_picker,resumeshift,busyslash,budget,
  tui*,...} (about 230 rows), plus cmd/ and remaining packages.
- Update and install path (16 rows, status missing): crates/update has helpers only. A port of Go update is in progress
  (agent a1284799); rows to be re-mapped when it lands.
- Checklist recount (plan/rust-parity-checklist.md vs `git log 2f86bb8..HEAD`, K6-K11, S02, S08, P3):
  not done in this pass.
- Go-vs-Rust behaviour differences found so far: none recorded in this pass (all mapped covered rows are
  name-level or read-level matches; see notes).

## Commits (this pass)

- 505fd8b test(core): parity tests for Go record split caps, mtime numbering, midnight
  (crates/core/tests/parity_record.rs, 3 tests, all pass: `cargo test -p chaude-core --test parity_record`
  -> 3 passed; 0 failed; 0 ignored).

## harness/record_test.go (14 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestRecordMidnightBoundary | crates/core/tests/parity_record.rs::midnight_boundary_keeps_newest_file_and_stamps_new_day | covered | new this pass |
| TestRecordSplitOnClearStartsNewFile | crates/core/src/record.rs::append_fills_one_file_then_splits_on_request | covered | split flag path |
| TestRecordSplitByCaps | crates/core/tests/parity_record.rs::split_by_caps_starts_new_file_at_101st_answer | covered | new; 100 answers cap, Rust MAX_ANSWERS = 100 |
| TestRecordSlugKorean | crates/core/src/record.rs::slugify_keeps_korean_and_bounds_length | covered | |
| TestRecordProjectName | crates/core/src/record_folder.rs::project_name_is_the_slugged_basename | covered | |
| TestRecordRoundTripParse | crates/core/src/record.rs::render_then_parse_round_trips_blocks | covered | |
| TestRecordNumberContinuesAfterRestart | crates/core/src/record.rs::last_number_reads_the_tail_of_a_big_file | covered | name-level match, body not compared |
| TestRecordEscapeHeaderInBody | crates/core/src/record.rs::body_line_that_looks_like_a_header_survives | covered | |
| TestRecordNumberIgnoresMtime | crates/core/tests/parity_record.rs::number_ignores_mtime | covered | new this pass |
| TestRecordTopicParse | crates/core/src/record.rs::topic_prefers_last_switch_then_last_cat_then_prev | covered | |
| TestRecordFolderListing | crates/core/src/record_folder.rs::listing_skips_hidden_and_empty_folders_newest_first | covered | |
| TestRecordFolderQueryFilter | crates/core/src/record_folder.rs::filter_matches_name_topic_or_text_case_insensitively | covered | |
| TestRecordFolderLoadAllInTimeOrder | crates/core/src/record_folder.rs::load_project_gives_all_files_in_rec_order | covered | |
| TestRecordMigrateLegacyRoot | crates/core/src/record_folder.rs::migrate_root_moves_legacy_files_into_owner_folder | covered | |

## settings/project_test.go and settings_test.go (10 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestEnsureProjectSettingsCreatesGuard | crates/core/src/settings/project.rs::creates_guard | covered | |
| TestEnsureProjectAgentsKeepExisting | crates/core/src/settings/project.rs::agents_written_and_existing_kept | covered | |
| TestEnsureProjectSettingsMergesExisting | crates/core/src/settings/project.rs::merges_existing_settings | covered | |
| TestMergeGuardRejectsBadJSON | crates/core/src/settings/project.rs::rejects_non_object_settings | covered | name-level match |
| TestEnsureProjectSettingsSkipsHome | crates/core/src/settings/project.rs::skips_home | covered | |
| TestEmbeddedHooksMatchRepo | crates/core/src/settings/project.rs::embedded_hooks_match_repo | covered | |
| TestEmbeddedAgentsMatchRepo | crates/core/src/settings/project.rs::embedded_agents_match_repo | covered | |
| TestHookPythonOrder | crates/core/src/settings/project.rs::hook_python_order | covered | |
| TestDefaultSrcPrecedence | crates/core/src/settings/mod.rs::default_src_precedence | covered | |
| TestGenSettingsCreatesSettingsFolder | crates/core/src/settings/mod.rs::gen_settings_creates_settings_folder | covered | |

## clean/clean_test.go and guard_roundtrip_test.go (10 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestApplyKeepsUserModifiedHook | crates/clean/src/tests.rs::apply_keeps_user_modified_hook | covered | |
| TestApplyKeepsHooksWithoutPackageCopy | crates/clean/src/tests.rs::apply_keeps_hooks_without_package_copy | covered | |
| TestDryRunChangesNothing | crates/clean/src/tests.rs::dry_run_changes_nothing | covered | |
| TestApplyRemovesManifestAndBacksUp | crates/clean/src/tests.rs::apply_removes_manifest_and_backs_up | covered | |
| TestApplyKeepsNotesRecordAndUserSettings | crates/clean/src/tests.rs::apply_keeps_notes_record_and_user_settings | covered | |
| TestClaudeMdWithoutBlockUntouched | crates/clean/src/tests.rs::claude_md_without_block_untouched | covered | |
| TestClaudeMdOnlyBlockIsDeleted | crates/clean/src/tests.rs::claude_md_only_block_is_deleted | covered | |
| TestBrokenMarkersFailBeforeAnyChange | crates/clean/src/tests.rs::broken_markers_fail_before_any_change | covered | |
| TestManifestMatchesRepo | crates/clean/src/tests.rs::manifest_matches_repo | covered | |
| TestGuardSettingsRemovedByClean | crates/clean/src/tests.rs::guard_settings_removed_by_clean | covered | |

## scripts/charcount_test.go (2 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestCharcountBOMAndCRLFMatchPlain | crates/scripts/tests/charcount.rs::bom_and_crlf_match_plain_lf | covered | |
| TestCharcountBOMOnlyStripped | crates/scripts/tests/charcount.rs::bom_only_is_stripped | covered | |

## update/update_test.go and install_path_test.go (16 rows, all missing)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestCompareVersion | none | missing | crates/update has no version compare; port in progress (a1284799) |
| TestUpToDateDownloadsNothing | none | missing | update port pending |
| TestUpdateUnixReplacesBinaryAtomically | none | missing | update port pending |
| TestUpdateWindowsRenamesOldAndCleanupRemovesIt | none | missing | update port pending |
| TestChecksumMismatchInstallsNothing | none | missing | update port pending |
| TestChecksumsRequired | none | missing | update port pending |
| TestNoAssetForPlatform | none | missing | update port pending |
| TestOfflineIsClearError | none | missing | update port pending |
| TestRejectedTokenIsDroppedAndRetried | none | missing | update port pending |
| TestUnknownCurrentVersionIsRefused | none | missing | update port pending |
| TestMainUsage | none | missing | update port pending |
| TestAssetNamePlatforms | none | missing | update port pending |
| TestNotWritableChangesNothing | none | missing | install path port pending |
| TestMigrateLegacyRemovesOldInstallAndPathLine | none | missing | install path port pending |
| TestMigrateLegacySkipsWhenRunningLegacyBinary | none | missing | install path port pending |
| TestMigrateLegacyNoopWithoutLegacyFiles | none | missing | install path port pending |

Rows 1-12 are update_test.go; rows 13-16 are install_path_test.go.

## Totals (this pass)

- Mapped rows: 52 (covered 36, missing 16, n-a 0). Unmapped: 268.
- Covered rows added with new Rust tests: 3 (parity_record.rs).
- Ignored diffs (#[ignore = "parity: ..."]): 0.
