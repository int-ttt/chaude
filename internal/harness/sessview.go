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
	turns []string // user prompts of the chosen session; nil while at level 1
	cur   int
}

// openSessView: "/view" lists the Claude Code sessions below the input box; "/view <id-prefix>" opens
// that session's prompts directly. Returns a message to show when nothing opened.
func (m *tuiModel) openSessView(arg string) string {
	l, msg := m.r.sessionList()
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
	m.sv.id, m.sv.turns, m.sv.cur = s.ID, prompts, 0
	return ""
}

// closeSessView leaves /view and returns the transcript to the tail (scroll-bottom rule).
func (m *tuiModel) closeSessView() {
	m.sv = nil
	m.vp.GotoBottom()
}

// svKey: esc backs out one level (prompts -> list -> normal screen); arrows select or step through prompts.
func (m *tuiModel) svKey(k tea.KeyMsg) {
	v := m.sv
	if v.turns != nil {
		switch k.String() {
		case "left", "up", "k", "pgup":
			v.cur = max(0, v.cur-1)
		case "right", "down", "j", "pgdown":
			v.cur = min(len(v.turns)-1, v.cur+1)
		case "home":
			v.cur = 0
		case "end":
			v.cur = len(v.turns) - 1
		case "esc":
			v.turns, v.cur = nil, 0
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
		t := fmt.Sprintf("%s%s  %s", mark, shortID(s.ID), s.label())
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
	head := lipgloss.NewStyle().Foreground(orange).Render(fmt.Sprintf("Session %s  prompt %d/%d", shortID(v.id), v.cur+1, len(v.turns))) +
		dimSt.Render("  ←/→ prev/next · esc back")
	cw := max(10, m.w-2)
	all := strings.Split(lipgloss.NewStyle().Width(cw).Render(v.turns[v.cur]), "\n")
	body := all
	if room := max(1, hh-1); len(all) > room {
		body = append(append([]string(nil), all[:room-1]...), dimSt.Render(fmt.Sprintf("… (+%d lines)", len(all)-(room-1))))
	}
	l := append([]string{ansiTrunc(head, m.w)}, body...)
	for len(l) < hh {
		l = append(l, "")
	}
	return strings.Join(l[:hh], "\n")
}
