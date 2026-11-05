# 261105 tui-cursor: JetBrains double cursor
status: done
cause: crates/cli/src/tui/mod.rs draw() emitted Show (?25h) every frame; Go bubbletea hides the hardware cursor (bubbletea tea.go:443) and only parks it for IME. Under JediTerm the shown hardware cursor sits one cell left (ime_col_shift -1) next to the drawn reverse-video cell.
changed: crates/cli/src/tui/mod.rs (hide_cursor field, Hide under JetBrains-JediTerm, new test draw_hides_the_real_cursor_under_jediterm_and_still_parks_it)
normal terminals: unchanged (Show still emitted when TERMINAL_EMULATOR != JetBrains-JediTerm)
check: cargo fmt --check -> PASS; cargo clippy -p chaude-cli --all-targets -D warnings -> PASS; cargo test -p chaude-cli -> PASS (257 lib, 4 bin)
doubt: not verified in a real JetBrains terminal; IME preedit placement with hidden cursor assumed same as Go.
