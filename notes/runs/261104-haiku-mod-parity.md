status: partial
scope: crates/cli/src/tui/mod.rs (+ crates/cli/Cargo.toml, Cargo.lock for libc)
commits:
  c06eeb9: permission y/n keys (E06-E08), Shift+Tab mode cycle (I18, C18), Ctrl+D quit (I35)
  6494ec0: status rate window (N01, E10), budget stop notice (E13, B14); libc unix dep
done_ids: E06 E07 E08 I18 C18 I35 N01 E10 E13 B14
gates: fmt --check PASS; clippy --workspace -D warnings PASS; cargo test --workspace PASS (203 passed, 0 failed)
harness (scripts/tui-diff.sh, all 10, after 6494ec0):
  startup-idle 93% | type-text 93% | multiline 93% | submit-stream 66% | tool-use 66%
  slash-menu 93% | scroll-up 40% | esc-interrupt 66% | resize-80x24 58% | exit 93% | usage 96%
  (unchanged vs before group 1; no scenario drives rate/budget/perm events)
partial_blocker_p_t_c: bar.rs hardcodes "P 0/ T 0/ C 0/"; UiEvent has no per-role ctx (Go uses roleCtxSnap). Engine-blocked.
blocker_scroll_resize:
  every Rust run prints "! bad stream line: missing field `type` at line 1 column 2" then "! claude exited"
  at startup (startup-idle.rs.txt), then "! claude is not running" on each send (scroll-up.rs.txt).
  cause: fake claude in scripts/tui-diff.sh exits '{}' for any argv starting "-p ". crates/core/src/proc.rs:50
  spawns the same argv eagerly; Go (deprecated core.go:152, same argv) shows no error, so spawn timing differs.
  owner: crates/core (proc.rs spawn timing) or scripts/tui-diff.sh (fake). Not mine. S02/S16/resize unmeasurable until fixed.
not_started: group 3 viewport fixes (S02 S16), group 4 mouse drag selection + clipboard (S08-S11 B25), group 5 remaining L/I/B/E/X.
doubts:
  - perm note: pushed "allowed"/"denied" plus a blank row; Go appends one block "note". Block spacing not verified vs Go.
  - Interrupted clears App.perm; core also clears pending_perm on interrupt. Matches, unverified in engine test.
  - libc localtime_r used for HH:MM (unix); UTC fallback off unix.
  - mod.rs at HEAD referenced usage.rs (untracked by others). Not my file; reported earlier.
