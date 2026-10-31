package harness

import (
	"strings"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func avModel(t *testing.T) (*tuiModel, *Repl) {
	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	r.tracker.onToolUse("a1", "Agent", map[string]any{"description": "fix bug", "name": "worker", "prompt": "do it"}, time.Now())
	r.tracker.onSubEvent(event{"type": "assistant", "parent_tool_use_id": "a1", "message": map[string]any{
		"content": []any{map[string]any{"type": "text", "text": "working on SUBLINE"}}}})
	return m, r
}

func TestAgentViewClickReadOnlyClose(t *testing.T) {
	m, _ := avModel(t)
	// find the panel row on screen and click it
	y := -1
	for i, l := range strings.Split(m.View(), "\n") {
		if strings.Contains(l, "worker") {
			y = i
		}
	}
	if y < 0 {
		t.Fatalf("panel row not rendered: %q", m.View())
	}
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonLeft, X: 5, Y: y})
	if m.av == nil || !strings.Contains(m.View(), "SUBLINE") {
		t.Fatalf("click did not open the live view: %v", m.View())
	}
	for _, k := range []tea.KeyMsg{{Type: tea.KeyRunes, Runes: []rune("hi")}, {Type: tea.KeyEnter}} {
		m.Update(k)
	}
	if m.in.Value() != "" || len(m.queue) != 0 || m.av == nil {
		t.Fatalf("read-only view accepted input: %q", m.in.Value())
	}
	m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.av != nil {
		t.Fatal("esc must close")
	}
	m.Update(tea.KeyMsg{Type: tea.KeyCtrlT})
	if m.av == nil {
		t.Fatal("ctrl+t must open")
	}
	m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("q")})
	if m.av != nil || m.in.Value() != "" {
		t.Fatal("q must close without typing")
	}
}

func TestUsagePopupOpenClose(t *testing.T) {
	m, _ := avModel(t)
	m.Update(usageMsg("Current session  45%"))
	if !strings.Contains(m.View(), "Current session") {
		t.Fatal("popup not shown")
	}
	m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("x")})
	if m.usage == "" || m.in.Value() != "" {
		t.Fatal("popup must swallow input")
	}
	m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.usage != "" || strings.Contains(m.View(), "Current session") {
		t.Fatal("esc must close popup")
	}
}

func TestWheelScrollHoldsPosition(t *testing.T) {
	m, _ := avModel(t)
	for i := 0; i < 80; i++ {
		m.blocks = append(m.blocks, block{"note", "line", false})
	}
	m.refresh()
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelUp, X: 5, Y: 2})
	off := m.vp.YOffset
	if m.vp.AtBottom() || !m.held {
		t.Fatal("wheel up must hold")
	}
	m.appendAsst("more")
	m.refresh()
	if m.vp.YOffset != off || !m.unseen {
		t.Fatal("update must keep offset and flag unseen")
	}
	m.vp.GotoBottom()
	m.refresh()
	if m.held || !m.vp.AtBottom() {
		t.Fatal("back at bottom must resume follow")
	}
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelUp, X: 5, Y: 2})
	m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("a")})
	if m.held || !m.vp.AtBottom() {
		t.Fatal("typing must resume follow")
	}
}

func TestScrollBottomOnInputAndUpdate(t *testing.T) {
	m, _ := avModel(t)
	for i := 0; i < 80; i++ {
		m.blocks = append(m.blocks, block{"note", "line", false})
	}
	m.refresh()
	if !m.vp.AtBottom() {
		t.Fatal("refresh must follow the tail")
	}
	m.vp.GotoTop()
	m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("a")})
	if !m.vp.AtBottom() {
		t.Fatal("typing must scroll to bottom")
	}
	m.vp.GotoBottom()
	m.appendAsst("more")
	m.refresh()
	if !m.vp.AtBottom() {
		t.Fatal("update must scroll to bottom")
	}
	m.vp.GotoTop()
	m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if !m.vp.AtBottom() {
		t.Fatal("submit must scroll to bottom")
	}
	// agent view keeps its own scroll; closing returns body to bottom
	m.Update(tea.KeyMsg{Type: tea.KeyCtrlT})
	m.av.off = 2
	m.refresh()
	if m.av == nil || m.av.off != 2 {
		t.Fatal("refresh must not reset agent view scroll")
	}
	m.vp.GotoTop()
	m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.av != nil || !m.vp.AtBottom() {
		t.Fatal("closing view must put body at bottom")
	}
}
