package harness

import (
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

// Bare /resume and multi-hit queries open the record picker; a single hit loads directly.
func TestRecordPickCandidates(t *testing.T) {
	root := t.TempDir()
	at := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	writeProjectTurn(t, root, "alpha", "배포 방법", at, false)
	writeProjectTurn(t, root, "beta", "정규식 설명", at, false)
	writeProjectTurn(t, root, "gamma", "배포 체크리스트", at, false)

	if ps, err := recordPick(root, ""); err != nil || len(ps) != 3 {
		t.Fatalf("bare: %d projects err %v, want 3", len(ps), err)
	}
	if ps, err := recordPick(root, "배포"); err != nil || len(ps) != 2 {
		t.Fatalf("multi-hit: %d projects err %v, want 2", len(ps), err)
	}
	if ps, err := recordPick(root, "정규식"); err != nil || ps != nil {
		t.Fatalf("single hit: %v err %v, want nil (load directly)", ps, err)
	}
	if ps, err := recordPick(root, "2"); err != nil || ps != nil {
		t.Fatalf("number: %v err %v, want nil", ps, err)
	}
}

// Arrow keys and the wheel move the record picker selection; Enter loads that folder and closes it.
func TestRecordPickerKeysWheelEnter(t *testing.T) {
	root := t.TempDir()
	at := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	writeProjectTurn(t, root, "alpha", "첫 질문", at, false)
	writeProjectTurn(t, root, "beta", "둘째 질문", at, false)
	ps, err := recordPick(root, "")
	if err != nil || len(ps) != 2 {
		t.Fatalf("setup: %d projects err %v", len(ps), err)
	}
	m := newTUIModel(&Repl{tracker: newAgentTracker()}, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	m.rpick, m.rpickSel = ps, 0

	m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.rpickSel != 1 {
		t.Fatalf("down key: sel %d, want 1", m.rpickSel)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.rpickSel != 0 {
		t.Fatalf("down wraps: sel %d, want 0", m.rpickSel)
	}
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelDown})
	if m.rpickSel != 1 {
		t.Fatalf("wheel down: sel %d, want 1", m.rpickSel)
	}
	m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelUp})
	if m.rpickSel != 0 {
		t.Fatalf("wheel up: sel %d, want 0", m.rpickSel)
	}
	m.Update(tea.KeyMsg{Type: tea.KeyDown})
	m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if m.rpick != nil {
		t.Fatal("enter must close the picker")
	}
	// ps is newest first, so sel 1 is the older folder; its answer must be the one loaded.
	want := "첫 질문"
	if ps[1].Name == "beta" {
		want = "둘째 질문"
	}
	found := false
	for _, b := range m.blocks {
		if b.text == want {
			found = true
		}
	}
	if !found {
		t.Fatalf("enter did not load %s blocks (want %q): %+v", ps[1].Name, want, m.blocks)
	}

	m.rpick, m.rpickSel = ps, 0
	m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.rpick != nil {
		t.Fatal("esc must cancel the picker")
	}
}
