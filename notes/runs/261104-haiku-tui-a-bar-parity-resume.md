# 261104 haiku-tui-a bar parity + resume (crates/cli/src/tui)
status: partial
commit: see git log -1 (mod.rs + this note only)
scope: crates/cli/src/tui only; Go truth deprecated/src/main/intt/
done:
  - mod.rs input box: cursor cell now via render::input_view_row (Go input.go View, I33); replaces inline split_cells + REV_ON formatting. Behavior same for cursor-at-end (space cell).
  - resumed background agents re-shown in agent list: already on rust-port (875adef, mod.rs UiEvent::AgentResumed arm). Not re-verified against Go in this run.
not done / blocked:
  - uncommitted trial by prior coder in bar.rs + render.rs: #![allow(dead_code)] replaced with "// TRIAL-ALLOW-REMOVED". Left untouched. Effect: build OK, 23 dead-code warnings (listed items unwired: wrap_linked, trunc_ellipsis, tool_summary, thinking_row, scroll_range_text, rotated_notice, quiet_notice, done_line, input_cursor_cells, highlight_view, link_spans, import_blocks, go_duration, fmt_tokens, bar_width, orphan_notice, tool_result_text, stage_agent_call/done, STAGE_MAIN_START/DONE, NEW_SESSION_NOTICE, ThinkingRow). clippy -D warnings would FAIL while trial stays. Decide: revert trial (restore allow) or wire the items.
  - attempted git checkout of bar.rs/render.rs to HEAD to undo the trial: DENIED by auto-mode classifier (irreversible local destruction). Not retried. Needs user decision.
  - bar P/T/C hardcoded "0/80k" (bar.rs): needs per-role context in core UiEvent. Blocked (core).
  - /view owned-session set empty (sessview TODO view-owned): core emits no session ids. Blocked (core).
  - record non-user kinds as plain note rows (overlays-wiring TODO record-kinds): not done.
  - commands.rs:164 allow(dead_code) remains.
checks:
  - cargo build -p chaude-cli -> PASS (exit 0, 23 dead-code warnings from uncommitted bar.rs/render.rs trial)
  - cargo test -p chaude-cli -> PASS (159 passed, 0 failed)
risks:
  - committed mod.rs alone depends on HEAD render::input_view_row (exists at HEAD).
