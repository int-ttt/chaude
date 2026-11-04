package harness

import (
	"fmt"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// sessActiveWindow: a session transcript written this recently is marked active (●).
const sessActiveWindow = 10 * time.Minute

// sessRows: session rows shown below the input box at once (scrolls around the selection).
const sessRows = 5

// sessView: /view state. Level 1 = session list below the input box (sel); level 2 = the user
// prompts of one session, one at a time (turns non-nil, cur = shown prompt). esc goes back one level.
type sessView struct {
	list  []sessionInfo
	sel   int
	id    string
	name  string   // agent name of the chosen session (shown instead of its id)
	turns []string // user prompts of the chosen session; nil while at level 1
	cur   int
	off   int // top body line of the shown prompt (↑/↓, pgup/pgdn, home/end, wheel)
}

// agentLabel: the name /view shows for a session: its agent (main, prompter, tui, coder…), else the
// transcript title. Never the session id.
func (s sessionInfo) agentLabel() string {
	switch {
	case s.Agent != "":
		return s.Agent
	case s.Name != "":
		return trunc(s.Name, 24)
	}
	return "session"
}

// noteOwn records a claude child this process started; its session id is known once it has spoken.
func (r *Repl) noteOwn(s *Session) {
	r.ownMu.Lock()
	defer r.ownMu.Unlock()
	r.owned = append(r.owned, s)
}

// filterOwn keeps the sessions whose id is in ids (order kept).
func filterOwn(l []sessionInfo, ids map[string]bool) []sessionInfo {
	var out []sessionInfo
	for _, s := range l {
		if ids[s.ID] {
			out = append(out, s)
		}
	}
	return out
}

// ownSessionList: the Claude Code sessions of this chaude process (main, rotations, resident slots).
// Sub-agents have no session file of their own; their events are in the parent session's transcript.
func (r *Repl) ownSessionList() ([]sessionInfo, string) {
	l, msg := r.sessionList()
	if msg != "" {
		return nil, msg
	}
	r.ownMu.Lock()
	ids := map[string]bool{}
	roles := map[string]string{}
	for id := range r.prevOwned {
		ids[id] = true
	}
	for _, s := range r.owned {
		if id := s.ID(); id != "" {
			ids[id] = true
			roles[id] = s.Role
		}
	}
	r.ownMu.Unlock()
	out := filterOwn(l, ids)
	for i := range out {
		out[i].Agent = roles[out[i].ID]
	}
	if len(out) == 0 {
		return nil, "no Claude Code sessions from this chaude session yet"
	}
	return out, ""
}

// openSessView: "/view" lists the Claude Code sessions below the input box; "/view <id-prefix>" opens
// that session's prompts directly. Returns a message to show when nothing opened.
func (m *tuiModel) openSessView(arg string) string {
	l, msg := m.r.ownSessionList()
	if msg != "" {
		return msg
	}
	m.sv = &sessView{list: l}
	if arg == "" {
		return ""
	}
	ms := matchSessions(l, arg)
	switch {
	case len(ms) == 0:
		m.sv = nil
		return "no session matching " + arg
	case len(ms) > 1:
		m.sv = nil
		return "ambiguous prefix, matches:\n" + formatSessionList(ms)
	}
	if msg := m.openSessPrompts(ms[0]); msg != "" {
		m.sv = nil
		return msg
	}
	return ""
}

// openSessPrompts moves to level 2 for session s (loads its user prompts). Returns an error message or "".
func (m *tuiModel) openSessPrompts(s sessionInfo) string {
	imp, err := readClaudeTranscript(s.Path)
	if err != nil {
		return "view: " + err.Error()
	}
	if len(imp.Turns) == 0 {
		return "no prompts in session " + shortID(s.ID)
	}
	prompts := make([]string, 0, len(imp.Turns))
	for _, t := range imp.Turns {
		prompts = append(prompts, t[0])
	}
	m.sv.id, m.sv.name, m.sv.turns, m.sv.cur, m.sv.off = s.ID, s.agentLabel(), prompts, 0, 0
	return ""
}

// wheelSel moves a picker cursor one row per wheel notch (wraps), shared by the /resume and /view pickers.
func wheelSel(sel *int, n int, b tea.MouseButton) {
	switch b {
	case tea.MouseButtonWheelUp:
		*sel = (*sel + n - 1) % n
	case tea.MouseButtonWheelDown:
		*sel = (*sel + 1) % n
	}
}

// closeSessView leaves /view and returns the transcript to the tail (scroll-bottom rule).
func (m *tuiModel) closeSessView() {
	m.sv = nil
	m.vp.GotoBottom()
}

// svBody: the shown prompt wrapped to the view width.
func (m *tuiModel) svBody() []string {
	return strings.Split(lipgloss.NewStyle().Width(max(10, m.w-2)).Render(m.sv.turns[m.sv.cur]), "\n")
}

// svScroll sets the top body line of the shown prompt, clamped so its last line can reach the bottom.
func (m *tuiModel) svScroll(top int) {
	m.sv.off = max(0, min(top, len(m.svBody())-max(1, m.vp.Height-1)))
}

// svKey: esc backs out one level (prompts -> list -> normal screen). Level 2: up/down/pgup/pgdn/home/end
// scroll the prompt text, left/right step through prompts.
func (m *tuiModel) svKey(k tea.KeyMsg) {
	v := m.sv
	if v.turns != nil {
		room := max(1, m.vp.Height-1)
		switch k.String() {
		case "left":
			v.cur, v.off = max(0, v.cur-1), 0
		case "right":
			v.cur, v.off = min(len(v.turns)-1, v.cur+1), 0
		case "up", "k":
			m.svScroll(v.off - 1)
		case "down", "j":
			m.svScroll(v.off + 1)
		case "pgup":
			m.svScroll(v.off - room)
		case "pgdown":
			m.svScroll(v.off + room)
		case "home":
			m.svScroll(0)
		case "end":
			m.svScroll(len(m.svBody()))
		case "esc":
			v.turns, v.cur, v.off = nil, 0, 0
		case "q", "ctrl+c":
			m.closeSessView()
		}
		return
	}
	n := len(v.list)
	switch k.String() {
	case "up", "k":
		v.sel = (v.sel + n - 1) % n
	case "down", "j":
		v.sel = (v.sel + 1) % n
	case "enter":
		if msg := m.openSessPrompts(v.list[v.sel]); msg != "" {
			m.blocks = append(m.blocks, block{"note", msg, false})
			m.refresh()
		}
	case "esc", "q", "ctrl+c":
		m.closeSessView()
	}
}

// sessLines: the session list drawn below the input box (same row style as the background panel).
// Empty at level 2, where the transcript area shows the prompts instead.
func (m *tuiModel) sessLines() []string {
	v := m.sv
	if v == nil || v.turns != nil || m.w < 10 {
		return nil
	}
	out := []string{ansiTrunc(lipgloss.NewStyle().Foreground(orange).Render("Sessions  (↑/↓ select · enter open · esc close)"), m.w)}
	from := max(0, min(v.sel-sessRows/2, len(v.list)-sessRows))
	for i := from; i < len(v.list) && i < from+sessRows; i++ {
		s := v.list[i]
		mark := "  "
		if time.Since(s.Mod) < sessActiveWindow {
			mark = "● "
		}
		t := fmt.Sprintf("%s%-12s  %s", mark, s.agentLabel(), s.label())
		if i == v.sel {
			t = lipgloss.NewStyle().Foreground(orange).Render("> " + t)
		} else {
			t = dimSt.Render("  " + t)
		}
		out = append(out, ansiTrunc(t, m.w))
	}
	return out
}

// svPromptLines renders level 2 as an overlay of exactly hh lines that replaces the transcript.
func (m *tuiModel) svPromptLines(hh int) string {
	v := m.sv
	body := m.svBody()
	m.svScroll(v.off) // window size or prompt may have changed
	room := max(1, hh-1)
	end := min(len(body), v.off+room)
	head := lipgloss.NewStyle().Foreground(orange).Render(fmt.Sprintf("%s  prompt %d/%d", v.name, v.cur+1, len(v.turns))) +
		dimSt.Render(fmt.Sprintf("  lines %d-%d/%d · ↑/↓ pgup/pgdn scroll · ←/→ prev/next · esc back", min(v.off+1, end), end, len(body)))
	l := append([]string{ansiTrunc(head, m.w)}, body[v.off:end]...)
	for len(l) < hh {
		l = append(l, "")
	}
	return strings.Join(l[:hh], "\n")
}
