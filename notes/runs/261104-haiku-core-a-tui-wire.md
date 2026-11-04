# 261104 haiku core A TUI wiring

status: done
scope: crates/cli/src/tui only (core untouched)

wired:
- UiEvent::RoleContext{role,tokens} -> bar P/T/C slots (bar::BarInput.role_tokens, rows())
- UiEvent::SessionIds{main,residents,owned} -> this_run, merged into /view owned list (open_sessions)
- UiEvent::NewSession -> bar::NEW_SESSION_NOTICE note
- UiEvent::RotatedFrom{old_session} -> rot_from; Rotated notice uses bar::rotated_notice(rot, reason, old)
- UiEvent::ToolInput{id,input} -> render::tool_summary rewrites the tool row (tool_rows index, retain on rewind)
- UiEvent::OrphanNotice{role,pid} -> bar::orphan_notice; model = welcome.model for main, role name for residents, cwd = welcome.cwd
- thinking_row in body(): fg agents running or no main text yet, hidden during perm prompt; got_text reset per turn
- done_line on AgentEnd: model "" (shown as 상속), description from agents list, result first line from ToolResult when a failed/agent tool

allows removed: bar.rs thinking_row, ThinkingRow, done_line, orphan_notice, rotated_notice, NEW_SESSION_NOTICE; render.rs tool_summary

still allowed:
- render.rs link_spans, wrap_linked: raw assistant text not kept per block; needs feed/flush redesign
- render.rs input_cursor_cells: test-only; no terminal cursor placement uses it (TODO(core-cursor) dropped, reason corrected)
- orphan model for residents is the role name (no per-role model in core event)

checks:
- cargo build -p chaude-cli -> PASS
- cargo test -p chaude-cli -> PASS (179 passed, 0 failed)
- cargo clippy -p chaude-cli --no-deps -- -D warnings -> PASS
- cargo fmt -p chaude-cli -- --check -> PASS

doubts:
- opus_running and resident_busy have no UiEvent; thinking_row gets false for both
- done_line model empty; summary approximates Go a.Summary by the first result line
- C maps to coder slot 0 only
- ToolInput rewrite is mostly a no-op where ToolStart already carries the summary
- orphan resident model label is role name, not model alias
