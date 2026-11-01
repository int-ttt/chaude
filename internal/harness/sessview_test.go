package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

// svModel: a TUI model whose Claude Code project dir (under a temp HOME) holds two sessions, newest first.
func svModel(t *testing.T) *tuiModel {
	home := t.TempDir()
	t.Setenv("HOME", home)
	cwd := "/work/proj"
	dir := filepath.Join(home, ".claude", "projects", "-work-proj")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	now := time.Now()
	writeSess(t, dir, "sessaaa1", now, "first prompt", "second prompt")
	writeSess(t, dir, "sessbbb2", now.Add(-time.Hour), "older prompt")
	r := &Repl{tracker: newAgentTracker()}
	r.a.Cwd = cwd
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	return m
}

func TestViewEscBacksOutToNormalScreen(t *testing.T) {
	m := svModel(t)
	m.Update(tea.KeyMsg{Type: tea.KeyEsc}) // no view open: esc must not break anything
	if m.sv != nil {
		t.Fatal("esc without /view must not open anything")
	}
	if msg := m.openSessView(""); msg != "" || m.sv == nil {
		t.Fatalf("/view did not open: %q", msg)
	}
	if v := m.View(); !strings.Contains(v, "Sessions") || !strings.Contains(v, "sessaaa1") || !strings.Contains(v, "● ") {
		t.Fatalf("session list not drawn below input:\n%s", v)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.sv != nil || strings.Contains(m.View(), "Sessions") {
		t.Fatal("esc in /view must return to the normal screen")
	}
}

func TestViewSessionSelectionAndPromptNavigation(t *testing.T) {
	m := svModel(t)
	m.openSessView("")
	m.Update(tea.KeyMsg{Type: tea.KeyDown}) // select the second session
	if m.sv.sel != 1 {
		t.Fatalf("arrow down did not select: sel=%d", m.sv.sel)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyUp})
	m.Update(tea.KeyMsg{Type: tea.KeyEnter}) // open the newest session
	v := m.View()
	if m.sv.turns == nil || !strings.Contains(v, "prompt 1/2") || !strings.Contains(v, "first prompt") {
		t.Fatalf("selected session prompts not shown:\n%s", v)
	}
	if strings.Contains(v, "Sessions") {
		t.Fatal("prompt level must not keep the session list")
	}
	m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if v := m.View(); !strings.Contains(v, "prompt 2/2") || !strings.Contains(v, "second prompt") {
		t.Fatalf("next prompt not shown:\n%s", v)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyEsc}) // level 2 -> level 1
	if m.sv == nil || m.sv.turns != nil || !strings.Contains(m.View(), "Sessions") {
		t.Fatal("esc from prompts must return to the session list")
	}
	m.Update(tea.KeyMsg{Type: tea.KeyEsc}) // level 1 -> normal screen
	if m.sv != nil {
		t.Fatal("second esc must leave /view")
	}
}

func TestViewArgOpensSessionDirectly(t *testing.T) {
	m := svModel(t)
	if msg := m.openSessView("sessbbb"); msg != "" || m.sv == nil || m.sv.turns == nil {
		t.Fatalf("/view <prefix> must open prompts: %q", msg)
	}
	if v := m.View(); !strings.Contains(v, "older prompt") {
		t.Fatalf("older session prompt missing:\n%s", v)
	}
	if msg := m.openSessView("nope"); msg == "" || m.sv != nil {
		t.Fatal("unmatched prefix must not open /view")
	}
}
