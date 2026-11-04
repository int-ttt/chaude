# 261104 haiku rust-test-parity-p2 (group 2 of 3)
status: partial
commits: pending (plan/rust-test-parity-p2.md, this file)
changed: plan/rust-test-parity-p2.md (new), notes/runs/261104-haiku-rust-test-parity-p2.md (new)
check: none run (no Rust test added or changed; rows are mapping only)
group2: 108 Go tests (harness files 23-41); record_test 14 pre-mapped; unmapped 94
tally: covered 29, unverified 16, missing 24, diff 1, n-a 24 (sum 94)
done:
- split header written (groups 1-3 counts)
- rows mapped for all 94 unmapped group-2 tests
remaining:
- body-check 16 unverified rows
- write crates/core/tests/parity_harness_p2.rs for 24 missing rows (copy engine.rs helpers)
- ignore-marked test for TestResidentCoderBusyRefuses (diff)
- group 3 update rows: mark "wait: update"
risk: covered rows are name-level except where "body read" is noted; no test run, so no Rust behaviour verified in this pass
