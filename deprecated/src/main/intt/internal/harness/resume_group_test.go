package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// grpSess writes one session file: each prompt is a user turn (2 min apart from start) with an assistant reply.
// A prompt of "/clear" is written as the Claude Code /clear command instead.
func grpSess(t *testing.T, dir, id string, start, mod time.Time, prompts ...string) {
	t.Helper()
	var b strings.Builder
	for i, p := range prompts {
		ts := start.Add(time.Duration(2*i) * time.Minute).UTC().Format(time.RFC3339Nano)
		if p == "/clear" {
			b.WriteString(`{"type":"user","timestamp":"` + ts + `","message":{"role":"user","content":"` + clearCommandMark + `"}}` + "\n")
			continue
		}
		b.WriteString(`{"type":"user","timestamp":"` + ts + `","message":{"role":"user","content":"` + p + `"}}` + "\n" +
			`{"type":"assistant","timestamp":"` + ts + `","message":{"role":"assistant","content":[{"type":"text","text":"ok"}]}}` + "\n")
	}
	f := filepath.Join(dir, id+".jsonl")
	if err := os.WriteFile(f, []byte(b.String()), 0o644); err != nil {
		t.Fatal(err)
	}
	os.Chtimes(f, mod, mod)
}

var grpT0 = time.Date(2026, 10, 30, 9, 0, 0, 0, time.UTC)

func TestGroupSessions(t *testing.T) {
	cases := []struct {
		name  string
		a, b  []string
		gap   time.Duration
		marks []time.Time
		want  int
	}{
		{"same run", []string{"parser plan"}, []string{"parser plan step"}, 10 * time.Minute, nil, 1},
		{"clear at end of first", []string{"parser plan", "/clear"}, []string{"parser plan step"}, 10 * time.Minute, nil, 2},
		{"clear at head of second", []string{"parser plan"}, []string{"/clear", "parser plan step"}, 10 * time.Minute, nil, 2},
		{"consecutive day", []string{"parser plan"}, []string{"parser plan step"}, 26 * time.Hour, nil, 1},
		{"two days apart", []string{"parser plan"}, []string{"parser plan step"}, 50 * time.Hour, nil, 2},
		{"no word overlap", []string{"parser plan"}, []string{"weather tomorrow"}, 10 * time.Minute, nil, 2},
		{"tui clear mark between", []string{"parser plan"}, []string{"parser plan step"}, 10 * time.Minute,
			[]time.Time{grpT0.Add(5 * time.Minute)}, 2},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			d := t.TempDir()
			grpSess(t, d, "a", grpT0, grpT0.Add(5*time.Minute), c.a...)
			bs := grpT0.Add(c.gap)
			grpSess(t, d, "b", bs, bs.Add(5*time.Minute), c.b...)
			gs := groupSessions(listClaudeSessions(d), c.marks)
			if len(gs) != c.want {
				t.Fatalf("groups = %d, want %d: %+v", len(gs), c.want, gs)
			}
		})
	}
}

func TestGroupOrderAndKey(t *testing.T) {
	d := t.TempDir()
	grpSess(t, d, "a", grpT0, grpT0.Add(5*time.Minute), "parser plan")
	grpSess(t, d, "b", grpT0.Add(10*time.Minute), grpT0.Add(15*time.Minute), "parser plan step")
	gs := groupSessions(listClaudeSessions(d), nil)
	if len(gs) != 1 {
		t.Fatalf("groups = %d, want 1", len(gs))
	}
	if k := gs[0].key(); k != "a,b" {
		t.Fatalf("key = %q, want time order a,b", k)
	}
	if gs[0].latest().ID != "b" {
		t.Fatalf("latest = %q, want b", gs[0].latest().ID)
	}
}

func TestMatchGroups(t *testing.T) {
	g1 := sessionGroup{Files: []sessionInfo{{ID: "s1", First: "claude harness refactor", Last: "fix resume"}}}
	g2 := sessionGroup{Files: []sessionInfo{{ID: "s2", First: "weather notes", Last: "tomorrow"}}}
	gs := []sessionGroup{g1, g2}
	cases := []struct {
		terms []string
		want  int
	}{
		{[]string{"claude"}, 1},
		{[]string{"clade"}, 1},             // typo: one edit from "claude"
		{[]string{"Weather"}, 1},           // case-insensitive
		{[]string{"s2"}, 1},                // session id
		{[]string{"proj"}, 2},              // cwd is part of the haystack
		{[]string{"claude", "weather"}, 0}, // all terms must match
		{[]string{"claude", "resume"}, 1},
		{[]string{"nothing"}, 0},
	}
	for _, c := range cases {
		if got := matchGroups(gs, "/w/proj", c.terms); len(got) != c.want {
			t.Errorf("terms %v: got %d groups, want %d", c.terms, len(got), c.want)
		}
	}
}

func TestLoadImportArgMergesInTimeOrder(t *testing.T) {
	d := t.TempDir()
	grpSess(t, d, "a", grpT0, grpT0, "one")
	grpSess(t, d, "b", grpT0.Add(time.Hour), grpT0.Add(time.Hour), "two")
	arg := filepath.Join(d, "a.jsonl") + "," + filepath.Join(d, "b.jsonl")
	path, imp, err := loadImportArg(arg, "")
	if err != nil {
		t.Fatal(err)
	}
	if filepath.Base(path) != "b.jsonl" {
		t.Fatalf("path = %s, want the last file", path)
	}
	if len(imp.Turns) != 2 || imp.Turns[0][0] != "one" || imp.Turns[1][0] != "two" {
		t.Fatalf("turns out of order: %+v", imp.Turns)
	}
	if !imp.Start.Equal(grpT0) {
		t.Fatalf("start = %v, want earliest %v", imp.Start, grpT0)
	}
}

func TestReadClaudeTranscriptClearFlags(t *testing.T) {
	cases := []struct {
		prompts       []string
		cleared, head bool
	}{
		{[]string{"one"}, false, false},
		{[]string{"one", "/clear"}, true, false},
		{[]string{"/clear", "one"}, true, true},
	}
	for _, c := range cases {
		d := t.TempDir()
		grpSess(t, d, "x", grpT0, grpT0, c.prompts...)
		imp, err := readClaudeTranscript(filepath.Join(d, "x.jsonl"))
		if err != nil {
			t.Fatal(err)
		}
		if imp.Cleared != c.cleared || imp.ClearHead != c.head {
			t.Errorf("%v: cleared=%v head=%v, want %v %v", c.prompts, imp.Cleared, imp.ClearHead, c.cleared, c.head)
		}
		if !imp.Start.Equal(grpT0) {
			t.Errorf("%v: start = %v, want %v", c.prompts, imp.Start, grpT0)
		}
	}
}

func TestClearMarks(t *testing.T) {
	p := filepath.Join(t.TempDir(), "marks.txt")
	at := time.Unix(1700000000, 5)
	if err := recordClearMark(p, "/w/a", at); err != nil {
		t.Fatal(err)
	}
	if err := recordClearMark(p, "/w/b", at.Add(time.Second)); err != nil {
		t.Fatal(err)
	}
	if got := loadClearMarks(p, "/w/a"); len(got) != 1 || !got[0].Equal(at) {
		t.Fatalf("marks for /w/a = %v", got)
	}
	if got := loadClearMarks(p, "/w/c"); len(got) != 0 {
		t.Fatalf("marks for /w/c = %v, want none", got)
	}
	if got := loadClearMarks(filepath.Join(t.TempDir(), "none"), "/w/a"); got != nil {
		t.Fatalf("missing file: %v", got)
	}
}

func TestResumeTarget(t *testing.T) {
	t.Setenv("HARNESS_RECORD_DIR", t.TempDir()) // empty record: no topic matches
	cases := []struct {
		args   []string
		terms  []string
		record bool
	}{
		{nil, nil, true},
		{[]string{"claude"}, nil, false},
		{[]string{"claude", "x", "y"}, []string{"x", "y"}, false},
		{[]string{"x"}, []string{"x"}, false},
		{[]string{"1"}, []string{"1"}, false},
	}
	for _, c := range cases {
		terms, record := resumeTarget("/w/proj", c.args)
		if record != c.record || strings.Join(terms, " ") != strings.Join(c.terms, " ") {
			t.Errorf("%v: terms=%v record=%v, want %v %v", c.args, terms, record, c.terms, c.record)
		}
	}
}
