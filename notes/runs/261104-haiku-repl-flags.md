# 261104 haiku repl flags (C2, F3, E3)

task: wire `chaude repl [flags]` (C2), all Go repl flags (F3), CHAUDE_MAX_WORKERS (E3) in crates/cli.
status: done (local commits on rust-port); verified on clean HEAD export only (see checks).
go_ref: deprecated/src/main/intt/internal/harness/repl.go:1279-1306 (flag set), 1316-1390 (Main), 1339-1353 (max workers floor)
core_dep: bed8233 Config fields + resolve_max_workers (committed; read-only for this task)

changed:
- crates/cli/src/repl_args.rs (new): Go-style interspersed parser, FLAGS table of 28 flags, apply() to Config, Go duration parser, unit tests per flag group
- crates/cli/src/main.rs: `mod repl_args`; `Some("repl") => run_tui(&args[1..])`; run_tui uses parser; E3 reads CHAUDE_MAX_WORKERS; apply_orch takes parsed value; parse_model/from_claude_arg removed with their tests
- repl_args.rs: accessor renamed from_claude() -> claude_source() (clippy wrong_self_convention)
- plan/rust-parity-checklist.md: C2, F3, E3 -> done; totals line fixed (done 63, differs 0, missing 0)

defaults checked against Go repl.go:1279-1306: model opus, compact-window 400000, max-auto-rotations -1, tail-turns 4, snapshot-chars 24000, auto-allow false, metrics-dir auto, max-drain 3, drain-timeout 60m, rotate-defer-timeout 10m, rotate-cooldown 2m, cache-ttl 1h, task-budget-usd 0, idle-rotate 3599s, max-workers DefaultMaxWorkers (3), orch m0.
worker floor: unset flag+env -> max(3, 8 + extra_coders) = 12 with Config default extra_coders 4 (Go repl.go:1350; core resolve_max_workers at HEAD).

checks:
- clean HEAD export (git archive HEAD + my 2 cli files, /tmp/chaude-head): cargo test -p chaude-cli -> 200 passed, 0 failed, exit 0
- clean HEAD export: cargo clippy -p chaude-cli --all-targets -- -W clippy::all -> exit 0; one warning crates/cli/src/main.rs:182 "items after a test module" (pre-existing layout, mod tests before fn main at HEAD)
- clean HEAD export: cargo clippy ... -D warnings -> FAIL in crates/core (pick_child.rs:58 collapsible_if, in HEAD); not in crates/cli
- working tree: cargo test -p chaude-cli -> 200 passed, 1 failed: repl_args::tests::defaults_match_go (left 3, right 12). Cause: uncommitted core/src/api.rs edit by another agent drops the resident floor in resolve_max_workers. HEAD core returns 12. Not touched.
- working tree: cargo clippy -D warnings -> FAIL in crates/core (other agents' files: api.rs:320 unused extra_coders, agent_store.rs, category.rs, pick_child.rs). No cli lint hits reported before core compile stopped.
- rustfmt --check repl_args.rs -> 0
- smoke on clean HEAD build (earlier): `repl -h` exit 0; `repl --nope` exit 2; `repl --max-workers=0` exit 2; `repl --cache-ttl=2h` exit 2

doubts:
- defaults_match_go fails on the working tree until the core api.rs edit is resolved (expected 12).
- resolve_model: Go flag default is "opus" (repl.go:1279); Rust keeps the settings main_model when orch m0 and no --model. Left as-is (existing behavior, resolve_model tests).
- idle-notice: Go default defaultIdleNotice not read; Rust uses the K4 30m value.
- -h prints the USAGE block, not Go's per-flag list.
- --cwd now sets cfg.cwd and the cwd used for the from-claude load; before it was always current_dir.
