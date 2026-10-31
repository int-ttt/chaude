# review: 261030-plan-usage-window.md
reviewer: haiku-checker (code/arithmetic verify) + opus-architect; sonnet-reviewer blocked by hook (opus may call haiku-* only)
verdict: conclusions hold (arithmetic PASS: sonnet_sub 1.8464, opus_sub 0.7751, haiku_sub 0.6679, haiku_main 0.1254, total 3.4148); steps 1,2,4 need edits

## improvements
- step1 fix: budget.go:28-29 charge() returns before delta logic when cap<=0 (default) -> last never updated. Move delta tracking into a new func costDelta(s *Session, total float64) float64 on Runner (map[*Session]float64), called regardless of cap; budget.charge uses it. Do not "reuse budget.go:33-38" as-is.
- step1 key: key by *Session (process), not session id string. resume reuses session id (classifier.go:292, switchchild.go:212); per-id keying would double-subtract. Plan text "last[session]" -> "last[*Session]".
- step2: no Go code parses modelUsage; shape known only from samples/probe*.jsonl (costUSD, inputTokens, cacheCreationEphemeral1hInputTokens...). Add: log raw map only if present (type assert map[string]any), no schema dependency; add fixture-based test from samples/probe1.jsonl.
- step4: setRate (repl.go:200-203) has no dedupe; rate_limit_event frequency unmeasured. Add compare of five_hour.utilization+resetsAt with previous value under existing lock; log only on change. Also log stage=rate from background path repl.go:539-540.
- step order: 4 (rate logging) first - lowest risk, gives the %-vs-spend time series needed to confirm the root cause; then 1, 3, 2, 5; 6 last after 1 day of data.
- tests: metrics_test.go has TestMetricsLog/TestMetricsNilAndDisabled/TestUsageFields; none covers metricscmd summing. Add TestMetricsCmdDeltaSum (old rows without cost_delta_usd -> per-process max fallback; note old rows lack process identity -> fallback per session id max, document as approximate).
- facts fix: repl.go:487/489 are inside stream case body; isMain filter defined at repl.go:474 only.
- add step 7 (doc): notes/FACTS.md line: "metrics cost_usd = CLI total_cost_usd 프로세스 누적, 합산 금지; 하위 에이전트 비용 포함". Agent: haiku-batch-worker.
- risk: total_cost_usd is client-side estimate (billing.md #4); 5h % is server value. Acceptance must compare trends, not equality.
- risk: subagent cold-start ~22k cc/spawn is an average incl. multi-turn subagents; verify per-spawn first-message cc before trimming agent prompts (step 6).
