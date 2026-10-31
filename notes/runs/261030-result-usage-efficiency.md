# result: unverified items + usage efficiency - 261030
a_rate_limit_event: verified. sample notes/runs/261030-rate-limit-event-sample.jsonl; path rate_limit_info.unifiedWindows.five_hour.utilization (0.24) matches fiveHourUtil; also seven_day.utilization, rateLimitType=five_hour
b_usage_rows: verified. live bin/chaude --auto-allow runs wrote stage=rate_raw, rate (0.25, 0.29), usage (session_pct 25 then 30, week 28)
c_13.78: not reconcilable exactly. sessions overlap 34/34 (no coverage gap); 31 sessions: metrics per-session-max 40.84 lies between transcript main-only 11.34 and main+sub 82.89 (assumed prices); transcript side is token x assumed price table, harness side is CLI total_cost_usd; naive cumulative sum 467.18 vs max 43.34 confirms cumulative cost_usd. live turn: cost_usd 0.0165 vs main cache_creation 27457 tok -> price/coverage basis differs; open
d_window_0: not reproduced. --since/--until 2026-10-29: 574 rows; 10-29T00:00..T12:00: 242 rows
e_pyright_t: fixed scripts/usage_report.py lines 182,184 (t -> _)
r7: internal/harness/turnspend.go(+_test.go), repl.go 4 insertions; live turn row has sub_msgs 3, sub_cache_creation_input_tokens 15352, spawns 1, spawn_types {haiku-scout:1}
checks: gofmt -l empty; go vet ok; go test ./internal/harness/... -count=1 ok; TurnSpend 4/4
gap: background turn row (repl.go ~411) has no sub_/spawns keys
usage_5h: 21 -> 30 (poll 01:15Z); 40 not reached; stopped to avoid spending usage for the number
review: sonnet-reviewer blocked by hook (v0.8.2 sonnet/opus subagents may only call haiku-*); haiku-checker used
plan: notes/runs/261030-plan-usage-efficiency.md ; review: notes/runs/261030-plan-usage-efficiency-review.md
