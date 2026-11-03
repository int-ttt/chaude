# 261104 haiku slash-menu: Go-exact "/" completion menu
scenario: slash-menu
before: 73% (16 changed lines, logs/tui-diff/slash-menu.diff)
after: 100% (0 changed lines, 30/30 rows)
source-truth: deprecated/src/main/intt/internal/harness/tui.go:1262-1279 (row = "  %-9s %s"; selected = orange #D97757, others dimSt grey #888888; no "> " mark; no per-row clip, terminal clips at full width), tui.go:192-230 (slashCmds, list verified equal to commands.rs SLASH_CMDS), tui.go:974-986 (suggestions)
changes:
- overlay.rs slash_popup: "  {:<9} {desc}" row, ORANGE_ON for selected, GREY_ON others, FG_OFF end; clip to width; tests use plain() to strip SGR
- mod.rs frame(): menu clipped to full w (was text_width w-1); 2 test expectations updated ("> /model" -> "  /model", selected row checks ORANGE_ON)
- commands.rs: removed module-level #![allow(dead_code)]; kept narrow #[allow(dead_code)] on PermMode::next (C18 shift+tab not wired) and UPDATE_START_NOTE (C20 /update not wired)
gates:
- cargo fmt --all -- --check -> rc 0
- cargo clippy --workspace --all-targets -- -D warnings -> rc 0
- cargo test --workspace -> rc 0 (chaude-cli bin 92 passed; all suites ok)
- scripts/tui-diff.sh slash-menu -> 100%
not done: Go Enter/Tab key paths already match in Rust (complete/dispatch); perm_open always false (no TUI permission prompt state yet)
