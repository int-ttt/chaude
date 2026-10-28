package harness

import (
	"strings"
	"testing"
)

func TestModeCycleAndParse(t *testing.T) {
	m := modeDefault
	var seen []string
	for i := 0; i < int(numModes); i++ {
		seen = append(seen, m.String())
		m = m.Next()
	}
	if m != modeDefault || seen[2] != "plan" {
		t.Fatal(seen)
	}
	if p, ok := parseMode("Plan"); !ok || p != modePlan {
		t.Fatal("plan")
	}
	if p, ok := parseMode("accept"); !ok || p != modeAcceptEdits {
		t.Fatal("accept")
	}
	if _, ok := parseMode("zzz"); ok {
		t.Fatal("bad")
	}
}

func TestDecideMode(t *testing.T) {
	cases := []struct {
		m    permMode
		tool string
		want string
	}{
		{modeDefault, "Edit", "ask"}, {modeAcceptEdits, "Edit", "allow"}, {modeAcceptEdits, "Bash", "ask"},
		{modePlan, "Read", "allow"}, {modePlan, "Bash", "deny"}, {modePlan, "Write", "deny"}, {modeAuto, "Bash", "allow"},
	}
	for _, c := range cases {
		if v, _ := decideMode(c.m, c.tool); v != c.want {
			t.Errorf("%v %s: %s want %s", c.m, c.tool, v, c.want)
		}
	}
}

func TestReplModeLine(t *testing.T) {
	var sb strings.Builder
	r := &Repl{out: &sb}
	r.modeLine([]string{"plan"})
	if permMode(r.mode.Load()) != modePlan {
		t.Fatal(sb.String())
	}
	r.modeLine([]string{"next"})
	if permMode(r.mode.Load()) != modeAuto {
		t.Fatal("next")
	}
	r.modeLine([]string{"bogus"})
	if permMode(r.mode.Load()) != modeAuto || !strings.Contains(sb.String(), "unknown mode") {
		t.Fatal(sb.String())
	}
}
