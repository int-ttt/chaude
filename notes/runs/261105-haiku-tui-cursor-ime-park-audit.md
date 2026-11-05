# 261105 haiku tui-cursor-ime-park-audit
status: no edit (cause not proven from code; hypotheses + trace below)
workstream: tui-cursor, prior a95c2b5 (hide hardware cursor under JediTerm)
repo: /Users/leekwangseok/PycharmProjects/claude-harness, branch rust-port

## Parity check (Go truth vs Rust)
- formula: Go tui.go:1018 curCol = 1+2+col+1+imeShift (1-based). Rust mod.rs:2246-2249 x = 1+pw+cur_col+ime_shift (0-based). Same cell.
- col units: Go input.go:173-181 Cursor() uses cells(). Rust input_rows mod.rs:1830-1860 uses UnicodeWidthChar. Both cell-based, Hangul = 2 cells in both.
- shift -1 for JetBrains-JediTerm: Go tui.go:177-188, Rust mod.rs:2271-2281. Same. Only TERMINAL_EMULATOR env gates it in both.
- park timing: Go cursorWriter tui.go:223-260 parks after EVERY write (moves down by upWas before next write). Rust parks only at end of draw() mod.rs:2302-2325, draw runs on dirty+FRAME or TICK (mod.rs:2700-2790).
- hide: bubbletea v1.3.4 standard_renderer.go:407 hideCursor, tea.go:443. Same as a95c2b5.
- overlay divergence: Go tui.go:1009 curCol=0 when perm != nil or !ready (no park). Rust cursor() always parks, even during permission prompt / overlays.

## Hypotheses (unverified, JediTerm runtime not available here)
H1: JediTerm preedit anchor is not exactly cursorX+1 in all states (Go comment cites TerminalPanel.drawInputMethodUncommitedChars, not re-checked). If anchor differs, preedit lands off by one.
H2: Hidden cursor (DECTCEM off, from a95c2b5) changes where JediTerm anchors preedit. Then "jump" could come from a95c2b5, not from the shift. Test: compare pre-a95c2b5 binary vs current under JediTerm.
H3: Parking under overlays (perm prompt, /view, /usage) where Go does not park. Would show preedit at input row while overlay is up. Symptom mismatch with report, low prior.
H4: TICK/FRAME redraws while busy re-emit MoveTo(x,y) each frame. Same coordinates, so not a jump source by itself.

## Checks run
- cargo test -p chaude-cli --lib ime_shift -> PASS (1 passed: tui::tests::ime_shift_follows_env_override_and_jediterm)

## User trace to capture (JetBrains terminal)
1. echo $TERMINAL_EMULATOR (expect JetBrains-JediTerm).
2. script -q ~/ime-trace.bin <chaude debug binary>; type "a" then Korean "하" (compose, no commit); then Esc or Enter-less exit.
3. Inspect: grep -a -o $'\e\[[0-9;]*[HGABr]\|\e\[?25[lh]' ~/ime-trace.bin | tail -40
   Need last MoveTo (row;col) and ?25l/?25h before and after composition.
4. Repeat with CHAUDE_IME_SHIFT=0 and with CHAUDE_IME_SHIFT=-1 to see which column the preedit lands on.
5. Note: does the offset appear only while busy (spinner), or also idle?

## Next (if user confirms H2)
- Test by toggling hide_cursor only, not the shift. No env switch exists yet.
