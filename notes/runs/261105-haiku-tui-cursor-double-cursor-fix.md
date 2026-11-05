# 261105 haiku tui-cursor-double-cursor-fix
status: done
workstream: tui-ime-cursor, follows 261105-haiku-tui-cursor-jediterm-defaults.md (commit 12c798a)
symptom: JetBrains terminal, no env vars, two cursors shown (drawn block in input row + hardware cursor parked at IME-shifted cell).
cause: 12c798a made CHAUDE_IME_HIDE default shown under JediTerm, so hardware cursor visible next to drawn block.
changed: crates/cli/src/tui/mod.rs
  - hide_cursor_for(hide, emulator): unset -> hidden only under TERMINAL_EMULATOR=JetBrains-JediTerm; "1" hides; "0" shows.
  - tui(): Hide queued once before the first draw when app.hide_cursor (no per-frame toggling).
  - draw(): emits Show only when not hidden; no Hide. Per-frame ?25l removed (before: draw() ended with Hide or Show every frame).
  - tests: hide_cursor_default_is_jediterm_only_and_env_overrides_it; draw_parks_the_real_cursor_without_toggling_visibility_when_hidden (2 draws, no ?25h/?25l).
  - ime_col_shift unchanged (default -1 under JediTerm). CursorShape (DECSCUSR steady block / restore on exit) unchanged.
notes: 261105-haiku-tui-cursor-jediterm-defaults.md default lines updated.
per_frame_toggling_before: yes, draw() ended with Hide (hidden) or Show (shown) on every frame call.
per_frame_toggling_after: none when hidden; shown mode still emits ?25h per frame (idempotent, unchanged).

## Checks
- cargo fmt --all -- --check -> PASS (exit 0)
- cargo clippy -p chaude-cli --all-targets -- -D warnings -> PASS (exit 0)
- cargo test -p chaude-cli -> PASS (265 lib passed, 1 ignored; 4 bin passed; 0 failed)
- cargo build -p chaude-cli -> PASS; target/debug/chaude rebuilt

## Doubts
- Not verified in a live JetBrains terminal by me; needs user check: one cursor, Hangul position with default shift -1.
- Hangul composition cursor invisibility under JediTerm is the known limitation from the earlier note (unchanged).

## Run
- cd /Users/leekwangseok/PycharmProjects/claude-harness && ./target/debug/chaude   (JetBrains: one drawn cursor)
- CHAUDE_IME_HIDE=0 ./target/debug/chaude   (shows hardware cursor too; double cursor expected)
