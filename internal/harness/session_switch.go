package harness

import (
	"fmt"
	"os"
)

// switchSession replaces the child with a session on model, resuming resume (a session id) when set.
// The session being left is parked (its id is kept, no handoff) so a later switch back can resume it.
func (r *Repl) switchSession(model, resume string) {
	if r.sess != nil && r.a.Model != model {
		if id := r.sess.SessionID; id != "" {
			if r.parked == nil {
				r.parked = map[string]string{}
			}
			r.parked[r.a.Model] = id
		}
	}
	if r.sess != nil {
		r.sess.Close()
	}
	r.a.Model = model
	if resume != "" {
		r.pendingSnapshot = "" // resumed session already has its context
	}
	r.resumeID = resume
	if err := r.start(); err != nil {
		fmt.Fprintln(os.Stderr, "chaude: cannot start claude:", err)
	}
}
