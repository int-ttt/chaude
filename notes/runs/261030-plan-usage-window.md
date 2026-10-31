# plan: usage-window (5h 00:20-05:20 KST, 47% in ~1h43m)
asof: 2026-10-29T17:03Z (=10-30 02:03 KST); window start 15:20Z; elapsed 1h43m
status: analysis done, no code changed; steps below = implementation plan (not executed)

## facts (verified by reads)
- metrics file: notes/metrics/2026-10-29.jsonl, last ts 16:58:51Z; 2026-10-30.jsonl absent. Data after 17:00Z is absent because now=17:03Z (window still running), not because logging stopped.
- window stage=turn: 27 rows, 4 sessions (bfc29c97, e66c18aa, 82e672e7, 0653d3a3), all model=haiku main; cc 265,582; cr 2,842,331; out 50,916.
- cost_usd = CLI total_cost_usd, cumulative per process (budget.go:6 comment; series monotone per session). metrics never deltas it: repl.go:857/864/868, repl.go:375/391; metricscmd.go:50-51 sums raw values.
- naive sum cost_usd in window = 11.18 USD (overcount). per-session max = 3.34 USD; delta vs pre-window = 3.18 USD.
- metrics tokens = main thread only (repl.go:474/487/489 isMain filter). total_cost_usd includes subagents: cost-state modelUsage bfc29c97 total 1.395 = haiku 0.229 + sonnet 0.698 + opus 0.469; e66c18aa 0.806 = haiku 0.269 + sonnet 0.537.
- transcripts (~/.claude/projects/-Users-...-claude-harness/<id>.jsonl + <id>/subagents/*.jsonl, 101 subagent files), dedup by message.id, ts>=15:20Z:
  - haiku main: 61 msgs, cc 285,152 (all 1h), cr 3,506,123, out 66,694
  - haiku sub: 493 msgs, cc 1,809,644 (all 5m), cr 9,259,302, out 698,182
  - sonnet sub: 102 msgs, cc 347,754 (5m), cr 1,056,416, out 87,135
  - opus sub: 10 msgs, cc 68,793 (5m), cr 74,837, out 20,808
- no other project/session had usage in window (checked all ~/.claude/projects).
- 5h % in status line = server rate_limit_event unifiedWindows.five_hour.utilization (ratestatus.go:11-35); not logged to metrics.
- FACTS.md prices (USD/MTok in/5m/1h/read/out): Opus 4/5/8/0.20/20; Sonnet 2/2.50/4/0.10/10; Haiku 0.10/0.125/0.20/0.01/0.50.
- /clear docs: "Start a new conversation with empty context"; cache deletion not stated. Cache TTL default 5m, refresh on hit; Claude Code main on subscription = 1h.

## inferences (not verified)
- API-equivalent by group (token x price, Haiku <=100k tier): sonnet sub ~1.85, opus sub ~0.78, haiku sub ~0.67, haiku main ~0.13; total ~3.4 USD ≈ per-session max 3.34. Share: subagents ~96%, Sonnet+Opus subs ~75%.
- user's 0.4 USD figure = main-thread tokens only; 11 USD = cumulative double count. Real ≈ 3.2-3.4 USD API-eq.
- 47% mapping: plan quota weighting unknown; Sonnet/Opus subagent usage is likely the driver of %. Not verifiable without utilization time series.
- main cache hypothesis rejected: main TTL 1h (cc1h only in main); gaps 304-842s did not cause large rewrites (e.g. 16:01 gap 407s cc 8,018).
- rotation first turn: 4 turns cc 30-45k at haiku 1h price ≈ 0.01 USD each; negligible.
- /clear: not a cause; no cache delete; at most one new small prefix write.
- subagent cold start: each spawn writes its prompt+context at 5m TTL; 1.81M+0.35M+0.07M cc across ~101 spawns ≈ 22k cc per spawn.

## root cause
- primary: subagent usage (Sonnet/Opus delegations + many Haiku scouts), invisible in metrics tokens.
- measurement: cost_usd logged cumulative, summed naively -> 11 USD illusion.
- not causes: /clear, main cache expiry, rotation.

## steps (implementation, Sonnet/Haiku)
1. haiku-coder: internal/harness/repl.go ~857-868 and ~375-391: add field "cost_delta_usd" = total_cost_usd - last[session] (reuse budget.go:33-38 logic; if d<0 then d=total). Keep "cost_usd" unchanged.
2. haiku-coder: same sites: log result["modelUsage"] verbatim as "model_usage" (per-model costUSD, tokens) when present.
3. haiku-coder: internal/harness/metricscmd.go:50-51: sum cost_delta_usd when present; fallback per-session max(cost_usd) for old rows. Print per-model totals from model_usage.
4. haiku-coder: repl.go 200-203 setRate: append metrics row stage="rate", fields five_hour_util, resets_at, ts (rate-limit 1 row per change).
5. haiku-coder: repl.go:615-622 status bar label "$x.xxxx" -> "세션누적 $x.xxxx".
6. sonnet-analyst (decision, not code): delegation policy - require cost-tier justification before Sonnet/Opus subagent; prefer haiku-scout; trim .claude/agents/*.md prompt size (cold-start cc). Optional: enable budget gate (budget.go, disabled by default) with 5h cap.

## edge cases
- CLI restart in same session id: cost resets -> d<0 -> d=total.
- classify one-shot rows: each own process -> cost already per-call; delta = cost.
- transcript duplicate lines per message.id: always dedupe.
- 100k+ Haiku tier pricing not applied in estimates.

## acceptance checks
- haiku-checker: on a replayed session, sum(cost_delta_usd) == last cost_usd (±0.0001).
- metricscmd for 2026-10-29 window prints ~3.18-3.34, not 11.18.
- stage=rate rows appear and track status line %.
- go test ./internal/harness/... PASS.

## must NOT change
- cost_usd field semantics/name; existing metrics rows; rotation/idle logic (3599s); /clear behavior; budget.go pricing table.
