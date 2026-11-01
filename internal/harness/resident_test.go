package harness

import (
	"io"
	"testing"
)

// stubSessions counts newSessionFn calls and restores NewSession on cleanup.
func stubSessions(t *testing.T) *int {
	t.Helper()
	var n int
	newSessionFn = func(SessionOpts) (*Session, error) {
		n++
		s, _, _ := fakeSession()
		return s, nil
	}
	t.Cleanup(func() { newSessionFn = NewSession })
	return &n
}

func residentRepl(t *testing.T, maxWorkers int) *Repl {
	t.Helper()
	return &Repl{out: io.Discard, tr: &Transcript{path: t.TempDir() + "/t.jsonl"}, tracker: newAgentTracker(),
		a: Args{Model: "haiku", Cwd: t.TempDir(), MaxWorkers: maxWorkers}, children: newChildSet(), resident: residentSlots("haiku", 1)}
}

func TestResidentSpawnsEager(t *testing.T) {
	started := stubSessions(t)
	r := residentRepl(t, 8)
	if err := r.start(); err != nil {
		t.Fatal(err)
	}
	if *started != 4 { // main + tui, coder, prompter
		t.Fatalf("spawned %d, want 4", *started)
	}
	if r.children.Len() != 3 {
		t.Fatalf("parked %d, want 3", r.children.Len())
	}
	if _, ok := r.children.Take(childKey(r.a.Cwd, "haiku", "tui")); !ok {
		t.Fatal("tui slot not parked under its role key")
	}
}

func TestResidentSlotsFour(t *testing.T) {
	s := residentSlots("opus", 1)
	want := []string{"main", "prompter", "tui", "coder"}
	if len(s) != len(want) {
		t.Fatalf("slots %d, want %d", len(s), len(want))
	}
	for i, role := range want {
		if s[i].Role != role {
			t.Fatalf("slot %d role %q, want %q", i, s[i].Role, role)
		}
	}
	if s[0].Model != "opus" {
		t.Fatalf("main model %q, want opus", s[0].Model)
	}
	for _, sl := range s[1:] {
		if sl.Model != "haiku" {
			t.Fatalf("%s model %q, want haiku", sl.Role, sl.Model)
		}
	}
}
