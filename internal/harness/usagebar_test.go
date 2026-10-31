package harness

import (
	"strings"
	"testing"
)

func TestRenderBar(t *testing.T) {
	cases := []struct {
		pct  float64
		want string
	}{
		{0, "[░░░░░░░░░░] 0%"},
		{50, "[█████░░░░░] 50%"},
		{100, "[██████████] 100%"},
		{130, "[██████████] 130% !"},
	}
	for _, c := range cases {
		if got := renderBar(c.pct); got != c.want {
			t.Errorf("renderBar(%v)=%q want %q", c.pct, got, c.want)
		}
	}
	if barWidth(0) != barWidth(100)-2 { // "0%" vs "100%" differ only by 2 label cells
		t.Errorf("width mismatch %d vs %d", barWidth(0), barWidth(100))
	}
}

func TestDecorateUsage(t *testing.T) {
	out := decorateUsage("Current session: 45% used\nother", 75_000)
	if !strings.HasPrefix(out, "Current session: 45% used  [") || !strings.Contains(out, "Context (now): [█████░░░░░] 50%") {
		t.Errorf("bad output:\n%s", out)
	}
}
