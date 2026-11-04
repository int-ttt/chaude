# 261104 haiku-tui phase 4 (crates/cli/src/tui)

done:
- record-kind asst: record_block renders asst via asst_text with ASST_FIRST lead; TODO(record-kinds) removed
- ToolResult arm uses render::tool_result_text (caveman hide kept)
- quiet_notice wired: App quiet_since/open_tools/quiet_noted; tick emits once after IDLE_NOTICE (30m) when busy with no open tool; new turn resets; test idle_notice_once_and_not_while_tool_open
- /view owned: read handoff::read_exit_handoff(handoff_file(record_dir(cwd))).prev_owned(); tui() sets cfg.handoff_path when None
- bar.rs: STAGE_MAIN_*/stage_agent_call/done removed (duplicates of mod.rs stage_text); module-level trial allow removed
- targeted #[allow(dead_code)] with reason on each still-unwired item (bar.rs 8, render.rs 8)
- plan/rust-port.md phase 4 status block added

blocked (core or engine data):
- bar P/T/C: hardcoded 0/80k; UiEvent has no per-role context
- /view this run's ids (main, rotated, resident): no UiEvent field
- /clear new-session notice: engine has no new-session event
- thinking_row, done_line, rotated_notice (old session id), orphan_notice, tool_summary (tool input JSON), import_blocks (paired turns), link_spans/wrap_linked (raw assistant text), input_cursor_cells (I30 core cursor)
- Config::default handoff_path None in cli main; set in tui() only (tui path)

checks:
- cargo build -p chaude-cli -> exit 0
- cargo test -p chaude-cli -> 160 passed, 0 failed
- cargo clippy -p chaude-cli -- -D warnings -> exit 0

doubts:
- tui() now writes .exit-handoff.json into record_dir on quit (Go parity); confirm the core owner accepts it
- targeted allows count as "wired" only for blocked items; the rest are wired
