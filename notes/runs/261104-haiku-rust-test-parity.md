# 261104 haiku rust-test-parity (Phase 6)
status: partial
commits: 505fd8b test(core): parity tests for Go record split caps, mtime numbering, midnight
changed: crates/core/tests/parity_record.rs (new, 3 tests), plan/rust-test-parity.md (new, 52 rows)
check: cargo test -p chaude-core --test parity_record -> 3 passed; 0 failed; 0 ignored
go_rows: 320 total; mapped 52 (covered 36, missing 16, n-a 0); unmapped 268; ignored diffs 0
missing_reason: update 12 + install_path 4 (crates/update port in progress, agent a1284799)
remaining:
- map harness/* (about 230 rows) into plan/rust-test-parity.md
- re-map update/install_path rows when the update port lands
- recount plan/rust-parity-checklist.md vs git log 2f86bb8..HEAD; fix K6-K11, S02, S08, P3 remaining in plan/rust-port.md (not started)
risk: covered rows are name-level for some (TestRecordNumberContinuesAfterRestart, TestMergeGuardRejectsBadJSON); bodies not compared
