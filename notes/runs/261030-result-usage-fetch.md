# result: usage-fetch (261030)
plan: notes/runs/261030-plan-usage-fetch.md; review: notes/runs/261030-plan-usage-fetch-review.md
live_usage_read: yes (`claude -p /usage --output-format json`, cost 0, text only). TestUsageLive: session_pct=21 week_pct=28 (resets Oct 30 12:50pm / Nov 1 5pm Asia/Seoul).
changed: internal/harness/usagepoll.go (new: parseUsage, var usageRaw w/ 60s CommandContext timeout, pollUsage async 5min throttle, pollUsageSync, turnCostUSD), usagepoll_test.go (new), ratestatus.go (fiveHourUtil, rateRawRow), ratestatus_test.go (TestSetRateTopLevelFallback; counts stage=rate only), metrics.go (Log mutex; was unguarded), repl.go (setRate uses fiveHourUtil; rate_raw once; pollUsage after fg turn L893 and bg turn L420; cost_turn_usd in both turn rows), scripts/usage_report.py (new, no mtime filter, --since/--until, usd_per_pct).
metrics rows new: stage=usage {session_pct, session_resets, week_pct, week_resets, source:"cli"} or {error}; stage=rate_raw {keys, rate_limit_type} once/process; turn.cost_turn_usd.
checks: gofmt -l empty; go vet exit 0; go test ./internal/harness/... -count=1 ok; go test -race -run 'Usage|SetRate|TurnCost' ok; live test PASS.
review: sonnet-reviewer blocked by hook (opus may call haiku-* only) -> haiku-checker review: 1 must-fix (no timeout) fixed round 2; items 2-8 PASS.
report_run --since 2026-10-30: rows 171; turn per-session-max 6.9277 vs naive 31.6552 USD (cumulative double count ~4.6x); classify 0.5580; usage rows 0 (code not yet run live in harness); rate rows 0.
items: 1 fixed (poll + fallback, awaiting live rows); 2 fixed (cost_turn_usd + max-per-session); 3 fixed (script in repo, window, no mtime); 4 open: naive-sum overcount likely part of 13.78 gap, transcript side not re-measured; 5 fixed by design (no mtime filter, explicit "no rows in window"), not reproduced.
40pct: undefined in notes; current measured 5h usage 21%; 40% not reached/measured.
