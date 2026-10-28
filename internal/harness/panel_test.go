package harness

import (
	"fmt"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestSessionPanelAndThinking(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80}
	if len(m.panelLines()) != 0 || strings.Contains(m.render(), "생각중...") {
		t.Fatal("idle: no panel, no indicator")
	}
	now := time.Now()
	r.tracker.onToolUse("a1", "Agent", map[string]any{"description": "fix bug", "prompt": "\nFirst line\nsecond"}, now)
	pl := m.panelLines()
	if len(pl) != 1 || !strings.Contains(pl[0], "fix bug") || !strings.Contains(pl[0], "First line") || strings.Contains(pl[0], "second") {
		t.Fatalf("panel: %q", pl)
	}
	if !strings.Contains(m.render(), "생각중...") {
		t.Fatal("thinking indicator missing while agent runs")
	}
	r.tracker.onToolResult("a1")
	if len(m.panelLines()) != 0 || strings.Contains(m.render(), "생각중...") {
		t.Fatal("agent done: panel and indicator must disappear")
	}
}

func TestEscInterruptClearsForegroundPanel(t *testing.T) {
	r := &Repl{tracker: newAgentTracker(), intr: make(chan struct{}, 1)}
	m := &tuiModel{r: r, w: 80}
	r.tracker.onToolUse("a1", "Agent", map[string]any{"description": "x"}, time.Now())
	r.tracker.onToolUse("b1", "Agent", map[string]any{"description": "bg", "run_in_background": true}, time.Now())
	m.interrupt()
	pl := m.bgLines()
	if len(m.panelLines()) != 0 || len(pl) != 1 || !strings.Contains(pl[0], "bg") {
		t.Fatalf("after esc only bg agent should remain: %q", pl)
	}
	r.tracker.onToolUse("a2", "Agent", map[string]any{"description": "y"}, time.Now())
	m.Update(doneMsg{}) // turn ended with no tool_result
	if pl := m.panelLines(); len(pl) != 0 || len(m.bgLines()) != 1 {
		t.Fatalf("after doneMsg: %q", pl)
	}
}

func TestIdleTickRefreshesWhenBackgroundFinishes(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	r.tracker.onToolUse("b1", "Agent", map[string]any{"description": "bg", "run_in_background": true}, time.Now())
	m.Update(spinner.TickMsg{})
	if strings.Contains(m.vp.View(), "생각중...") || !strings.Contains(m.View(), "bg") {
		t.Fatal("bg only: no indicator, bg row visible")
	}
	r.tracker.onNotification("<task-notification><tool-use-id>b1</tool-use-id></task-notification>")
	m.Update(spinner.TickMsg{})
	if m.lastRun != 0 {
		t.Fatalf("tick must call refresh (lastRun=%d)", m.lastRun)
	}
	if m.bgH != 0 || m.vp.Height != 30-m.inH-3-m.popH {
		t.Fatalf("tick must relayout: bgH=%d vp.Height=%d", m.bgH, m.vp.Height)
	}
	if strings.Contains(m.View(), "● bg") {
		t.Fatal("bg row must vanish on idle tick after bg agent finished")
	}
}

func TestCursorRowMatchesPanelAfterTick(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	m.Update(spinner.TickMsg{})
	base := m.curUp.Load()
	r.tracker.onToolUse("f1", "Agent", map[string]any{"description": "fg"}, time.Now())
	m.View() // layout changes inside the frame, before any key
	if m.panelH == 0 || m.curUp.Load() != base+int32(m.panelH) {
		t.Fatalf("curUp=%d want %d (panelH=%d)", m.curUp.Load(), base+int32(m.panelH), m.panelH)
	}
	if got := lipgloss.Height(m.View()); got > 30 {
		t.Fatalf("view height %d", got)
	}
	r.tracker.onToolResult("f1")
	m.Update(spinner.TickMsg{})
	if m.curUp.Load() != base || m.vp.Height != 30-m.inH-3-m.popH {
		t.Fatalf("after tick curUp=%d want %d vp=%d", m.curUp.Load(), base, m.vp.Height)
	}
}

func TestPanelMoreRowAndSmallHeight(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80}
	now := time.Now()
	for i := 0; i < 5; i++ {
		r.tracker.onToolUse(fmt.Sprintf("a%d", i), "Agent", map[string]any{"description": fmt.Sprintf("t%d", i)}, now.Add(time.Duration(i)))
	}
	pl := m.panelLines()
	if len(pl) != 4 || !strings.Contains(pl[3], "+2 more") || !strings.Contains(pl[2], "t2") {
		t.Fatalf("want 3 rows + '+2 more': %q", pl)
	}
	for _, h := range []int{8, 10, 12, 14, 20} {
		m2 := newTUIModel(r, make(chan string, 1))
		m2.Update(tea.WindowSizeMsg{Width: 80, Height: h})
		if got := lipgloss.Height(m2.View()); got > h {
			t.Fatalf("h=%d: view height %d overflows", h, got)
		}
	}
}

func TestThinkingOnlyForForeground(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80}
	r.tracker.onToolUse("b1", "Agent", map[string]any{"description": "bg", "run_in_background": true}, time.Now())
	if strings.Contains(m.render(), "생각중...") {
		t.Fatal("(a) background only: no thinking")
	}
	r.tracker.onToolUse("a1", "Agent", map[string]any{"description": "fg"}, time.Now())
	if !strings.Contains(m.render(), "생각중...") {
		t.Fatal("(b) foreground running: thinking expected")
	}
}

func TestBackgroundRowsBelowViewport(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	r.tracker.onToolUse("b1", "Agent", map[string]any{"description": "bgjob", "run_in_background": true}, time.Now())
	ls := strings.Split(m.View(), "\n")
	bgi, boxi := -1, -1
	for i, l := range ls {
		if strings.Contains(l, "bgjob") {
			bgi = i
		}
		if strings.Contains(l, "╭") {
			boxi = i
		}
	}
	if bgi != m.vp.Height || boxi <= bgi {
		t.Fatalf("(c) bg row at %d, viewport height %d, input box at %d", bgi, m.vp.Height, boxi)
	}
}

func TestBackgroundSmallHeightNoOverflow(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	now := time.Now()
	for i := 0; i < 6; i++ {
		r.tracker.onToolUse(fmt.Sprintf("b%d", i), "Agent", map[string]any{"description": "b", "run_in_background": true}, now.Add(time.Duration(i)))
		r.tracker.onToolUse(fmt.Sprintf("f%d", i), "Agent", map[string]any{"description": "f"}, now.Add(time.Duration(i)))
	}
	for _, h := range []int{6, 8, 10, 12, 14, 20} {
		m := newTUIModel(r, make(chan string, 1))
		m.Update(tea.WindowSizeMsg{Width: 80, Height: h})
		if got := lipgloss.Height(m.View()); got > h {
			t.Fatalf("(d) h=%d: view height %d overflows", h, got)
		}
	}
}

func TestStalledOutputFileKeepsAgentRunningUntilDoneSignal(t *testing.T) {
	dir := t.TempDir()
	p := filepath.Join(dir, "notes", "runs", "x.md")
	os.MkdirAll(filepath.Dir(p), 0o755)
	r := &Repl{tracker: newAgentTracker()}
	m := &tuiModel{r: r, w: 80}
	// outPathRe needs a relative notes/runs path; run from dir
	wd, _ := os.Getwd()
	os.Chdir(dir)
	defer os.Chdir(wd)
	r.tracker.onToolUse("b1", "Agent", map[string]any{"description": "bg", "run_in_background": true, "prompt": "write notes/runs/x.md"}, time.Now())
	os.WriteFile("notes/runs/x.md", []byte("partial"), 0o644)
	old := time.Now().Add(-time.Hour)
	os.Chtimes("notes/runs/x.md", old, old)
	if len(r.tracker.running()) != 1 || len(m.bgLines()) != 1 {
		t.Fatal("stalled file alone must not finish a running agent")
	}
	r.tracker.onToolUse("f1", "Agent", map[string]any{"description": "fg"}, time.Now())
	if !strings.Contains(m.render(), "생각중...") {
		t.Fatal("thinking indicator must stay while foreground agent runs")
	}
	os.WriteFile("notes/runs/x.md", []byte("result\n<!-- agent:done -->"), 0o644)
	os.Chtimes("notes/runs/x.md", old, old)
	if len(m.bgLines()) != 1 {
		t.Fatal("file marker must not finish the agent")
	}
	r.tracker.onNotification("<task-notification><tool-use-id>b1</tool-use-id></task-notification>")
	if len(m.bgLines()) != 0 {
		t.Fatal("done signal must remove the row")
	}
	r.tracker.onToolResult("f1")
	if strings.Contains(m.render(), "생각중...") {
		t.Fatal("indicator must vanish after completion")
	}
}

func queueModel(t *testing.T) (*tuiModel, chan string) {
	r := &Repl{tracker: newAgentTracker()}
	sub := make(chan string, 8)
	m := newTUIModel(r, sub)
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	return m, sub
}

func typeEnter(m *tuiModel, s string) {
	m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(s)})
	m.Update(tea.KeyMsg{Type: tea.KeyEnter})
}

func TestQueueWhileBusyNotSent(t *testing.T) {
	m, sub := queueModel(t)
	typeEnter(m, "first")
	if got := <-sub; got != "first" || !m.busy {
		t.Fatalf("idle send: %q busy=%v", got, m.busy)
	}
	typeEnter(m, "second")
	select {
	case got := <-sub:
		t.Fatalf("sent while busy: %q", got)
	default:
	}
	if len(m.queue) != 1 || m.queue[0] != "second" || m.in.Value() != "" {
		t.Fatalf("queue: %q input=%q", m.queue, m.in.Value())
	}
}

func TestQueueSentOnDoneInOrder(t *testing.T) {
	m, sub := queueModel(t)
	typeEnter(m, "first")
	<-sub
	typeEnter(m, "second")
	typeEnter(m, "third")
	if len(m.queue) != 2 || m.queue[0] != "second" || m.queue[1] != "third" {
		t.Fatalf("queue order: %q", m.queue)
	}
	m.Update(doneMsg{})
	if got := <-sub; got != "second" || !m.busy || len(m.queue) != 1 {
		t.Fatalf("after 1st done: %q busy=%v q=%q", got, m.busy, m.queue)
	}
	m.Update(doneMsg{})
	if got := <-sub; got != "third" || len(m.queue) != 0 {
		t.Fatalf("after 2nd done: %q q=%q", got, m.queue)
	}
	m.Update(doneMsg{})
	if m.busy || len(sub) != 0 {
		t.Fatal("empty queue: stay idle")
	}
}

func TestQueueLinesDimAboveInput(t *testing.T) {
	lipgloss.SetColorProfile(termenv.ANSI256)
	m, sub := queueModel(t)
	typeEnter(m, "first")
	<-sub
	typeEnter(m, "later prompt")
	v := m.View()
	want := dimSt.Render("  대기 later prompt")
	if !strings.Contains(v, want) || !strings.Contains(want, "\x1b[") {
		t.Fatalf("queue line not dim-styled:\n%q", v)
	}
	lines := strings.Split(v, "\n")
	qi, bi := -1, -1
	for i, l := range lines {
		if strings.Contains(l, "대기 later prompt") {
			qi = i
		}
		if strings.Contains(l, "╭") {
			bi = i
		}
	}
	if qi < 0 || bi != qi+1 {
		t.Fatalf("queue line %d must sit directly above input box %d", qi, bi)
	}
}

func TestBackgroundOnlyDoesNotQueue(t *testing.T) {
	m, sub := queueModel(t)
	m.r.tracker.onToolUse("b1", "Agent", map[string]any{"description": "bg", "run_in_background": true}, time.Now())
	typeEnter(m, "now")
	if got := <-sub; got != "now" || len(m.queue) != 0 {
		t.Fatalf("bg only must send directly: %q q=%q", got, m.queue)
	}
}

func TestQueueSmallHeightNoOverflow(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	for _, h := range []int{8, 10, 12, 20} {
		m := newTUIModel(r, make(chan string, 1))
		m.Update(tea.WindowSizeMsg{Width: 80, Height: h})
		m.queue = []string{"a", "b", "c", "d", "e", "f", "g"}
		if got := lipgloss.Height(m.View()); got > h {
			t.Fatalf("h=%d: view height %d overflows", h, got)
		}
	}
}
