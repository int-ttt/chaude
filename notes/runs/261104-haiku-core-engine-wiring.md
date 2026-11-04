task: core-engine wiring (Go truth deprecated/src/main/intt/internal/harness/)
branch: rust-port (not pushed)
owned: crates/core/src only

item1: model-switch handoff snapshot
  go_ref: repl.go switchChild (snapshot only when live==nil && resumeID=="" && sess!=nil; keep pending; else makeSnapshot("route "+target) when turns>0); makeSnapshotN handoff turn
  rust: api.rs switch_needs_handoff (pure), start_switch_handoff (dispatch Target::Main holds prompt), finish_handoff -> complete_switch; orch_child no longer overwrites pending snapshot
  test: api::tests::switch_handoff_only_for_fresh_child_without_pending_snapshot -> PASS
  parity stub NOT un-ignored: crates/core/tests/parity_harness_p3_ignored.rs:64-68 is out of owned scope and its body is panic!("not ported"); un-ignoring would fail. Left ignored.
  gaps: Fresh/Resume targets do not run handoff (Go only route switch); send-failure fallback drops old snapshot; idle_deadline ZERO vs Go neverCold; resume with busy child leaves no proc (pre-existing)

item2: pick_child limit
  go_ref: dispatch.go pickChild compares thresholdFor(target); opusThreshold 200_000
  rust: api.rs pick_for uses rotate::threshold_for(model, cfg.rotate_threshold)
  test: pick_child::tests::pick_child_uses_threshold_for_model -> PASS

item3: parking skips busy children
  go_ref: idlesweep.go sweepIdle busy = Tracker.running()>0, expiry requires !busy. EvictLRU does not check busy (kept as Go).
  rust: child_set.rs Parked.busy, expired() filters busy; api.rs park_active sets busy = !agents.is_empty()
  test: child_set::tests::expired_skips_busy_parked_children -> PASS
  doubt: busy is a park-time snapshot (no tracker for parked children); can go stale.

checks:
  cargo test --workspace (HOME=mktemp, CARGO_HOME/RUSTUP_HOME pointed at real caches): exit 0; passed 596, ignored 51, failed 0
  cargo test -p chaude-core --lib: 231 passed, 0 failed
  cargo clippy --workspace --all-targets: exit 0, 0 warnings
  cargo fmt --all --check: exit 0 (after cargo fmt -p chaude-core, changed api.rs only)

env:
  disk ~714Mi free; fresh HOME re-downloads toolchain (~1.5GB) -> used CARGO_HOME/RUSTUP_HOME override.
  temp HOME dirs left in $TMPDIR (cleanup denied by auto-mode classifier; not retried).

commit: crates/core/src/api.rs, child_set.rs, pick_child.rs, role_coexist.rs, notes/runs/261104-haiku-core-wiring-final.md, notes/runs/261104-haiku-core-engine-wiring.md
