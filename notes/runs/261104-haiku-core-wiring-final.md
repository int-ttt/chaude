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
