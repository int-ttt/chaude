# 261105 haiku loop5 items 1-3

status: partial (items 1-2 committed, item 3 mutation-only, checks green)
head: 8920f95

## item 1: env record dir -> Config.record_dir
- commit: c139a3a test(core)
- api.rs: Config.record_dir (Option<PathBuf>, default None); Engine::record_root uses it, else record::dir_for(cwd); used by clear_mark and record_turn_save.
- removed unsafe set_var HARNESS_RECORD_DIR in: api.rs test parked_non_coder_can_use_tool_is_denied_on_prompter_and_tui; tests/engine.rs fake(); tests/parity_harness_p2.rs fake(); tests/parity_final3_resident.rs fake_claude (Once) + engine_cfg.
- narrow: parked test 1 passed; engine 97 passed; parity_harness_p2 2 passed; parity_final3_resident 11 passed.
- NOT converted (remaining env mutation in tests):
  - tests/engine.rs:297-302 CHAUDE_ORCH set/remove. Prod reads env directly: api.rs:998 orch::Router::new(env), api.rs:3868 main_rule env. Needs Config.orch_mode plus CLI plumbing.
  - tests/engine.rs:2210-2239 HOME set/restore. Prod: osenv::home_dir (osenv.rs:72), resident.rs:55 env HOME. Needs home threading.
  - crates/cli/src/main.rs:33 set_var CHAUDE_ORCH is production, not a test; left alone.

## item 2: reap_drains seam
- commit: 8920f95 refactor(core)
- api.rs: new fn finished_drains(&[(epoch, elapsed)], running, timeout) -> Vec<(epoch, timed_out)>; reap_drains calls it with self.agents.len().
- test agent_tracker_drain_half go_finished now calls finished_drains with tr.running().len() (same seam as prod).
- behavior unchanged. DOUBT: prod running count is self.agents (HashMap in State), not agent_store::AgentTracker; AgentTracker is not wired into the engine (only tests use it). Equivalence is by construction, not by a shared object.

## item 3: mutation check idle gate
- no commit (by brief).
- mutation: crates/cli/src/tui/mod.rs tick gate `if self.busy || n != self.last_run` -> `if true || ...`.
- result: idle_tick_gate_skips_unless_busy_or_count_moved FAILED at mod.rs:3548 "idle, count same: skipped" (left "edited", right "task b1").
- restored from /tmp backup; git diff clean for crates/cli.

## checks
- scripts/test-isolated.sh run1: EXIT=0, 709 passed / 0 failed / 3 ignored (28 result blocks)
- scripts/test-isolated.sh run2: EXIT=0, 709 passed / 0 failed / 3 ignored (28 result blocks)
- clippy --workspace --all-targets (api.rs touched): rc=0, 0 warnings/errors
- cargo fmt --all --check: rc=0
- release: cargo build --release -p chaude-cli --target aarch64-apple-darwin: rc=0
  - path: /Users/leekwangseok/PycharmProjects/claude-harness/target/aarch64-apple-darwin/release/chaude
  - size: 4044224 bytes (3.9M)
  - --version: chaude 0.10.2
- no install, no push.

## not done / for main
- notes file uncommitted (matches prior loop5 pattern).
- HOME and CHAUDE_ORCH test env mutation remain (see item 1).
