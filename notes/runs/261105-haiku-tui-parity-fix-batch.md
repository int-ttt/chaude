# 261105 haiku tui-parity fix batch (E08 E20 done; E05 E21 E24 open)

status: partial
branch: rust-port (not pushed)
truth: deprecated/src/main/intt/internal/harness/

## item results
item1 test-isolated.sh: present (c746bb6). E25 files (d890a8d) committed and clean.
item2 E08: done, 6aa7ab5. plain() sets Config.no_ui; idle Ask denied with Go note. Go truth repl.go:572-600, 431. Stale "no plain mode" note corrected in notes/runs/261105-haiku-tui-parity-e08.md.
item3 E20: done, 028dc58. api.rs persist_agent on AgentStart, resume, finish, abandoned turn; interrupt_running(true) at actor exit. Test running_agent_is_stored_interrupted_on_shutdown (engine.rs). Plan row [x]. Go agentstore.go:94-96, 230-239; repl.go:1208.
item3 E05: open, not started. Needs new UiEvent for sub-agent assistant text (api.rs drops parent_tool_use_id events ~2008) and TUI feed to agentview. Go agentview.go:19, repl.go:426/686, livechild.go:33.
item3 E21: open, not started. Idle child-initiated turn (Go repl.go:478 call, 518 def, livechild.go:97). Rust idle_turn (api.rs ~579) exists; no idle turn record emit found. Scope undecided.
item3 E24: open, not started. Needs engine startup load("") (AgentStore::load, agent_store.rs:251), a UiEvent carrying restored agents, TUI restored view and dismiss. Go repl.go:398, 1081; agentview.go:326, 346.
extra: da6fc9a fixes stale test slash_mode_and_model_are_not_sent_to_the_engine_as_text (cli, mod.rs:3825) to the C07 behavior (/model NAME is submitted as the full line). The test was failing in the acceptance run.

## acceptance
disk: / 1.2Gi free at final check (no ENOSPC, no cleanup done)
cargo fmt --all --check -> PASS
cargo clippy --workspace --all-targets -> 0 warning/error lines
scripts/test-isolated.sh --no-fail-fast -> 0 failed in every test binary
totals: 642 passed (sum of "test result" lines, counted by hand), ignored 64 (counted by hand)
note: doc tests not checked in the grep summary.

## commits
6aa7ab5 fix(tui): plain mode denies an idle permission ask with the Go note (E08)
028dc58 fix(tui): running agents are stored interrupted on exit (E20)
da6fc9a test(tui): /model NAME expectation follows C07 (sent to the engine)

## open risks
- pre-existing: plain mode mid-turn can_use_tool emits PermissionRequest nothing answers (possible hang). Not fixed.
- E08: idle note prints at the next input line (plain() drains events only inside submit loop).
- E20: only foreground and abandoned-turn paths persist done; background agents persist running until exit.
