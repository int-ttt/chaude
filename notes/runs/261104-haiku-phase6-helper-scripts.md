# 261104 phase6 helper scripts + checklist risk items (agent: haiku)
task: phase 6 parity, 7 helper scripts (checklist row C12) to Rust; verify E3/E4/F3 refs and Go test count
status: done
done:
- crates/scripts already ported (7 modules, tracked since f970607); verified not wired into crates/cli
- added crates/scripts/tests/smoke.rs (7 tests: registry, -h exit 0 for all 7, usage errors, compress-archive, extract-transcript, usage-report)
- cargo test -p chaude-scripts --offline: unit 3 pass, charcount 5 pass, smoke 7 pass, doctests 0
- cargo fmt -p chaude-scripts -- --check: clean; cargo clippy -p chaude-scripts --all-targets -D warnings: clean
- checklist: C12 note (ported, hookup pending), E3 refs repl.go:1305/1345/1350 + dispatch.go:16, E4 = CHAUDE_IME_SHIFT tui.go:173-178 (+183), F3 = 28 Var calls minus model/orch = 26
- Go test count: 320 (grep ^func Test deprecated, 68 files); plan/rust-port.md:52 312 -> 320
- totals unchanged: 63 rows, done 39, differs 7, missing 17
hookups needed (not applied; crates/cli owned by other coder):
- crates/cli/Cargo.toml [dependencies]: chaude-scripts = { path = "../scripts" }
- crates/cli/src/main.rs match before `Some(cmd) =>`: Some(cmd) if chaude_scripts::has(cmd) => chaude_scripts::run(cmd, &args[1..]),
- optional: USAGE text add chaude_scripts::NAMES
doubts:
- checklist has 1 row (C12) for 7 scripts, not 7 rows; kept as one row
- .claude/worktrees/agent-ad1868a6b951cd8e3 notes/docs describe CHAUDE_MAX_WORKERS as coder pool cap 1..5 default 5, which conflicts with deprecated repl.go (default DefaultMaxWorkers); not touched
- no CLI smoke run of the 7 commands via binary (not wired)
