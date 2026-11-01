package harness

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestMetricsLog(t *testing.T) {
	dir := t.TempDir()
	m := newMetrics(dir, map[string]any{"lean": true})
	for i := 0; i < 2; i++ {
		if err := m.Log(map[string]any{"event": "turn", "n": i}); err != nil {
			t.Fatalf("Log: %v", err)
		}
	}
	entries, err := os.ReadDir(dir)
	if err != nil || len(entries) != 1 {
		t.Fatalf("want 1 file in %s, got %d (err=%v)", dir, len(entries), err)
	}
	data, err := os.ReadFile(filepath.Join(dir, entries[0].Name()))
	if err != nil {
		t.Fatal(err)
	}
	lines := strings.Split(strings.TrimRight(string(data), "\n"), "\n")
	if len(lines) != 2 {
		t.Fatalf("want 2 lines, got %d", len(lines))
	}
	for _, line := range lines {
		var rec map[string]any
		if err := json.Unmarshal([]byte(line), &rec); err != nil {
			t.Fatalf("invalid JSON %q: %v", line, err)
		}
		if _, ok := rec["ts"].(string); !ok {
			t.Errorf("missing ts in %q", line)
		}
		if _, ok := rec["flags"].(map[string]any); !ok {
			t.Errorf("missing flags in %q", line)
		}
	}
}

func TestMetricsNilAndDisabled(t *testing.T) {
	var nilM *Metrics
	if err := nilM.Log(map[string]any{"a": 1}); err != nil {
		t.Errorf("nil Metrics Log: %v", err)
	}
	dir := t.TempDir()
	if err := newMetrics("", nil).Log(map[string]any{"a": 1}); err != nil {
		t.Errorf("disabled Log: %v", err)
	}
	entries, _ := os.ReadDir(dir)
	if len(entries) != 0 {
		t.Errorf("disabled wrote %d entries", len(entries))
	}
}

func TestUsageFields(t *testing.T) {
	got := usageFields(map[string]any{
		"input_tokens":                float64(10),
		"cache_read_input_tokens":     float64(20),
		"cache_creation_input_tokens": float64(30),
		"output_tokens":               float64(5),
	})
	want := map[string]int{"input_tokens": 10, "cache_read_input_tokens": 20, "cache_creation_input_tokens": 30, "output_tokens": 5, "ctx": 60}
	for k, w := range want {
		if got[k] != w {
			t.Errorf("%s=%v want %d", k, got[k], w)
		}
	}
}
