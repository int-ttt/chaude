# 261104 haiku core config parity (C2/F3/E3)

status: done (commit bed8233, local, not pushed)
scope: crates/core/src/api.rs, crates/core/tests/engine.rs, crates/core/tests/fixtures/handoff_short_turn.jsonl
not touched: crates/cli, crates/core/src/settings/

config fields (Config in api.rs, Go default in parens):
- max_workers: usize = 3 (DEFAULT_MAX_WORKERS; Go -max-workers). Resolver: resolve_max_workers(Option<usize>, &str env, extra_coders) -> Result<usize,String>
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

engine effect:
- tail_turns: remember_turn, install_import (replaces TAIL_TURNS)
- snapshot_chars: build_snapshot (replaces SNAPSHOT_CHARS)
- no_handoff_validate: finish_handoff skips validate_handoff (problems empty, valid, no retry)
- max_auto_rotations: rotation_wanted via auto_rotate_allowed; counter Engine.rotations++ in rotate_child
- rotate_cooldown: auto_rotate_allowed; Engine.last_rot_fail set in finish_handoff held branch
- no engine effect yet: max_workers (no worker pool; resolver only), show_rotation (TUI always renders Rotated), max_drain, drain_timeout, no_category_rotate, no_delegation_rule, agents_file, clients_file

tests:
- api::tests::max_workers_* (3) PASS
- engine: max_auto_rotations_zero_blocks_context_rotation PASS
- engine: no_handoff_validate_accepts_short_note_without_retry PASS (fixture handoff_short_turn.jsonl)
- engine: tail_turns_limits_imported_turns_in_snapshot PASS
- engine: snapshot_chars_clips_the_snapshot PASS
- rotate_cooldown: no engine test. held branch needs empty recent and empty note, not reachable in normal flow. doubt.

checks:
- cargo fmt --check -p chaude-core -> PASS
- cargo test -p chaude-core -> PASS (160 lib, 65 engine, 3 other; 0 failed)
- cargo clippy -p chaude-core -- -D warnings -> PASS
- cargo clippy -p chaude-core --all-targets -- -D warnings -> FAIL, pre-existing: crates/core/src/record_turn.rs:261 field_reassign_with_default (not touched here)
- cargo build -p chaude-cli -> PASS (1 warning, pre-existing: crates/cli/src/tui/render.rs:895 linkify unused)

doubts:
- max_auto_rotations counts all rotate_child exits (ctx and idle). Go counter scope not verified against repl.go.
- resolve_max_workers floor = 8 + extra_coders applies only when flag and env are both unset (Go repl.go:1341-1353).
- The commit bundles the Config fields and engine wiring in one commit (api.rs cannot be split without interactive staging).
- Untracked crates/core/tests/parity_harness_p1.rs and p3.rs and crates/update/* are not mine; not committed.
