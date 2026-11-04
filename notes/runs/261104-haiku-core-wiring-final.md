# 261104 haiku core-wiring final (core coder, fresh)

status: partial
branch: rust-port (not pushed)
scope: crates/core/src only (+ plan/rust-port.md, plan/rust-parity-checklist.md)

## done (commits)
- 58a9313 style(core): rustfmt turn_spend, usage_poll, lib mod order
- 13e16a1 feat(core): background /usage poll after each turn, throttled 5 min
- 8602f15 feat(core): delegation and orch rules in the main child system prompt
- 1381908 feat(core): per-turn subagent spend merged into the turn metrics row
- 5413778 feat(core): /clear writes a resume clear mark in the project record dir
- 0a37d09 feat(core): client registry rows, startup reconcile, /clear ends the row
  - live row on child start (Go switchchild.go:263), mark_ended "clear" on /clear (repl.go:1405)
  - startup reconcile emits OrphanNotice (role "", pid = child_pid, model = row model); never killed
  - session_id on the live row = opts.resume (fresh session has ""; Rust does not record the sid into the row)
- a9c1d6d feat(core): mark agents a crashed harness left running as interrupted at start
  - AgentStore::new(agents_file).interrupt_running(false) (Go repl.go start, agentstore.go:96)
- plan/rust-port.md: K8/K12 marked done (checklist already done); phase 3 wired/open lists
- plan/rust-parity-checklist.md: verified note after a9c1d6d

## not wired (open, not done)
- orch Router route/record: needs multi-child routing (api.rs has single child path)
- haiku_pool WorkerPool note_edit (Go repl.go:743): needs curTask and edit path hook in on_line
- pick_child + ChildSet (Go dispatch.go:91): needs multi-child set
- AgentTracker persistence per agent: AgentTracker not used by api.rs (only the startup sweep is wired)

## checks
- cargo fmt -p chaude-core: clean (core files)
- cargo fmt --all --check: FAIL only in crates/update/tests/parity_update.rs (other coder, not touched)
- cargo build --workspace: PASS
- cargo test -p chaude-core --lib: 227 passed, 0 failed, 0 ignored (baseline 225)
- cargo test --workspace: 590 passed, 0 failed, 50 ignored (sum of all "test result" lines)
- cargo clippy --workspace --all-targets: core 0 warnings; 1 warning crates/cli/src/main.rs "items after a test module" (not mine)
- flaky check, 3x each: rotation_waits_for_background_agent_then_runs_at_defer_timeout PASS x3; slow_agent_result_is_delivered_and_timer_is_paused_without_interrupt PASS x3
  - no test named "session-swap-defer" exists; closest is the defer rotation test above

## risks
- Config::default() has clients_file and agents_file under ~/.chaude; tests in crates/core/tests/engine.rs use Config::default(), so they write real clients.json/agents.json rows (owner = test pid, reconciled on next start)

## 261104 core-wiring pass (core-engine coder)
status: partial
done:
- part1 ~/.chaude isolation: commit 968d17f (engine and parity tests use temp clients/agents files).
- part2 orch Router: commit a2432b7. dispatch routes via orch::Router (mode f only); end_turn records the workspace turn.
  Untagged line: switch to main model via orch_child only when it differs from opts.model. Tagged line: resume or fresh haiku session.
checks:
- cargo fmt -p chaude-core: ran.
- cargo test -p chaude-core: PASS, no FAILED (227 unit, 66 engine, 3, 9 with 2 ignored, 1, 0 with 13 ignored, 9, 9 with 35 ignored, 3, 0).
- cargo clippy -p chaude-core --all-targets: 0 warnings.
- ~/.chaude baseline: clients.json size=7719 mtime=1793767519; agents.json size=180614 mtime=1793767975.
- ~/.chaude now: clients.json unchanged; agents.json size=180611 mtime=1793768375. Live chaude pid 42593 (started ~04:00 before this check) is a real harness; not a test. Core test code has no unisolated Config::default().
- grep Config::default() in crates/core tests and src: 4 struct-update sites, all with explicit agents_file and clients_file (or testutil::isolated).
open:
- WorkerPool note_edit (haiku_pool.rs) not wired in on_tool_use.
- pick_child / ChildSet not wired.
- No unit test yet for dispatch routing. Go snapshot handling on main-model switch unverified.
- No parity_* ignored test un-ignored (p3_ignored 13, p2 2, p12 35 remain ignored).
- cargo test --workspace not rerun in this pass.

## core engine wiring log (261104, see 261104-haiku-core-engine-wiring.md)
- wired WorkerPool note_edit (on_tool_use, haiku only) and ChildSet/pick_child into orch_child (park, UseLive, Resume, StartNew + pending snapshot); monotonic epoch_seq.
- checks: chaude-core tests pass; HOME=$(mktemp -d) workspace tests 591 passed, 0 failed, 51 ignored; no temp .chaude; fmt --check 0; clippy --workspace --all-targets 0 warnings.
- still open: go_rotate_keeps_pending_snapshot stub; no un-ignore done.
