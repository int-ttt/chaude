package harness

import (
	"fmt"
	"time"
)

// idleDue reports whether the active child's prompt cache has gone cold: it fires once per lastAPI value,
// and only after a turn happened (lastAPI is zero for a fresh child).
func (r *Repl) idleDue(now time.Time) bool {
	r.clockMu.Lock()
	defer r.clockMu.Unlock()
	ttl := r.ttlSeen
	if ttl == "" {
		ttl = r.ttl
	}
	deadline := idleDeadline(r.idleRotate, ttl)
	if !expiredZero(deadline) { // expiredZero: "this deadline can expire"; idle expiry off -> never due
		return false
	}
	if r.lastAPI.IsZero() || now.Sub(r.lastAPI) < deadline {
		return false
	}
	if r.idleFiredFor.Equal(r.lastAPI) {
		return false
	}
	r.idleFiredFor = r.lastAPI
	return true
}

// sweepIdle ends parked children that died or went cold and reports whether the active child
// should be rotated (the caller then sends the internal "/idle-rotate" line). UI goroutine, not busy.
// A parked child with delegated agents still running is left alone until they finish.
func (r *Repl) sweepIdle(now time.Time) bool {
	if r.children != nil {
		deadline := idleDeadline(r.idleRotate, r.effTTL())
		type kc struct {
			k string
			c *liveChild
		}
		r.children.mu.Lock()
		list := make([]kc, 0, len(r.children.m))
		for k, c := range r.children.m {
			list = append(list, kc{k, c})
		}
		r.children.mu.Unlock()
		for _, e := range list {
			busy := e.c.Tracker != nil && len(e.c.Tracker.running()) > 0
			ttl := deadline
			if isExtraCoder(e.c.Role) && (ttl <= 0 || extraCoderTTL < ttl) {
				ttl = extraCoderTTL
			}
			switch {
			case childDead(e.c):
				if c, ok := r.children.Take(e.k); ok {
					go c.Sess.Close()
					r.reg.MarkEnded(r.a.Cwd, c.Model, "child-exit", now)
				}
			case ttl > 0 && !busy && now.Sub(e.c.lastAPI()) >= ttl:
				if c, ok := r.children.Take(e.k); ok {
					go c.Sess.Close()
					r.reg.MarkEnded(r.a.Cwd, c.Model, "expired", now)
					go r.p(dim+"── parked "+c.Model+" session expired (cache cold) ──"+rst, "\n") // Send from Update would deadlock
				}
			}
		}
	}
	return r.sess != nil && r.idleDue(now)
}

// idleRotateNow rotates the active child because its cache went cold. Runs on the sender goroutine.
// rotate keeps an existing pendingSnapshot (snapshot_kept) instead of replacing it.
func (r *Repl) idleRotateNow() {
	if r.sess == nil {
		return
	}
	d := idleDeadline(r.idleRotate, r.effTTL())
	r.reg.MarkEnded(r.a.Cwd, r.a.Model, "expired", time.Now())
	r.p(dim+"── idle: cache cold, rotating ──"+rst, "\n")
	r.rotate(fmt.Sprintf("idle %ds", int(d/time.Second)))
}
