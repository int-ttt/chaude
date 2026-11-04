# 261104 haiku core engine wiring
task: wire WorkerPool + pick_child/ChildSet into crates/core/src/api.rs (Go truth deprecated/src/main/intt/internal/harness/)
changed: crates/core/src/api.rs
done:
- WorkerPool in State (pool, cur_task); dispatch sets cur_task; on_tool_use calls note_edit for Write/Edit/NotebookEdit when model == haiku and sid non-empty
- ChildSet<ParkedProc> in State (children); orch_child: park_active on model switch (parks under child_key(cwd, model, ""), upserts PARKED row), pick_child -> UseLive (restore_child, own epoch) | Resume (resume_session) | StartNew (restart_child + pending_snapshot = with_reco(build_snapshot("route MODEL")) when switched and recent turns exist)
- epoch_seq monotonic allocator (next_epoch) replaces epoch += 1 in restart_child and resume_session; restored parked child keeps its old epoch
- unit test api::tests::stamp_time_maps_go_zero_to_epoch_and_keeps_instant
checks:
- cargo test -p chaude-core: 227 lib + integration all pass
- HOME=$(mktemp -d) cargo test --workspace: exit 0; passed 591, failed 0, ignored 51 (sum of "test result" lines)
- temp HOME: no .chaude (only .cargo, .rustup created by toolchain); live ~/.chaude untouched
- cargo fmt --all --check: exit 0
- cargo clippy --workspace --all-targets: exit 0, 0 warnings
config test audit: Config::default() in crates/core tests always overrides agents_file and clients_file (parity_harness_p2.rs:159, parity_harness_p3.rs:17, engine.rs:165, lib.rs:64 helper). cli/src/repl_args.rs:297 is an args-only test helper (no engine). cli/src/main.rs:101 is production.
not done / remaining:
- parity_harness_p3_ignored go_rotate_keeps_pending_snapshot still ignored stub (no rotate-with-pending-snapshot test ported)
- no new dispatch/Router unit test; orch.rs route_f_mode_new_resume_rotate already covers Router
- Go makeSnapshot runs an LLM handoff turn; Rust uses build_snapshot directly (deviation)
doubts:
- pick_child limit uses cfg.rotate_threshold; Go limit source not verified
- park_active does not check busy; a parked busy child's late lines are dropped by epoch
- resume branch: resume_session refuses while busy, leaving no proc (pre-existing behavior)
