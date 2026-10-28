package harness

import (
	"strings"
	"testing"
)

func TestConductorRule(t *testing.T) {
	for _, w := range []string{"haiku-", "sonnet-analyst", "opus-architect", "Never write the answer", "pushback", "corrections"} {
		if !strings.Contains(conductorRule, w) {
			t.Errorf("conductorRule missing %q", w)
		}
	}
	if conductorModel != "sonnet" {
		t.Fatal(conductorModel)
	}
}
