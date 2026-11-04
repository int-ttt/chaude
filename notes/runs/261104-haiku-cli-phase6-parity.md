# 261104 haiku cli phase6 parity

scope: crates/cli, plan/rust-parity-checklist.md, notes/runs. No crates/core edits.

## group 1: install_import wiring + --from-claude / --import-dry-run (C5, C6)
- status: done (local commit pending at time of writing)
- changed: crates/cli/src/tui/mod.rs, crates/cli/src/main.rs, plan/rust-parity-checklist.md
- /import: import_view -> show_import (replaces transcript, Go importedMsg) and queues import_install; event loop calls engine.install_import after the key action
- --from-claude ID|latest|PATH (and =form): load_import_arg before TUI; startup import shown and installed (tui and plain)
- --import-dry-run: prints core dry_run_report; exit 0 when reco preserved, 1 otherwise; needs --from-claude (exit 2)
- checks:
  - cargo build -p chaude-cli -> exit 0
  - cargo test -p chaude-cli -> 185 passed, 0 failed
  - cargo clippy -p chaude-cli --no-deps -- -D warnings -> exit 0
  - cargo fmt -p chaude-cli -- --check -> exit 0
  - smoke: chaude --from-claude <fixture> --import-dry-run -> exit 0, turns=1
  - smoke: missing id -> "import failed" exit 1; dry-run without from -> exit 2
- untested: plain-mode startup import with a live engine (no claude child run)

## remaining (Phase 6 queue)
- G4 helper scripts (C12): add chaude-scripts path dep + match arm; coordinator request c0f73d9c3cd2a5fb
- G1 repl subcommand + 26 flags (C2, F3, C13): needs core fields for max-workers, max-auto-rotations, tail-turns, snapshot-chars, show-rotation, no-handoff-validate, max-drain, drain-timeout, rotate-cooldown, no-category-rotate, no-delegation-rule, agents-file, clients-file -> mark "needs core: <field>" where Config lacks them
- G3 clean (C8): path dep crates/clean
- G5 update (C9, C15): crates/update lacks Main / CleanupOld -> needs core or update fn
- G6 differs rows: C11, C14, E5, K5, S02, S08, S23, E3
