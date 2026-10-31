# explore: livechild / switchchild / idle sweep / budget / cache TTL / ListAgents (read-only)
date: 2026-10-29
scope: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness

## live child lifecycle
- type liveChild: children.go:35 (Sess, Tracker, Model, TurnFrom, TTLSeen, parked atomic.Bool, LastCtx, Files, Cat, ...)
- childSet: children.go:71; Park children.go:80; Take children.go:91; CloseAll; Expired
- childKey(cwd, model): children.go:16
- parkOld(): switchchild.go:77 -> unbindActive; closes if lastAPI zero, else Park under childKey + reg.Upsert state=parked
- retire(): switchchild.go:97 -> drain if delegated agents running and MaxDrain>0, else Close
- endParked / endAllParked: switchchild.go:115 / 131 (reason marks registry row expired)
- unbindActive(): livechild.go:105 (sets parked=true first, saves lastAPI/TTLSeen/LastCtx/Files; clears r.sess, r.cur)
- bindChild(): livechild.go:135 (restores fields; appends "## While this session was parked" turns to pendingSnapshot; parked=false last)
- parkedIdleEvent(): livechild.go:31 (parked child: denies can_use_tool with "denied: session parked (another model is active)", rate limit, text buffering, result -> budget.charge + addNotice "[parked <model> background reply]")

## switch logic
- routeChildren(): switchchild.go:56 (haiku same-model worker -> switchSession; else switchChild if model != r.a.Model)
- switchChild(target, resumeID): switchchild.go:144
  - pickChild(): dispatch.go:61 -> actUseLive (in-memory warm child) / actResume (registry parked row with SessionID) / actStartNew
  - actUseLive: Take child; if dead -> Close and start new; else bindChild(live) (no restart)
  - snapshot: makeSnapshot unless live/resume; handoff held -> error "handoff held"
  - parkOld(old) then start(); on failure restores old via bindChild or restarts with snapshot
- effTTL(): switchchild.go:16 (observed ttlSeen, else --cache-ttl flag)
- warmDeadline(): switchchild.go:26 (idleDeadline or neverCold = 24*365h)

## idle sweep (3599s rotate)
- idleDeadline(idleRotate, ttl): clients.go:148 (ttl "1h" gives idleRotate; not read in full here)
- default --idle-rotate: repl.go:1020 = 3599*time.Second; help: "0=off; idle rotation runs in the TUI only"
- sweepIdle(now): idlesweep.go:34
  - dead parked child -> Take, Close, reg.MarkEnded "child-exit"
  - deadline>0 && !busy && now-lastAPI >= deadline -> Take, Close, MarkEnded "expired", prints "parked <model> session expired (cache cold)"
  - busy = parked tracker has running delegated agents -> left alone
  - returns r.sess != nil && idleDue(now)
- idleDue(now): idlesweep.go:10 (fires once per lastAPI value via idleFiredFor)
- caller: tui.go:381 -> m.submit <- "/idle-rotate" (tui.go:383); handled tui.go:1145 -> idleRotateNow() (idlesweep.go:69) -> rotate("idle Ns")
- ttl default: repl.go:1018 --cache-ttl default "1h" (auto|1h|5m)

## budget flag and default
- --task-budget-usd: repl.go:1019 default 0 = OFF ("per-task USD cap (0=off)")
- repl.go:185 newBudget only when TaskBudgetUSD > 0; nil = off
- repl.go:221 also passes --max-budget-usd to claude when > 0
- budget.go: cap<=0 = off; charge(s,total) :27; chargeOneShot :43; allow(est) :53; over(running) :63; estimate(model,ctx,warm,ttl) :100
- budget check in turn path: repl.go:449 (allow(estNext)) -> budgetStop; mid-turn over() at repl.go:494; budgetStop repl.go:592 logs stage=budget

## cache TTL observe
- observedTTL(usage, prev): core.go:79 -> "1h" if cache_creation.ephemeral_1h_input_tokens>0, "5m" if ephemeral_5m>0, else prev
- called from touchMain(): livechild.go:10-28 (assistant event usage, main-agent only) -> r.ttlSeen
- withCacheTTL(env, ttl): core.go:42 sets CLAUDE_CODE_PROMPT_CACHE_TTL for "1h"/"5m"; "auto" leaves env

## ListAgents / peer discovery
- ListAgents: NOT defined or called in any Go or doc file under the repo (grep --include go/md/py/sh, excluding samples/).
  - only hits: samples/probe_misc.jsonl:9 (Claude Code init tool list, from another project claude-lean), a "list-agents" slash command in the same sample, and counts in notes/runs/261029-usage-aggregate*.md.
- harness peer view is file-based instead: clientRegistry at ~/.chaude/clients.json (clients.go:33-45, defaultClientsFile)
  - rows keyed (Cwd, Model); fields state live|parked|expired, session_id, last_api, owner_pid, child_pid
  - Upsert clients.go:104; Get clients.go:89; reconcile clients.go:175 (startup, repl.go:1113): dead owner rows -> parked/expired/orphan-alive
  - pickChild reads registry for resumable parked rows (dispatch.go:70)
- idle peer sessions (parked) are shown how: not located in grep of tui/agentview for a list renderer. Only the registry and the "[parked <model> background reply]" notice were found.

## NOT FOUND
- no ListAgents implementation
- no explicit UI listing of peer sessions located (did not read tui.go rendering)
- idleDeadline body (clients.go:148-) read only via grep
