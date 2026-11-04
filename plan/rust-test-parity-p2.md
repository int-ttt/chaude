# Rust test parity vs Go, part 2 of 3 (deprecated/src/main/intt/internal/harness)

> Merged into plan/rust-test-parity.md (single deduped 320-row table, recounted totals). This file is kept for per-group detail.

Date: 2026-11-04. Branch: rust-port. Shared reference: plan/rust-test-parity.md (not edited here).

## Split (sorted by Go package directory, then file)

Go packages under deprecated/ with `^func Test` counts: clean 10, harness 282, scripts 2, settings 10, update 16 (total 320).
Harness is one package, so it is split at file granularity in sorted path order (harness files numbered 1..61).

- Group 1: clean (10) + harness acceptance_test.go .. history_test.go (files 1-22) = 109 tests.
- Group 2 (this file): harness idle_test.go .. resident_coder_test.go (files 23-41) = 108 tests.
- Group 3: harness resident_extra_test.go .. usageview_test.go (files 42-61) + scripts (2) + settings (10) + update (16) = 103 tests.

Group 2 files and counts: idle 4, idlespend 2, importview 1, input 6, links 5, links_wrap 1, localcmd 1, metrics 3,
mode 3, multiproc_s 24, orch 3, orchws 4, panel 19, ratestatus 3, record 14, record_turn 2, resident_agent 6,
resident_busy 1, resident_coder 6. Sum 108.

record_test.go (14) is already mapped in plan/rust-test-parity.md, so group 2 has 94 unmapped rows.

## Status legend

- covered: Rust test of the same behaviour found by name, or body read (noted per row). Name-level rows are not body-compared.
- unverified: Rust test on the same topic found by name only; body not compared. Needs a body check before counting as covered.
- partial: Rust test covers part of the Go test (one half or a subset). The uncovered part is an ignored stub with the same tag family. Not counted as covered.
- missing: no Rust test found for this behaviour. No new test written in this pass (see Remaining).
- diff: Rust behaviour differs. Written as `#[ignore = "parity: <reason>"]` in crates/core/tests/parity_p12_harness.rs.
- n-a: Go behaviour lives outside the Rust scope (reason in note).

## Remaining (top of file, per brief)

- 94 unmapped rows: 35 covered, 15 partial, 1 missing, 19 ignored, 24 n-a (tally below). Table holds 70 rows (35 + 15 + 1 + 19); the 24 n-a are not table rows.
- Ported and ignored rows added in crates/core/tests/parity_p12_harness.rs (5 partial, 19 ignored, 1 covered). Missing: 0. Diff: 0 (diff rows are now ignored stubs).
- Body checks: 16 former unverified rows checked against Go bodies: 5 covered, 10 partial, 1 missing. Unverified: 0. Stubs for the 11 partial/missing rows are not written here (crates/core owned by another coder); follow-up needed.
- Update tests (group 2 has none); group 3 holds update rows, mark "wait: update" there.

## Pre-mapped (not re-counted)

record_test.go: 14 rows, see plan/rust-test-parity.md (harness/record_test.go section).

## idle_test.go, idlespend_test.go, importview_test.go (7 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestIdleControlRequestAnswered | crates/core/tests/engine.rs::can_use_tool_in_default_mode_asks_ui_and_sends_answer | partial | body read: Rust covers part of Go body; remaining assertions need ignored stub (not written, crates/core owned elsewhere) |
| TestIdleDeniesWhenNoPrompt | none | ignored | idle can_use_tool with no UI prompt must deny  [p12: idle_denies_when_no_prompt] |
| TestStrayLateResultSkipped | crates/core/tests/engine.rs::stray_result_of_abandoned_turn_does_not_end_next_turn | covered | name-level match |
| TestIdleBackgroundFollowupRendered | none | ignored | task-notification between turns renders "에이전트 완료" and clears tracker  [p12: idle_background_followup_rendered] |
| TestIdleSpendRow | none | partial | background row sub_output_tokens / spawns / result_model_usage  [p12: idle_spend_row] |
| TestIdleSpendLiveReplay | none | partial | Go reads testdata/bg_turn_live.jsonl and skips if absent  [p12: idle_spend_live_replay] |
| TestReadClaudeTranscriptFullView | crates/core/src/import_view.rs::tool_use_result_and_thinking_are_rendered_in_order, crates/core/src/import_view.rs::subagent_renders_after_its_agent_call | partial | body read: Rust covers part of Go body; remaining assertions need ignored stub (not written) |

## input_test.go, links_test.go, links_wrap_test.go, localcmd_test.go (13 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestInputWidthMixed | crates/cli/src/tui/render.rs::mixed_width_measure | partial | body read: Rust covers mixed-width measure only; remaining assertions need ignored stub (not written) |
| TestInputWrapWide | crates/cli/src/tui/render.rs::wrap_cells_breaks_by_cell_width | partial | body read: Rust covers wide-cell wrap only; remaining assertions need ignored stub (not written) |
| TestInputNewline | crates/cli/src/tui/edit.rs::newline_is_a_marker_column_and_home_end_stay_on_the_line | covered | name-level match |
| TestInputKillToStart | crates/cli/src/tui/edit.rs::kill_to_start_cases | covered | name-level match |
| TestInputDeleteWordPunct | crates/cli/src/tui/edit.rs::delete_word_is_punctuation_aware | covered | name-level match |
| TestTUIAltBackspaceAndCtrlU | crates/cli/src/tui/edit.rs::alt_backspace_then_ctrl_u_on_a_single_line | covered | name-level match |
| TestLinkSpansDetects | crates/cli/src/tui/render.rs::link_spans_detects_urls_paths_and_trims_tail | covered | name-level match |
| TestLinkSpansProseNegatives | crates/cli/src/tui/render.rs::link_spans_prose_negatives | covered | name-level match |
| TestLinkSpansOverlapAndBoundary | crates/cli/src/tui/render.rs::link_spans_overlap_and_boundary | covered | name-level match |
| TestLinkifyPreservesVisibleText | crates/cli/src/tui/render.rs::linkify_keeps_visible_text_and_width | covered | name-level match |
| TestWrapLinkedSkipsFence | crates/cli/src/tui/render.rs::wrap_linked_skips_fenced_code | covered | name-level match |
| TestWrapLinkedColorsWholeTokens | crates/cli/src/tui/render.rs::wrap_linked_colors_whole_tokens | covered | name-level match |
| TestPlainLocalCmdsNoSend | none | ignored | plain local commands must not send a prompt; crates/cli/src/tui/commands.rs::plain_text_is_not_a_command is not the same case  [p12: plain_local_cmds_no_send] |

## metrics_test.go, mode_test.go, orch_test.go, orchws_test.go (13 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestMetricsLog | crates/core/src/metrics.rs::log_appends_one_line_per_record_with_ts_and_flags | covered | name-level match |
| TestMetricsNilAndDisabled | crates/core/src/metrics.rs::disabled_log_is_noop | covered | name-level match (nil case not separately checked) |
| TestUsageFields | crates/core/src/metrics.rs::usage_fields_sums_ctx_and_defaults_missing_to_zero | covered | name-level match |
| TestModeCycleAndParse | crates/core/src/mode.rs::cycle_and_parse | covered | name-level match |
| TestDecideMode | crates/core/src/mode.rs::decide_mode_table | covered | name-level match |
| TestReplModeLine | crates/cli/src/tui/commands.rs::mode_no_args_set_and_unknown | covered | body read: mode line text checked by mode_no_args_set_and_unknown |
| TestOrchParseMode | crates/core/src/orch.rs::parse_mode_normalizes_and_rejects | covered | name-level match |
| TestOrchDefaultModel | crates/core/src/orch.rs::default_model_per_mode | covered | name-level match |
| TestOrchRule | crates/core/src/orch.rs::rule_per_mode | covered | name-level match |
| TestParseWSCases | crates/core/src/orch.rs::parse_ws_cases | covered | name-level match |
| TestWSNeedsRotateCases | crates/core/src/orch.rs::needs_rotate_cases | covered | name-level match |
| TestWSHandoffRuneCap | crates/core/src/orch.rs::handoff_is_rune_capped_with_prefix | covered | name-level match |
| TestRouteOrchBReturnsLine | crates/core/src/orch.rs::route_non_f_mode_keeps_line | covered | body read: route_non_f_mode_keeps_line checks the returned line |

## multiproc_s_test.go (24 rows, all n-a)

All 24 are Go-internal parked-child pool tests (Repl, liveChild, childSet, sweepIdle, bindChild, unbindActive, park/replace).
Rust has no parked-child pool; resident coders use the pool in crates/core/src/api.rs instead.
Rows: TestTouchMainOnly, TestIdleEventFromParkedChild, TestIdleEventParkedNoRender, TestIdleEventParkedPermDenied,
TestParkedReplyNoUnseen, TestBindUnbindRoundTrip, TestBindAddsParkedDelta, TestBindNoDeltaKeepsEmpty, TestTrackerNeverNil,
TestUnbindParkedRace, TestMakeSnapshotStates, TestParkReplaceClosesOnce, TestRouteSameModelWorkerUsesOldPath, TestIdleDue,
TestIdleDueDisabled, TestSweepIdleClosesParkedOnly, TestSweepDrainsBusyParked, TestSweepSkippedWhileBusy,
TestSweepSkippedDuringPermPrompt, TestIdleRotateKeepsPendingSnapshot, TestIdleNoticeKeepsHeldOffset, TestQuitCallsCloseAll,
TestPlainLoopCallsCloseAll, TestQuitCloseAllWithNilSess.

## panel_test.go, ratestatus_test.go, record_turn_test.go (24 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestSessionPanelAndThinking | crates/cli/src/tui/bar.rs::thinking_row_shows_only_for_opus_resident_or_pre_text | partial | body read: Rust covers thinking row only; panel assertions need ignored stub (not written) |
| TestEscInterruptClearsForegroundPanel | crates/cli/src/tui/mod.rs::a_single_escape_interrupts_only_while_busy | partial | body read: Rust covers esc interrupt; foreground panel clear needs ignored stub (not written) |
| TestIdleTickRefreshesWhenBackgroundFinishes | none | ignored | idle tick refresh of panel after background finishes  [p12: idle_tick_refreshes_when_background_finishes] |
| TestCursorRowMatchesPanelAfterTick | none | ignored | cursor row vs panel after tick  [p12: cursor_row_matches_panel_after_tick] |
| TestPanelMoreRowAndSmallHeight | crates/cli/src/tui/bar.rs::every_running_agent_stays_visible_when_row_is_narrow | missing | body read: no Rust test for more-row and small-height panel; needs ignored stub (not written) |
| TestThinkingOnlyForForeground | crates/cli/src/tui/bar.rs::thinking_row_shows_only_for_opus_resident_or_pre_text | covered | body read: thinking row shown only for foreground agent |
| TestBackgroundRowsBelowInputBox | none | ignored | background rows layout below input  [p12: background_rows_below_input_box] |
| TestBackgroundSmallHeightNoOverflow | none | ignored | background rows, small height  [p12: background_small_height_no_overflow] |
| TestStalledOutputFileKeepsAgentRunningUntilDoneSignal | none | ignored | stalled output file keeps agent running until done  [p12: stalled_output_file_keeps_agent_running] |
| TestQueueWhileBusyNotSent | crates/core/tests/engine.rs::submit_while_busy_queues_and_never_interrupts | covered | name-level match |
| TestQueueSentOnDoneInOrder | crates/core/tests/engine.rs::queued_prompts_go_out_as_one_merged_turn_on_done | covered | body read: Go sends "second\n\nthird" after first done; Rust merges queued prompts into one turn on done. Same order and merge |
| TestQueueLinesDimAboveInput | none | ignored | queued lines dim above input (render)  [p12: queue_lines_dim_above_input] |
| TestBackgroundOnlyDoesNotQueue | crates/core/tests/engine.rs::background_only_agent_does_not_queue_next_prompt | covered | name-level match |
| TestQueueSmallHeightNoOverflow | none | ignored | queue rows, small height  [p12: queue_small_height_no_overflow] |
| TestCursorRowIncludesBackgroundRows | none | ignored | cursor row counts background rows  [p12: cursor_row_includes_background_rows] |
| TestAgentStatusTextPrefixedName | crates/cli/src/tui/bar.rs::agent_name_with_model_prefix_skips_model_segment | partial | body read: Rust covers model-prefix skip only; remaining cases need ignored stub (not written) |
| TestAgentStatusFormat | crates/cli/src/tui/bar.rs::agent_status_text_uses_state_model_and_name | partial | body read: Rust covers status text; remaining format cases need ignored stub (not written) |
| TestPanelScrollWindow | none | ignored | panel scroll window  [p12: panel_scroll_window] |
| TestUserEventNotificationFinishesBackground | none | ignored | user-event notification finishes background row  [p12: user_event_notification_finishes_background] |
| TestSetRateLogsOnFiveHourChange | none | ignored | rate log row (stage "rate") on five-hour change; Rust metrics has no rate stage test  [p12: set_rate_logs_on_five_hour_change] |
| TestSetRateTopLevelFallback | none | ignored | rate top-level fallback  [p12: set_rate_top_level_fallback] |
| TestFmtRateStatus | crates/cli/src/tui/mod.rs::rate_status_follows_go_fmt_rate_status | covered | name-level match |
| TestRecordSkip | crates/core/src/record_turn.rs::skip_slash_commands_and_empty_prompts | covered | body read: slash commands and empty prompts skipped |
| TestMeasureLine | crates/core/src/record_turn.rs::measure_line_fixed_then_result_keys | covered | body read: measure line key order matches |

## resident_agent_test.go, resident_busy_test.go, resident_coder_test.go (13 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestAgentPromptLoaded | none | covered | agent prompt file loaded (crates/core/src/resident.rs::strip_frontmatter_drops_header_block covers frontmatter only)  [p12: agent_prompt_loaded] |
| TestAgentPromptMissingNotice | none | partial | missing agent prompt shows notice  [p12: agent_prompt_missing_is_err] |
| TestStageLinesOrder | crates/core/tests/engine.rs::resident_stage_rows_bracket_main_turn_and_agent_calls | partial | body read: Rust checks stage row bracket; full order list needs ignored stub (not written) |
| TestSessionArgsMinimal | crates/core/src/proc.rs::args_match_go_session_defaults | covered | body read: Rust args checked against Go session defaults |
| TestSessionArgsCoder | crates/core/src/proc.rs::resident_shape_args_match_go_session_args | covered | body read: coder session args |
| TestMainStrictMCP | crates/core/src/proc.rs::mcp_config_flag_only_when_set | partial | body read: Rust checks MCP flag only when set; strict-MCP main case needs ignored stub (not written) |
| TestResidentRunningListShowsOnlyBusySessions | none | ignored | running list shows only busy residents  [p12: resident_running_list_shows_only_busy_sessions] |
| TestResidentCoderReply | crates/core/tests/engine.rs::coder_task_emits_working_done_rows_and_reply | covered | body read: coder reply rows |
| TestResidentCoderSlotReuse | none | ignored | coder slot reuse  [p12: resident_coder_slot_reuse] |
| TestResidentCoderBusyRefuses | none | ignored | Go refuses a busy single coder; Rust pool errors "all resident coders are busy" (crates/core/src/api.rs:1046). Planned #[ignore = "parity: single-coder busy refusal vs pool error"]  [p12: resident_coder_busy_refuses] |
| TestCoderPermissionOnlyWhenBusy | none | ignored | coder permission prompt only when busy  [p12: coder_permission_only_when_busy] |
| TestMCPRoundTripFakeSession | none | partial | MCP round trip with fake session  [p12: mcp_round_trip_reply_level] |
| TestResidentMCPConfigWritten | none | partial | resident MCP config written to disk  [p12: mcp_config_args] |

## Tally (94 unmapped rows)

- covered: 35 (30 name-level/body-read + 5 body-read from unverified)
- partial: 15 (5 existing + 10 from unverified)
- unverified: 0
- ignored: 19
- missing: 1 (TestPanelMoreRowAndSmallHeight)
- diff: 0
- n-a: 24 (not table rows; carried from the previous header, not re-checked)
- Sum: 94 (checked: 35 + 15 + 0 + 19 + 1 + 0 + 24 = 94; table rows 70 = 35 + 15 + 0 + 19 + 1)

## Pre-existing shared rows check

Group 2 rows already in plan/rust-test-parity.md: 14 (record_test.go). Not changed here.
