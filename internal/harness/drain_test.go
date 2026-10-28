package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestAgentTrackerAndDrain(t *testing.T) {
	dir := t.TempDir()
	old, _ := os.Getwd()
	_ = os.Chdir(dir)
	defer os.Chdir(old)
	out := "notes/runs/x.md"
	tr := newAgentTracker()
	now := time.Now()
	tr.onToolUse("a", "Agent", map[string]any{"description": "d", "prompt": "write " + out, "run_in_background": true}, now)
	tr.onToolUse("b", "Task", map[string]any{"description": "fg"}, now)
	tr.onToolUse("c", "Bash", map[string]any{}, now)
	if len(tr.agents) != 2 || tr.agents["a"].OutPath == "" {
		t.Fatalf("tracking: %+v", tr.agents)
	}
	tr.onToolResult("a") // background: launch ack, not done
	tr.onToolResult("b")
	if r := tr.running(); len(r) != 1 || r[0].ID != "a" {
		t.Fatalf("running: %v", r)
	}
	d := &drain{tr: tr, since: now, timeout: time.Hour}
	if done, _ := d.finished(now); done {
		t.Fatal("done before file")
	}
	_ = os.MkdirAll(filepath.Dir(out), 0o755)
	_ = os.WriteFile(out, []byte("x"), 0o644)
	if done, _ := d.finished(time.Now()); done {
		t.Fatal("done before quiet period")
	}
	past := time.Now().Add(-time.Minute)
	_ = os.Chtimes(out, past, past)
	if done, to := d.finished(time.Now()); !done || to {
		t.Fatal("expected done")
	}
	tr2 := newAgentTracker()
	tr2.onToolUse("z", "Agent", map[string]any{"run_in_background": true}, now)
	d2 := &drain{tr: tr2, since: now, timeout: time.Minute}
	if done, to := d2.finished(now.Add(2 * time.Minute)); !done || !to {
		t.Fatal("expected timeout")
	}
}

type bufWC struct{ strings.Builder }

func (*bufWC) Close() error { return nil }

func TestDrainConsumeDenies(t *testing.T) {
	ch := make(chan string, 4)
	w := &bufWC{}
	d := &drain{sess: &Session{lines: ch, stdin: w}}
	ch <- `{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool"}}`
	ch <- `{"type":"assistant"}`
	close(ch)
	var kinds []string
	d.consume(func(k string, _ event) { kinds = append(kinds, k) })
	if len(kinds) != 2 || kinds[0] != "drain_deny" || kinds[1] != "drain_event" {
		t.Fatalf("kinds %v", kinds)
	}
	if !strings.Contains(w.String(), `"behavior":"deny"`) || !strings.Contains(w.String(), "r1") {
		t.Fatalf("resp %q", w.String())
	}
}
