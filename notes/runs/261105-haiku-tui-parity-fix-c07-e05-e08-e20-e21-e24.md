# 261105 haiku tui-parity fix batch (C07 E05 E08 E20 E21 E24) - blocked, no edits

status: blocked
branch: rust-port (not pushed, nothing committed by this run)
truth: deprecated/src/main/intt/internal/harness/

## blocker
wait-gate: E25 files still dirty after 15+ min polling (60s interval, two loops)
  dirty: crates/core/src/resident.rs crates/core/src/api.rs crates/cli/src/tui/mod.rs crates/core/tests/engine.rs
  HEAD: e42302f (E25 commit not present)
disk: / 1.5Gi free (88%) at start; final2 recorded ENOSPC on cargo test. Risk for workspace test.
action: no source edits made (rule: do not touch the four dirty files before they are clean).

## done (read-only)
Go truth locations:
- interruptRunning (own/shutdown): agentstore.go:94-96; markInterrupted: agentstore.go:230-239 (E20)
- markInterrupted caller on exit: repl.go:1208 (E20)
- startup interrupt of stale 'running': repl.go:1373 (E24 side)
- restored panel: repl.go:398 (prev.sortedRestored), tui.go:1468; sortedRestored agentview.go:326; dismissRestored agentview.go:346; dismiss caller repl.go:1081 (E24)
- onSubEvent wiring: repl.go:426, repl.go:686, livechild.go:33; def agentview.go:19 (E05)
- idleFinish: repl.go:478 (call), repl.go:518 (def), livechild.go:97 (E21)
- idle permission note: repl.go:592 (E08)
- /clear and /rotate: session commands, not turns (Rust api.rs:1010-1020 matches)

Rust state:
- E05: sub_event_lines agentview.rs:118, push_log agentview.rs:171 have only test callers (agentview.rs:420-446).
  api.rs:2008 `if env.parent_tool_use_id.is_some() { return; }` drops all sub-agent Assistant events. Go wires it at 3 sites. Needs new UiEvent path (core + TUI). Not a pure wire-up.
- E08: api.rs:2097-2099 comment says engine always has UI, so Go idle deny note unreachable in Rust. Go note text repl.go:592. Decision needed: port the plain note or keep as unreachable.
- E24: agent_store.rs:780 sorted_restored, :792 dismiss_restored exist with tests (agent_store.rs:895-909). No engine startup restore and no UiEvent carries restored agents. Core-owned.
- E20/E21: api.rs has no agent-row persistence on exit; idle turn record path not found in grep (notice() api.rs:2364 is E25 work).
- C07: api.rs:1010-1020 handles /clear and /rotate only; no model switch path. Needs core rotate-with-model + TUI /model NAME unblock (mod.rs:1007-1011).

## remaining (for next run once gate clears)
1. C07 core rotate-with-model (Go tui.go:1088-1098, repl.go:1417-1420); TUI unblock mod.rs:1007-1011; test in engine.rs.
2. E05 sub-agent events to agent log (Go agentview.go:19, repl.go:426/686, livechild.go:33): needs UiEvent variant; test via fake stream.
3. E08 decision: port Go plain note or document as unreachable (user decision suggested).
4. E20 agent-row persistence on interrupt (agentstore.go:230-239).
5. E21 idle turn record (repl.go:518-560).
6. E24 engine startup restore + UiEvent for restored agents (agentstore.go/repl.go:398).
7. Plan marks: plan/tui-parity.md lines 167, 201, 204, 216, 217, 220 (not yet edited).
acceptance (to run later): HOME temp per brief; clippy --workspace --all-targets 0 warnings; cargo fmt --all --check.
