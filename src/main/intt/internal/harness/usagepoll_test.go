package harness

import (
	"encoding/json"
	"errors"
	"math"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func init() { os.Setenv("HARNESS_USAGE_POLL", "0") } // keep unit tests from spawning claude

const liveUsageSample = "Current session: 18% used · resets Oct 30 at 12:49pm (Asia/Seoul)\n" +
	"Current week (all models): 27% used · resets Nov 1 at 4:59pm (Asia/Seoul)"

// metricRows reads every *.jsonl row under dir.
func metricRows(t *testing.T, dir string) []map[string]any {
	t.Helper()
	files, err := filepath.Glob(filepath.Join(dir, "*.jsonl"))
	if err != nil {
		t.Fatal(err)
	}
	var out []map[string]any
	for _, f := range files {
		data, err := os.ReadFile(f)
		if err != nil {
			t.Fatalf("read %s: %v", f, err)
		}
		for _, line := range strings.Split(strings.TrimRight(string(data), "\n"), "\n") {
			var m map[string]any
			if err := json.Unmarshal([]byte(line), &m); err != nil {
				t.Fatalf("bad row %q: %v", line, err)
			}
			out = append(out, m)
		}
	}
	return out
}

func TestParseUsage(t *testing.T) {
	rec, ok := parseUsage(liveUsageSample)
	if !ok {
		t.Fatal("live sample: want ok")
	}
	if rec["session_pct"] != 18.0 || rec["session_resets"] != "Oct 30 at 12:49pm (Asia/Seoul)" {
		t.Errorf("session fields: %v", rec)
	}
	if rec["week_pct"] != 27.0 || rec["week_resets"] != "Nov 1 at 4:59pm (Asia/Seoul)" {
		t.Errorf("week fields: %v", rec)
	}

	if rec, ok := parseUsage("Current week (all models): 27% used"); ok {
		t.Errorf("no session line: want ok=false, got %v", rec)
	}
	if rec, ok := parseUsage("Current session: 5% used"); !ok || rec["session_pct"] != 5.0 {
		t.Errorf("no resets part: got ok=%v rec=%v", ok, rec)
	} else if _, has := rec["session_resets"]; has {
		t.Errorf("no resets part: session_resets should be omitted, got %v", rec)
	}
}

func TestPollUsageLogsRow(t *testing.T) {
	dir := t.TempDir()
	r := &Repl{metrics: newMetrics(dir, nil)}
	old := usageRaw
	defer func() { usageRaw = old }()
	usageRaw = func() (string, error) { return liveUsageSample, nil }

	r.pollUsageSync()
	rows := metricRows(t, dir)
	if len(rows) != 1 {
		t.Fatalf("want 1 row, got %d", len(rows))
	}
	if rows[0]["stage"] != "usage" || rows[0]["session_pct"] != 18.0 {
		t.Errorf("row: %v", rows[0])
	}
}

func TestPollUsageError(t *testing.T) {
	dir := t.TempDir()
	r := &Repl{metrics: newMetrics(dir, nil)}
	old := usageRaw
	defer func() { usageRaw = old }()
	usageRaw = func() (string, error) { return "", errors.New("claude missing") }

	r.pollUsageSync()
	rows := metricRows(t, dir)
	if len(rows) != 1 || rows[0]["stage"] != "usage" || rows[0]["error"] != "claude missing" {
		t.Fatalf("want one usage error row, got %v", rows)
	}
}

func TestTurnCostUSDDelta(t *testing.T) {
	r := &Repl{}
	check := func(sess string, cost any, want float64) {
		t.Helper()
		got, ok := r.turnCostUSD(sess, cost)
		if !ok || math.Abs(got-want) > 1e-9 {
			t.Errorf("turnCostUSD(%s, %v) = %v, %v; want %v", sess, cost, got, ok, want)
		}
	}
	check("a", 0.438, 0.438) // first turn of a session
	check("a", 0.963, 0.525) // cumulative grows: this turn's share
	check("b", 0.100, 0.100) // new session starts from 0
	check("b", 0.050, 0.050) // cumulative went down: restart from 0
	if _, ok := r.turnCostUSD("b", nil); ok {
		t.Error("nil cost: want ok=false")
	}
}

// TestUsageLive hits the real claude CLI; run with HARNESS_LIVE_USAGE=1.
func TestUsageLive(t *testing.T) {
	if os.Getenv("HARNESS_LIVE_USAGE") != "1" {
		t.Skip("set HARNESS_LIVE_USAGE=1 to run against the real claude CLI")
	}
	text, err := usageRaw()
	if err != nil {
		t.Fatalf("usageRaw: %v", err)
	}
	rec, ok := parseUsage(text)
	if !ok {
		t.Fatalf("parseUsage: no session line in %q", text)
	}
	t.Logf("session_pct=%v session_resets=%v week_pct=%v week_resets=%v", rec["session_pct"], rec["session_resets"], rec["week_pct"], rec["week_resets"])
}
