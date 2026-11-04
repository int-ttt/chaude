task: Go→Rust test parity, part 1 of 3
date: 261104
status: done
split: test-level, sorted by package dir then file then line; part 1 = positions 1-107 (clean 10 + harness 97)
groups: part 1 = 1-107 (107), part 2 = 108-213 (106), part 3 = 214-320 (107)
rows_written: plan/rust-test-parity-p1.md (107 rows)
totals: covered 58, ignored 1, missing 48, n-a 0
added_tests: crates/core/tests/parity_harness_p1.rs (3 tests)
check: cargo test -p chaude-core --test parity_harness_p1 -> PASS (2 passed, 0 failed, 1 ignored)
ignored: harness/dispatch_test.go::TestMaxWorkersDefault | Go default 3, Rust default max(3, 8 + extra_coders)
top_diffs:
  - Go default workers 3 vs Rust resident floor 8
  - no agent store, child set/park, client registry, cache-TTL registry, category rotation, worker pool, forward rule, pick child in Rust
  - idle_deadline and apply_env are private, no public test seam
  - agent view scroll and slash-completer history paths not in Rust TUI tests
doubts:
  - name-level covered rows (name differs) need a human pass; listed with note "name-level"
  - TestWithRecoRoundTripKorean marked covered by with_reco_round_trip_and_preservation; Korean input not confirmed
  - TestMainStatusRendering mapped to busy_row_sits_in_the_transcript_flow_with_go_text by name only
commits: see git log (no push)
