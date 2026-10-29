package harness

import (
	"strings"
	"testing"
)

func TestMergePromptsOneTurn(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80, busy: true, submit: make(chan string, 4)}
	m.queue = []string{"첫째", "둘째", "셋째"}
	m.Update(doneMsg{})
	if len(m.queue) != 0 || !m.busy {
		t.Fatalf("queue=%v busy=%v", m.queue, m.busy)
	}
	select {
	case got := <-m.submit:
		if got != "첫째\n\n둘째\n\n셋째" {
			t.Fatalf("merged=%q", got)
		}
	default:
		t.Fatal("nothing sent")
	}
	select {
	case got := <-m.submit:
		t.Fatalf("second turn sent: %q", got)
	default:
	}
}

func TestMainStatusRendering(t *testing.T) {
	r := &Repl{tracker: newAgentTracker(), a: Args{Model: "opus"}}
	m := &tuiModel{r: r, w: 80}
	if m.placeholder() != "대기 - 프롬프트 입력" {
		t.Fatalf("idle placeholder: %q", m.placeholder())
	}
	m.busy, m.curPrompt = true, "버그 고쳐줘\n자세히"
	if got := m.mainStatusText(); got != "생각중-opus-메인(버그 고쳐줘)" {
		t.Fatalf("status: %q", got)
	}
	if !strings.Contains(ansiStrip(m.render()), "생각중-opus-메인(버그 고쳐줘)") {
		t.Fatal("render missing status row")
	}
}
