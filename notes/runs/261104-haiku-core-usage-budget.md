# haiku-coder: core usage/budget remaining list
date: 261104
status: partial
done:
- usage_poll.rs: parse_usage (Go regex semantics, incl. first-occurrence + bad-number fail), usage_row, log_usage, USAGE_POLL_EVERY_SECS
- usage_poll.rs tests: Go TestParseUsage, TestPollUsageLogsRow, TestPollUsageError, regex edges (parse_usage_regex_edges)
- turn_spend.rs: TurnSpend, TurnCost (Go turnspend.go, turnCostUSD) + Go-mirrored tests
- lib.rs: pub mod turn_spend; pub mod usage_poll; (committed hunk only; other agent's haiku_pool reorder left unstaged)
check: cd crates/core && cargo test --lib -- turn_spend usage_poll -> 11 passed, 0 failed
remaining:
- proc.rs: observed TTL + clean env (not started)
- budget: not started
- agent store tracker: not started
- RoleCoexist: not started
- api.rs hooks incl. usage poll throttle (usagePollEvery/usagePolling wiring): not listed, not started
doubts:
- USAGE_POLL_EVERY_SECS is const only; throttle not wired into api.rs
- Go regex `\s` set assumed ASCII whitespace (skip_ws); unicode whitespace not covered
