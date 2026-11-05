# 261105 haiku tui-cursor-jediterm-defaults
status: done
workstream: tui-cursor, follows 261105-haiku-tui-cursor-ime-hide-toggle.md and 261105-haiku-tui-cursor-steady-blink.md
user_verified: JetBrains, Hangul position correct with CHAUDE_IME_SHIFT=-1 and CHAUDE_IME_HIDE=0 or 1 alike.
changed: crates/cli/src/tui/mod.rs
  - hide_cursor_for(hide) no longer reads TERMINAL_EMULATOR. Default false (shown, Go parity). Only "1" hides.
  - draw(), cursor(), App fields: comments updated. hide_cursor field default already false.
  - test hide_cursor_default_is_jediterm_only_and_env_overrides_it -> hide_cursor_default_is_shown_and_env_overrides_it
  - test draw_hides_the_real_cursor_under_jediterm_and_still_parks_it -> draw_hides_the_real_cursor_when_hidden_and_still_parks_it
  - ime_col_shift unchanged: default -1 under TERMINAL_EMULATOR=JetBrains-JediTerm, CHAUDE_IME_SHIFT overrides. Confirmed.
  - CursorShape (DECSCUSR steady block on start, default on exit) unchanged.

## Defaults now
- CHAUDE_IME_SHIFT: -1 under JediTerm, 0 elsewhere; env override kept.
- CHAUDE_IME_HIDE: unset -> shown (0); "1" -> hidden; "0" or other -> shown; env override kept.
- Cursor shape: steady block (DECSCUSR 2) while TUI runs, terminal default (0) on exit.

## Known limitation (not fixed)
- Cursor invisible during Hangul composition in JetBrains JediTerm. Go shows the same. JediTerm limitation, leave as is.

## Checks
- cargo fmt --all -- --check -> PASS (exit 0)
- cargo clippy -p chaude-cli --all-targets -- -D warnings -> PASS (exit 0)
- cargo test -p chaude-cli -> PASS (265 lib passed, 1 ignored; 4 bin passed)
- cargo build -p chaude-cli -> PASS; target/debug/chaude rebuilt 19:06

## Doubts
- Steady-cursor effect in JediTerm not re-verified after this change (only the hide default changed).

## Run
- cd /Users/leekwangseok/PycharmProjects/claude-harness && ./target/debug/chaude   (defaults: shift -1 under JediTerm, shown cursor, steady block)
- CHAUDE_IME_HIDE=1 ./target/debug/chaude   (hide override)
