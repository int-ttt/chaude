package harness

import (
	"encoding/json"
	"fmt"
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
	AgentID     string // handle from the launch result ("agentId: <id>"); SendMessage resumes the agent by it
	Started     time.Time
	ResultPath  string // durable result file, relative to cwd: notes/runs/<YYMMDD>-agent-<id>.md
	ResultDone  bool   // the result block was appended to ResultPath (background agents)
	Done        bool
	Interrupted bool      // child gone before the agent finished (restored from disk or harness exit)
	Restored    bool      // loaded from the agent store, not started in this process
	Dismissed   bool      // restored agent hidden from the panel (still in the agent view)
	Summary     string    // last text line of the agent
	startMod    time.Time // output file mtime/size when the agent started
	startSize   int64
}

// doneLine: the one-line completion notice of a finished agent (TUI note).
func (a *agentRec) doneLine() string {
	kind, model := "[agent 완료]", a.Model
	if a.Failed {
		kind = "[agent 실패]"
	}
	if model == "" {
		model = "상속"
	}
	what := a.Summary
	if what == "" {
		what = lastSummary(a.Log)
	}
	if what == "" {
		what = a.Task
	}
	return trunc(strings.Join(strings.Fields(fmt.Sprintf("%s %s %s · %s", kind, model, a.Name, what)), " "), 120)
}

// markDoneLocked marks a done (caller holds t.mu and persists). Returns its notice only on the
// transition from running, so each agent is announced once.
func markDoneLocked(a *agentRec) string {
	if a.Done {
		return ""
	}
	a.Done = true
	return a.doneLine()
}

// agentTracker records delegations of one session.
type agentTracker struct {
	mu     sync.Mutex // turn goroutine writes, UI goroutine reads the status count
	agents map[string]*agentRec

	store      *agentStore
	cwd        string        // project dir: background results are appended under cwd/notes/runs (empty: off)
	parent     func() string // session id of the child that owns this tracker
	transcript func() string // that child's Claude Code transcript path
	notify     func(string)  // receives one completion line per finished agent (nil: silent)
}

func newAgentTracker() *agentTracker { return &agentTracker{agents: map[string]*agentRec{}} }

// emit delivers completion lines; called after t.mu is released (the sink may block on the UI loop).
func (t *agentTracker) emit(lines []string) {
	if t.notify == nil {
		return
	}
	for _, l := range lines {
		t.notify(l)
	}
}

// agentNotice shows a completion line as a TUI note (no-op outside the TUI). Sent from a goroutine:
// the notice can fire inside the bubbletea Update loop (doneMsg, interrupt), where a synchronous
// Send would wait on the loop it is running in.
func (r *Repl) agentNotice(s string) {
	if r.ui != nil {
		go r.ui.send(noteMsg(s))
	}
}

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
		Model: str(input["model"]), Name: sess, Background: bg, Started: now, ResultPath: agentResultRel(id, now)}
	if st, err := os.Stat(a.OutPath); a.OutPath != "" && err == nil {
		a.startMod, a.startSize = st.ModTime(), st.Size()
	}
	t.agents[id] = a
	t.persistLocked()
}

// onToolResult: foreground agents finish with their tool_result; background ones only "launch".
func (t *agentTracker) onToolResult(id string) { t.onToolResultText(id, "") }

var launchIDRe = regexp.MustCompile(`agentId:\s*([A-Za-z0-9_-]+)`)

// onToolResultText: an "Async agent launched" result means the agent runs in the background even
// when run_in_background was absent from the input (agent-definition default); it stays running and
// its agentId is kept for SendMessage. A SendMessage result that resumed an agent puts it back on the
// active list.
func (t *agentTracker) onToolResultText(id, text string) {
	t.mu.Lock()
	var lines []string
	if a := t.agents[id]; a != nil {
		if strings.Contains(text, "Async agent launched") {
			a.Background = true
			if m := launchIDRe.FindStringSubmatch(text); m != nil {
				a.AgentID = m[1]
			}
			t.persistLocked()
		} else if !a.Background {
			if l := markDoneLocked(a); l != "" {
				lines = append(lines, l)
			}
			t.persistLocked()
		}
	}
	t.resumeLocked(text, time.Now())
	t.mu.Unlock()
	t.emit(lines)
}

// resumeLocked (caller holds t.mu): a SendMessage result {"success":true,...,"resumedAgentId":"<id>"}
// restarts that agent (Done=false, Started=now). success:false changes nothing.
func (t *agentTracker) resumeLocked(text string, now time.Time) {
	if !strings.Contains(text, "resumedAgentId") {
		return
	}
	var r struct {
		Success        bool   `json:"success"`
		ResumedAgentID string `json:"resumedAgentId"`
	}
	if json.Unmarshal([]byte(strings.TrimSpace(text)), &r) != nil || !r.Success || r.ResumedAgentID == "" {
		return
	}
	for _, a := range t.agents {
		if a.AgentID != "" && (a.AgentID == r.ResumedAgentID || len(a.AgentID) >= 8 && strings.HasPrefix(r.ResumedAgentID, a.AgentID)) {
			a.Done, a.Started = false, now
			t.persistLocked()
			return
		}
	}
}

var notifIDRe = regexp.MustCompile(`<tool-use-id>([^<]+)</tool-use-id>`)

// onNotification marks the background agents named by a <task-notification> as done.
func (t *agentTracker) onNotification(text string) bool {
	if !strings.Contains(text, "<task-notification>") {
		return false
	}
	t.mu.Lock()
	found := false
	var lines []string
	var wrote []*agentRec
	for _, m := range notifIDRe.FindAllStringSubmatch(text, -1) {
		if a := t.agents[m[1]]; a != nil {
			if l := markDoneLocked(a); l != "" {
				lines = append(lines, l)
			}
			if a.Background && !a.ResultDone {
				wrote = append(wrote, a)
			}
			found = true
		}
	}
	if found {
		t.persistLocked()
	}
	t.mu.Unlock()
	t.emit(lines)
	t.writeResults(wrote, text)
	return found
}

// onUserEvent applies a "user" stream event: tool_results finish foreground agents and any
// <task-notification> (string content or text block) finishes the background agents it names.
// Reports whether a notification was seen.
func (t *agentTracker) onUserEvent(ev event) bool {
	seen := false
	msg := asMap(ev["message"])
	if c, ok := msg["content"].(string); ok && t.onNotification(c) {
		seen = true
	}
	for _, bi := range asList(msg["content"]) {
		b := asMap(bi)
		switch str(b["type"]) {
		case "tool_result":
			t.onToolResultText(str(b["tool_use_id"]), resultText(b["content"]))
		case "text":
			if t.onNotification(str(b["text"])) {
				seen = true
			}
		}
	}
	return seen
}

// finishForeground marks every foreground agent done (turn ended without tool_result: esc, error).
func (t *agentTracker) finishForeground() {
	t.mu.Lock()
	var lines []string
	for _, a := range t.agents {
		if !a.Background {
			if l := markDoneLocked(a); l != "" {
				lines = append(lines, l)
			}
		}
	}
	t.persistLocked()
	t.mu.Unlock()
	t.emit(lines)
}

// finishOldestBackground marks the longest-running background agent done (fallback when the
// completion notice itself was not visible on stdout).
func (t *agentTracker) finishOldestBackground() {
	t.mu.Lock()
	var old *agentRec
	for _, a := range t.agents {
		if !a.Done && a.Background && (old == nil || a.Started.Before(old.Started)) {
			old = a
		}
	}
	var lines []string
	var wrote []*agentRec
	if old != nil {
		if l := markDoneLocked(old); l != "" {
			lines = append(lines, l)
		}
		if !old.ResultDone {
			wrote = append(wrote, old)
		}
		t.persistLocked()
	}
	t.mu.Unlock()
	t.emit(lines)
	t.writeResults(wrote, "")
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

// opusRunning: any running agent on the opus model (name prefix or model field).
func (t *agentTracker) opusRunning() bool {
	for _, a := range t.running() {
		if strings.HasPrefix(a.Name, "opus-") || a.Model == "opus" {
			return true
		}
	}
	return false
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

func isCoder(a *agentRec) bool { return strings.Contains(a.Name, "coder") }

// countRunningCoders: running agents whose name contains "coder" (status line C1..C4).
func (t *agentTracker) countRunningCoders() int {
	n := 0
	for _, a := range t.sortedRunning() {
		if isCoder(a) {
			n++
		}
	}
	return n
}

// roleSlot maps a running agent to its status-line slot: "P" prompter, "T" tui, "C" coder;
// "" for unknown, finished or other agents.
func (t *agentTracker) roleSlot(id string) string {
	if id == "" {
		return ""
	}
	for _, a := range t.sortedRunning() {
		if a.ID != id {
			continue
		}
		switch {
		case strings.Contains(a.Name, "prompter"):
			return "P"
		case strings.Contains(a.Name, "tui"):
			return "T"
		case isCoder(a):
			return "C"
		}
		return ""
	}
	return ""
}
