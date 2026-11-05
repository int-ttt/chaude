# 261105 haiku tui-cursor-park-parity
status: partial (parity fix done; composing +1 cell symptom NOT reproduced or proven here)
workstream: tui-cursor, follows 261105-haiku-tui-cursor-ime-park-audit.md and 261105-tui-cursor-jediterm-double-cursor.md
repo: /Users/leekwangseok/PycharmProjects/claude-harness, branch rust-port

## Done
- (1) park after every write: already equivalent. draw() ends with MoveTo(x,y) on every call (mod.rs ~2335).
  Other stdout writers do not move the cursor: copy_text OSC52 (mod.rs:2050), MouseCellMotion (mod.rs:2434), Clear(All) on resize (mod.rs ~2767). No change.
- (1b) overlay park: Go tui.go:1009 curCol=0 when perm != nil. Rust draw() now skips MoveTo when app.perm.is_some(). commit 78fe256.
  Go also checks !ready; Rust has no ready flag (size always set at start). Not ported.
- (2) shift math vs Go: identical.
  Go tui.go:1018 curCol = max(1, 1+2+col+1+imeShift) 1-based. Rust mod.rs cursor() x = 1+pw+cur_col+ime_shift 0-based, pw = width("> ") = 2. Same cell.
  Empty input: both give text-start cell (0-based 3) plus shift.
  Wide Hangul: Go cells() and Rust UnicodeWidthChar both count 2 cells. Reverse cell drawn at split_cells(row, cur_col) (mod.rs ~1949), same column.
- CHAUDE_IME_SHIFT: Go imeColShift tui.go:177, Rust ime_col_shift mod.rs ~2272. Both: trim, parse int, fallback to TERMINAL_EMULATOR=JetBrains-JediTerm -> -1, else 0. Values add to the 0-based parked column.

## Cause analysis (unproven)
- Shift -1 under JediTerm should land preedit on the drawn cell if JediTerm anchors at cursorX+1 (Go comment, not rechecked against JediTerm source).
- Reported +1 offset fits either:
  H-A: TERMINAL_EMULATOR not visible to chaude (shift 0 applied), so preedit at cursorX+1. Fix: CHAUDE_IME_SHIFT=-1.
  H-B: shift -1 applied, JediTerm anchor is cursorX+2 while the cursor is hidden. Fix: CHAUDE_IME_SHIFT=-2.
- Cannot tell A from B without the JetBrains runtime.

## Checks
- cargo test -p chaude-cli --lib draw_ -> PASS (9 passed, incl. new draw_does_not_park_the_cursor_under_a_permission_prompt)
- cargo fmt --all -- --check -> PASS (only my new assert was off-format; applied rustfmt)
- cargo clippy -p chaude-cli --all-targets -- -D warnings -> PASS (exit 0)
- cargo test -p chaude-cli -> PASS (262 lib, 4 bin, exit 0)
- cargo build -p chaude-cli -> PASS; target/debug/chaude rebuilt 2026-11-05 18:38
- New test not run against pre-fix code (no revert check).

## Changed
- crates/cli/src/tui/mod.rs (commit 78fe256, 21 insertions, 2 deletions)

## User trace (JetBrains terminal, no code change needed)
1. echo $TERMINAL_EMULATOR inside the chaude terminal.
   - empty: run CHAUDE_IME_SHIFT=-1 /Users/leekwangseok/PycharmProjects/claude-harness/target/debug/chaude
   - JetBrains-JediTerm: run CHAUDE_IME_SHIFT=-2 /Users/leekwangseok/PycharmProjects/claude-harness/target/debug/chaude
2. Compose a Hangul syllable without committing; check it sits on the reverse cell.
3. CHAUDE_IME_SHIFT=0 disables the shift for comparison.

## Remaining
- Runtime verification in JetBrains (not available here).
