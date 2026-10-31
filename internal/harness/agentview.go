package harness

import (
	"fmt"
	"sort"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// maxAgentLog caps the live transcript kept per agent (oldest lines drop first).
const maxAgentLog = 400

type usageMsg string // /usage output: shown in a popup instead of the transcript

// onSubEvent records one stream event of a sub-agent (parent_tool_use_id = the Agent call id) into
// that agent's live transcript. Main-thread events and unknown parents are ignored.
func (t *agentTracker) onSubEvent(ev event) {
	p, _ := ev["parent_tool_use_id"].(string)
	if p == "" {
		return
	}
	var add []string
	switch str(ev["type"]) {
	case "assistant":
		for _, bi := range asList(asMap(ev["message"])["content"]) {
			b := asMap(bi)
			switch str(b["type"]) {
			case "text":
				if s := strings.TrimSpace(str(b["text"])); s != "" {
					add = append(add, strings.Split(s, "\n")...)
				}
			case "tool_use":
				n := str(b["name"])
				if a := toolSummary(asMap(b["input"])); a != "" {
					n += "(" + a + ")"
				}
				add = append(add, "● "+n)
			}
		}
	case "user":
		for _, bi := range asList(asMap(ev["message"])["content"]) {
			if b := asMap(bi); str(b["type"]) == "tool_result" {
				s := strings.TrimSpace(resultText(b["content"]))
				l := strings.Split(s, "\n")
				x := "  ⎿  " + trunc(l[0], 160)
				if len(l) > 1 {
					x += fmt.Sprintf(" … (+%d lines)", len(l)-1)
				}
				add = append(add, x)
			}
		}
	}
	if len(add) == 0 {
		return
	}
	t.mu.Lock()
	defer t.mu.Unlock()
	if a := t.agents[p]; a != nil {
		a.Log = append(a.Log, add...)
		if n := len(a.Log); n > maxAgentLog {
			a.Log = append([]string(nil), a.Log[n-maxAgentLog:]...)
		}
	}
}

// snapshot copies an agent's record and log under the lock (nil if unknown).
func (t *agentTracker) snapshot(id string) *agentRec {
	t.mu.Lock()
	defer t.mu.Unlock()
	a := t.agents[id]
	if a == nil {
		return nil
	}
	c := *a
	c.Log = append([]string(nil), a.Log...)
	return &c
}

// sortedAll: every known agent, running ones first (oldest first), then finished ones.
func (t *agentTracker) sortedAll() []*agentRec {
	t.mu.Lock()
	var l []*agentRec
	for _, a := range t.agents {
		l = append(l, a)
	}
	t.mu.Unlock()
	sort.Slice(l, func(i, j int) bool {
		if l[i].Done != l[j].Done {
			return !l[i].Done
		}
		if l[i].Started.Equal(l[j].Started) {
			return l[i].ID < l[j].ID
		}
		return l[i].Started.Before(l[j].Started)
	})
	return l
}

// agentView: read-only live view of one agent (id into the tracker); off = lines scrolled up from the tail.
type agentView struct {
	id  string
	off int
}

func (m *tuiModel) openAgentView(id string) {
	m.usage = ""
	m.av = &agentView{id: id}
}

// openAgentKey: keyboard entry (ctrl+t): first running agent, else the newest known one.
func (m *tuiModel) openAgentKey() {
	l := m.r.tracker.sortedAll()
	if len(l) == 0 {
		m.blocks = append(m.blocks, block{"note", "no agents yet", false})
		m.refresh()
		return
	}
	m.openAgentView(l[0].ID)
}

// cycleAgent moves the view to the next/previous agent (tab / shift+tab).
func (m *tuiModel) cycleAgent(d int) {
	l := m.r.tracker.sortedAll()
	for i, a := range l {
		if a.ID == m.av.id {
			m.av = &agentView{id: l[(i+d+len(l))%len(l)].ID}
			return
		}
	}
}

// avKey: the view is read-only, so only close/scroll/switch keys do anything; everything else is dropped.
func (m *tuiModel) avKey(k tea.KeyMsg) {
	switch k.String() {
	case "esc", "q", "ctrl+c":
		m.av = nil
		m.vp.GotoBottom()
	case "tab", "right":
		m.cycleAgent(1)
	case "shift+tab", "left":
		m.cycleAgent(-1)
	case "up", "k":
		m.av.off++
	case "down", "j":
		m.av.off = max(0, m.av.off-1)
	case "pgup":
		m.av.off += max(1, m.vp.Height-2)
	case "pgdown":
		m.av.off = max(0, m.av.off-max(1, m.vp.Height-2))
	case "home":
		m.av.off = maxAgentLog + 1000
	case "end":
		m.av.off = 0
	}
}

// panelClick maps a left click at screen row y to the agent on that panel row (nil if none).
func (m *tuiModel) panelClick(y int) *agentRec {
	_, fa := m.rowsA(false, 0)
	_, ba := m.rowsA(true, len(fa))
	y0 := m.vp.Height + m.popH + m.queueH + m.inH + 2 // first panel row: transcript, suggestions, queue, input box
	i := y - y0
	all := append(append([]*agentRec{}, fa...), ba...)
	if i < 0 || i >= len(all) {
		return nil
	}
	return all[i]
}

func (m *tuiModel) click(v tea.MouseMsg) {
	if v.Action != tea.MouseActionPress || v.Button != tea.MouseButtonLeft || m.perm != nil || m.pick != nil || m.rw.open {
		return
	}
	if a := m.panelClick(v.Y); a != nil {
		m.openAgentView(a.ID)
	}
}

// agentViewLines renders the overlay (exactly hh lines) that replaces the transcript.
func (m *tuiModel) agentViewLines(hh int) string {
	a := m.r.tracker.snapshot(m.av.id)
	if a == nil {
		m.av = nil
		return ""
	}
	head := lipgloss.NewStyle().Foreground(orange).Render("Agent (read-only)") +
		dimSt.Render("  esc/q close · tab next · ↑/↓ pgup/pgdn scroll")
	title := ansiTrunc("  "+agentDot(agentState(a))+dimSt.Render(" "+agentStatusText(a, m.r.a.Model)), m.w)
	var body []string
	cw := max(10, m.w-2)
	for _, l := range a.Log {
		body = append(body, strings.Split(lipgloss.NewStyle().Width(cw).Render(l), "\n")...)
	}
	if len(body) == 0 {
		body = []string{dimSt.Render("(no output from this agent yet)")}
		if a.Prompt != "" {
			body = append(body, dimSt.Render("task: "+a.Prompt))
		}
	}
	room := max(1, hh-2)
	m.av.off = min(m.av.off, max(0, len(body)-room))
	end := len(body) - m.av.off
	from := max(0, end-room)
	l := append([]string{ansiTrunc(head, m.w), title}, body[from:end]...)
	for len(l) < hh {
		l = append(l, "")
	}
	return strings.Join(l[:hh], "\n")
}

// usagePopup renders the centered /usage box over a blank hh-line area.
func (m *tuiModel) usagePopup(hh int) string {
	body := lipgloss.NewStyle().Foreground(orange).Render("Usage") + dimSt.Render("  (esc/q/enter close)") + "\n\n" + m.usage
	box := lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(orange).Padding(0, 1).
		MaxWidth(max(10, m.w-2)).Render(body)
	return lipgloss.Place(m.w, hh, lipgloss.Center, lipgloss.Center, box)
}

func (m *tuiModel) usageKey(k tea.KeyMsg) {
	switch k.String() {
	case "esc", "q", "enter", "ctrl+c":
		m.usage = ""
		m.vp.GotoBottom()
	}
}

// persistLocked writes every tracked agent to the store (caller holds t.mu).
func (t *agentTracker) persistLocked() {
	if t.store == nil {
		return
	}
	var parent, tp string
	if t.parent != nil {
		parent = t.parent()
	}
	if t.transcript != nil {
		tp = t.transcript()
	}
	recs := make([]savedAgent, 0, len(t.agents))
	for _, a := range t.agents {
		if a.Restored {
			continue // stored by its own process (interrupted at that exit)
		}
		if a.Done && a.Summary == "" {
			a.Summary = lastSummary(a.Log)
		}
		recs = append(recs, a.saved(parent, tp))
	}
	t.store.put(recs)
}

// restore adds a stored agent as finished/interrupted (never running: its child is gone).
func (t *agentTracker) restore(s savedAgent) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if _, ok := t.agents[s.ID]; ok {
		return
	}
	a := &agentRec{ID: s.ID, Task: s.Task, Prompt: s.Prompt, Model: s.Model, Name: s.Name, OutPath: s.OutPath,
		Background: s.Background, Started: s.Started, Summary: s.Summary, Restored: true, Done: true,
		Failed: s.Status == stoFailed, Interrupted: s.Status == stoInterrupted || s.Status == stoRunning}
	if a.Interrupted && s.Summary != "" {
		a.Log = []string{"(restored) " + s.Summary}
	}
	t.agents[s.ID] = a
}

// interruptRunning marks unfinished agents interrupted and stores them (harness shutdown).
func (t *agentTracker) interruptRunning() {
	t.mu.Lock()
	defer t.mu.Unlock()
	for _, a := range t.agents {
		if !a.Done {
			a.Done, a.Interrupted = true, true
		}
	}
	t.persistLocked()
}

// sortedRestored: restored interrupted agents still shown in the panel, oldest first.
func (t *agentTracker) sortedRestored() []*agentRec {
	t.mu.Lock()
	var l []*agentRec
	for _, a := range t.agents {
		if a.Restored && a.Interrupted && !a.Dismissed {
			c := *a
			l = append(l, &c)
		}
	}
	t.mu.Unlock()
	sort.Slice(l, func(i, j int) bool {
		if l[i].Started.Equal(l[j].Started) {
			return l[i].ID < l[j].ID
		}
		return l[i].Started.Before(l[j].Started)
	})
	return l
}

// dismissRestored hides restored agents from the panel (the next user message was sent).
func (t *agentTracker) dismissRestored() {
	t.mu.Lock()
	defer t.mu.Unlock()
	for _, a := range t.agents {
		a.Dismissed = true
	}
}

// panelWheel scrolls the agent panels below the input box when the wheel is over them; reports whether it handled v.
func (m *tuiModel) panelWheel(v tea.MouseMsg) bool {
	d := 0
	switch v.Button {
	case tea.MouseButtonWheelUp:
		d = -1
	case tea.MouseButtonWheelDown:
		d = 1
	default:
		return false
	}
	if m.panelH+m.bgH == 0 || v.Y < m.vp.Height+m.popH+m.queueH+m.inH+2 {
		return false
	}
	m.panOff = max(0, min(m.panOff+d, len(m.r.tracker.running())+len(m.r.tracker.sortedRestored())))
	return true
}
