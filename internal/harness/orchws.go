// Package harness: mode f workspace routing. A "[ws:<id>]" prefix pins a line to a
// Haiku session kept per workspace id; untagged lines go to the main model.
package harness

import (
	"regexp"
	"time"
)

// wsRe matches a leading "[ws:<id>]" tag (id: 1-32 of [A-Za-z0-9_-]).
var wsRe = regexp.MustCompile(`^\[ws:([A-Za-z0-9_-]{1,32})\]\s*`)

// parseWS returns the workspace id and the rest of the line; ok is false when there is no valid tag.
func parseWS(line string) (id, rest string, ok bool) {
	m := wsRe.FindStringSubmatch(line)
	if m == nil {
		return "", line, false
	}
	return m[1], line[len(m[0]):], true
}

// wsState is the per-workspace Haiku session record kept by mode f.
type wsState struct {
	SessionID string
	Turns     int
	LastUsed  time.Time
	Ctx       int
	LastText  string
}

const (
	wsMaxTurns   = 20
	wsMaxCtx     = 40000
	wsMaxIdle    = 270 * time.Second
	wsHandoffMax = 1500
)

// wsNeedsRotate reports whether a resumable workspace session must be replaced by a fresh one.
func wsNeedsRotate(st *wsState, now time.Time) bool {
	if st == nil || st.SessionID == "" {
		return false
	}
	return st.Turns >= wsMaxTurns || st.Ctx >= wsMaxCtx || now.Sub(st.LastUsed) > wsMaxIdle
}

// wsHandoff carries the last reply of a rotated workspace into its fresh session (rune-capped).
func wsHandoff(id string, st *wsState) string {
	text := ""
	if st != nil {
		text = st.LastText
	}
	return trunc("[ws:"+id+" handoff] "+text, wsHandoffMax)
}

// orchSwitch moves the active child to model (resuming worker when set).
func (r *Repl) orchSwitch(model, worker string) {
	if r.reg != nil && r.children != nil {
		r.routeChildren(model, "", worker)
		return
	}
	if worker != "" || model != r.a.Model {
		r.switchSession(model, worker)
	}
}

// orchFresh starts a new session on model even when it is already active.
// Non-children mode uses switchSession(model, "") (closes and restarts the child).
// Children mode goes through routeChildren, which is a no-op when model is already active.
func (r *Repl) orchFresh(model string) {
	if r.reg != nil && r.children != nil {
		r.routeChildren(model, "", "")
		return
	}
	r.switchSession(model, "")
}

// routeOrch routes a line in orch modes other than m0. Only mode f routes; b-e keep the main model.
func (r *Repl) routeOrch(line string) string {
	if r.a.Orch != "f" {
		return line
	}
	if r.orchMain == "" {
		r.orchMain = r.a.Model
	}
	id, rest, ok := parseWS(line)
	if !ok {
		r.curWS = ""
		if r.a.Model != r.orchMain {
			r.orchSwitch(r.orchMain, "")
		}
		return line
	}
	if r.ws == nil {
		r.ws = map[string]*wsState{}
	}
	st := r.ws[id]
	now := time.Now()
	r.curWS = id
	if st != nil && st.SessionID != "" && !wsNeedsRotate(st, now) {
		r.p(dim+"[ws "+id+" → haiku (resume)]"+rst, "\n")
		r.orchSwitch("haiku", st.SessionID)
		return rest
	}
	handoff := ""
	if st != nil && st.SessionID != "" {
		handoff = wsHandoff(id, st)
	}
	r.ws[id] = &wsState{LastUsed: now}
	r.p(dim+"[ws "+id+" → haiku (new)]"+rst, "\n")
	r.orchFresh("haiku")
	if handoff != "" {
		return handoff + "\n\n" + rest
	}
	return rest
}

// wsRecord stores the finished turn's session, context and reply for the current workspace.
func (r *Repl) wsRecord(reply string) {
	if r.a.Orch != "f" || r.curWS == "" || r.ws == nil {
		return
	}
	st := r.ws[r.curWS]
	if st == nil {
		st = &wsState{}
		r.ws[r.curWS] = st
	}
	if r.sess != nil {
		st.SessionID = r.sess.SessionID
	}
	st.Turns++
	st.LastUsed = time.Now()
	st.Ctx = r.lastCtx
	st.LastText = reply
}
