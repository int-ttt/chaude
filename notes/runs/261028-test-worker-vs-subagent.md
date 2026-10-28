# worker (claude -p) vs Task subagent, measured 261028
setup: claude 2.1.312, --model haiku (claude-haiku-5-5), cwd=scratchpad dir (not repo; no repo CLAUDE.md), --permission-mode acceptEdits, stream-json --verbose; 1 run per cell (n=1, noisy)
worker = `claude -p` fresh per task. subagent = `claude -p` parent (haiku) told to delegate via Agent tool (general-purpose, haiku); parent cost is INCLUDED in subagent totals (the realistic comparison for a conductor, but parent here is haiku with tiny context).
tasks: T1 fix 1-line bug in calc.py; T2 3-line summary of log.txt -> summary.md; T3 4-sentence announcement draft -> draft.md
source: result.modelUsage (all models/subagents summed) + result.usage; raw: scratchpad out/*.jsonl

## table (totals per task)
| run | input | output | cache_create | cache_read | cost USD | wall s |
|---|---|---|---|---|---|---|
| worker T1 (cold cache) | 6 | 329 | 22200 | 43581 | 0.0050 | 10.6 (api) |
| worker T2 | 6 | 787 | 11977 | 51753 | 0.0033 | 5.2 (api) / 6.1 wall |
| worker T3 | 4 | 651 | 11261 | 31137 | 0.0029 | 8.0 (api) / 8.9 wall |
| subagent T1 (parent+sub) | 16 | 1409 | 31904 | 99399 | 0.0068 | 15.1 wall |
| subagent T2 | 16 | 2341 | 23871 | 115101 | 0.0063 | 13.3 wall |
| subagent T3 | 12 | 1305 | 22949 | 74838 | 0.0052 | 10.5 wall |
| worker --resume (T2 on T1 session) | 6 | 491 | 3838 | 71088 | 0.0017 | 9.9 wall |
sums: worker T1-3 cost 0.0112; subagent T1-3 cost 0.0183 (1.6x). Resume T2 cost 0.0017 vs fresh T2 0.0033 (~half).
(r2 incremental = result.usage; result.modelUsage was cumulative incl. T1: cc 26038, cr 114669, cost 0.00677 -> minus T1 0.00504 = 0.00173)

## startup system tokens (first API call of session, cache_create+cache_read, input 2)
- worker: T1 21687 (all cache_create, cold); T2 19629 (8127 cc + 11502 cr); T3 19635
- subagent's own first call: T1 14706 (cc); T2 14679 (8522 cc + 6157 cr); T3 14717 -> ~5k smaller than a worker (~15k vs ~19.6-21.7k)
- parent first call (conductor, includes Agent tool def): s1 19.7k, s2 19.7k, s3 19.7k (same as a worker)
- resume first call: 23258 (1058 cc + 22200 cr) = prior context cache-hit, only new prompt written

## cache TTL
- worker sessions wrote cache_creation as ephemeral_1h (22200/11977/11261 all 1h). Subagent: 5m bucket 17006/10902/10895 (sub) and 1h for parent (14898/12969/12054). Matches cache-ttl note in plan 4-c.
- 1h cache writes cost 2x base input; cost numbers above already include this.

## observations
- cost: worker < subagent for fresh one-shot tasks here, because subagent run = parent session (~19.7k system) + subagent session (~14.7k), both paid. A worker run pays only one ~19.7k. Per-task overhead of worker is ~5k tokens bigger than the subagent's own, but the subagent route also needs a parent turn.
- caveat: in real use the conductor already exists, so its marginal cost is only the extra Agent/delegate turn(s) and the result inserted in context, not a fresh 19.7k. Marginal comparison: worker ~19.7k fresh system tokens vs subagent ~14.7k -> subagent cheaper by ~5k tokens/task (cache_create at 1h 2x vs 5m 1.25x makes worker's gap larger in $). NOT measured separately; inference.
- wall time: worker 5-10 s vs subagent 10-15 s (extra parent hops: Agent call + final reply).
- resume: second task on same worker reuses cache (22200 cr hit), cost 0.0017 ~ half of fresh worker (0.0033), and ~1/4 tokens created. Benefit grows with larger worker context. 
- worker system overhead at project cwd NOT measured (this used scratchpad with no CLAUDE.md/hooks); a project cwd with CLAUDE.md + SessionStart hooks adds more to workers. Not tested.
- n=1 per cell, haiku only, tiny tasks; output tokens include thinking (T2/T3 haiku thinking 386/365).
