# explore: summary of internal/harness budget.go, idlesweep.go, haikupool.go (read-only)
status: done
scope: budget.go (124 lines), idlesweep.go (77 lines), haikupool.go (88 lines)

## budget.go
purpose: per-task USD ledger (U9), in memory only. cap<=0 = off.
types: budget{mu, cap, spent, running, last map[*Session]float64}; modelPrice{in, write5m, write1h, read, out}; priceTable map.
funcs:
- newBudget(cap)
- reset(): spent=0, running=0; per-process baselines kept
- charge(s, total): books delta vs last[s]; drop (new process / CLI reset) books whole total
- chargeOneShot(cost): books separate one-shot process cost (classifier, /usage); ignores cost<=0
- allow(est): false if spent>=cap or spent+est>cap
- over(running): sets running; true if spent+running>=cap
- Spent(); resumeBaseline() returns 0 always
- estimate(model, ctx, warm, ttl); midTurnEstimate(model, usage)
constants priceTable (USD per 1M tokens; source docs.claude.com pricing, fetched 2026-10-29 per comment; alias opus=Opus 5.5, sonnet=Sonnet 5.5, haiku=Haiku 5.5 up to 100k; prices not verified by me):
- opus: in 4, write5m 5, write1h 8, read 0.20, out 20
- sonnet: in 2, write5m 2.50, write1h 4, read 0.10, out 10
- haiku: in 0.10, write5m 0.125, write1h 0.20, read 0.01, out 0.50
- estimate: (ctx*per + 4000*out)/1e6; per = read if warm, else write5m, or write1h if ttl=="1h". 4000 = assumed output tokens. unknown model = 0.
- midTurnEstimate: (cache_read*read + cache_creation*write5m + input*in + output*out)/1e6 from one main assistant usage event. cache creation always priced at write5m.
gating: OFF by default. --task-budget-usd default 0 (repl.go:1019). newBudget only if >0 (repl.go:185-186). Same value also passed to claude as --max-budget-usd backstop when >0 (repl.go:221-222).
counts: spent = booked USD of current task (reset per task). running = in-flight estimate passed to over().

## idlesweep.go
purpose: cold-prompt-cache detection; ends parked children; rotates active child.
funcs:
- idleDue(now): fires once per lastAPI value (idleFiredFor guard); needs non-zero lastAPI and now-lastAPI >= deadline. ttl = ttlSeen, else r.ttl.
- sweepIdle(now): parked child dead (childDead) -> Take, Close, MarkEnded "child-exit". Parked, not busy (no running Tracker agents), idle >= deadline -> Take, Close, MarkEnded "expired", prints dim "parked <model> session expired (cache cold)". Returns r.sess!=nil && idleDue(now).
- idleRotateNow(): MarkEnded "expired", prints "idle: cache cold, rotating", rotate("idle %ds").
literal constants: none in this file.
deadline comes from idleDeadline (clients.go:148-160): idleRotate<=0 -> 0 (off); ttl=="1h" -> idleRotate; else min(idleRotate, 290s). expiredZero(d) = d>0.
defaults: --idle-rotate = 3599*time.Second (repl.go:1020), so ON by default; 0 = off. Flag help says idle rotation runs in TUI only. --cache-ttl default "1h" (repl.go:1018).
resulting deadline with defaults: 3599s (ttl 1h). With ttl 5m or auto (unobserved): 290s.
counts: idle time = now minus lastAPI of that child (last API event time).

## haikupool.go
purpose: in-memory pool of Haiku sessions with edit history; candidates for next edit-type prompt. Per chaude process.
types: haikuWorker{ID, Task, Files []string, Updated time.Time}; workerPool{mu, l map[string]*haikuWorker}.
const: maxCandidates = 5 (sessions returned, not files).
funcs:
- newWorkerPool()
- noteEdit(id, task, file): ignores empty id/file; Task = trunc(task, 80); Files deduped; Updated=now
- candidates(): newest first by Updated, max 5
- has(id)
- candidateText(c): lines "- <id> | task: <task> | edited: <files>\n"; files joined ", " then trunc 160; "" if empty
gating/wiring: not checked.

## not found / not checked
- trunc() definition, liveChild, Tracker.running(), rotate(), childDead body (switchchild.go:35) not read.
- haikupool.go call sites not checked.
