package harness

import (
	"testing"
	"time"
)

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
