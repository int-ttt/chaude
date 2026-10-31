# review: 261030-plan-usage-fetch.md
verdict: approach C holds; 5 improvements, apply during implementation.
1 test isolation (risk: real `claude` spawned by any test driving the turn loop): add in usagepoll_test.go `func init() { os.Setenv("HARNESS_USAGE_POLL", "0") }`; pollUsageSync must NOT check the env (only pollUsage does); TestUsageLive calls usageRaw directly.
2 concurrency: pollUsage goroutine calls r.metrics.Log concurrently with the turn loop. Verify Metrics.Log (metrics.go L22) holds a mutex; if not, add `mu sync.Mutex` lock around the write. Run `go test -race ./internal/harness/ -run 'Usage|SetRate' -count=1` as extra check.
3 timestamps: confirm whether Metrics.Log adds a time field (ts/time). If none, add `"ts": time.Now().Format(time.RFC3339)` to the usage row only (not changing other rows); usage_report.py reads ts first.
4 rate_raw diagnostic must log keys only (no values) to keep rows small and avoid leaking account data.
5 ordering: Go steps 1-6 first and pass tests; then python step 7; then live check (HARNESS_LIVE_USAGE=1) last, single run, to keep spawn count low.
