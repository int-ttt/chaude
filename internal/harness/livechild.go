package harness

import (
	"fmt"
	"strings"
	"time"
)

// touchMain updates the active child's clock from main-agent events only (subagent events are ignored).
func (r *Repl) touchMain(ev event, now time.Time) {
	if !isMainEvent(ev) {
		return
	}
	switch str(ev["type"]) {
	case "stream_event":
		if str(asMap(ev["event"])["type"]) == "message_start" {
			r.clockMu.Lock()
			r.lastAPI = now
			r.clockMu.Unlock()
		}
	case "assistant":
		if u := asMap(asMap(ev["message"])["usage"]); len(u) > 0 {
			r.clockMu.Lock()
			r.ttlSeen = observedTTL(u, r.ttlSeen)
			r.clockMu.Unlock()
		}
	}
}

// parkedIdleEvent handles events from a child that is parked-live: no rendering, no permission prompt.
func (r *Repl) parkedIdleEvent(c *liveChild, ev event) {
	if c.Tracker != nil {
		c.Tracker.onSubEvent(ev)
	}
	t := str(ev["type"])
	if k := residentRoleKey(c.Role); t == "assistant" && k != "" { // resident usage counts for its P/T/C slot, subagent or not
		if u := asMap(asMap(ev["message"])["usage"]); len(u) > 0 {
			r.setRoleCtx(k, ctxTokens(u))
		}
	}
	switch {
	case t == "control_request" && str(asMap(ev["request"])["subtype"]) == "can_use_tool":
		if s, ok := c.Sess.(*Session); ok && s != nil {
			req := asMap(ev["request"])
			if in := asMap(req["input"]); coderMayUse(c, r.a.Cwd, str(req["tool_name"]), in) {
				_ = s.RespondPermission(str(ev["request_id"]), true, in, "")
			} else {
				_ = s.RespondPermission(str(ev["request_id"]), false, nil, "denied: session parked (another model is active)")
			}
		}
	case t == "rate_limit_event":
		r.setRate(asMap(ev["rate_limit_info"]))
	case t == "stream_event" && isMainEvent(ev):
		e := asMap(ev["event"])
		switch str(e["type"]) {
		case "message_start":
			c.setLastAPI(time.Now())
		case "content_block_delta":
			if d := asMap(e["delta"]); str(d["type"]) == "text_delta" {
				c.mu.Lock()
				c.reply.WriteString(str(d["text"]))
				c.stream = true
				c.mu.Unlock()
			}
		}
	case t == "assistant" && isMainEvent(ev):
		c.mu.Lock()
		if !c.stream {
			for _, bi := range asList(asMap(ev["message"])["content"]) {
				if b := asMap(bi); str(b["type"]) == "text" {
					c.reply.WriteString(str(b["text"]))
				}
			}
		}
		c.mu.Unlock()
	case t == "user":
		if c.Tracker != nil && c.Tracker.onUserEvent(ev) {
			c.mu.Lock()
			c.sawNotif = true
			c.mu.Unlock()
		}
	case t == "result":
		c.mu.Lock()
		seen := c.sawNotif
		c.sawNotif = false
		txt := strings.TrimSpace(c.reply.String())
		c.reply.Reset()
		c.stream = false
		ask := c.await
		c.await = nil
		c.mu.Unlock()
		c.endBusy()
		if ask != nil {
			ask <- txt // sendResident caller; buffered, one value
		}
		if !seen && txt != "" && c.Tracker != nil {
			c.Tracker.finishOldestBackground() // same fallback as idleFinish: notice not visible on stdout
		}
		if s, ok := c.Sess.(*Session); ok && s != nil {
			if tc, ok := ev["total_cost_usd"].(float64); ok {
				r.budget.charge(s, tc)
			}
		}
		if txt != "" && ask == nil {
			r.addNotice(fmt.Sprintf("[parked %s background reply] %s", c.Model, trunc(txt, 300)))
		}
	}
}

func (r *Repl) addNotice(s string) {
	r.noticeMu.Lock()
	r.notices = append(r.notices, s)
	r.noticeMu.Unlock()
}

func (r *Repl) takeNotices() []string {
	r.noticeMu.Lock()
	defer r.noticeMu.Unlock()
	n := r.notices
	r.notices = nil
	return n
}

func (r *Repl) closeAllChildren() {
	if r.children != nil {
		r.children.CloseAll()
	}
}

// unbindActive detaches the active child into a liveChild record and clears the active slot.
// The caller parks or closes the returned child. Returns nil when there is no active session.
func (r *Repl) unbindActive() *liveChild {
	if r.sess == nil {
		return nil
	}
	c := r.cur
	if c == nil {
		c = &liveChild{Sess: r.sess, Tracker: r.tracker, Model: r.a.Model}
	}
	// Sess/Tracker/Model are set when the child is created (start) and are read by the idle callback: not rewritten here.
	c.parked.Store(true) // first: the idle callback must stop rendering before state moves
	c.TurnFrom = len(r.tr.turns)
	r.clockMu.Lock()
	c.setLastAPI(r.lastAPI)
	c.TTLSeen = r.ttlSeen
	r.clockMu.Unlock()
	c.LastCtx = r.lastCtx
	c.Files = r.files
	c.PendingSnapshot = r.pendingSnapshot
	c.LastRotFail = r.lastRotFail
	c.Cat = r.cat
	r.sess = nil
	r.pendingSnapshot = ""
	r.cur = nil
	if r.tracker == nil {
		r.tracker = newAgentTracker()
		r.tracker.notify = r.agentNotice
	}
	return c
}

// bindChild makes c the active child again, adding what happened while it was parked.
func (r *Repl) bindChild(c *liveChild) {
	s, _ := c.Sess.(*Session)
	r.sess = s
	r.tracker = c.Tracker
	if r.tracker == nil {
		r.tracker = newAgentTracker()
		r.tracker.notify = r.agentNotice
	}
	r.a.Model = c.Model
	r.clockMu.Lock()
	r.lastAPI = c.lastAPI()
	r.ttlSeen = c.TTLSeen
	r.clockMu.Unlock()
	r.lastCtx = c.LastCtx
	r.files = c.Files
	r.pendingSnapshot = c.PendingSnapshot
	r.lastRotFail = c.LastRotFail
	r.cat = c.Cat
	if c.TurnFrom < len(r.tr.turns) {
		limit := 6000
		if strings.Contains(c.Model, "haiku") {
			limit = 3000
		}
		var b strings.Builder
		b.WriteString("## While this session was parked\n")
		for _, t := range r.tr.turns[c.TurnFrom:] {
			b.WriteString("user: " + t[0] + "\nassistant: " + t[1] + "\n")
		}
		d := trunc(b.String(), limit)
		if r.pendingSnapshot != "" {
			r.pendingSnapshot += "\n\n"
		}
		r.pendingSnapshot += d
	}
	r.cur = c
	c.parked.Store(false) // last
}
