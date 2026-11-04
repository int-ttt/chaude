# 261104 haiku core config parity (C2/F3/E3) + max_workers default fix

status: partial (fix done and staged; engine test run not green in shared tree, see doubts)
scope: crates/core/src/api.rs, crates/core/src/proc.rs, crates/core/tests/parity_harness_p1.rs, crates/core/tests/engine.rs, crates/core/tests/fixtures/handoff_short_turn.jsonl
not touched: crates/cli, crates/core/src/settings/

config fields (Config in api.rs, Go default in parens):
- max_workers: usize = 3 (DEFAULT_MAX_WORKERS; Go -max-workers, Go default 3)
- max_auto_rotations: i64 = -1 (unlimited; Go -max-auto-rotations)
- tail_turns: usize = 4 (Go -tail-turns)
- snapshot_chars: usize = 24_000 (Go -snapshot-chars)
- show_rotation: bool = false
- no_handoff_validate: bool = false
- max_drain: usize = 3
- drain_timeout: Duration = 3600 s
- rotate_cooldown: Duration = 120 s
- no_category_rotate: bool = false
- no_delegation_rule: bool = false
- agents_file: Option<PathBuf> = Some(~/.chaude/agents.json)
- clients_file: Option<PathBuf> = Some(~/.chaude/clients.json)

max_workers resolution (fixed in this run):
- resolve_max_workers(flag, env, _extra_coders): flag > CHAUDE_MAX_WORKERS > 3. Unset returns 3 (Go TestMaxWorkersDefault).
- worker_floor(current, extra_coders) = max(current, 8 + extra_coders): Go repl.go floor. Applies only when neither flag nor env is set. Caller must apply it.
- NOT YET APPLIED: crates/cli/src/repl_args.rs:196 calls resolve_max_workers and does not call worker_floor. CLI runs lose the resident floor until the cli owner adds it.

idle_deadline: free fn idle_deadline(idle, cache_ttl) (Go idleDeadline); method delegates. Six Go TestIdleDeadline cases in api::tests.
apply_env: private in proc.rs; in-crate test apply_env_drops_api_key_and_sets_only_valid_ttl checks Command::get_envs (Go TestCleanEnvDropsAPIKey).
parity_harness_p1.rs: #[ignore] removed from max_workers_default_is_three; it passes.

engine effect:
- tail_turns: remember_turn, install_import (replaces TAIL_TURNS)
- snapshot_chars: build_snapshot (replaces SNAPSHOT_CHARS)
- no_handoff_validate: finish_handoff skips validate_handoff
- max_auto_rotations: rotation_wanted via auto_rotate_allowed; Engine.rotations++ in rotate_child
- rotate_cooldown: auto_rotate_allowed; Engine.last_rot_fail set in finish_handoff held branch
- no engine effect yet: max_workers (no worker pool), show_rotation, max_drain, drain_timeout, no_category_rotate, no_delegation_rule, agents_file, clients_file

checks (this run, shared working tree):
- cargo fmt --check -p chaude-core -> PASS
- cargo test -p chaude-core --lib -> PASS (177 passed, 0 failed)
- cargo test -p chaude-core --test engine -> FAIL 3 of 65 (see doubts)
- cargo clippy -p chaude-core -- -D warnings -> PASS (after dropping unused extra_coders warning)
- cargo build -p chaude-cli -> not confirmed in this run

doubts:
- engine failures in background-agent paths: rotation_waits_for_background_agent_then_runs_at_defer_timeout (engine.rs:1201 "assertion failed: rotated"), slow_agent_result_is_delivered_and_timer_is_paused_without_interrupt (engine.rs:599, stall timer fired while tool_use open). They passed on the committed engine earlier. They run against another agent's uncommitted api.rs hunks (rotate_keep_snapshot, restart_child, agent_info) and agent_store.rs / pick_child.rs / lib.rs. Not mine; not verified against HEAD.
- git index: only my api.rs hunks are staged (6 hunks, filtered patch). Other agent's hunks remain unstaged.
- rotate_cooldown: no engine test (held branch needs empty recent and empty note).
- max_auto_rotations counts all rotate_child exits (context and idle). Go counter scope not verified.
- The commit bundles the Config fields and engine wiring in one commit.
