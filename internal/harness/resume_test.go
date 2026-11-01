package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func writeSess(t *testing.T, dir, id string, mod time.Time, prompts ...string) {
	var b string
	for _, p := range prompts {
		b += `{"type":"user","message":{"role":"user","content":"` + p + `"}}` + "\n" +
			`{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"ok"}]}}` + "\n"
	}
	f := filepath.Join(dir, id+".jsonl")
	if err := os.WriteFile(f, []byte(b), 0o644); err != nil {
		t.Fatal(err)
	}
	os.Chtimes(f, mod, mod)
}

func TestListClaudeSessions(t *testing.T) {
	d := t.TempDir()
	now := time.Now()
	writeSess(t, d, "old", now.Add(-2*time.Hour), "first  old", "last old")
	writeSess(t, d, "new", now, "only")
	os.WriteFile(filepath.Join(d, "empty.jsonl"), []byte("{}\n"), 0o644)
	l := listClaudeSessions(d)
	if len(l) != 2 || l[0].ID != "new" || l[1].ID != "old" {
		t.Fatalf("order/skip wrong: %+v", l)
	}
	if l[1].Turns != 2 || l[1].First != "first old" || l[1].Last != "last old" {
		t.Fatalf("parse wrong: %+v", l[1])
	}
	if listClaudeSessions(filepath.Join(d, "none")) != nil {
		t.Fatal("expected nil")
	}
}

func TestViewHelpers(t *testing.T) {
	d := t.TempDir()
	now := time.Now()
	writeSess(t, d, "abc123def", now, "hello", "second")
	writeSess(t, d, "abd999", now.Add(-time.Hour), "x")
	l := listClaudeSessions(d)
	if n := len(matchSessions(l, "abc")); n != 1 {
		t.Fatalf("prefix abc: got %d", n)
	}
	if n := len(matchSessions(l, "ab")); n != 2 {
		t.Fatalf("prefix ab: got %d", n)
	}
	if s := formatSessionList(l); !strings.Contains(s, "abc123de") || !strings.Contains(s, "hello") {
		t.Fatalf("list wrong:\n%s", s)
	}
	imp, err := readClaudeTranscript(l[0].Path)
	if err != nil {
		t.Fatal(err)
	}
	s := formatTranscript(l[0], imp)
	for _, want := range []string{"[1] user:\nhello", "[2] user:\nsecond", "ok"} {
		if !strings.Contains(s, want) {
			t.Fatalf("transcript missing %q:\n%s", want, s)
		}
	}
}

func TestResumeSourceArg(t *testing.T) {
	for _, a := range [][]string{nil, {"claude"}, {"Claude"}} {
		if m := resumeSourceMsg(a); m != "" {
			t.Fatalf("%v rejected: %s", a, m)
		}
	}
	for _, a := range [][]string{{"codex"}, {"claude", "x"}} {
		if resumeSourceMsg(a) == "" {
			t.Fatalf("%v accepted", a)
		}
	}
	r := &Repl{}
	if _, m := r.sessionList("other"); m == "" {
		t.Fatal("sessionList must reject unknown source")
	}
}
