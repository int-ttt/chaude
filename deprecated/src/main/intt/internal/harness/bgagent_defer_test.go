package harness

import (
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestRotateDeferDue(t *testing.T) {
	now := time.Date(2026, 11, 1, 12, 0, 0, 0, time.UTC)
	min10 := 10 * time.Minute
	cases := []struct {
		name    string
		running int
		since   time.Time
		timeout time.Duration
		want    bool
	}{
		{"nothing running", 0, time.Time{}, min10, true},
		{"deferral off", 2, time.Time{}, 0, true},
		{"running, not yet held", 1, time.Time{}, min10, false},
		{"running, held 9m", 1, now.Add(-9 * time.Minute), min10, false},
		{"running, held 10m", 1, now.Add(-10 * time.Minute), min10, true},
		{"finished while held", 0, now.Add(-1 * time.Minute), min10, true},
	}
	for _, c := range cases {
		if got := rotateDeferDue(c.running, c.since, now, c.timeout); got != c.want {
			t.Errorf("%s: rotateDeferDue = %v, want %v", c.name, got, c.want)
		}
	}
}

// deferRepl builds a Repl with a background agent "bg1" running and a temp project dir.
func deferRepl(t *testing.T, now time.Time) (*Repl, string) {
	t.Helper()
	dir := t.TempDir()
	r := &Repl{out: io.Discard, a: Args{Cwd: dir, RotateDeferTimeout: 10 * time.Minute, SnapshotChars: 100000},
		tr: &Transcript{path: filepath.Join(dir, "t.jsonl")}, tracker: newAgentTracker()}
	r.tracker.cwd = dir
	r.tracker.onToolUse("bg1", "Agent", map[string]any{"description": "write notes", "name": "haiku-coder",
		"prompt": "do it", "run_in_background": true}, now)
	return r, dir
}

func TestRotateHeldWhileBackgroundAgentRuns(t *testing.T) {
	now := time.Date(2026, 11, 1, 12, 0, 0, 0, time.UTC)
	r, _ := deferRepl(t, now)
	// rotate returns at the deferral gate: r.sess is nil, so reaching the handoff would panic.
	r.rotate("ctx 9 >= 8")
	if r.deferredRot != "ctx 9 >= 8" || r.deferSince.IsZero() {
		t.Fatalf("held rotation not recorded: reason=%q since=%v", r.deferredRot, r.deferSince)
	}
	if r.rotations != 0 {
		t.Fatalf("rotations = %d, want 0 while held", r.rotations)
	}
	if r.deferredDue(time.Now()) {
		t.Fatal("deferredDue true while the background agent runs")
	}
	if !r.deferredDue(time.Now().Add(11 * time.Minute)) {
		t.Fatal("deferredDue false after the 10 min timeout")
	}
}

func TestRotateDeferReleasedWhenAgentFinishes(t *testing.T) {
	now := time.Date(2026, 11, 1, 12, 0, 0, 0, time.UTC)
	r, _ := deferRepl(t, now)
	if held := r.deferRotate("manual"); !held {
		t.Fatal("deferRotate did not hold with a running background agent")
	}
	r.tracker.onUserEvent(event{"message": map[string]any{"content": "<task-notification><tool-use-id>bg1</tool-use-id></task-notification>"}})
	if !r.deferredDue(time.Now()) {
		t.Fatal("held rotation not due after the background agent finished")
	}
	if r.deferRotate("manual") {
		t.Fatal("deferRotate held after the agent finished")
	}
	if r.deferredRot != "" || !r.deferSince.IsZero() {
		t.Fatalf("deferral state not cleared: reason=%q since=%v", r.deferredRot, r.deferSince)
	}
}

func TestDeferRotateTimeoutProceeds(t *testing.T) {
	now := time.Now()
	r, _ := deferRepl(t, now)
	r.deferSince, r.deferredRot = now.Add(-11*time.Minute), "ctx 9 >= 8"
	if r.deferRotate("ctx 9 >= 8") {
		t.Fatal("deferRotate still held after the 10 min timeout")
	}
	if r.deferredRot != "" || !r.deferSince.IsZero() {
		t.Fatalf("timeout did not clear the hold: reason=%q since=%v", r.deferredRot, r.deferSince)
	}
	if !strings.Contains(readFileT(t, r.tr.path), "rotate_defer") {
		t.Fatal("timeout not logged to the transcript")
	}
}

func TestBackgroundResultFileAndHandoff(t *testing.T) {
	now := time.Date(2026, 11, 1, 12, 0, 0, 0, time.UTC)
	r, dir := deferRepl(t, now)
	r.tracker.onToolUse("bg2", "Agent", map[string]any{"description": "still going", "prompt": "x", "run_in_background": true}, now)

	r.tracker.onUserEvent(event{"message": map[string]any{"content": "<task-notification><tool-use-id>bg1</tool-use-id><summary>ALL GOOD</summary></task-notification>"}})

	path := filepath.Join(dir, "notes", "runs", "261101-agent-bg1.md")
	body := readFileT(t, path)
	if !strings.Contains(body, "agent bg1") || !strings.Contains(body, "ALL GOOD") || !strings.Contains(body, "task: write notes") {
		t.Fatalf("result file missing notice/task: %q", body)
	}
	// a repeated notice must not append a second block
	r.tracker.onUserEvent(event{"message": map[string]any{"content": "<task-notification><tool-use-id>bg1</tool-use-id></task-notification>"}})
	if n := strings.Count(readFileT(t, path), "## "); n != 1 {
		t.Fatalf("result blocks = %d, want 1", n)
	}

	snap := r.buildSnapshot("note")
	for _, want := range []string{"## Background agents", "- haiku-coder (bg1): done, result file: notes/runs/261101-agent-bg1.md",
		"- still going (bg2): running, result file: notes/runs/261101-agent-bg2.md"} {
		if !strings.Contains(snap, want) {
			t.Errorf("handoff missing %q\n%s", want, snap)
		}
	}
	if _, err := os.Stat(filepath.Join(dir, "notes", "runs", "261101-agent-bg2.md")); err == nil {
		t.Error("running agent got a result file before it finished")
	}
}

func readFileT(t *testing.T, p string) string {
	t.Helper()
	b, err := os.ReadFile(p)
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}
