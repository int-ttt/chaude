# plan: orchestration options (cost-min) — 261030
status: plan, nothing measured yet. All usd deltas = estimates from 10-29 metrics, unverified.
inputs: spawns 612 (scout 229, sonnet-analyst 181, coder 59, opus 36, batch-worker 32, runner 31); short spawns (<=3 msgs) = 4.22 usd; first-message cache write = 13% of cache-write cost; sonnet-analyst + opus-architect ~= 69% of 10-29 usd (S = 10-29 total usd).

## descriptor decision
decision: NO for now (keep 261030-plan-prompt-descriptor.md as plan, do not implement).
why: it targets rotation/cold-switch events only; the ~150K->~2K saving is a plan estimate; the cost drivers are spawn count and sonnet/opus agents (69%), not rotation. Rotation handoff saving = ~150k cache READ (cheap); real saving only on cold switch (~150k cache WRITE each).
gate (test M0): count rotate + cold-switch events per day in notes/metrics/2026-10-28..30.jsonl. If cold switches x (150k x write price of that model) >= 1 usd/day -> switch to YES and implement the descriptor plan.

## options
B (Opus main + Haiku I/O, threshold delegation): Opus main decides inline; ALL file reads/writes via haiku-scout/haiku-coder/haiku-batch-worker; sonnet-analyst only for judgment over >=3 sources or >~8k tokens of material; no sonnet relay hop.
  delegation rule: one scout call per decision (batch all reads), RETURN cap 4000 chars; merge would-be short spawns (<=3 msgs) into one brief; reuse same haiku-coder instance per workstream.
  expected delta: sonnet-analyst spawns -50% (relay ones) -> about -10..-20% S; short-spawn merge -> up to ~-2 usd of 4.22; fewer spawns -> first-message write (13%) falls proportionally.
  risk: Opus main context grows (more Opus cache read per turn); 150k rotation hits earlier.
C (B + scout digests instead of sonnet-analyst): haiku-scout returns structured digest (key: value, <=4000 chars); Opus decides from digest; sonnet-analyst only on escalation or after Haiku fails twice.
  expected delta: sonnet-analyst ~0 -> about -20..-35% S, minus extra Opus tokens (net uncertain).
  risk: Haiku digest drops nuance -> wrong decision/rework; Opus per-token cost replaces Sonnet on long synthesis.

## new options D/E/F (261030, design only, not measured)
basis: 612 spawns 10-29..10-30; first-message cache write 6.33 usd = 13% -> period total ~48.7 usd, ~0.010 usd/spawn; short spawns (<=3 msgs) 4.22 usd; sonnet-analyst+opus-architect ~69% of 10-29 usd. All deltas = estimate vs B.
D (batch consolidation, one-shot fan-out): Opus main writes one task brief, then issues ALL Haiku calls (scout reads, coder edits on disjoint files, checker) in ONE message in parallel; no intermediate agent, no sequential ping-pong; second round only on FAIL.
  rules: coder briefs must name disjoint file sets; sum of RETURN caps <= 8000 chars per round; max 2 rounds per task.
  expected delta: spawns -30..-40% -> first write -1.9..-2.5 usd; short-spawn merge beyond B -1..-2 usd; fewer Opus turns -> less Opus cache read. est -3..-8% vs B.
  risk: wrong upfront plan -> whole parallel round wasted; parallel edit conflicts; larger single return into Opus context.
E (Haiku-only pipeline + deterministic result-length gate): Haiku main does all work; no Sonnet/Opus except escalation after 2 Haiku failures. Hook gate (main-thread-guard.py or agent-model-pin.py) measures bytes of paths named in brief; allow spawn only if declared RETURN cap <= 0.25 x raw bytes, else block -> main does it inline.
  expected delta: most of the ~69% Sonnet/Opus share moves to Haiku token price (~1/3 Sonnet, ~1/5 Opus); gate removes most short spawns (4.22) and their first writes. est -30..-45% vs B, before rework.
  risk: design/judgment quality drops -> rework eats savings; gate misfires on briefs without paths or RETURN cap; highest test cost.
F (long-lived Haiku per workstream): Opus main routes all briefs of one workstream (workstream id in brief) to the same live Haiku child (internal/harness/livechild.go); rotate at 40k tokens or 20 turns or idle >270s (before 5-min cache TTL) with <=1500-char handoff.
  expected delta: first write paid once per workstream -> 6.33 -> ~1.3..2.5 usd; minus context-growth cache reads and TTL-miss rewrites. est -4..-10% vs B.
  risk: needs harness routing (Agent tool cannot continue a spawned agent; must go through livechild); stale context contaminates later briefs; one child is serial; TTL miss = full rewrite costlier than fresh spawn.

## comparison B/C/D/E/F
| option | mechanism | est. cost vs B | main risk | test cost |
|---|---|---|---|---|
| B | Opus main, Haiku I/O, sonnet-analyst by threshold | 0 (ref) | Opus context growth, early 150k rotation | low: config/prompt, T1-T5 x2 |
| C | B + scout digest instead of sonnet-analyst | -5..-15% (net uncertain) | digest loses nuance -> rework | low: T1-T5 x2 |
| D | one brief, parallel Haiku fan-out in one message | -3..-8% | bad upfront plan wastes round; edit conflicts | low: prompt-only, T1-T5 x2 + 1 conflict case |
| E | Haiku-only + hook result-length gate | -30..-45% (before rework) | judgment quality, rework | high: hook + unit test + T1-T5 x2 + sonnet-reviewer quality check |
| F | one live Haiku per workstream, capped rotation | -4..-10% | TTL miss rewrite, stale context, Go harness change | medium-high: livechild routing + idle/TTL scenario + T1-T5 x2 |
ranking (risk-adjusted): D > F > C > B > E. raw savings: E > F ~ D > C > B. D and F are orthogonal and can stack.

## test plan (do not run now)
common setup: same start commit, fresh session per run, same 5 prompts, 2 runs per option (cold + warm cache). Reuse headless comparison harness from 261030-sonnet-analyst-ab-headless.md and ab-v2-*.jsonl format. Cost source: notes/metrics/<date>.jsonl usd fields, summed per task.
minimum task set:
  T1 lookup: one-file question ("rotation threshold 값은?") -> answer matches notes/DECISIONS.md:46 (150k).
  T2 multi-file search+explain: "usagepoll.go와 ratestatus.go 관계 설명" -> checker rubric (3 named facts present).
  T3 small code fix + narrow test: seeded one-line bug in internal/harness -> `go test ./internal/harness -run <Name>` PASS.
  T4 design comparison: 2-3 options with trade-offs from given facts -> rubric (options, trade-offs, choice+why).
  T5 notes write: save given text to notes/runs/ verbatim -> diff empty.
pass/fail: haiku-checker scores each task PASS/FAIL against the rubric above (blind to option).
metrics per option: usd per task, usd total, spawn count, short-spawn count, first-message cache-write usd, Opus/Sonnet share.
acceptance:
  B accepted if total usd <= 0.85 x M0 AND PASS count >= M0 (M0 = current routing, measured from notes/metrics 10-29..10-30) (no task regresses).
  C accepted if total usd <= 0.85 x B AND PASS count >= B; any T4 FAIL -> reject C.
  M0: see descriptor gate.
order: M0 (metrics read only) -> B -> C (C only if B accepted) -> D -> F -> E (E last, only if D or F accepted).

## [haiku] steps
- haiku-scout: M0 event count from notes/metrics/*.jsonl (read only).
- haiku-runner: run T1-T5 per option, 2 runs, save notes/runs/261030-orch-<B|C|D|E|F>-r<1|2>.jsonl.
- haiku-checker: score PASS/FAIL per task, write notes/runs/261030-orch-score.md.

## must NOT change
- v0.8.2 rule: sonnet/opus subagents have no file/shell access.
- rotation threshold 150k (DECISIONS 2026-10-30).
- option configs applied only for test runs; default routing stays current until acceptance.
