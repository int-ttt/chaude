package harness

import "testing"

func TestRecordSkip(t *testing.T) {
	cases := []struct {
		prompt string
		want   bool
	}{
		{"분석 a", false},
		{"측정", false},
		{"/resume", true},
		{"", true},
		{"hello", false},
	}
	for _, c := range cases {
		if got := recordSkip(c.prompt); got != c.want {
			t.Errorf("recordSkip(%q) = %v, want %v", c.prompt, got, c.want)
		}
	}
}

func TestMeasureLine(t *testing.T) {
	fields := map[string]any{"stage": "turn", "start_ms": int64(1), "duration_ms": int64(1200), "cost_usd": 0.01234,
		"result_model_usage": map[string]any{}, "result_input_tokens": 10}
	want := "[측정] duration_ms=1200 cost_usd=0.01234 result_input_tokens=10"
	if got := measureLine(fields); got != want {
		t.Errorf("measureLine = %q, want %q", got, want)
	}
	if got := measureLine(map[string]any{}); got != "" {
		t.Errorf("measureLine(empty) = %q, want \"\"", got)
	}
}
