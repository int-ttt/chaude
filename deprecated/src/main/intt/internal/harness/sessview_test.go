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
	t.Setenv("USERPROFILE", home) // os.UserHomeDir reads USERPROFILE on Windows
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
	r.noteOwn(&Session{SessionID: "sessaaa1", Role: "coder"})
	r.noteOwn(&Session{SessionID: "sessbbb2"})
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
	if v := m.View(); !strings.Contains(v, "Sessions") || !strings.Contains(v, "coder") || strings.Contains(v, "sessaaa1") || !strings.Contains(v, "● ") {
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

// /view lists only the sessions this chaude process spawned; other Claude Code sessions on disk stay hidden.
func TestViewListsOnlyOwnSessions(t *testing.T) {
	m := svModel(t)
	m.r.owned = nil // no child of this process yet
	if _, msg := m.r.ownSessionList(); msg == "" {
		t.Fatal("no spawned child yet: /view must not list foreign sessions")
	}
	m.r.noteOwn(&Session{SessionID: "sessaaa1"}) // this process main child; sessbbb2 is another run
	l, msg := m.r.ownSessionList()
	if msg != "" || len(l) != 1 || l[0].ID != "sessaaa1" {
		t.Fatalf("own list = %d %q, want only sessaaa1", len(l), msg)
	}
	if msg := m.openSessView("sessbbb"); msg == "" || m.sv != nil {
		t.Fatalf("foreign session must not open: %q", msg)
	}
	if msg := m.openSessView(""); msg != "" || len(m.sv.list) != 1 {
		t.Fatalf("/view list = %d %q, want 1", len(m.sv.list), msg)
	}
	if v := m.View(); strings.Contains(v, "sessbbb2") {
		t.Fatalf("foreign session drawn in /view list:\n%s", v)
	}
}

// The /view session list takes the same wheel cursor as the /resume picker (wraps around).
func TestViewWheelMovesSessionCursor(t *testing.T) {
	m := svModel(t)
	m.r.noteOwn(&Session{SessionID: "sessaaa1"})
	m.r.noteOwn(&Session{SessionID: "sessbbb2"})
	m.openSessView("")
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelDown})
	if m.sv.sel != 1 {
		t.Fatalf("wheel down: sel=%d, want 1", m.sv.sel)
	}
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelDown})
	if m.sv.sel != 0 {
		t.Fatalf("wheel down past end: sel=%d, want 0 (wrap)", m.sv.sel)
	}
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelUp})
	if m.sv.sel != 1 {
		t.Fatalf("wheel up: sel=%d, want 1", m.sv.sel)
	}
}

func TestViewPromptScrollsInsideTranscript(t *testing.T) {
	m := svModel(t)
	m.openSessView("")
	m.Update(tea.KeyMsg{Type: tea.KeyEnter}) // open the first session: level 2
	m.sv.turns = []string{strings.Repeat("long prompt text ", 200)}
	m.sv.cur, m.sv.off = 0, 0
	last := len(m.svBody()) - max(1, m.vp.Height-1)
	if last <= 3 {
		t.Fatalf("prompt too short to scroll: body=%d lines", len(m.svBody()))
	}
	m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.sv.off != 1 {
		t.Fatalf("down: off=%d, want 1", m.sv.off)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyPgDown})
	if m.sv.off <= 1 {
		t.Fatalf("pgdown did not move: off=%d", m.sv.off)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyEnd})
	if m.sv.off != last {
		t.Fatalf("end: off=%d, want %d", m.sv.off, last)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.sv.off != last {
		t.Fatalf("down past end: off=%d, want clamp at %d", m.sv.off, last)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyHome})
	if m.sv.off != 0 {
		t.Fatalf("home: off=%d, want 0", m.sv.off)
	}
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelDown})
	if m.sv.off != 3 {
		t.Fatalf("wheel down: off=%d, want 3", m.sv.off)
	}
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelUp})
	if m.sv.off != 0 {
		t.Fatalf("wheel up: off=%d, want 0", m.sv.off)
	}
}
