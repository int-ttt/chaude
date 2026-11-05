# 261105 haiku tui-cursor-ime-hide-toggle
status: done
workstream: tui-cursor, follows 261105-haiku-tui-cursor-park-parity.md and 261105-tui-cursor-jediterm-double-cursor.md
changed: crates/cli/src/tui/mod.rs (hide_cursor_for + CHAUDE_IME_HIDE, test hide_cursor_default_is_jediterm_only_and_env_overrides_it)
default: unchanged (Hide only under TERMINAL_EMULATOR=JetBrains-JediTerm)
CHAUDE_IME_HIDE=1: always hide; =0: never hide (Show, Go-style shown cursor); other/unset: default

## Checks
- cargo fmt --all -- --check -> PASS (exit 0)
- cargo clippy -p chaude-cli --all-targets -- -D warnings -> PASS (exit 0)
- cargo test -p chaude-cli -> PASS (263 lib passed, 1 ignored; 4 bin passed)
- cargo build -p chaude-cli -> PASS; target/debug/chaude rebuilt 18:43

## Clipping review (draw / cursor / input_view_row)
- draw() clears and rewrites a whole changed row (Clear(CurrentLine) + row text). No partial erase of the cell before the cursor. Input row is in chrome, not the scroll-region body, so body_shift does not touch it.
- Reverse cell is at the display column of the cursor char (split_cells); wide char is reverse-drawn as 2 cells. No reverse cell at col-1 in normal flow.
- Park x = 3 + cur_col + ime_shift; with shift -2 the park lands on the first cell of the previous wide char, so a JediTerm preedit anchored at cursorX+1 lands on its second cell. This matches the reported half-cover. Expected from the shift value, not a draw bug.
- Shared with Go (not fixed): cursor at end of a row filled exactly to width appends a reverse space one column past inner (input.go View / input_view_row). Rust side() does not clip; Go same.
- Not verified at runtime in JetBrains.

## Test commands (binary: /Users/leekwangseok/PycharmProjects/claude-harness/target/debug/chaude)
- hide=0 shift=-1: CHAUDE_IME_HIDE=0 CHAUDE_IME_SHIFT=-1 /Users/leekwangseok/PycharmProjects/claude-harness/target/debug/chaude
- hide=0 shift=-2: CHAUDE_IME_HIDE=0 CHAUDE_IME_SHIFT=-2 /Users/leekwangseok/PycharmProjects/claude-harness/target/debug/chaude
- hide=1 shift=-1: CHAUDE_IME_HIDE=1 CHAUDE_IME_SHIFT=-1 /Users/leekwangseok/PycharmProjects/claude-harness/target/debug/chaude
- hide=1 shift=-2: CHAUDE_IME_HIDE=1 CHAUDE_IME_SHIFT=-2 /Users/leekwangseok/PycharmProjects/claude-harness/target/debug/chaude
