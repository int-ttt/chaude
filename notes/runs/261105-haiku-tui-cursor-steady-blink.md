# 261105 haiku tui-cursor-steady-blink
status: done
workstream: tui-cursor, follows 261105-haiku-tui-cursor-ime-hide-toggle.md
symptom: JetBrains user, Hangul correct with CHAUDE_IME_HIDE=0 or 1 (same result); cursor keeps blinking.
cause: crates/cli/src/tui/mod.rs (TermGuard::enter / restore_terminal) never set a cursor shape, so the terminal default (blinking block) applies to the shown/parked cursor. CHAUDE_IME_HIDE does not change the shape, which fits "same result" for both values.
fix: CursorShape(bool) Command (DECSCUSR CSI Ps SP q): steady block (Ps 2) on enter, terminal default (Ps 0) on restore_terminal.
changed: crates/cli/src/tui/mod.rs (CursorShape struct, enter + restore lists, test cursor_shape_is_steady_on_start_and_default_on_exit)
not changed: per-frame MoveTo/Show/Hide in draw() (mod.rs ~2346-2355). Skipping redundant cursor writes needs state across frames; other writers (release_copy OSC52, MouseCellMotion toggle) move the real cursor, so a stateful skip could leave IME parking wrong. Not done.
other per-frame: draw() runs every TICK (80ms) while busy (mod.rs ~2793-2802), re-emitting Show/Hide + park. Input row itself is rewritten only when its text changes.
reverse cell: render.rs:1068 input_view_row uses SGR 7 only; no SGR 5 blink.

## Go (deprecated/src/main/intt/internal/harness/tui.go)
- No HideCursor/ShowCursor/DECSCUSR anywhere in the harness. Real cursor stays at terminal default shape and is parked per frame by cursorWriter (tui.go:223-250, up N + column C after each Write).
- bubbletea v1.3.4 (/Users/leekwangseok/go/pkg/mod/github.com/charmbracelet/bubbletea@v1.3.4): hideCursor only on HideCursor() msg (tea.go:442, screen.go:101); harness never sends it. Correction: 261105-tui-cursor-jediterm-double-cursor.md said bubbletea hides the cursor; that is not what v1.3.4 does for this program.
- bubbles v0.20.0 textinput blinks its cursor, but the harness input is custom (input.go), no bubbles cursor.
So Go shows the terminal-default (blinking) cursor too; Rust now diverges by steady block.

## Checks
- cargo fmt --all -- --check -> PASS (exit 0)
- cargo test -p chaude-cli cursor_shape_is_steady -> PASS (1 passed)
- cargo clippy -p chaude-cli --all-targets -- -D warnings -> PASS (no warnings)
- cargo test -p chaude-cli -> PASS (265 lib passed, 1 ignored; 4 bin passed)
- cargo build -p chaude-cli -> PASS; target/debug/chaude rebuilt

## Doubts
- Not verified in JetBrains. JediTerm DECSCUSR support assumed. If blinking persists, next suspect is the per-frame Show/Hide (draw tail) resetting JediTerm's blink timer.
- Hide=1 path still hides; blink with Hide=1 would mean JediTerm draws its cursor regardless of DECTCEM.

## User verify
1. cd /Users/leekwangseok/PycharmProjects/claude-harness && CHAUDE_IME_HIDE=0 ./target/debug/chaude (JetBrains terminal)
2. Cursor should be a steady block in the input box while the TUI runs; after quit, the shell cursor returns to its normal shape.
3. Repeat with CHAUDE_IME_HIDE=1; report whether blinking remains.
