# haiku-coder: usage_poll + turn_spend (Go usagepoll.go, turnspend.go port)
date: 261104
status: done
changed:
- crates/core/src/usage_poll.rs (new): parse_usage, usage_row, log_usage, USAGE_POLL_EVERY_SECS
- crates/core/src/turn_spend.rs (new, written earlier): TurnSpend, TurnCost
- crates/core/src/lib.rs (M, shared with other agents): added `pub mod turn_spend;` `pub mod usage_poll;` after `pub mod settings;`
check: cd crates/core && cargo test --lib -- turn_spend usage_poll -> PASS (10 passed, 0 failed, 198 filtered)
doubts:
- parse_usage uses hand-written matcher (no regex dep). Go regex edge cases (e.g. "1." or unicode whitespace) not byte-verified against Go.
- USAGE_POLL_EVERY_SECS is only a const; throttle lives in caller (not wired).
- git diff --stat does not cover untracked new files.
