# 261104 haiku cli phase6 parity

scope: crates/cli, plan/rust-parity-checklist.md, notes/runs. No crates/core edits.

## commits (local, rust-port)
- 3ea00e8 feat(cli): /import and --from-claude install into the next prompt; --import-dry-run (C5, C6)
- 63993fb feat(cli): wire the 7 helper scripts into chaude (C12)
- 7520b12 build(cli): lock the chaude-scripts dependency edge
- c08b23e feat(cli): wire clean subcommand to chaude-clean (C8)
- a2b6b34 feat(cli): Go help text (C11) and leading -flag routes to the repl path (C13)

## done rows
- C5 --from-claude: load_import_arg before TUI; startup import shown and installed (tui and plain)
- C6 --import-dry-run: core dry_run_report; exit 0 when reco preserved, 1 otherwise; without --from-claude exit 2
- /import: show_import replaces transcript (Go importedMsg); import_install taken in event loop -> engine.install_import
- C8 clean: chaude_clean::main
- C12 helper scripts: chaude_scripts::has/run arm (smoke: chaude charcount FILE exit 0)
- C11 help: Go usage verbatim (test usage_lists_the_helper_scripts)
- C13 leading -flag: routed to run_tui like Go default branch (routing only)

## checks (last full run, after a2b6b34 edits)
- cargo build -p chaude-cli -> exit 0
- cargo test -p chaude-cli -> 186 passed, 0 failed
- cargo clippy -p chaude-cli --no-deps -- -D warnings -> exit 0
- cargo fmt -p chaude-cli -- --check -> exit 0
- smoke: --from-claude fixture --import-dry-run -> exit 0 (turns=1); missing id -> exit 1; dry-run w/o from -> exit 2
- smoke: chaude clean <empty dir> -> exit 0; chaude -x --no-such < /dev/null -> exit 0 (routes to plain run_tui)

## untested
- plain-mode startup import with a live engine (no claude child run)
- C13 unknown-flag rejection (Go repl flag errors): needs F3 parser

## remaining
- C2 + F3 repl subcommand and 26 flags: many need core Config fields that do not exist (max-workers, max-auto-rotations, tail-turns, snapshot-chars, show-rotation, no-handoff-validate, max-drain, drain-timeout, rotate-cooldown, no-category-rotate, no-delegation-rule, agents-file, clients-file). needs core: Config fields.
- E3 CHAUDE_MAX_WORKERS: read on repl path; Config has no max_workers. needs core: Config.max_workers (dispatch.go:15-26 resolveMaxWorkers).
- C9 update + C15 CleanupOld: crates/update has http/archive/fsutil only; Go update.go is 625 lines (CleanupOld at :199, Main at :292). Not started. needs port into crates/update or core.
- C14 unknown command text: Go %q quoting, same pending decision as resume quote.
- Not touched (other coder): E4, E5, S23, K5, K12, tui/render.rs, tui/commands.rs, /update.
