workstream: tui-stage-timing
crate: chaude-cli
path: crates/cli/src/tui/mod.rs
commit: none (not requested)
status: done

changed:
- action rows: start stamp HH:MM:SS after lead ("● " / "· "); finished rows get " → HH:MM:SS (elapsed)" at end (fmt via bar::fmt_secs)
- new helpers: clock_hms, clock_hhmmss (clock_hhmm kept for rate-limit text), action_line, tool_line
- ToolRow / StageRow tracking (tool_rows, stage_rows); rewind trims them
- queued input: submit() no longer echoes while busy and not a command; echo_prompt() called on QueueTexts drain (busy, non-empty -> empty) with "\n\n"-merged text
- tests updated (3): transcript_blocks_follow_go_leads_and_blank_rows, tool_input_rewrites_the_tool_row_summary, stage_rows_are_grey_and_separated_by_a_blank_row; helper untime()
- tests added (3): action_line_prefixes_the_start_stamp_and_appends_the_end_stamp, queued_prompt_is_not_echoed_while_busy, queued_prompt_echoes_once_when_it_is_sent

checks:
- cargo test -p chaude-cli -> PASS (lib 268 passed, 0 failed, 1 ignored; bin 4 passed)
- cargo clippy -p chaude-cli --all-targets -> PASS (exit 0, no warnings)
- cargo fmt -p chaude-cli -- --check -> PASS (ran cargo fmt -p chaude-cli once; touched mod.rs formatting only)

doubts:
- git status at session start already listed crates/cli/src/tui/mod.rs as modified; the diff therefore may include pre-existing uncommitted hunks besides this change (not separated)
- on_eof clears queue while busy still true: queued prompts discarded on exit are not echoed (no drain)
- TUI busy flag can lag engine; prompts queued while TUI busy is false would not be echoed at drain-time if the drain path differs (unverified at runtime)
- Stage Done/Failed row is untimed text; the Working row is rewritten with end stamp
- plain (non-TUI) mode still echoes "> line" (unchanged)
- end-to-end runtime behaviour not exercised; only unit tests
