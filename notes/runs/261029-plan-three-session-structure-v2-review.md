# review: 261029-plan-three-session-structure-v2.md

date: 261029
reviewer: opus-architect
scope: improvements only; plan not edited; nothing implemented
inputs: plan v2, 261029-analysis-usage-patterns-pre1027.md, 261029-analysis-opus-cache-tradeoff.md, DECISIONS.md:15-42, repl.go:26-27,:695,:870-871
verdict: premise is mostly right, but the plan reads the "large context" numbers as a goal. The data shows cache-hit reuse was the goal and large context was the cost. Fix 7 / 5 / 6 / 4 as below.

## 270k question (explicit answer)
answer: keep 270k for opus/sonnet. Do not raise it. Apply it per session, not per model. Change the rotation policy so the main cost lever is the warm handoff at 59:59, not the 270k threshold.
why:
- per-call avg ctx pre-10-26: opus-5 5.0e8/1,330 = ~376k, sonnet-5 4.9e8/1,155 = ~424k. Per-record average holds up even if usage is double-counted (sum and count grow together).
- 96.6% cache_read means cache hits worked. The big context is what made reads costly. It is not something to copy (cache-tradeoff: A cost grows ~quadratically with turns).
- cost model (assumption, not in repo; ratios from public Anthropic pricing as I recall, verify = plan Q2): read 0.1x, 1h write 2x, output 5x input. C0 (fresh session) 30k, growth g = 15k/call (= cache_creation 42.3M / 2,799 assistant records, an upper bound), handoff output 5k.
  per-call cost (input-token-equiv, including constant 2x*g write = 30k):
  R=150k 51.5k / R=200k 50.8k / R=270k 52.0k / R=400k 56.6k / no rotation at ~400k avg ~70k
  -> the curve is flat at 150-270k. 400k costs ~9% more per call. 270k is near optimal and needs no change.
- bigger lever: one re-entry after >1h idle at C=400k costs 2x400k = 800k-equiv (~15 calls). Pattern = night bursts + multi-day gaps (18 active days over ~7 weeks), so expiry rewrites outweigh the threshold choice. DECISIONS:23 warm handoff at 59:59 avoids this. The plan must make it the default for big sessions.
- 400k compact window (repl.go:871): with Bash-heavy turns, one answer can carry ctx from <270k past 400k inside the turn (rotation runs only after the answer). Autocompact then fires mid-turn. 270k leaves ~130k headroom. Raising the threshold removes that headroom.
user-only choice: DECISIONS:40(2) ("Opus/Sonnet keep context, no threshold switching") vs :41 (270k). If the user picks continuity over cost: threshold 350k AND compact window >= 500k (1M-context models only), cost ~+5%. Otherwise keep 270k. The plan should state this as the Q1 answer and pass only this choice to the user.

## improvements (numbered; where in plan)
1. [7, 10.Q1] Replace the "tension" paragraph and Q1 with the answer above: 270k kept, per session, cost table, and the DECISIONS:40/:41 choice as the only user question. Delete 5절 "비용 상한은 270k 회전이다". The cost ceiling is the 1h-expiry rewrite (2xC), not 270k.
2. [1, 3] Restate the premise: "cache-hit reuse inside a live session" not "large context reuse". 3절 "세션당 컨텍스트를 크게 쌓는 것이 정상 동작" -> "keep the session warm while turns continue; context size is capped by 270k". Also: "~6 sessions" is the total over 18 days, not concurrent. VocalAnalyze = 4 sessions, 95%, so concurrent live exec sessions ≈ 1-2. Size all lists/limits for 1-2, not 6.
3. [3, 10.Q7] opus-in-design-session (v1 Q3 assumption) conflicts with this pattern. opus-5 was a main executor (output 1.22M, Bash 93%). Bash output inside the design session fills 270k fast and rotates away design nuance. It also breaks DECISIONS:40(1) ("Opus main only for design turns"). Change: execution-heavy opus work goes to a separate opus exec session (same rules as sonnet exec). The design session takes only design turns. Resolve Q7 this way instead of leaving it open.
4. [6] Replace the Opus keep/end judgment with a deterministic rule. Within 1h, keep is always right (an idle session costs 0, and the cache only matters if used). After 1h, keep has no cache value. The only real decision is at last_used+3570s:
   - ctx >= 60k (assumption): warm handoff (read ~0.1C) then end; the next prompt starts fresh from the snapshot.
   - ctx < 60k: end with the deterministic snapshot (0 tokens, DECISIONS:19 option).
   - ping (keep warm) only when the active-hours heuristic says return is likely within the next hour (e.g. inside the user's usual window; pre-10-26 UTC 13-15 = 41%). Max 1 consecutive ping. Ping cost ~0.1C vs rewrite 2C, so ping pays off if P(return within 1h) > ~5% (assumed ratios).
   Opus call: none for this decision. Remove "context size tilts toward keep" (v2 보강, 6절). Large C argues for warm-handoff-end, not keep, because a large C is near 270k anyway. Remove 9.3 risk accordingly.
5. [4] The classifier per prompt is not needed here. Most prompts continue the same Bash loop in one project. Change to sticky routing: skip the classifier when (a) ack/slash (already), (b) the previous turn was in a live exec session <1h ago and the prompt has no model cue ('opus로', 'sonnet으로') and no [cat-switch] happened. Classify only on: first prompt after idle-end/rotation, explicit model cue, cat-switch, or a short-prompt heuristic miss. Expected saving: one sonnet -p call (uncached, tmpdir cwd) plus a few seconds of latency on most prompts. Add metrics field classify_skipped=true|false.
6. [4, 2.4] Do not send model:/size per prompt to Opus. Route-log stays on disk. At design turns Opus gets only a digest (counts per model, list of overrides/misroutes = prompts where the user forced a model or reran on another model). This keeps correction 4 (input+answer delivered = available on file) without growing the design context.
7. [4 v2 보강] "Reuse the same-model session if alive and same cat/file": add "and ctx < 270k - margin (e.g. 240k)". Otherwise a reused session rotates right after the next answer. If above the margin, rotate first with a warm handoff, then route.
8. [7] Add a mid-turn overshoot guard. Log per-turn ctx delta (max, p95) in metrics. If p95 delta > 130k (= 400k - 270k), raise compact window for 1M-context models (e.g. 600k) or add a pre-turn rotation check (rotate before sending if ctx >= 240k and the prompt kind is exec). Without this, Bash-heavy turns can hit autocompact silently.
9. [7] effectiveThreshold (repl.go:695) is per model. The implementation step must specify a per-session struct {id, model, lastCtx, threshold, lastUsed, cat, files} and say that rotation/keep-end read it. The plan lists the need but gives no data shape. Put it in 7 or 8 so Sonnet/Haiku can implement it without re-deciding.
10. [5, 10.Q4] Merge Q4 into item 4: ping policy = active-hours heuristic + max 1 consecutive. Q4 then only keeps the billing question (is a ping billed as a normal read?). Q6 in cache-tradeoff is still open.
11. [10.Q11] Give concrete re-check triggers instead of an open question. Re-check if, over 7 days: concurrent live exec sessions median > 2, OR per-call ctx median < 100k, OR calls between rotations median < 6 (rotation overhead dominates), OR classifier override rate > 10%. All of these come from metrics already planned.
12. [1 한계] Add the data caveats that affect this review: g = 15k/call is an upper bound (it includes post-expiry rewrites and possibly split records). 754k fable-5 calls exceed the 400k compact window, so that pattern cannot occur under the harness at all. Token per-session n=3.
13. [ordering] Implementation order to put in the plan: (1) metrics: per-session ctx, per-turn delta, turn interval, classify_skipped; (2) per-session state struct + 270k per session; (3) deterministic 59:59 warm-handoff/end; (4) sticky routing; (5) separate opus exec session; (6) ping heuristic last, after the billing check. Steps 1-4 are bounded code changes for haiku-coder; step 5 touches route()/session lifecycle and should go to sonnet-analyst.

## not to change
- haiku 80k threshold, correction 6 (design file + haiku-scout), draining on rotation, category rotation, Korean reply rule.
