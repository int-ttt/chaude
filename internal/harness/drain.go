package harness

import (
	"os"
	"regexp"
	"sort"
	"strings"
	"sync"
	"time"
)

var outPathRe = regexp.MustCompile(`notes/runs/[^\s"'` + "`" + `)]+\.md`)

// agentRec is one delegated agent seen via tool_use (name Agent/Task).
type agentRec struct {
	ID          string
	Task        string
	Prompt      string // first line of the prompt handed to the agent
	Model       string // `model` input of the Agent call (empty: inherits the session model)
	Name        string // `name` input, else subagent_type: the session name shown in the panel
	Failed      bool
	Log         []string // live transcript lines of the sub-agent (text, tool calls, results)
	OutPath     string
	Background  bool
	Started     time.Time
	Done        bool
	Interrupted bool      // child gone before the agent finished (restored from disk or harness exit)
	Restored    bool      // loaded from the agent store, not started in this process
	Dismissed   bool      // restored agent hidden from the panel (still in the agent view)
	Summary     string    // last text line of the agent
	startMod    time.Time // output file mtime/size when the agent started
	startSize   int64
}

// agentTracker records delegations of one session.
type agentTracker struct {
	mu     sync.Mutex // turn goroutine writes, UI goroutine reads the status count
	agents map[string]*agentRec

	store      *agentStore
	parent     func() string // session id of the child that owns this tracker
	transcript func() string // that child's Claude Code transcript path
}

func newAgentTracker() *agentTracker { return &agentTracker{agents: map[string]*agentRec{}} }

func (t *agentTracker) onToolUse(id, name string, input map[string]any, now time.Time) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if name != "Agent" && name != "Task" {
		return
	}
	if _, ok := t.agents[id]; ok {
		return
	}
	p := str(input["prompt"])
	bg, _ := input["run_in_background"].(bool)
	sess := str(input["name"])
	if sess == "" {
		sess = str(input["subagent_type"])
	}
	a := &agentRec{ID: id, Task: str(input["description"]), Prompt: firstLine(p), OutPath: outPathRe.FindString(p),
		Model: str(input["model"]), Name: sess, Background: bg, Started: now}
	if st, err := os.Stat(a.OutPath); a.OutPath != "" && err == nil {
		a.startMod, a.startSize = st.ModTime(), st.Size()
	}
	t.agents[id] = a
	t.persistLocked()
}

// onToolResult: foreground agents finish with their tool_result; background ones only "launch".
func (t *agentTracker) onToolResult(id string) { t.onToolResultText(id, "") }

// onToolResultText: an "Async agent launched" result means the agent runs in the background even
// when run_in_background was absent from the input (agent-definition default); it stays running.
func (t *agentTracker) onToolResultText(id, text string) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if a := t.agents[id]; a != nil && !a.Background && strings.Contains(text, "Async agent launched") {
		a.Background = true
		t.persistLocked()
		return
	}
	if a := t.agents[id]; a != nil && !a.Background {
		a.Done = true
		t.persistLocked()
	}
}

var notifIDRe = regexp.MustCompile(`<tool-use-id>([^<]+)</tool-use-id>`)

// onNotification marks the background agents named by a <task-notification> as done.
func (t *agentTracker) onNotification(text string) bool {
	if !strings.Contains(text, "<task-notification>") {
		return false
	}
	t.mu.Lock()
	defer t.mu.Unlock()
	found := false
	for _, m := range notifIDRe.FindAllStringSubmatch(text, -1) {
		if a := t.agents[m[1]]; a != nil {
			a.Done, found = true, true
		}
	}
	if found {
		t.persistLocked()
	}
	return found
}

// finishForeground marks every foreground agent done (turn ended without tool_result: esc, error).
func (t *agentTracker) finishForeground() {
	t.mu.Lock()
	defer t.mu.Unlock()
	for _, a := range t.agents {
		if !a.Background {
			a.Done = true
		}
	}
	t.persistLocked()
}

// finishOldestBackground marks the longest-running background agent done (fallback when the
// completion notice itself was not visible on stdout).
func (t *agentTracker) finishOldestBackground() {
	t.mu.Lock()
	defer t.mu.Unlock()
	var old *agentRec
	for _, a := range t.agents {
		if !a.Done && a.Background && (old == nil || a.Started.Before(old.Started)) {
			old = a
		}
	}
	if old != nil {
		old.Done = true
		t.persistLocked()
	}
}

func (t *agentTracker) running() []*agentRec {
	t.mu.Lock()
	defer t.mu.Unlock()
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

// counts returns running foreground and background agents (for the status line).
func (t *agentTracker) counts() (fg, bg int) {
	for _, a := range t.running() {
		if a.Background {
			bg++
		} else {
			fg++
		}
	}
	return
}

func firstLine(s string) string {
	for _, l := range strings.Split(s, "\n") {
		if l = strings.TrimSpace(l); l != "" {
			return l
		}
	}
	return ""
}

// sortedRunning: running agents, oldest first (stable order for the session panel).
func (t *agentTracker) sortedRunning() []*agentRec {
	l := t.running()
	sort.Slice(l, func(i, j int) bool {
		if l[i].Started.Equal(l[j].Started) {
			return l[i].ID < l[j].ID
		}
		return l[i].Started.Before(l[j].Started)
	})
	return l
}
