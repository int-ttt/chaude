package harness

import (
	"strings"
	"testing"
	"time"
)

func TestSpecterIdleNoElapsed(t *testing.T) {
	r := &Repl{tracker: newAgentTracker(), a: Args{Model: "opus"}}
	m := &tuiModel{r: r, w: 80, busy: false, turnStart: time.Now().Add(-125 * time.Second), lastTool: "Read(x)"}
	s := m.mainStatusText()
	if strings.Contains(s, " · 2m") {
		t.Fatalf("elapsed shown while idle: %q", s)
	}
	if strings.Contains(s, "Read(x)") {
		t.Fatalf("lastTool shown while idle: %q", s)
	}
}

func TestSpecterRowRenders(t *testing.T) {
	r := &Repl{tracker: newAgentTracker(), a: Args{Model: "opus"}}
	m := &tuiModel{r: r, w: 80, busy: true, turnStart: time.Now().Add(-125 * time.Second), lastTool: "Read(x)"}
	s := m.mainStatusText()
	if !strings.Contains(s, "2m05s") {
		t.Fatalf("elapsed missing: %q", s)
	}
	if !strings.Contains(s, "Read(x)") {
		t.Fatalf("lastTool missing: %q", s)
	}
}
