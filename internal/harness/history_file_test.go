package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
)

func TestHistoryFileRoundTrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), ".chaude", "history.jsonl")
	if got := loadInputHistory(path); got != nil {
		t.Fatalf("missing file: %v", got)
	}
	in := []string{"one", "two\nlines", "/usage", "한글"}
	if err := saveInputHistory(path, in); err != nil {
		t.Fatal(err)
	}
	if got := loadInputHistory(path); strings.Join(got, "|") != strings.Join(in, "|") {
		t.Fatalf("round trip: %q", got)
	}
}

func TestHistoryFileLoadRules(t *testing.T) {
	path := filepath.Join(t.TempDir(), "history.jsonl")
	body := "\"a\"\nnot json\n\"a\"\n\"\"\n\"b\"\n"
	if err := os.WriteFile(path, []byte(body), 0o600); err != nil {
		t.Fatal(err)
	}
	if got := loadInputHistory(path); strings.Join(got, "|") != "a|b" {
		t.Fatalf("dedupe/skip: %q", got)
	}
	var many []string
	for i := 0; i < maxHistory+7; i++ {
		many = append(many, string(rune('a'+i%26))+string(rune('A'+i/26)))
	}
	if err := saveInputHistory(path, many); err != nil {
		t.Fatal(err)
	}
	got := loadInputHistory(path)
	if len(got) != maxHistory || got[len(got)-1] != many[len(many)-1] {
		t.Fatalf("cap: len=%d", len(got))
	}
}

func TestHistoryFileFor(t *testing.T) {
	if got, want := historyFileFor("/proj"), filepath.Join("/proj", ".chaude", "history.jsonl"); got != want {
		t.Fatal(got)
	}
}

func TestRecordHistoryPersistsAndNewModelLoads(t *testing.T) {
	dir := t.TempDir()
	r := &Repl{tracker: newAgentTracker()}
	r.a.Cwd = dir
	m := newTUIModel(r, make(chan string, 1))
	m.loadHistory(dir)
	if m.histPath != filepath.Join(dir, ".chaude", "history.jsonl") {
		t.Fatal(m.histPath)
	}
	m.recordHistory("first run")
	m.recordHistory("first run")
	m2 := newTUIModel(r, make(chan string, 1))
	m2.loadHistory(dir)
	m2.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	if got := m2.hist.items; len(got) != 1 || got[0] != "first run" {
		t.Fatalf("next session items=%v", got)
	}
	m2.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m2.in.Value() != "first run" {
		t.Fatalf("Up recall=%q", m2.in.Value())
	}
}

func TestUpDownHistoryWithSlashCompleter(t *testing.T) {
	m := &tuiModel{r: &Repl{tracker: newAgentTracker()}, w: 80, submit: make(chan string, 1)}
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	m.hist.items = []string{"older prompt", "/mo"}
	m.hist.idx = len(m.hist.items)
	press := func(k string) {
		t.Helper()
		m.Update(tea.KeyMsg{Type: map[string]tea.KeyType{"up": tea.KeyUp, "down": tea.KeyDown}[k]})
	}

	m.in.SetValue("/")
	if len(m.suggestions()) == 0 {
		t.Fatal("expected completer open for /")
	}
	press("up") // completer top item: Up must reach history, not wrap
	if m.in.Value() != "/mo" || !m.hist.Browsing() {
		t.Fatalf("Up at completer top: %q browsing=%v", m.in.Value(), m.hist.Browsing())
	}
	press("up") // recalled entry starts with "/": history still wins over completer
	if m.in.Value() != "older prompt" {
		t.Fatalf("Up on recalled slash prompt: %q", m.in.Value())
	}
	press("down")
	if m.in.Value() != "/mo" {
		t.Fatalf("Down back: %q", m.in.Value())
	}
	press("down")
	if m.in.Value() != "/" || m.hist.Browsing() {
		t.Fatalf("Down to draft: %q browsing=%v", m.in.Value(), m.hist.Browsing())
	}
}

func TestUpMovesCompleterWhenNotAtTop(t *testing.T) {
	m := &tuiModel{r: &Repl{tracker: newAgentTracker()}, w: 80, submit: make(chan string, 1)}
	m.Update(tea.WindowSizeMsg{Width: 80, Height: 30})
	m.hist.items = []string{"older prompt"}
	m.hist.idx = len(m.hist.items)
	m.in.SetValue("/")
	m.sel = 2
	m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.sel != 1 || m.in.Value() != "/" || m.hist.Browsing() {
		t.Fatalf("sel=%d value=%q browsing=%v", m.sel, m.in.Value(), m.hist.Browsing())
	}
	m.Update(tea.KeyMsg{Type: tea.KeyDown})
	m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.sel != 3 || m.in.Value() != "/" {
		t.Fatalf("Down moves completer: sel=%d value=%q", m.sel, m.in.Value())
	}
}

func TestHistoryDedupeConsecutive(t *testing.T) {
	dir := t.TempDir()
	r := &Repl{tracker: newAgentTracker()}
	r.a.Cwd = dir
	m := newTUIModel(r, make(chan string, 1))
	m.loadHistory(dir)
	m.recordHistory("p")
	m.recordHistory("p") // consecutive duplicate: skipped
	m.recordHistory("q")
	m.recordHistory("p") // not consecutive with the earlier "p": kept
	if got := strings.Join(m.hist.items, "|"); got != "p|q|p" {
		t.Fatalf("same session: %q", got)
	}
	m2 := newTUIModel(r, make(chan string, 1))
	m2.loadHistory(dir)
	m2.recordHistory("p") // last loaded entry is "p": skipped across restart
	if got := strings.Join(m2.hist.items, "|"); got != "p|q|p" {
		t.Fatalf("after restart: %q", got)
	}
	if got := loadInputHistory(m2.histPath); strings.Join(got, "|") != "p|q|p" {
		t.Fatalf("file: %q", got)
	}
}
