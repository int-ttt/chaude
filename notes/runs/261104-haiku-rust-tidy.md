# 261104 haiku rust-tidy run

agent: haiku (rust-port tidy)
branch: rust-port
status: done

## item 1 (done earlier)
- crates/update/tests/parity_update.rs: rustfmt --edition 2024. check: cargo fmt -p chaude-update -- --check -> exit 0
- crates/cli/src/main.rs: mod tests moved to end of file (clippy::items_after_test_module). move-only.
  check: cargo clippy -p chaude-cli --all-targets -> 0 warnings, 0 errors (exit 0)

## item 2 (done)
- plan/rust-test-parity-p2.md: 16 former unverified rows checked against Go bodies.
  covered 5, partial 10, missing 1, unverified 0.
- partial/missing stubs not written here (crates/core owned by another coder). follow-up.

## item 3 (done)
- plan/rust-test-parity.md: rewritten as one deduped table, 320 rows (Go test | Go file | Rust test | status | note).
- totals: covered 228, partial 17, ignored 48, missing 2, n-a 25. sum 320.
- ignored by reason: core seam/API not wired 14, TUI behaviour not in Rust 23, no Rust test yet 2,
  Rust differs by design 4, bin-only/cli-private helper 5. sum 48.
- orphans added: TestRewindState covered (crates/cli/src/tui/rewind.rs::rewind_state_follows_go);
  TestAgentCounts missing.
- update rows: 16 covered (p3, crates/update/tests/parity_update.rs).
- pointer line added to plan/rust-test-parity-p1.md, -p2.md, -p3.md. p-files kept.
- merge bug fixed: status index (was counting note column); multiproc n-a status/note swapped, fixed.

## not done / doubts
- partial 17 and missing 2 lack ignored stubs in crates/core (follow-up for crates/core owner).
- ignored reason grouping is keyword-based (/tmp/classify.py); coarse, verify if used for release notes.
- n-a 25 include 24 Go parked-child pool tests (p2) and 1 live usage test (HARNESS_LIVE_USAGE=1).

## out of scope (not staged)
- crates/core/src/api.rs (modified by another agent), .claude/hooks/main-thread-guard.py,
  notes/runs/261104-haiku-cli-phase6-parity.md, notes/runs/261104-haiku-mod-parity.md, plan/caveman-structure.md
