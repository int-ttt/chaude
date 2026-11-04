# Go->Rust test parity group 2 step 2 (run note)
status: done (p2 file green; full chaude-core run still in progress at write time, not claimed)
date: 261104
branch: rust-port
scope: body-check 16 unverified rows; add tests for core-feasible missing rows; no src edits; no push

## body-check (16 unverified)
covered:
- TestReadClaudeTranscriptFullView -> import_view.rs tool_use_result_and_thinking_are_rendered_in_order, subagent_renders_after_its_agent_call
- TestInputWidthMixed -> cli tui/render.rs mixed_width_measure (width only; cursor/backspace are Go inputBox state, not ported to a testable API)
- TestInputWrapWide -> cli tui/render.rs wrap_cells_breaks_by_cell_width
- TestReplModeLine -> cli tui/commands.rs mode_no_args_set_and_unknown
- TestRouteOrchBReturnsLine -> core orch.rs route_non_f_mode_keeps_line
- TestThinkingOnlyForForeground -> cli tui/bar.rs thinking_row_shows_only_for_opus_resident_or_pre_text
- TestAgentStatusTextPrefixedName -> cli tui/bar.rs agent_name_with_model_prefix_skips_model_segment
- TestAgentStatusFormat -> cli tui/bar.rs agent_status_text_uses_state_model_and_name (text parts only)
- TestRecordSkip -> core record_turn.rs skip_slash_commands_and_empty_prompts
- TestMeasureLine -> core record_turn.rs measure_line_fixed_then_result_keys
- TestStageLinesOrder -> engine.rs resident_stage_rows_bracket_main_turn_and_agent_calls (event order prompter before tui; Go text lines are UI-side)
missing:
- TestIdleControlRequestAnswered -> not added: fake script has no idle injection (needs fake change; out of scope)
- TestIdleDeniesWhenNoPrompt -> not added: same reason
- TestSessionPanelAndThinking, TestEscInterruptClearsForegroundPanel, TestPanelMoreRowAndSmallHeight -> needs pub (cli is bin-only; needs lib target)
- TestMainStrictMCP -> missing, added in parity_harness_p2.rs (main_child_args_use_strict_mcp_and_keep_slash_commands)

## tests added: crates/core/tests/parity_harness_p2.rs
- resident_coder_slot_reuse_spawns_no_extra (TestResidentCoderSlotReuse) -> ok
- resident_coder_busy_refuses_second_task (TestResidentCoderBusyRefuses) -> ok
- mcp_protocol_answers_and_skips_notifications (TestMCPRoundTripFakeSession, protocol) -> ok
- mcp_bridge_round_trip_over_loopback (TestMCPRoundTripFakeSession, socket) -> ok
- mcp_config_is_written_with_bridge_command_and_token_env (TestResidentMCPConfigWritten) -> ok
- agent_prompt_body_loads_from_cwd_without_frontmatter (TestAgentPromptLoaded) -> ok
- idle_spend_live_fixture_parses_line_by_line (TestIdleSpendLiveReplay, parse-only) -> ok
- main_child_args_use_strict_mcp_and_keep_slash_commands (TestMainStrictMCP) -> ok
- ignored (diff): mcp_config_args_point_at_unix_socket (Rust tcp:<addr> vs Go .mcp.sock)
- ignored (diff): agent_prompt_missing_gives_korean_notice_and_default_session (Rust Err, no Go notice)
- ignored (diff): idle_spend_row_counts_subagent_tokens_and_spawns (no bg spend row; body is panic placeholder)

## checks
- rustfmt --check --edition 2024 crates/core/tests/parity_harness_p2.rs -> exit 0
- cargo test -p chaude-core --test parity_harness_p2 (after main-args test) -> 8 passed, 0 failed, 3 ignored (30.86s)
- note: lib build briefly broken by another agent's uncommitted api.rs edit (E0502 at api.rs:2066); retried, then built.
