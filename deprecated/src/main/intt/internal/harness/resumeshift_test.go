package harness

import (
	"io"
	"strings"
	"testing"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
)

func cursorColFor(t *testing.T, term, override string) int32 {
	t.Helper()
	t.Setenv("TERMINAL_EMULATOR", term)
	t.Setenv("CHAUDE_IME_SHIFT", override)
	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	m.Update(spinner.TickMsg{})
	m.in.Insert([]rune("가나"))
	m.syncCursor()
	return m.curCol.Load()
}

// JediTerm draws IME preedit text at cursorX+1 (TerminalPanel.drawInputMethodUncommitedChars), so the
// uncommitted syllable shows one cell right of where it lands; the real cursor must sit one cell left.
func TestJediTermIMEColumnShift(t *testing.T) {
	def := cursorColFor(t, "", "")
	if def != 1+2+4+1 {
		t.Fatalf("default col %d, want %d", def, 1+2+4+1)
	}
	if got := cursorColFor(t, "JetBrains-JediTerm", ""); got != def-1 {
		t.Fatalf("JediTerm col %d, want %d", got, def-1)
	}
	if got := cursorColFor(t, "JetBrains-JediTerm", "0"); got != def {
		t.Fatalf("override 0: col %d, want %d", got, def)
	}
}

// /resume (picker -> /import) must show the imported conversation, not only a status note.
func TestImportShowsTurnsInTUI(t *testing.T) {
	r := &Repl{tracker: newAgentTracker()}
	m := newTUIModel(r, make(chan string, 1))
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	m.blocks = []block{{"user", "/import abc", false}}
	m.Update(importedMsg{turns: [][2]string{{"첫 질문", "첫 답변"}, {"둘째 질문", "둘째 답변"}}})
	var got []string
	for _, b := range m.blocks {
		got = append(got, b.kind+":"+b.text)
	}
	s := strings.Join(got, "|")
	for _, w := range []string{"user:첫 질문", "asst:첫 답변", "user:둘째 질문", "asst:둘째 답변"} {
		if !strings.Contains(s, w) {
			t.Fatalf("blocks %q missing %q", s, w)
		}
	}
	if strings.Contains(s, "/import abc") {
		t.Fatalf("old screen kept: %q", s)
	}
}

// After an import the current child has seen nothing; a model switch (/model) must not
// ask it for a handoff (wasted turn, empty note) nor replace the import snapshot.
func TestRotateKeepsPendingSnapshot(t *testing.T) {
	old, _, oldIn := fakeSession()
	var started int
	newSessionFn = func(SessionOpts) (*Session, error) {
		started++
		s, _, _ := fakeSession()
		return s, nil
	}
	defer func() { newSessionFn = NewSession }()
	r := &Repl{out: io.Discard, a: Args{}, tr: &Transcript{path: t.TempDir() + "/t.jsonl"}, tracker: newAgentTracker(),
		sess: old, pendingSnapshot: "IMPORT SNAPSHOT", lastReco: ""}
	r.tr.turns = [][2]string{{"q", "a"}}
	done := make(chan struct{})
	go func() { r.rotate("route opus"); close(done) }()
	select {
	case <-done:
	case <-time.After(3 * time.Second):
		t.Fatal("rotate asked the fresh child for a handoff (blocked on its reply)")
	}
	select {
	case l := <-oldIn:
		t.Fatalf("handoff prompt sent to child: %.80s", l)
	default:
	}
	if r.pendingSnapshot != "IMPORT SNAPSHOT" {
		t.Fatalf("snapshot replaced: %.80q", r.pendingSnapshot)
	}
	if started != 1 || r.sess == old {
		t.Fatalf("new session not started (started=%d)", started)
	}
}
