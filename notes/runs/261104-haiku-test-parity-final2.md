# test parity final2 (plan/rust-test-parity.md)
status: partial
scope: tests only under crates/core/tests; no edits to crates/core/src or crates/cli/src; no push
done:
- added crates/core/tests/parity_final2.rs::agent_counts (Go TestAgentCounts, history_test.go:87-99), fg/bg via AgentTracker::running() filtered by AgentRec.background
- plan/rust-test-parity.md:154 missing -> covered
- totals: covered 228->229, missing 2->1, sum 320 (229+17+48+1+25)
run:
- cmd: HOME=$(mktemp -d) cargo test -p chaude-core --test parity_final2
- result: 1 passed, 0 failed, 0 ignored
not done (remaining from brief):
- result_model_usage stub (TestIdleSpendRow): record_turn.rs:256 is the only writer, TurnSpend has no field
- main spawn-once, MCP pipe round trip, MCP config write-once, registry wiring, prompter notice text
- engine-level between-turn canUse / tui stage order (parity_engine_final2.rs)
- cli stub file crates/cli/tests/parity_final2_tui.rs (cli is bin-only, so integration tests cannot import it)
- partial rows: 17 unchanged
- ignored design rows (4): not yet re-verified; TestMaxWorkersDefault (dispatch_test.go) listed ignored, but parity_harness_p1.rs::max_workers_default_is_three asserts Ok(3) and needs a run to confirm the row
src changes needed: none found yet
