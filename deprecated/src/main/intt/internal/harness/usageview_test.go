package harness

import (
	"fmt"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/x/ansi"
)

func usageModel(t *testing.T) *tuiModel {
	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	var rows []string
	for i := 0; i < 60; i++ {
		rows = append(rows, fmt.Sprintf("row%02d", i))
	}
	m.usage = strings.Join(rows, "\n")
	return m
}

func TestUsageViewScrollBounds(t *testing.T) {
	m := usageModel(t)
	room := m.usageRoom()
	maxTop := len(m.usageBody()) - room
	if maxTop <= 0 {
		t.Fatalf("test needs overflow: body=%d room=%d", len(m.usageBody()), room)
	}

	m.usageKey(tea.KeyMsg{Type: tea.KeyPgDown})
	if m.usageTop != min(room, maxTop) {
		t.Errorf("PgDown top = %d, want %d", m.usageTop, min(room, maxTop))
	}
	m.usageKey(tea.KeyMsg{Type: tea.KeyEnd})
	if m.usageTop != maxTop {
		t.Errorf("End top = %d, want %d", m.usageTop, maxTop)
	}
	m.usageKey(tea.KeyMsg{Type: tea.KeyDown})
	if m.usageTop != maxTop {
		t.Errorf("Down past end top = %d, want clamp %d", m.usageTop, maxTop)
	}
	m.usageKey(tea.KeyMsg{Type: tea.KeyPgUp})
	m.usageKey(tea.KeyMsg{Type: tea.KeyPgUp})
	m.usageKey(tea.KeyMsg{Type: tea.KeyPgUp})
	if m.usageTop != 0 {
		t.Errorf("PgUp past top = %d, want 0", m.usageTop)
	}
	m.usageKey(tea.KeyMsg{Type: tea.KeyUp})
	if m.usageTop != 0 {
		t.Errorf("Up at top = %d, want 0", m.usageTop)
	}

	// Rendered view is exactly vp.Height rows and shows the scrolled slice.
	m.usageKey(tea.KeyMsg{Type: tea.KeyEnd})
	out := ansi.Strip(m.usageLines(m.vp.Height))
	if n := len(strings.Split(out, "\n")); n != m.vp.Height {
		t.Errorf("usage view rows = %d, want %d", n, m.vp.Height)
	}
	if !strings.Contains(out, "row59") || strings.Contains(out, "row00") {
		t.Errorf("end view not showing tail:\n%s", out)
	}
}

func TestUsageViewWheelAndRefreshKeepScroll(t *testing.T) {
	m := usageModel(t)
	m.usageKey(tea.KeyMsg{Type: tea.KeyDown})
	m.usageScroll(m.usageTop + 5)
	top := m.usageTop
	if top == 0 {
		t.Fatal("scroll did not move")
	}
	// Refresh with same length content keeps the scroll position.
	m.Update(usageMsg(m.usage))
	if m.usageTop != top {
		t.Errorf("refresh reset scroll: %d -> %d", top, m.usageTop)
	}
	// Refresh with shorter content clamps instead of going out of range.
	m.Update(usageMsg("only\none\ntwo"))
	if m.usageTop != 0 {
		t.Errorf("short refresh top = %d, want 0", m.usageTop)
	}
}

func TestUsageViewEscCloses(t *testing.T) {
	m := usageModel(t)
	m.usageKey(tea.KeyMsg{Type: tea.KeyEsc})
	if m.usage != "" {
		t.Errorf("esc did not close usage view: %q", m.usage)
	}
}

func TestUsageOpenNotInline(t *testing.T) {
	oldF, oldS := usageFetch, usageSend
	defer func() { usageFetch, usageSend = oldF, oldS }()
	usageFetch = func(int) string { return "Current session  45%" }
	got := make(chan tea.Msg, 1)
	usageSend = func(_ *Repl, msg tea.Msg) { got <- msg }

	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	before := len(m.blocks)
	m.openUsage()
	if m.usage != "조회 중…" {
		t.Errorf("usage = %q, want loading text", m.usage)
	}
	if len(m.blocks) != before {
		t.Errorf("/usage added %d transcript block(s), want 0", len(m.blocks)-before)
	}
	<-got
}
