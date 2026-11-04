# tui-panel-wiring: S12 S16 S17 B02 + popH (261104)
agent: haiku-tui-panel-wiring
branch: rust-port (not pushed)
status: done (fmt --all blocked by out-of-scope file, see doubts)
paths: crates/cli/src/tui/mod.rs, crates/cli/src/tui/bar.rs, plan/tui-parity.md

## rows
- S12 PgUp/PgDn: av key path vp_h = body_h(cols, rows) (mod.rs av key path), was CHROME-based. Go avKey agentview.go:135-150. Test: none for PgUp (gap).
- S16 panel wheel: App.pan_off, panel_wheel (mod.rs), overlay_wheel(up, row), call sites pass m.row. Go panelWheel tui.go:541-567, agentview.go:368. Tests: wheel_over_the_panels_scrolls_the_agent_window, wheel_above_the_panels_is_not_taken_by_them.
- S17 range indicator: agent_lines draws bar::scroll_range_text, window starts at pan_off; dead_code allow removed (bar.rs). Go tui.go:1490. Tests: panel_shows_a_scroll_row_when_agents_do_not_fit (existing), wheel test asserts "2-4/5".
- B02 lastTool: App.last_tool set on ToolStart (trunc 40 runes), cleared on Busy(true), busy_row uses it. Go tui.go:900-906, sendPrompt lastTool reset. Test: busy_row_shows_the_last_tool_and_a_new_turn_clears_it.
- popH: queue_lines and agent_lines subtract popup_h() = min(suggest().len(), SLASH_MAX_ROWS). Go popH tui.go:1448-1495 area. Test: slash_menu_takes_room_from_the_queue.

## checks
- cargo test -p chaude-cli tui (CARGO_HOME/RUSTUP_HOME real): 205 passed, 0 failed, 1 ignored (was 201 before; +4 new)
- cargo clippy --workspace --all-targets: exit 0, 0 warning/error lines
- cargo fmt --all --check: exit 1; only diff is crates/core/tests/parity_final3_resident.rs (untracked, another agent). mod.rs formatted via rustfmt, clean.
- HOME=$(mktemp -d) cargo test without CARGO_HOME override: failed "No space left on device" (disk 98-96% full). Used real CARGO_HOME instead.
- full workspace test not run.

## doubts
- git add -p is interactive and unsupported here; all hunks in the 3 files were checked as this workstream's, so whole-file add used.
- PgUp/PgDn live-height has no dedicated test.
- disk nearly full (327Mi free); each HOME=mktemp run re-extracts cargo registry into temp (33G in $TMPDIR).
