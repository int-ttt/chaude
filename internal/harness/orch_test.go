package harness

import (
	"strings"
	"testing"
)

func TestOrchParseMode(t *testing.T) {
	cases := []struct {
		in      string
		want    string
		wantErr bool
	}{
		{"", "m0", false},
		{"  ", "m0", false},
		{"m0", "m0", false},
		{"B", "b", false},
		{" c ", "c", false},
		{"D", "d", false},
		{"e", "e", false},
		{"F", "f", false},
		{"x", "", true},
		{"m1", "", true},
	}
	for _, c := range cases {
		got, err := parseOrch(c.in)
		if (err != nil) != c.wantErr {
			t.Fatalf("parseOrch(%q) err=%v, wantErr=%v", c.in, err, c.wantErr)
		}
		if got != c.want {
			t.Errorf("parseOrch(%q) = %q, want %q", c.in, got, c.want)
		}
	}
}

func TestOrchDefaultModel(t *testing.T) {
	cases := map[string]string{
		"m0": "", "b": "opus", "c": "opus", "d": "opus", "e": "haiku", "f": "opus", "": "",
	}
	for mode, want := range cases {
		if got := orchDefaultModel(mode); got != want {
			t.Errorf("orchDefaultModel(%q) = %q, want %q", mode, got, want)
		}
	}
}

func TestOrchRule(t *testing.T) {
	if got := orchRule("m0"); got != "" {
		t.Errorf("orchRule(m0) = %q, want empty", got)
	}
	for _, mode := range []string{"b", "c", "d", "e", "f"} {
		if orchRule(mode) == "" {
			t.Errorf("orchRule(%q) empty, want non-empty", mode)
		}
	}
	if !strings.Contains(orchRule("c"), orchRuleB) {
		t.Errorf("orchRule(c) does not contain orchRuleB")
	}
}
