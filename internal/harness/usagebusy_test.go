package harness

import (
	"strings"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func TestUsageAllowedWhileBusy(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80, busy: true, submit: make(chan string, 4)}
	m.in.SetValue("/exit")
	m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if len(m.blocks) == 0 || m.blocks[len(m.blocks)-1].text != busySlashMsg {
		t.Fatalf("blocks=%v", m.blocks)
	}
	if busySlashMsg != "생각 중에는 /usage, /사용량 외의 명령을 사용할 수 없습니다. 답변이 끝난 뒤 다시 입력해 주세요." {
		t.Fatal("message text")
	}
	select {
	case s := <-m.submit:
		t.Fatalf("sent %q", s)
	case <-time.After(10 * time.Millisecond):
	}
	// /사용량 is local (no claude call): answered inline even while busy, nothing sent to the child
	m.in.SetValue("/사용량")
	m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if last := m.blocks[len(m.blocks)-1]; last.kind != "note" || !strings.Contains(last.text, "사용량 (구독 한도") {
		t.Fatalf("/사용량 while busy: %+v", last)
	}
	select {
	case s := <-m.submit:
		t.Fatalf("sent %q", s)
	case <-time.After(10 * time.Millisecond):
	}
}

func TestUsageSlashAcceptedWhileBusy(t *testing.T) {
	oldF, oldS := usageFetch, usageSend
	defer func() { usageFetch, usageSend = oldF, oldS }()
	usageFetch = func(int) string { return "Current session  45%" }
	got := make(chan tea.Msg, 1)
	usageSend = func(_ *Repl, msg tea.Msg) { got <- msg }
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80, busy: true, submit: make(chan string, 4)}
	m.in.SetValue("/usage")
	m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	select {
	case msg := <-got:
		if u, ok := msg.(usageMsg); !ok || string(u) != "Current session  45%" {
			t.Fatalf("msg=%v", msg)
		}
	case <-time.After(time.Second):
		t.Fatal("/usage not run while busy")
	}
	if len(m.queue) != 0 || len(m.submit) != 0 || m.in.Value() != "" {
		t.Fatalf("queue=%v submit=%d in=%q", m.queue, len(m.submit), m.in.Value())
	}
}

func TestUsageMsgWhileBusyOpensPopup(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80, busy: true, submit: make(chan string, 4)}
	m.Update(usageMsg("Current session  45%"))
	if m.usage == "" || !m.busy {
		t.Fatalf("usage=%q busy=%v", m.usage, m.busy)
	}
}
