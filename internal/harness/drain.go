package harness

import (
	"os"
	"regexp"
	"time"
)

var outPathRe = regexp.MustCompile(`notes/runs/[^\s"'` + "`" + `)]+\.md`)

// agentRec is one delegated agent seen via tool_use (name Agent/Task).
type agentRec struct {
	ID         string
	Task       string
	OutPath    string
	Background bool
	Started    time.Time
	Done       bool
}

// agentTracker records delegations of one session.
type agentTracker struct{ agents map[string]*agentRec }

func newAgentTracker() *agentTracker { return &agentTracker{agents: map[string]*agentRec{}} }

func (t *agentTracker) onToolUse(id, name string, input map[string]any, now time.Time) {
	if name != "Agent" && name != "Task" {
		return
	}
	if _, ok := t.agents[id]; ok {
		return
	}
	p := str(input["prompt"])
	bg, _ := input["run_in_background"].(bool)
	t.agents[id] = &agentRec{ID: id, Task: str(input["description"]), OutPath: outPathRe.FindString(p),
		Background: bg, Started: now}
}

// onToolResult: foreground agents finish with their tool_result; background ones only "launch".
func (t *agentTracker) onToolResult(id string) {
	if a := t.agents[id]; a != nil && !a.Background {
		a.Done = true
	}
}

// pollFiles marks background agents done when the output file is non-empty and unchanged for quiet.
func (t *agentTracker) pollFiles(now time.Time, quiet time.Duration) {
	for _, a := range t.agents {
		if a.Done || a.OutPath == "" {
			continue
		}
		st, err := os.Stat(a.OutPath)
		if err != nil || st.Size() == 0 {
			continue
		}
		if now.Sub(st.ModTime()) >= quiet {
			a.Done = true
		}
	}
}

func (t *agentTracker) running() []*agentRec {
	var l []*agentRec
	for _, a := range t.agents {
		if !a.Done {
			l = append(l, a)
		}
	}
	return l
}

// drain is an old session kept alive until its agents finish (or time out).
type drain struct {
	sess    *Session
	tr      *agentTracker
	since   time.Time
	timeout time.Duration
}

func (d *drain) finished(now time.Time) (done, timedOut bool) {
	d.tr.pollFiles(now, 2*time.Second)
	if len(d.tr.running()) == 0 {
		return true, false
	}
	if now.Sub(d.since) >= d.timeout {
		return true, true
	}
	return false, false
}

// consume drains the old child's stdout (so it never blocks), auto-denies permission requests,
// and reports every event to log. Returns when the child's stdout closes.
func (d *drain) consume(log func(kind string, ev event)) {
	for line := range d.sess.lines {
		ev := parseEvent(line)
		if str(ev["type"]) == "control_request" && str(asMap(ev["request"])["subtype"]) == "can_use_tool" {
			_ = d.sess.RespondPermission(str(ev["request_id"]), false, nil, "denied: session is draining")
			log("drain_deny", ev)
			continue
		}
		log("drain_event", ev)
	}
}
