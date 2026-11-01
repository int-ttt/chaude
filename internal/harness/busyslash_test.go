package harness

import (
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

// While an answer runs: /version and /usage (and other read-only/UI commands) run now;
// conversation-affecting commands (/resume, /clear, ...) are refused and never sent to the session.
func TestBusySlashAllowedAndRefused(t *testing.T) {
	oldF, oldS := usageFetch, usageSend
	defer func() { usageFetch, usageSend = oldF, oldS }()
	usageFetch = func(int) string { return "Current session  45%" }
	got := make(chan tea.Msg, 1)
	usageSend = func(_ *Repl, msg tea.Msg) { got <- msg }

	allowed := []string{"/version", "/usage", "/help", "/mode"}
	refused := []string{"/resume", "/clear", "/rotate", "/model sonnet", "/cost"}

	for _, c := range allowed {
		r := &Repl{tracker: newAgentTracker()}
		m := &tuiModel{r: r, w: 80, busy: true, submit: make(chan string, 4)}
		m.in.SetValue(c)
		m.Update(tea.KeyMsg{Type: tea.KeyEnter})
		if n := len(m.blocks); n > 0 && m.blocks[n-1].text == busySlashMsg {
			t.Fatalf("%s refused while busy", c)
		}
		if len(m.submit) != 0 || len(m.queue) != 0 {
			t.Fatalf("%s: submit=%d queue=%v", c, len(m.submit), m.queue)
		}
		if c == "/usage" {
			select {
			case <-got:
			case <-time.After(time.Second):
				t.Fatal("/usage not run while busy")
			}
		}
	}

	for _, c := range refused {
		r := &Repl{tracker: newAgentTracker()}
		m := &tuiModel{r: r, w: 80, busy: true, submit: make(chan string, 4)}
		m.in.SetValue(c)
		m.Update(tea.KeyMsg{Type: tea.KeyEnter})
		if n := len(m.blocks); n == 0 || m.blocks[n-1].text != busySlashMsg {
			t.Fatalf("%s not refused while busy: blocks=%v", c, m.blocks)
		}
		if len(m.submit) != 0 || len(m.queue) != 0 {
			t.Fatalf("%s: submit=%d queue=%v", c, len(m.submit), m.queue)
		}
	}
}

func TestVersionNoteShowsChaudeVersion(t *testing.T) {
	old := ChaudeVersion
	defer func() { ChaudeVersion = old }()
	ChaudeVersion = "9.9.9"
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80, submit: make(chan string, 4)}
	m.in.SetValue("/version")
	m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	n := len(m.blocks)
	if n == 0 || m.blocks[n-1].kind != "note" || m.blocks[n-1].text != "chaude 9.9.9 · Claude Code "+claudeVersion() {
		t.Fatalf("blocks=%v", m.blocks)
	}
}
