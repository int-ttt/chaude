# plan: usage-fetch (261030)
goal: record 5h/week usage % from live `/usage` into notes/metrics, fix cost_usd semantics, add repo report script with window mode.
facts (scout 261030):
- live: `claude -p /usage --output-format json` exit 0, total_cost_usd 0, result text has "Current session: 18% used · resets Oct 30 at 12:49pm (Asia/Seoul)" and "Current week (all models): 27% used · resets Nov 1 at 4:59pm (Asia/Seoul)". No structured fields.
- 2026-10-30.jsonl: 0 stage=rate rows. setRate (repl.go L203) reads only unifiedWindows.five_hour.utilization; fmtRateStatus (ratestatus.go L12-27) also falls back to top-level info["utilization"] when rateLimitType in ("","five_hour"). Real event shape unverified.
- turn cost_usd is cumulative per session (67c8cd4e: 0.438->0.963 monotonic). naive sum 3.587 vs per-session max sum 0.972.
- measure.py lives only in /tmp/claude-usage (mtime prefilter L57, no window CLI).
options:
- A passive only (rate_limit_event): free, but never fired so far; shape unknown.
- B active poll of `claude -p /usage` (0 USD, verified here), regex parse, throttled async. Cost: one process spawn per poll.
- C = B + fix A fallback + one-time raw-key diagnostic row. CHOSEN: B is the only verified source; A fix is cheap and removes the setRate/fmtRateStatus mismatch.
## steps
### 1 internal/harness/ratestatus.go
- add `func fiveHourUtil(info map[string]any) (float64, bool)` containing exactly the existing utilization lookup+fallback from fmtRateStatus; fmtRateStatus calls it (behavior unchanged).
### 2 internal/harness/repl.go setRate
- replace the inline `rateNum(asMap(asMap(info["unifiedWindows"])["five_hour"])["utilization"])` with `fiveHourUtil(info)`.
- add field `rawRateLogged bool` (guarded by rateMu); first non-nil info per process logs `{"stage":"rate_raw","keys":<sorted top-level keys []string>,"rate_limit_type":str(info["rateLimitType"])}`. Once only.
### 3 new internal/harness/usagepoll.go
- `var usageRe = regexp.MustCompile(`Current session:\s*([0-9.]+)%\s*used(?:\s*·\s*resets\s*([^\n]+))?`)` and `usageWeekRe` for `Current week \(all models\):\s*([0-9.]+)%\s*used(?:\s*·\s*resets\s*([^\n]+))?`.
- `func parseUsage(text string) (rec map[string]any, ok bool)`: ok if session match; rec keys: session_pct float64, session_resets string (trimmed, omit if empty), week_pct float64 + week_resets (only if week matched).
- `var usageRaw = func() (string, error)`: same exec as usageFetch in tui.go L701-715 (claude -p /usage --output-format json --no-session-persistence, Env cleanEnv(0), Dir os.TempDir()), returns str(j["result"]) or error. Do NOT change usageFetch.
- `const usagePollEvery = 5 * time.Minute`
- `func (r *Repl) pollUsage()`: return if os.Getenv("HARNESS_USAGE_POLL")=="0" or r.metrics==nil; throttle with new Repl fields `usageMu sync.Mutex; usagePollAt time.Time; usagePolling bool` (skip if polling or now-usagePollAt < usagePollEvery); set usagePollAt=now, usagePolling=true; run goroutine: text,err:=usageRaw(); rec: if err -> {"stage":"usage","error":err.Error()}; else parseUsage ok -> rec with "stage":"usage","source":"cli"; not ok -> {"stage":"usage","error":"parse","head":first 120 chars}. r.metrics.Log(rec); clear usagePolling.
- for tests: `func (r *Repl) pollUsageSync()` same body without goroutine/throttle is acceptable; pollUsage calls it in goroutine.
### 4 call site
- in the turn loop right after usageLine is called for a finished turn (repl.go), call `r.pollUsage()`. Not in idle path.
### 5 turn row per-turn cost
- where the stage "turn" row is logged: add `cost_turn_usd` = cost_usd minus previous cost_usd of the same SessionID (Repl fields `lastCostSess string; lastCost float64`); if session changed or cost < prev -> cost_turn_usd = cost_usd. Keep cost_usd unchanged (cumulative) for backward compat.
### 6 tests internal/harness/usagepoll_test.go
- TestParseUsage: the live sample text above -> 18, 27, resets strings; text without "Current session" -> ok false.
- TestPollUsageLogsRow: stub usageRaw, temp metrics, pollUsageSync -> one row stage=usage session_pct=18.
- TestPollUsageError: stub returns error -> row with error.
- TestSetRateTopLevelFallback (ratestatus_test.go): info {"rateLimitType":"five_hour","utilization":0.42} -> stage=rate row five_hour 0.42, plus one rate_raw row.
- TestUsageLive: t.Skip unless HARNESS_LIVE_USAGE=1; calls real usageRaw + parseUsage, asserts ok, t.Logf values.
- existing TestSetRateLogsOnFiveHourChange must still pass (adjust only if it counts total rows; then count stage=rate rows).
### 7 new scripts/usage_report.py (stdlib only, python3)
- args: --metrics-dir (default notes/metrics), --since/--until (ISO date or datetime, local), --session ID optional.
- reads every *.jsonl in dir (NO mtime filter); row time = ts|time field if present else end_ms/1000; rows without time included only when no window given.
- output: (a) turn cost: per-session max(cost_usd) sum, naive sum, sum of cost_turn_usd when present; (b) classify cost sum; (c) usage series: each stage=usage row time, session_pct, week_pct; delta session_pct first->last in window and turn-cost delta over same span -> usd_per_pct (print "n/a" when <2 points or delta 0); (d) rate rows count, rate_raw keys.
- exit 0; prints "no rows in window" instead of zero silently.
## edge cases
- reset of 5h window: session_pct drops -> series delta computed over last monotonic segment only.
- `·` is U+00B7; regex must work if absent (optional group).
- claude missing -> error row, no panic; never block turn loop.
## acceptance
- gofmt -l internal/harness empty; go vet ./internal/harness/...; go test ./internal/harness/... -count=1 PASS.
- HARNESS_LIVE_USAGE=1 go test ./internal/harness -run TestUsageLive -count=1 -v shows parsed values.
- python3 scripts/usage_report.py --since 2026-10-30 runs, exit 0.
## must NOT change
- usageFetch/decorateUsage behavior and /usage popup; status line format; existing metrics field names; no commit/push.
## unresolved items mapping
1 rate% never recorded -> steps 1-4 (+rate_raw diagnostic). 2 cumulative cost -> step 5 + report (a). 3 mtime/window -> step 7. 4 13.78 USD gap -> report prints naive vs max sums; transcript side not re-measured here, stays open. 5 2nd-run zero -> step 7 no mtime filter + explicit "no rows" message.
## 40% target
- not defined in notes (grep 0 hits). Treat as: report measured session_pct; never claim 40% without a usage row showing it.
