# Rust test parity vs Go, part 2 of 3 (deprecated/src/main/intt/internal/harness)

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
- missing: no Rust test found for this behaviour. No new test written in this pass (see Remaining).
- diff: Rust behaviour differs. Intended as `#[ignore = "parity: <reason>"]`, not yet written.
- n-a: Go behaviour lives outside the Rust scope (reason in note).

## Remaining (top of file, per brief)

- 94 unmapped rows: 29 covered, 16 unverified, 24 missing, 1 diff, 24 n-a (tally below).
- No new file crates/<crate>/tests/parity_harness_p2.rs written in this pass. Missing rows (24) and diff (1) are the add/ignore work for the next pass.
- Body checks still open: 16 unverified rows.
- Update tests (group 2 has none); group 3 holds update rows, mark "wait: update" there.

## Pre-mapped (not re-counted)

record_test.go: 14 rows, see plan/rust-test-parity.md (harness/record_test.go section).

## idle_test.go, idlespend_test.go, importview_test.go (7 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestIdleControlRequestAnswered | crates/core/tests/engine.rs::can_use_tool_in_default_mode_asks_ui_and_sends_answer | unverified | topic match (permission answer); Go case is between turns, Rust case is in-turn; body not compared |
| TestIdleDeniesWhenNoPrompt | none | missing | idle can_use_tool with no UI prompt must deny |
| TestStrayLateResultSkipped | crates/core/tests/engine.rs::stray_result_of_abandoned_turn_does_not_end_next_turn | covered | name-level match |
| TestIdleBackgroundFollowupRendered | none | missing | task-notification between turns renders "에이전트 완료" and clears tracker |
| TestIdleSpendRow | none | missing | background row sub_output_tokens / spawns / result_model_usage |
| TestIdleSpendLiveReplay | none | missing | Go reads testdata/bg_turn_live.jsonl and skips if absent |
| TestReadClaudeTranscriptFullView | crates/core/src/import_view.rs::tool_use_result_and_thinking_are_rendered_in_order, crates/core/src/import_view.rs::subagent_renders_after_its_agent_call | unverified | two Rust tests cover the two halves; single body not compared |

## input_test.go, links_test.go, links_wrap_test.go, localcmd_test.go (13 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestInputWidthMixed | crates/cli/src/tui/render.rs::mixed_width_measure | unverified | topic match (mixed-width cells) |
| TestInputWrapWide | crates/cli/src/tui/render.rs::wrap_cells_breaks_by_cell_width | unverified | topic match (wide cell wrap) |
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
| TestPlainLocalCmdsNoSend | none | missing | plain local commands must not send a prompt; crates/cli/src/tui/commands.rs::plain_text_is_not_a_command is not the same case |

## metrics_test.go, mode_test.go, orch_test.go, orchws_test.go (13 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestMetricsLog | crates/core/src/metrics.rs::log_appends_one_line_per_record_with_ts_and_flags | covered | name-level match |
| TestMetricsNilAndDisabled | crates/core/src/metrics.rs::disabled_log_is_noop | covered | name-level match (nil case not separately checked) |
| TestUsageFields | crates/core/src/metrics.rs::usage_fields_sums_ctx_and_defaults_missing_to_zero | covered | name-level match |
| TestModeCycleAndParse | crates/core/src/mode.rs::cycle_and_parse | covered | name-level match |
| TestDecideMode | crates/core/src/mode.rs::decide_mode_table | covered | name-level match |
| TestReplModeLine | crates/cli/src/tui/commands.rs::mode_no_args_set_and_unknown | unverified | topic match (mode line text) |
| TestOrchParseMode | crates/core/src/orch.rs::parse_mode_normalizes_and_rejects | covered | name-level match |
| TestOrchDefaultModel | crates/core/src/orch.rs::default_model_per_mode | covered | name-level match |
| TestOrchRule | crates/core/src/orch.rs::rule_per_mode | covered | name-level match |
| TestParseWSCases | crates/core/src/orch.rs::parse_ws_cases | covered | name-level match |
| TestWSNeedsRotateCases | crates/core/src/orch.rs::needs_rotate_cases | covered | name-level match |
| TestWSHandoffRuneCap | crates/core/src/orch.rs::handoff_is_rune_capped_with_prefix | covered | name-level match |
| TestRouteOrchBReturnsLine | crates/core/src/orch.rs::route_non_f_mode_keeps_line | unverified | topic match (route returns line); name differs |

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
| TestSessionPanelAndThinking | crates/cli/src/tui/bar.rs::thinking_row_shows_only_for_opus_resident_or_pre_text | unverified | topic match (thinking row) |
| TestEscInterruptClearsForegroundPanel | crates/cli/src/tui/mod.rs::a_single_escape_interrupts_only_while_busy | unverified | topic match (esc interrupt) |
| TestIdleTickRefreshesWhenBackgroundFinishes | none | missing | idle tick refresh of panel after background finishes |
| TestCursorRowMatchesPanelAfterTick | none | missing | cursor row vs panel after tick |
| TestPanelMoreRowAndSmallHeight | crates/cli/src/tui/bar.rs::every_running_agent_stays_visible_when_row_is_narrow | unverified | topic match (small height/narrow row) |
| TestThinkingOnlyForForeground | crates/cli/src/tui/bar.rs::thinking_row_shows_only_for_opus_resident_or_pre_text | unverified | topic match |
| TestBackgroundRowsBelowInputBox | none | missing | background rows layout below input |
| TestBackgroundSmallHeightNoOverflow | none | missing | background rows, small height |
| TestStalledOutputFileKeepsAgentRunningUntilDoneSignal | none | missing | stalled output file keeps agent running until done |
| TestQueueWhileBusyNotSent | crates/core/tests/engine.rs::submit_while_busy_queues_and_never_interrupts | covered | name-level match |
| TestQueueSentOnDoneInOrder | crates/core/tests/engine.rs::queued_prompts_go_out_as_one_merged_turn_on_done | covered | body read: Go sends "second\n\nthird" after first done; Rust merges queued prompts into one turn on done. Same order and merge |
| TestQueueLinesDimAboveInput | none | missing | queued lines dim above input (render) |
| TestBackgroundOnlyDoesNotQueue | crates/core/tests/engine.rs::background_only_agent_does_not_queue_next_prompt | covered | name-level match |
| TestQueueSmallHeightNoOverflow | none | missing | queue rows, small height |
| TestCursorRowIncludesBackgroundRows | none | missing | cursor row counts background rows |
| TestAgentStatusTextPrefixedName | crates/cli/src/tui/bar.rs::agent_name_with_model_prefix_skips_model_segment | unverified | topic match |
| TestAgentStatusFormat | crates/cli/src/tui/bar.rs::agent_status_text_uses_state_model_and_name | unverified | topic match |
| TestPanelScrollWindow | none | missing | panel scroll window |
| TestUserEventNotificationFinishesBackground | none | missing | user-event notification finishes background row |
| TestSetRateLogsOnFiveHourChange | none | missing | rate log row (stage "rate") on five-hour change; Rust metrics has no rate stage test |
| TestSetRateTopLevelFallback | none | missing | rate top-level fallback |
| TestFmtRateStatus | crates/cli/src/tui/mod.rs::rate_status_follows_go_fmt_rate_status | covered | name-level match |
| TestRecordSkip | crates/core/src/record_turn.rs::skip_slash_commands_and_empty_prompts | unverified | topic match (skip slash commands and empty prompts) |
| TestMeasureLine | crates/core/src/record_turn.rs::measure_line_fixed_then_result_keys | unverified | topic match (measure line keys) |

## resident_agent_test.go, resident_busy_test.go, resident_coder_test.go (13 rows)

| Go test | Rust test | status | note |
|---|---|---|---|
| TestAgentPromptLoaded | none | missing | agent prompt file loaded (crates/core/src/resident.rs::strip_frontmatter_drops_header_block covers frontmatter only) |
| TestAgentPromptMissingNotice | none | missing | missing agent prompt shows notice |
| TestStageLinesOrder | crates/core/tests/engine.rs::resident_stage_rows_bracket_main_turn_and_agent_calls | unverified | topic match (stage row order) |
| TestSessionArgsMinimal | crates/core/src/proc.rs::args_match_go_session_defaults | covered | body read: Rust args checked against Go session defaults |
| TestSessionArgsCoder | crates/core/src/proc.rs::resident_shape_args_match_go_session_args | covered | body read: coder session args |
| TestMainStrictMCP | crates/core/src/proc.rs::mcp_config_flag_only_when_set | unverified | topic match (strict MCP flag); body not compared |
| TestResidentRunningListShowsOnlyBusySessions | none | missing | running list shows only busy residents |
| TestResidentCoderReply | crates/core/tests/engine.rs::coder_task_emits_working_done_rows_and_reply | covered | body read: coder reply rows |
| TestResidentCoderSlotReuse | none | missing | coder slot reuse |
| TestResidentCoderBusyRefuses | none | diff | Go refuses a busy single coder; Rust pool errors "all resident coders are busy" (crates/core/src/api.rs:1046). Planned #[ignore = "parity: single-coder busy refusal vs pool error"] |
| TestCoderPermissionOnlyWhenBusy | none | missing | coder permission prompt only when busy |
| TestMCPRoundTripFakeSession | none | missing | MCP round trip with fake session |
| TestResidentMCPConfigWritten | none | missing | resident MCP config written to disk |

## Tally (94 unmapped rows)

- covered: 29
- unverified: 16
- missing: 24
- diff: 1
- n-a: 24
- Sum: 94 (checked: 29 + 16 + 24 + 1 + 24 = 94)

## Pre-existing shared rows check

Group 2 rows already in plan/rust-test-parity.md: 14 (record_test.go). Not changed here.
