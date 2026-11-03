task: TUI parity render (spec plan/tui-parity.md groups L, R)
branch: rust-port
owned: crates/cli/src/tui/render.rs (new), crates/cli/src/tui/mod.rs (line 7 `pub mod render;` only)
not_touched: edit.rs, bar.rs, commands.rs

gates:
  cargo test -p chaude-cli: 79 passed, 0 failed
  cargo clippy -p chaude-cli --all-targets -- -D warnings: Finished, 0 warnings
  cargo fmt -p chaude-cli -- --check: exit 0

done (pure fn + unit test in render.rs):
  R01 palette consts ORANGE_ON/GREY_ON (tui.go:252-256) - no test in Go
  R03 md_bold - TestMdBold cases ported
  R04 is_caveman_block / is_status_line - TestIsCavemanBlock cases ported
  R05 tool_result_text (caveman hidden, first line, "… (+N lines)") - TestToolResultCavemanHidden logic ported; no Go test for the summary line
  R07 perm_note - string format only (tui.go:499)
  R08 link_spans / mark_links - TestLinkSpansDetects, ProseNegatives, OverlapAndBoundary ported
  R09 linkify - TestLinkifyPreservesVisibleText ported
  R10 wrap_linked fence skip - TestWrapLinkedSkipsFence ported
  R11 restyle + wrap_linked - TestWrapLinkedColorsWholeTokens ported (14 cases)
  R13 cell_width - TestInputWidthMixed ported
  R14 trunc_ellipsis - no Go test
  R15 render_bar / bar_width - TestRenderBar ported
  R16 decorate_usage / commas - TestDecorateUsage ported
  R17 pct_value - TestDecorateUsage path ported
  R18 DIM_ON/RESET/BOLD consts - constants only
  R19 import_blocks (100 cap, note) - logic only; TestImportShowsTurnsInTUI needs TUI model, not ported

partial:
  R06 tool_summary: priority keys, newline to space, 100-char cut ported. Go JSON fallback NOT ported (needs serde_json, not an allowed dep). Returns None for unknown keys. Test: tool_summary_uses_priority_keys.
  R12 wrap: wrap_cells implements input.go rows() rule (TestInputWrapWide ported). Doubt: Go output pane uses lipgloss word wrap; not reproduced. Only the input-box rule is in render.rs.

blocked (architecture or engine needs):
  L01 altscreen full layout: Rust is inline (scrollback + 3-row area), not altscreen.
  L02 viewport height per frame, L03 "starting…", L06 bg rows, L07 panel rows, L08 small-height overflow, L13 overlay screens, L14 cursor row tracking: depend on altscreen viewport / agent tracker model.
  L04 message blocks: needs transcript event model in core (engine).
  L05 welcome banner: needs effectiveThreshold (engine).
  R20 selection highlight by content coordinates: no selection model in inline Rust TUI.
  R05 full source: tool_result content from core events (engine).
  R07 full source: control_request can_use_tool event (engine).
  R16/R17 live numbers: usage/context values from core (engine).
  R19 full source: transcript import from core (engine).

not implemented in this run (no code):
  L09 suggestion popup cap (maxPop=8), L10 input height cap (maxInputRows=5), L11 queue lines dim, L12 status line: pure parts could go in layout.rs; layout.rs was not created in this run.
  R02 no OSC background query: no code path checked.

doubts:
  - wrap semantics for output pane (R12) differ from Go lipgloss word wrap.
  - R06 fallback output differs for unknown tool input keys until JSON is available.
  - render.rs has #![allow(dead_code)]: nothing calls it yet.
