package harness

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestSetRateLogsOnFiveHourChange(t *testing.T) {
	dir := t.TempDir()
	r := &Repl{metrics: newMetrics(dir, nil)}
	info := func(u float64) map[string]any {
		return map[string]any{"unifiedWindows": map[string]any{"five_hour": map[string]any{"utilization": u}}}
	}
	rows := func() []map[string]any {
		file := filepath.Join(dir, time.Now().UTC().Format("2006-01-02")+".jsonl")
		data, err := os.ReadFile(file)
		if err != nil {
			t.Fatalf("read %s: %v", file, err)
		}
		var out []map[string]any
		for _, line := range strings.Split(strings.TrimRight(string(data), "\n"), "\n") {
			var m map[string]any
			if err := json.Unmarshal([]byte(line), &m); err != nil {
				t.Fatalf("bad row %q: %v", line, err)
			}
			out = append(out, m)
		}
		return out
	}
	count := func() int {
		n := 0
		for _, row := range rows() {
			if row["stage"] == "rate" {
				n++
			}
		}
		return n
	}

	r.setRate(info(0.37))
	r.setRate(info(0.37))
	if n := count(); n != 1 {
		t.Fatalf("same value twice: want 1 rate row, got %d", n)
	}
	r.setRate(info(0.40))
	if n := count(); n != 2 {
		t.Fatalf("changed value: want 2 rate rows, got %d", n)
	}
	for _, row := range rows() {
		if row["stage"] != "rate" && row["stage"] != "rate_raw" {
			t.Errorf("row stage: got %v, want rate", row["stage"])
		}
	}
}

func TestSetRateTopLevelFallback(t *testing.T) {
	dir := t.TempDir()
	r := &Repl{metrics: newMetrics(dir, nil)}
	r.setRate(map[string]any{"rateLimitType": "five_hour", "utilization": 0.42})

	var rate, raw []map[string]any
	for _, row := range metricRows(t, dir) {
		switch row["stage"] {
		case "rate":
			rate = append(rate, row)
		case "rate_raw":
			raw = append(raw, row)
		}
	}
	if len(rate) != 1 || rate[0]["five_hour"] != 0.42 {
		t.Fatalf("want one rate row five_hour=0.42, got %v", rate)
	}
	if len(raw) != 1 {
		t.Fatalf("want one rate_raw row, got %d", len(raw))
	}
	if keys, _ := raw[0]["keys"].([]any); len(keys) != 2 {
		t.Fatalf("rate_raw keys: want 2 key names, got %v", raw[0]["keys"])
	}
}

func TestFmtRateStatus(t *testing.T) {
	now := time.Date(2026, 10, 29, 14, 20, 0, 0, time.Local)
	reset := time.Date(2026, 10, 29, 15, 50, 0, 0, time.Local).Unix()
	cases := []struct {
		name string
		info map[string]any
		want string
	}{
		{"fraction with epoch reset", map[string]any{
			"unifiedWindows": map[string]any{"five_hour": map[string]any{"utilization": 0.37, "resetsAt": float64(reset)}},
		}, "14:20/15:50 · 37%"},
		{"fraction as string with ms reset", map[string]any{
			"unifiedWindows": map[string]any{"five_hour": map[string]any{"utilization": "0.37", "resetsAt": float64(reset) * 1000}},
		}, "14:20/15:50 · 37%"},
		{"percent value", map[string]any{
			"unifiedWindows": map[string]any{"five_hour": map[string]any{"utilization": 37}},
		}, "14:20/14:20 · 37%"},
		{"top-level fallback", map[string]any{
			"utilization": 0.37, "resetsAt": float64(reset),
		}, "14:20/15:50 · 37%"},
		{"no resetsAt", map[string]any{
			"unifiedWindows": map[string]any{"five_hour": map[string]any{"utilization": 0.37}},
		}, "14:20/14:20 · 37%"},
		{"no utilization", map[string]any{
			"unifiedWindows": map[string]any{"five_hour": map[string]any{"resetsAt": float64(reset)}},
		}, ""},
	}
	for _, c := range cases {
		if got := fmtRateStatus(c.info, now); got != c.want {
			t.Errorf("%s: got %q, want %q", c.name, got, c.want)
		}
	}
}
