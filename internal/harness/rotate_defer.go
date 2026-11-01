package harness

import (
	"fmt"
	"sort"
	"time"
)

// rotateDeferDue reports whether a rotation may proceed now: no background agent is running,
// deferral is off (timeout <= 0), or the wait since `since` reached the timeout.
func rotateDeferDue(running int, since, now time.Time, timeout time.Duration) bool {
	if running == 0 || timeout <= 0 {
		return true
	}
	return !since.IsZero() && now.Sub(since) >= timeout
}

// bgRunning: background agents of the current session that have not finished, oldest first.
func (t *agentTracker) bgRunning() []*agentRec {
	var l []*agentRec
	for _, a := range t.sortedRunning() {
		if a.Background {
			l = append(l, a)
		}
	}
	return l
}

// deferRotate holds a rotation while background agents run, for at most RotateDeferTimeout.
// Returns true when the rotation was held; rotate then returns without touching the session.
func (r *Repl) deferRotate(reason string) bool {
	bg := r.tracker.bgRunning()
	now := time.Now()
	r.deferMu.Lock()
	if rotateDeferDue(len(bg), r.deferSince, now, r.a.RotateDeferTimeout) {
		timedOut := len(bg) > 0 && !r.deferSince.IsZero()
		r.deferSince, r.deferredRot = time.Time{}, ""
		r.deferMu.Unlock()
		if timedOut {
			r.tr.add("rotate_defer", kv{"reason", reason}, kv{"running", len(bg)}, kv{"timed_out", true})
			r.p(dim+fmt.Sprintf("── rotation: defer timeout, %d background agent(s) still running ──", len(bg))+rst, "\n")
		}
		return false
	}
	first := r.deferSince.IsZero()
	if first {
		r.deferSince = now
	}
	r.deferredRot = reason
	r.deferMu.Unlock()
	if first {
		r.tr.add("rotate_defer", kv{"reason", reason}, kv{"running", len(bg)}, kv{"timed_out", false})
		r.p(dim+fmt.Sprintf("── rotation deferred: %d background agent(s) running ──", len(bg))+rst, "\n")
	}
	return true
}

// deferredDue reports whether a held rotation can run now (UI tick: no agent work, lock only).
func (r *Repl) deferredDue(now time.Time) bool {
	r.deferMu.Lock()
	pending, since := r.deferredRot != "", r.deferSince
	r.deferMu.Unlock()
	return pending && rotateDeferDue(len(r.tracker.bgRunning()), since, now, r.a.RotateDeferTimeout)
}

// runDeferredRotate runs the held rotation on the sender goroutine.
func (r *Repl) runDeferredRotate() {
	r.deferMu.Lock()
	reason := r.deferredRot
	r.deferredRot = ""
	r.deferMu.Unlock()
	if reason != "" {
		r.rotate(reason)
	}
}

// bgHandoffLines lists the session's background agents for the handoff note: running ones and
// finished ones with a written result, each with its result file path.
func (t *agentTracker) bgHandoffLines() []string {
	t.mu.Lock()
	var l []*agentRec
	for _, a := range t.agents {
		if a.Background && a.ResultPath != "" && (!a.Done || a.ResultDone) {
			l = append(l, a)
		}
	}
	t.mu.Unlock()
	sort.Slice(l, func(i, j int) bool {
		if l[i].Started.Equal(l[j].Started) {
			return l[i].ID < l[j].ID
		}
		return l[i].Started.Before(l[j].Started)
	})
	var out []string
	for _, a := range l {
		state := "running"
		if a.Done {
			state = "done"
		}
		name := a.Name
		if name == "" {
			name = a.Task
		}
		out = append(out, fmt.Sprintf("- %s (%s): %s, result file: %s", name, a.ID, state, a.ResultPath))
	}
	return out
}
