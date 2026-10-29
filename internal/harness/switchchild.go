package harness

import (
	"fmt"
	"os"
	"strings"
	"time"
)

func ownPID() int { return os.Getpid() }

// neverCold is the warm deadline used when --idle-rotate is 0 (idle expiry off).
const neverCold = 24 * 365 * time.Hour

// effTTL is the cache TTL to plan with: the observed one when known, else the --cache-ttl flag.
func (r *Repl) effTTL() string {
	r.clockMu.Lock()
	defer r.clockMu.Unlock()
	if r.ttlSeen != "" {
		return r.ttlSeen
	}
	return r.ttl
}

// warmDeadline is how old a child's last API call may be for its cache to count as warm.
func (r *Repl) warmDeadline() time.Duration {
	if d := idleDeadline(r.idleRotate, r.effTTL()); d > 0 {
		return d
	}
	return neverCold
}

type deadChecker interface{ Dead() bool }

func childDead(c *liveChild) bool {
	d, ok := c.Sess.(deadChecker)
	return ok && d.Dead()
}

func sessID(c closer) string {
	if s, ok := c.(*Session); ok && s != nil {
		return s.ID()
	}
	return ""
}

// childPID is the claude process id of c (0 for fakes and nil).
func childPID(c closer) int {
	if s, ok := c.(*Session); ok && s != nil {
		return s.PID()
	}
	return 0
}

// routeChildren is route()'s tail when the client registry and child set are on.
func (r *Repl) routeChildren(model, task, worker string) string {
	if model == "haiku" && r.a.Model == "haiku" && worker != "" {
		r.p(dim+"[route → haiku (기존 세션 "+trunc(worker, 8)+")]"+rst, "\n")
		r.switchSession("haiku", worker) // same-model worker switch keeps the old park-resume path
		return task
	}
	if model == "haiku" && worker != "" {
		r.p(dim+"[route → haiku (기존 세션 "+trunc(worker, 8)+")]"+rst, "\n")
	} else {
		r.p(dim+"[route → "+model+"]"+rst, "\n")
	}
	if model != r.a.Model {
		if err := r.switchChild(model, worker); err != nil {
			r.p(dim+"[switch failed: "+err.Error()+"]"+rst, "\n")
		}
	}
	return task
}

// parkOld unbinds the active child and keeps it alive under its (cwd, model) key (or closes it when it never called the API).
// Returns the unbound child and whether it was parked.
func (r *Repl) parkOld() (*liveChild, bool) {
	old := r.unbindActive()
	if old == nil {
		return nil, false
	}
	if old.lastAPI().IsZero() {
		old.Sess.Close()
		return old, false
	}
	key := childKey(r.a.Cwd, old.Model)
	if prev, ok := r.children.Take(key); ok && prev != nil && prev != old {
		r.retire(prev)
	}
	r.children.Park(key, old)
	r.reg.Upsert(clientRec{Cwd: r.a.Cwd, Model: old.Model, Role: old.Role, SessionID: sessID(old.Sess), State: clientParked,
		LastAPI: old.lastAPI(), LastCtx: old.LastCtx, OwnerPID: ownPID(), ChildPID: childPID(old.Sess)})
	return old, true
}

// retire ends a child that left the set: drained when delegated agents still run, else closed.
func (r *Repl) retire(c *liveChild) {
	s, ok := c.Sess.(*Session)
	if ok && s != nil && c.Tracker != nil && len(c.Tracker.running()) > 0 && r.a.MaxDrain > 0 {
		s.StopIdle()
		d := &drain{sess: s, tr: c.Tracker, since: time.Now(), timeout: r.a.DrainTimeout}
		r.draining = append(r.draining, d)
		go d.consume(func(kind string, ev event) {
			r.drainMu.Lock()
			defer r.drainMu.Unlock()
			r.tr.add(kind, kv{"type", str(ev["type"])}, kv{"session", s.SessionID})
		})
		return
	}
	c.Sess.Close()
}

// endParked ends the parked child under (cwd, model): retired (drained when agents run, else closed),
// its registry row marked expired with reason. Reports whether a child was found.
func (r *Repl) endParked(model, reason string) bool {
	if r.children == nil {
		return false
	}
	c, ok := r.children.Take(childKey(r.a.Cwd, model))
	if !ok || c == nil {
		return false
	}
	if c.Sess != nil {
		r.retire(c)
	}
	r.reg.MarkEnded(r.a.Cwd, model, reason, time.Now())
	return true
}

// endAllParked ends every parked child (see endParked).
func (r *Repl) endAllParked(reason string) {
	if r.children == nil {
		return
	}
	for _, k := range r.children.Expired(time.Now(), 0) {
		if i := strings.IndexByte(k, 0); i >= 0 {
			r.endParked(k[i+1:], reason)
		}
	}
}

// switchChild moves the active slot to target. The child being left stays alive (parked-live) so a later
// switch back reuses its warm prompt cache. resumeID is a session id to resume (haiku worker), or "".
func (r *Repl) switchChild(target, resumeID string) error {
	if target == r.a.Model {
		if resumeID != "" {
			r.switchSession(target, resumeID)
		}
		return nil
	}
	now := time.Now()
	key := childKey(r.a.Cwd, target)
	act := actStartNew
	if resumeID == "" {
		act = pickChild(r.children, r.reg, key, r.a.Cwd, target, r.thresholdFor(target), now, r.warmDeadline())
	}
	var live *liveChild
	if act == actUseLive {
		c, ok := r.children.Take(key)
		if ok && !childDead(c) {
			live = c
		} else {
			act = actStartNew
			if ok {
				c.Sess.Close()
			}
		}
	} else if c, ok := r.children.Take(key); ok { // cold child under the key: end it, the registry decides
		r.retire(c)
	}
	if act == actResume {
		if rec, ok := r.reg.Get(r.a.Cwd, target); ok {
			resumeID = rec.SessionID
		} else {
			act = actStartNew
		}
	}
	if resumeID == "" && act == actStartNew {
		if pid := r.parked[target]; pid != "" {
			delete(r.parked, target)
			resumeID = pid
		}
	}
	snap := ""
	if live == nil && resumeID == "" && r.sess != nil {
		switch {
		case r.pendingSnapshot != "":
			snap = r.pendingSnapshot
		case len(r.tr.turns) > 0:
			var st snapState
			snap, st = r.makeSnapshot("route " + target)
			if st == snapHeld {
				r.lastRotFail = time.Now()
				r.p(dim+"── switch held: handoff failed ──"+rst, "\n")
				return fmt.Errorf("handoff held")
			}
		}
	}
	if snap != "" {
		snap = r.withDesignPath(target, snap)
	}
	oldModel := r.a.Model
	old, parked := r.parkOld()
	var err error
	if live != nil {
		r.bindChild(live)
		r.reg.Upsert(clientRec{Cwd: r.a.Cwd, Model: target, SessionID: sessID(live.Sess), State: clientLive,
			LastAPI: live.lastAPI(), LastCtx: live.LastCtx, OwnerPID: ownPID(), ChildPID: childPID(live.Sess)})
		return nil
	}
	r.a.Model = target
	r.resumeID = resumeID
	r.pendingSnapshot = ""
	if resumeID == "" {
		r.pendingSnapshot = snap
	}
	if err = r.start(); err != nil {
		r.a.Model = oldModel
		r.resumeID = ""
		if old != nil && parked {
			if c, ok := r.children.Take(childKey(r.a.Cwd, old.Model)); ok {
				r.bindChild(c)
				r.reg.Upsert(clientRec{Cwd: r.a.Cwd, Model: old.Model, SessionID: sessID(c.Sess), State: clientLive,
					LastAPI: c.lastAPI(), LastCtx: c.LastCtx, OwnerPID: ownPID(), ChildPID: childPID(c.Sess)})
			}
		} else if old != nil {
			r.pendingSnapshot = snap // old child was closed: start it again with the snapshot
			_ = r.start()
		}
		r.p(dim+"chaude: cannot start claude: "+err.Error()+rst, "\n")
		return err
	}
	r.reg.Upsert(clientRec{Cwd: r.a.Cwd, Model: target, SessionID: sessID(r.sess), State: clientLive, OwnerPID: ownPID(), ChildPID: childPID(r.sess)})
	return nil
}

// isDesignPlanPath matches an opus design file: .../notes/runs/*-plan-*.md, not a review.
func isDesignPlanPath(fp string) bool {
	if !strings.Contains(fp, "/notes/runs/") {
		return false
	}
	b := fp[strings.LastIndexByte(fp, '/')+1:]
	return strings.HasSuffix(b, ".md") && strings.Contains(b, "-plan-") && !strings.Contains(b, "-review")
}

// recordDesignPath remembers the last design file the opus client wrote (other models and other files are ignored).
func (r *Repl) recordDesignPath(model, fp string) {
	if r.reg == nil || model != "opus" || !isDesignPlanPath(fp) {
		return
	}
	rec, ok := r.reg.Get(r.a.Cwd, model)
	if !ok {
		rec = clientRec{Cwd: r.a.Cwd, Model: model, State: clientLive, SessionID: sessID(r.sess), OwnerPID: ownPID(),
			ChildPID: childPID(r.sess), LastAPI: time.Now()}
	}
	rec.DesignPath = fp
	r.reg.Upsert(rec)
}

// withDesignPath appends the recorded design file of model to snap. A missing or empty file is skipped
// (metrics only; no user error).
func (r *Repl) withDesignPath(model, snap string) string {
	rec, ok := r.reg.Get(r.a.Cwd, model)
	if !ok || rec.DesignPath == "" {
		return snap
	}
	if st, err := os.Stat(rec.DesignPath); err != nil || st.Size() == 0 {
		_ = r.metrics.Log(map[string]any{"stage": "design_path", "missing": true, "path": rec.DesignPath})
		return snap
	}
	return snap + "\n\n설계 파일: " + rec.DesignPath + " (먼저 읽고 이어서 진행)"
}
