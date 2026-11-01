package harness

import (
	"strings"
	"testing"

	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
)

// A selection is stored in content coordinates, so lines appended below (streaming output) and a
// different scroll offset must keep it on the same text.
func TestSelectionAnchorStaysOnContentAfterAppend(t *testing.T) {
	lines := []string{"alpha", "beta gamma", "delta"}
	sel := textSel{set: true, anchor: textPos{1, 0}, head: textPos{1, 4}}
	if got := sel.text(lines); got != "beta" {
		t.Fatalf("before append: got %q, want %q", got, "beta")
	}
	lines = append(lines, "streamed one", "streamed two", "streamed three")
	if got := sel.text(lines); got != "beta" {
		t.Fatalf("after append: got %q, want %q", got, "beta")
	}
	// highlight follows the content line, not the screen row: with yOffset 1 the selected line is row 0
	frame := strings.Join(lines[1:3], "\n")
	out := highlightView(frame, lines, 1, sel)
	rows := strings.Split(out, "\n")
	if !strings.Contains(rows[0], "\x1b[7m") || strings.Contains(rows[1], "\x1b[7m") {
		t.Fatalf("highlight on wrong row after scroll: %q", out)
	}
	// with yOffset 0 the same selection is row 1
	rows = strings.Split(highlightView(strings.Join(lines[0:2], "\n"), lines, 0, sel), "\n")
	if strings.Contains(rows[0], "\x1b[7m") || !strings.Contains(rows[1], "\x1b[7m") {
		t.Fatalf("highlight not on content line after scroll back: %v", rows)
	}
}

func TestSelectionMultiLineAndReversedOrder(t *testing.T) {
	lines := []string{"one", "two", "three"}
	sel := textSel{set: true, anchor: textPos{2, 2}, head: textPos{0, 1}}
	if got := sel.text(lines); got != "ne\ntwo\nth" {
		t.Fatalf("got %q", got)
	}
	if (textSel{set: true, anchor: textPos{0, 1}, head: textPos{0, 1}}).text(lines) != "" {
		t.Fatal("press and release on same cell must select nothing")
	}
}

// Releasing a non-empty drag copies the selected text through the clipboard writer and shows the note;
// a plain click (press and release on one cell) copies nothing.
func TestReleaseCopiesSelection(t *testing.T) {
	var copied []string
	m := &tuiModel{
		lines: []string{"alpha", "beta gamma"},
		vp:    viewport.New(80, 5),
		clip: func(s string) error {
			copied = append(copied, s)
			return nil
		},
	}
	press := tea.MouseMsg{X: 0, Y: 1, Button: tea.MouseButtonLeft, Action: tea.MouseActionPress}
	drag := tea.MouseMsg{X: 4, Y: 1, Button: tea.MouseButtonLeft, Action: tea.MouseActionMotion}
	release := tea.MouseMsg{X: 4, Y: 1, Button: tea.MouseButtonLeft, Action: tea.MouseActionRelease}
	m.selectMouse(press)
	m.selectMouse(drag)
	if len(copied) != 0 {
		t.Fatalf("copied before release: %q", copied)
	}
	ok, cmd := m.selectMouse(release)
	if !ok || cmd == nil {
		t.Fatalf("release: handled=%v cmd=%v, want handled with a status timer", ok, cmd)
	}
	if len(copied) != 1 || copied[0] != "beta" {
		t.Fatalf("copied = %q, want [beta]", copied)
	}
	if m.copyNote != "복사됨 (4자)" {
		t.Fatalf("copyNote = %q", m.copyNote)
	}
	m.selectMouse(tea.MouseMsg{X: 2, Y: 0, Button: tea.MouseButtonLeft, Action: tea.MouseActionPress})
	m.selectMouse(tea.MouseMsg{X: 2, Y: 0, Button: tea.MouseButtonLeft, Action: tea.MouseActionRelease})
	if len(copied) != 1 {
		t.Fatalf("plain click copied: %q", copied)
	}
}
