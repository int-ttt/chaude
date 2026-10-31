package harness

import (
	"bufio"
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// idleSpendRepl returns a Repl whose metrics go to dir; HARNESS_USAGE_POLL=0 keeps pollUsage from running /usage.
func idleSpendRepl(t *testing.T, dir string) *Repl {
	t.Setenv("HARNESS_USAGE_POLL", "0")
	return &Repl{out: io.Discard, a: Args{}, tr: &Transcript{path: t.TempDir() + "/t.jsonl"}, tracker: newAgentTracker(),
		metrics: newMetrics(dir, nil), sess: &Session{SessionID: "s1"}}
}

// bgRows returns the stage=turn rows with source=background logged under dir.
func bgRows(t *testing.T, dir string) []map[string]any {
	t.Helper()
	files, err := filepath.Glob(filepath.Join(dir, "*.jsonl"))
	if err != nil {
		t.Fatal(err)
	}
	var out []map[string]any
	for _, file := range files {
		data, err := os.ReadFile(file)
		if err != nil {
			t.Fatalf("read %s: %v", file, err)
		}
		for _, line := range strings.Split(strings.TrimRight(string(data), "\n"), "\n") {
			var m map[string]any
			if err := json.Unmarshal([]byte(line), &m); err != nil {
				t.Fatalf("bad row %q: %v", line, err)
			}
			if m["stage"] == "turn" && m["source"] == "background" {
				out = append(out, m)
			}
		}
	}
	return out
}

func TestIdleSpendRow(t *testing.T) {
	dir := t.TempDir()
	r := idleSpendRepl(t, dir)
	s := r.sess
	feed := func(line string) {
		var ev event
		if err := json.Unmarshal([]byte(line), &ev); err != nil {
			t.Fatalf("bad event %q: %v", line, err)
		}
		r.idleEvent(s, r.tracker, ev)
	}
	// subagent message (parent_tool_use_id set): its usage counts toward sub_*
	feed(`{"type":"assistant","parent_tool_use_id":"tu1","message":{"id":"m1","content":[],"usage":{"input_tokens":3,"output_tokens":7,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}`)
	// main-thread Agent spawn
	feed(`{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tu1","name":"Agent","input":{"subagent_type":"Explore"}}]}}`)
	// turn result
	feed(`{"type":"result","session_id":"s1","total_cost_usd":0.01,"usage":{"input_tokens":3,"output_tokens":2},"modelUsage":{"claude-haiku-4-5":{"costUSD":0.01}}}`)

	rows := bgRows(t, dir)
	if len(rows) != 1 {
		t.Fatalf("background rows = %d, want 1", len(rows))
	}
	row := rows[0]
	if got := row["sub_output_tokens"]; got != float64(7) {
		t.Errorf("sub_output_tokens = %v, want 7", got)
	}
	if got := row["spawns"]; got != float64(1) {
		t.Errorf("spawns = %v, want 1", got)
	}
	mu, ok := row["result_model_usage"].(map[string]any)
	if !ok {
		t.Fatalf("result_model_usage = %v, want map", row["result_model_usage"])
	}
	if _, ok := mu["claude-haiku-4-5"]; !ok {
		t.Errorf("result_model_usage = %v, missing claude-haiku-4-5", mu)
	}
}

func TestIdleSpendLiveReplay(t *testing.T) {
	const file = "testdata/bg_turn_live.jsonl"
	f, err := os.Open(file)
	if err != nil {
		t.Skipf("%s missing: %v", file, err)
	}
	defer f.Close()

	dir := t.TempDir()
	r := idleSpendRepl(t, dir)
	s := r.sess
	seenResult := false
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<24)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" {
			continue
		}
		var ev event
		if err := json.Unmarshal([]byte(line), &ev); err != nil {
			t.Fatalf("bad line %q: %v", line, err)
		}
		if !seenResult {
			if str(ev["type"]) == "result" {
				seenResult = true
			}
			continue
		}
		r.idleEvent(s, r.tracker, ev)
	}
	if err := sc.Err(); err != nil {
		t.Fatal(err)
	}
	rows := bgRows(t, dir)
	if len(rows) == 0 {
		t.Log("no background rows logged from", file)
		return
	}
	last := rows[len(rows)-1]
	keys := make([]string, 0, len(last))
	for k := range last {
		keys = append(keys, k)
	}
	t.Logf("last background row keys (%d): %v", len(keys), keys)
}
