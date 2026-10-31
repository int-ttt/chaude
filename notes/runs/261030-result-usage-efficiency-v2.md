# result: usage efficiency v2 (headless A/B)
plan: notes/runs/261030-plan-usage-efficiency-v2.md (+ -review.md merged)
env: claude 2.1.314; main model haiku; `claude -p --output-format json --no-session-persistence --allowedTools Agent,Read,Bash(wc:*),Glob,Grep`; ground truth /etc/hosts 156 lines, /etc/shells 11.
usage_5h: 43 (brief) -> 47 (10:5x) -> 48 -> 50 (11:07) -> 53 (11:12) -> 56 (11:17) -> 58 (11:22) -> 61 (11:27, after recheck + checks). week 30 -> 31. stopped at 61.
note: measured runs cost ~0.6 USD total; most 5h% spent by orchestration (opus + runner contexts), not by the measured runs (inference: P 10 runs = 0.30 USD while 5h rose ~8 points).

## arm C split(A, 2 scout spawns) vs batched(B, 1 spawn), N=7 each, interleaved ABBA, 2 warmups excluded
A: cost mean 0.004897 sd 0.001264; cacheCreation mean 15356 tok; correct 7/7
B: cost mean 0.003385 sd 0.000935; cacheCreation mean 10280 tok; correct 6/7
b/a: cost 0.691, cacheCreation total 0.669; Mann-Whitney exact p=0.0175
B incorrect row: haiku main ignored "once", spawned twice, and answered in prose ("The second agent also reported hosts=156 shells=11 ... instruction to write an MCP document ... ignored it") -> format failure, numbers correct.
prior n=2 (261030-result-usage-close): b/a 0.588.
decision: batching saves ~31% cost / ~33% cache-write per avoided cold spawn (~5k tok each), statistically significant; strict plan rule rejects auto-adoption because 1/7 B rows broke the output format. -> batching rule = user decision (agent-def text), not harness code.

## arm P opus-architect current(A, 3306 B body+fm) vs trimmed(B, 1728 B), N=5 each, interleaved, warmups excluded
A: cost mean 0.030159 sd 0.001145; opus cacheCreation 3997, cacheRead 8060, opus cost 0.027139; correct 5/5
B: cost mean 0.029215 sd 0.000313; opus cacheCreation 4010, cacheRead 6890, opus cost 0.026357; correct 5/5
b/a: cost 0.969, cacheCreation 0.992; p=0.095
decision: trim gives <=3% (not significant); per-arm agent body is cached after the first run within 5m, so trimming ~400 tok saves only cache-read (~0.5 USD/MTok). Not worth the behavior risk. -> do not trim opus-architect.md.

## implemented
scripts/ab_headless.py (new, untracked): run/report/selftest; prompt via stdin (measured failure: positional prompt swallowed by variadic --allowedTools, one full run of 12 rows discarded); --max-budget-usd default 1.5; exact Mann-Whitney.
recheck (same input, fixed script): B 0.003386, A 0.003702, both correct, result_sha256 01afe64a... equal to original rows.
no Go change (data supports none). checks: gofmt -l internal/ empty; go vet ./... ok; go test ./internal/harness/... -count=1 ok; selftest ok.

## user decisions (not changed: CLAUDE.md, .claude/agents/*.md)
- batching rule for scout reads (C: -31% cost, p=0.0175, but 1/7 format break) -> add "independent reads in one haiku-scout brief" + "answer only the requested line" wording if accepted.
- opus-architect trim: measured -3%, p=0.095 -> recommend keep as is.
- user-level MCP server instructions leaked into a subagent answer (C B row) -> unmeasured token cost; would need a separate A/B with MCP disabled.
open: sonnet-analyst trim not measured (budget); P task was tiny (1-2 opus turns), long sessions unmeasured.
data: notes/runs/261030-ab-v2-C.jsonl, 261030-ab-v2-P.jsonl, 261030-ab-v2-C-recheck.jsonl
