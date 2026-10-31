# explore: summary of internal/harness livechild.go, switchchild.go, clients.go (read-only)
date: 2026-10-29
sources: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/{livechild.go(170 lines), switchchild.go(272), clients.go(207)}

## livechild.go
purpose: main-agent clock tracking; event handling for parked-live children; unbind/bind of active child; notices; close-all.
funcs:
- touchMain(ev, now): main events only. stream_event message_start -> r.lastAPI=now. assistant with usage -> r.ttlSeen=observedTTL(usage, ttlSeen).
- parkedIdleEvent(c, ev): no render, no prompt.
  - control_request can_use_tool -> RespondPermission(false, "denied: session parked (another model is active)").
  - rate_limit_event -> setRate.
  - message_start (main) -> c.setLastAPI(now).
  - content_block_delta text_delta -> append to c.reply, c.stream=true.
  - assistant (main) -> if !c.stream append text blocks to c.reply.
  - result -> flush reply; budget.charge(s, total_cost_usd) if present; notice "[parked <model> background reply] " + trunc(txt, 300).
- addNotice / takeNotices: append / drain r.notices under noticeMu.
- closeAllChildren: r.children.CloseAll() if set.
- unbindActive(): nil if r.sess nil. Sets c.parked=true FIRST; c.TurnFrom=len(r.tr.turns); copies lastAPI, TTLSeen, LastCtx, Files, PendingSnapshot, LastRotFail, Cat. Clears r.sess, pendingSnapshot, cur. Creates tracker if nil.
- bindChild(c): restores sess/tracker/model/lastAPI/ttlSeen/state. If turns were added while parked: appends "## While this session was parked" + user/assistant turns from TurnFrom, trunc to 6000 chars (3000 if model contains "haiku") onto pendingSnapshot. Sets r.cur=c; c.parked=false LAST.
constants/numbers: limit 6000 (non-haiku), 3000 (haiku); notice trunc 300. Turn index counts r.tr.turns pairs.
lifecycle: parked flag true before state moves, false after bind (idle callback must not render mid-move).
what numbers count: lastAPI = time of last main message_start (API call start). TurnFrom = turn-pair index.

## switchchild.go
purpose: TTL/warm-deadline math, child routing, park/retire/end of child processes, switching models.
consts/vars:
- neverCold = 24*365h (used when idleRotate == 0).
- effTTL(): ttlSeen if observed, else r.ttl (--cache-ttl flag).
- warmDeadline(): idleDeadline(r.idleRotate, effTTL()) if >0, else neverCold.
funcs:
- ownPID(): os.Getpid().
- deadChecker iface; childDead(c): Sess.Dead().
- sessID(c), childPID(c): session ID / claude PID; 0 for fakes/nil.
- routeChildren(model, task, worker): haiku->haiku with worker -> switchSession (park-resume path). Otherwise if model != r.a.Model -> switchChild; error prints "[switch failed: ...]".
- parkOld(): unbindActive. If lastAPI zero (never called API) -> Sess.Close, return (old,false). Else key=childKey(cwd, model); retire prev under same key; children.Park(key, old); reg.Upsert state=parked, OwnerPID, ChildPID.
- retire(c): if Tracker has running delegated agents AND MaxDrain>0 -> StopIdle, add drain{timeout: DrainTimeout} with consume goroutine. Else Sess.Close().
- endParked(model, reason): take parked child under key; retire; reg.MarkEnded(expired, reason).
- endAllParked(reason): for children.Expired(now, 0) keys -> endParked(model part after NUL).
- switchChild(target, resumeID):
  - same model: switchSession if resumeID, else nil.
  - act = pickChild(children, reg, key, cwd, target, thresholdFor(target), now, warmDeadline()) when no resumeID. Actions: actUseLive / actResume / actStartNew.
  - useLive: take child; if dead -> Close, start new. Cold child under key -> retire.
  - actResume: resumeID = reg row SessionID, else start new. r.parked[target] pid used if actStartNew.
  - snapshot (only when no live child and no resume and r.sess set): pendingSnapshot, else makeSnapshot("route "+target); snapHeld -> lastRotFail=now, error "handoff held".
  - parkOld; live -> bindChild, reg live, return.
  - else set a.Model, resumeID; start(). On error: revert model; re-bind old parked child, or restart old child with snap; print "chaude: cannot start claude". On success reg.Upsert live.
- isDesignPlanPath(fp): path contains /notes/runs/, basename *.md, contains "-plan-", not "-review".
- recordDesignPath(model, fp): only model=="opus" and plan path -> reg row DesignPath=fp.
- withDesignPath(model, snap): if row DesignPath set and file exists and size>0 -> append "\n\n설계 파일: <path> (먼저 읽고 이어서 진행)". Missing/empty -> metrics log stage=design_path missing=true, snap unchanged.
lifecycle: parkOld keeps child alive (parked-live) so switching back reuses warm cache if within warmDeadline.
what numbers count: warmDeadline measured from lastAPI (last message_start).

## clients.go
consts: clientLive="live", clientParked="parked", clientExpired="expired".
clientRec (json): cwd, model, role (design|exec), session_id, state, last_api, last_ctx, ended_reason, design_path, owner_pid (harness process), child_pid (claude -p process, 0 = none). Key = (Cwd, Model).
clientRegistry: path + mutex. File default ~/.chaude/clients.json.
- readLocked: missing -> empty; broken JSON -> nil.
- writeLocked: sort by (cwd, model); MarshalIndent with one-space indent; dir 0o755; tmp file "<path>.tmp" mode 0o600 then rename.
- Get(cwd, model), Upsert(rec): replace (cwd,model) row, keep old DesignPath if new empty, rewrite whole file.
- MarkEnded(cwd, model, reason, now): state=expired, EndedReason=reason, ChildPID=0; creates row with LastAPI=now if missing.
- isWarm(rec, limit, now, deadline): state live|parked AND now-LastAPI < deadline AND LastCtx < limit.
- idleDeadline(idleRotate, ttl): idleRotate<=0 -> 0; ttl=="1h" -> idleRotate; else min(idleRotate, short) with short=290s.
- expiredZero(deadline): returns deadline>0 (name reads opposite to meaning; "true = can expire").
- reconcile(now, deadline) []orphans: only live|parked rows with dead OwnerPID.
  - ChildPID alive -> state=expired, reason "orphan-alive" (returned, not killed).
  - live row -> parked if SessionID != "" and now-LastAPI < deadline, else expired; reason "harness-killed".
  - parked row (child dead) -> stays parked; reason set "harness-killed".
  - ChildPID cleared to 0; file rewritten only if changed. Rows of live owners untouched.
lifecycle: child_pid set on park/live, cleared on MarkEnded and reconcile.

## doubts (not verified by running)
- parkOld with lastAPI zero closes the child but does not touch the registry row; old row may stay live until reconcile.
- reconcile default branch: parked row keeps state=parked but gets EndedReason "harness-killed" (comment says only "stays parked").
- expiredZero name vs. semantics: check callers.
- endAllParked passes deadline 0 to children.Expired; semantics live in another file, not read.
- idleDeadline: any ttl other than "1h" (incl. "5m" and unobserved "auto") uses 290s cap.
