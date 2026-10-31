task: read-only summary of children.go, dispatch.go, clients.go
date: 2026-10-29
status: done

[children.go]
childKey(cwd, model): cwd "" -> os.Getwd; filepath.Abs; EvalSymlinks; key = cwd + "\x00" + model
liveChild: Sess(closer), Tracker, Model, Role, LastAPI, LastCtx, Files, TurnFrom; parked atomic.Bool (true unbindActive..bindChild); reply buf; stream bool; mu guards LastAPI+reply
childSet{mu, m map[key]*liveChild}, newChildSet()
Park(key,c): replace; Close old Sess if different (outside lock)
Take(key): remove+return
Expired(now, deadline): keys with now-lastAPI >= deadline, sorted; does not remove
CloseAll(): empty map under lock, Close all Sess in parallel goroutines, wait

[dispatch.go]
DefaultMaxWorkers=3; resolveMaxWorkers(flagSet, flagVal, env): flag > CHAUDE_MAX_WORKERS > 3; <1 is error
checkWorkerCap(set, max): len(set.m) >= max -> error (existing children untouched)
childAction: actStartNew | actUseLive | actResume
pickChild(set, reg, key, cwd, model, limit, now, deadline):
  1. set.m[key] live and isWarm(LastAPI, LastCtx) -> actUseLive
  2. reg.Get(cwd, model) must be parked with SessionID, else actStartNew
  3. OwnerPID set, != self, and pidAlive -> actStartNew
  4. !isWarm(row) -> actStartNew; else actResume

[clients.go]
states: live | parked | expired
clientRec keys: (Cwd, Model); fields Role, SessionID, LastAPI, LastCtx, EndedReason, DesignPath, OwnerPID, ChildPID
clientRegistry{path, mu}; file ~/.chaude/clients.json (defaultClientsFile)
readLocked: missing/broken -> nil; writeLocked: sort, tmp "path.tmp" + rename, mode 0600
Get(cwd, model); Upsert(rec) keeps DesignPath when rec empty, keeps other rows
MarkEnded(cwd, model, reason, now): state=expired, ChildPID=0
isWarm(rec, limit, now, deadline): state live|parked, now-LastAPI < deadline, LastCtx < limit
idleDeadline(idleRotate, ttl): 0 disables; ttl "1h" -> idleRotate; else min(idleRotate, 290s)
expiredZero(deadline): returns deadline > 0 (name reads inverted)
reconcile(now, deadline): for rows with dead OwnerPID:
  ChildPID alive -> expired "orphan-alive" (returned as orphans, not killed)
  live -> parked if SessionID and within deadline, else expired; reason "harness-killed"
  parked -> stays parked, reason "harness-killed"; ChildPID=0 written

[callers, not read in depth]
switch/dispatch flow: internal/harness/switchchild.go:78 (unbindActive), :135 (Expired(now,0) sweep), :153-178 (pickChild then actUseLive/actResume/actStartNew fallbacks), :206/:222 (bindChild)
bind/unbind/close: internal/harness/livechild.go:99 (CloseAll), :105 unbindActive, :135 bindChild
registry reconcile: internal/harness/repl.go:1113
pidAlive: internal/harness/agentstore.go:95

[doubts]
d1: readLocked returns nil on broken JSON; next Upsert writes only the new row, dropping other rows silently
d2: clients.go mu is per registry instance; cross-process read-modify-write on clients.json has no file lock; shared "path.tmp" name
d3: pickChild reads LastCtx of liveChild without c.mu (lastAPI() is locked, LastCtx is not)
d4: expiredZero name is inverted relative to its meaning
d5: resume/drain policy (switchchild.go, livechild.go) not verified here
