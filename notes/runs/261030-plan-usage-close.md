# plan: usage-close (261030)
goal: close 3 open items (c_13.78 gap, background spend row, spend-field consistency) + apply safe efficiency practice; no commit/push.
facts: idleEvent repl.go:289, idleFinish repl.go:380 (called only at :340-341); r.spend reset at runTurn :483, fed only at handle :501; bg row :~410-417 has cost_turn_usd + result_* but no sub_*/spawns; modelUsage never read (grep empty); usage_report.py sums cost_usd/cost_turn_usd only; no headless prompt mode; 5h% = newest stage=usage row session_pct (30 @01:15Z).

## S1 haiku-coder: internal/harness/repl.go
- idleTurn struct: add field `spend *turnSpend`.
- idleEvent top: `if r.idle.spend == nil { r.idle.spend = newTurnSpend() }` then `r.idle.spend.onEvent(ev)`.
- idleFinish, after fields map built, before metrics.Log: `if st.spend != nil { for k, v := range st.spend.fields() { fields[k] = v } }`.
- foreground row (~:905) and bg row (~:416): `if mu, ok := asMap(result)["modelUsage"].(map[string]any); ok && len(mu) > 0 { fields["result_model_usage"] = mu }` (nil-safe for foreground result).
- test (new internal/harness/idlespend_test.go): build Repl like existing tests (grep "&Repl{" *_test.go), feed idleEvent: assistant event with parent_tool_use_id + usage, Agent tool_use, then result with modelUsage; assert logged bg row has sub_* keys, spawns, result_model_usage. Also replay test: if testdata/bg_turn_live.jsonl exists, feed all events after the first `result` line to idleEvent and t.Log the bg row keys; skip if file missing.
## S2 haiku-coder: scripts/usage_report.py
- for stage=turn rows in window: print `sub_<key> sum: N` for every numeric key starting with sub_, `spawns sum: N` (sum numeric; if map, sum values per key), and per source (foreground/background) row counts.
- per-session check: for sessions where every turn row has cost_turn_usd, compare sum(cost_turn_usd) to max(cost_usd) - (first row cost_usd - first row cost_turn_usd); print `session cost check: sessions=N mismatched=M (tol 1e-6)`.
## S3 haiku-runner: gofmt -l internal/ (empty), go vet ./..., go test ./internal/harness/... -count=1, python3 -I scripts/usage_report.py --since 2026-10-30 (no error).
## S4 haiku-runner: live capture (real run, minimal cost)
- mkdir -p /tmp/bgcap (empty cwd: no project CLAUDE.md prefix); cd there.
- `(printf '%s\n' '<user msg json>'; sleep 150) | claude -p --model haiku --input-format stream-json --output-format stream-json --verbose > /tmp/bgcap/bg.jsonl`; user msg text: "Use the Agent tool once: subagent_type general-purpose, run_in_background true, prompt 'Reply with the single word ok. Use no tools.' Then reply only: started". When notified that the agent finished, reply only: done".
- copy to internal/harness/testdata/bg_turn_live.jsonl; report: count of result events, whether events after first result exist, whether they carry parent_tool_use_id.
- run go test -run 'Idle' ./internal/harness/ -count=1 -v; report bg row keys.
## S5 haiku-runner: c_13.78 settle from capture
- per result event: total_cost_usd, sum(modelUsage[*].costUSD), per-model tokens, usage.cache_creation.ephemeral_5m/1h_input_tokens.
- tokens x price (notes/research/billing.md) main-only vs all models; report table. cause verdict: (a) total_cost_usd includes subagent models? (b) cache TTL 1h priced 2x vs assumed 1.25x? (c) cumulative vs per-turn?
## S6 haiku-runner: item 3 fixed window
- window --since 2026-10-30T00:00:00Z --until <ts of capture end>; run usage_report.py; independently sum same keys with python3 -I one-liner over notes/metrics/2026-10-30.jsonl; must match exactly.
## S7 review: sonnet-reviewer blocked by hook for opus subagents (v0.8.2) -> haiku-checker on diff + test output.
## efficiency (no CLAUDE.md/agent edits mid-measurement)
- applied now: batched independent scout reads (3 reads in 2 spawns), Haiku for all mechanical steps, live capture in empty cwd (no project prefix), follow-ups folded into next brief.
- deferred: R6 opus-architect prompt trim, agent-def edits -> need fixed-prompt A/B; no headless mode exists; measure later with sub_* + result_model_usage now logged.
## must NOT change: CLAUDE.md, .claude/agents/*, .claude/settings.json, existing metric keys.
## acceptance: S3 all pass; bg row has sub_*/spawns (unit + live replay if bg events exist); report sums == independent sums; c_13.78 cause stated with numbers.
## merged review (261030-plan-usage-close-review.md)
- S2 corrected: sum spawns (int), spawn_types per key, sub_msgs + sub_* tokens, missing=0; session check = per-row cost_turn_usd vs cost_usd delta (sorted by ts, same session), print `cost_turn check: rows=N mismatched=M resets=R`.
- S4 corrected: flags --include-partial-messages --verbose; exact input JSON line; single prompt, no "done" turn; sleep 150 then EOF.
- S1 test: no network (pollUsage gated), &Session{} and Transcript as idle_test.go:42.
