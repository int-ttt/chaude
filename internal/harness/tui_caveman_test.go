package harness

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
)

func TestIsCavemanBlock(t *testing.T) {
	cases := []struct {
		in   string
		want bool
	}{
		{"status: done\nreason: -\nclaim: a.go:1 | x", true},
		{"[harness: note]\nstatus: partial\nreason: cap", true},
		{"status: done", false},
		{"status: running\nreason: -", false},
		{"읽기 결과입니다.", false},
		{"", false},
	}
	for _, c := range cases {
		if got := isCavemanBlock(c.in); got != c.want {
			t.Errorf("isCavemanBlock(%q) = %v, want %v", c.in, got, c.want)
		}
	}
}

func TestToolResultCavemanHidden(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	count := func() int {
		n := 0
		for _, b := range m.blocks {
			if b.kind == "result" {
				n++
			}
		}
		return n
	}
	before := count()
	m.Update(toolResultMsg("status: done\nreason: -\nclaim: a.go:1 | x"))
	if got := count(); got != before {
		t.Errorf("caveman result shown: result blocks %d -> %d", before, got)
	}
	m.Update(toolResultMsg("ok\nmore"))
	if got := count(); got != before+1 {
		t.Errorf("plain result not shown: result blocks %d -> %d, want +1", before, got)
	}
}
