package harness

import (
	"strings"
	"testing"
	"time"
)

func TestResidentRunningListShowsOnlyBusySessions(t *testing.T) {
	r := residentRepl(t, 8)
	s, _, _ := fakeSession()
	c := &liveChild{Sess: s, Tracker: newAgentTracker(), Model: "haiku", Role: "tui"}
	c.setLastAPI(time.Now())
	r.children.Park(childKey(r.a.Cwd, "haiku", "tui"), c)
	m := &tuiModel{r: r, w: 80}

	// idle resident: no row, no 생각중 indicator
	if len(m.panelLines()) != 0 || strings.Contains(m.render(), "생각중-") {
		t.Fatal("idle resident session must not be listed")
	}

	// busy resident: one row in the panel, 생각중 indicator shown, tick sees the change
	c.beginBusy("review the diff", time.Now())
	pl := m.panelLines()
	if len(pl) != 1 || !strings.Contains(pl[0], "tui") || !strings.Contains(pl[0], "review the diff") {
		t.Fatalf("busy resident row: %q", pl)
	}
	if !strings.Contains(m.render(), "생각중-") {
		t.Fatal("thinking indicator missing while resident session handles a command")
	}
	if m.runningN() != 1 {
		t.Fatalf("runningN = %d, want 1", m.runningN())
	}

	// command finished: row and indicator removed
	c.endBusy()
	if len(m.panelLines()) != 0 || strings.Contains(m.render(), "생각중-") {
		t.Fatal("finished resident session must drop out of the running list")
	}
	if m.runningN() != 0 {
		t.Fatalf("runningN after finish = %d, want 0", m.runningN())
	}
}
