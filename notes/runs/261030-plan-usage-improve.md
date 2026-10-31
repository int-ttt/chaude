# 261030-plan-usage-improve
status: v2 (v1 + review notes/runs/261030-plan-usage-improve-review.md merged)
user: "앞으로 개선할 부분을 계속 개선하고, 개선하는 동안 사용량을 지속적으로 추적해 사용량을 많이 잡는 원인과 해결 솔루션을 구상해 파일로 정리."
inputs: notes/runs/261030-plan-usage-window.md, notes/runs/261030-plan-usage-window-review.md, notes/research/prompt-cache-ttl-clear.md, notes/metrics/2026-10-28.jsonl, notes/metrics/2026-10-29.jsonl, notes/FACTS.md, notes/DECISIONS.md, notes/pastes/8eef3280.md
deliverable: notes/runs/261030-usage-causes-and-solutions.md

## decision
problem: 5h % rises fast; metrics jsonl has no subagent usage; cost_usd is cumulative (naive sum 11.18 USD vs per-session max 3.34); no % time series, so root cause (subagent spend) is an estimate.
options:
- A analysis only, no code: cheapest; no time series, root cause stays unconfirmed.
- B step 4 of window plan (stage=rate rows) + transcript-based measurement now: low-risk additive code; gives % vs spend series from now on.
- C window-plan steps 1-5 now: more code risk in one round (budget.go cap<=0 early return, unknown modelUsage schema), more spend.
choice: B. why: window-plan review ordered step 4 first; lowest risk; transcript measurement already covers subagent spend; steps 1,3,2,5 next round.

## steps
M0 baseline (haiku-runner): find Claude Code transcripts for this repo (~/.claude/projects/*claude-harness*/, incl. subagent jsonl). Script /tmp/claude-usage/measure.py (python3 -I). Per day (10-29, 10-30) and per model and per agent type (main vs subagent; subagent type if derivable): msgs, input, output, cache_read, cache_creation split 5m/1h, API-equivalent USD by notes/FACTS.md prices. Per-spawn first-message cache_creation by agent type. Per-session cost delta from notes/metrics/*.jsonl (max-min of cost_usd per session). Save output notes/runs/261030-usage-m0.txt.
S1 read (haiku-scout): where the status line gets the 5h rate value (commit e3c499d), and the metrics writer function used for stage=turn rows; file:line + signatures.
S2 code (haiku-coder): add a metrics row {ts, stage:"rate", session, five_hour_pct, weekly_pct if available, reset if available} written from the existing rate value; only on value change or <=1 per turn; errors skipped; no new API/CLI calls. Add a unit test for the change-only write rule.
S3 check (haiku-checker): go build ./... and go test ./internal/harness/... ; report exact PASS/FAIL and diff stat.
M1 re-measure (haiku-runner): rerun /tmp/claude-usage/measure.py; save notes/runs/261030-usage-m1.txt; delta M1-M0 = this round's spend (observer line).
D1 write (haiku-batch-worker): deliverable in Korean: facts vs estimates separated; measured deltas; causes; solutions ranked by expected effect; open items.
R1 review (sonnet-reviewer): deliverable vs m0/m1 numbers and inputs; fix list.

## edge cases
- 5h % not available (API/CLI failed, not subscription): no rate row.
- transcripts missing subagent files: report as NOT FOUND, do not guess.
- cost_usd reset inside a session (rotate): delta per session id, not per file.
- Haiku price tier above 100k context: use the >100k row when message input+cache_read+cache_creation >100k.

## acceptance
- deliverable exists; every number cites m0/m1 or an input file.
- go build/test PASS, or code change reverted and reported.
- M1-M0 delta reported.

## must NOT change
- budget.go behavior, rotate thresholds, router defaults, settings.json, CLAUDE.md, .claude/agents/*, hooks.
- no commit, no push.

## next round (not this round)
- window-plan step 1 costDelta(*Session) independent of cap; step 3 metricscmd delta sum; step 2 raw modelUsage map; step 5 status-line label "세션누적"; step 6 delegation policy after >=1 day of rate rows.
