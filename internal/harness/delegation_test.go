package harness

import (
	"strings"
	"testing"
)

func TestDelegationRuleNamesHaikuCoder(t *testing.T) {
	if !strings.Contains(delegationRule, "haiku-coder") {
		t.Fatalf("delegationRule must name the haiku-coder agent: %q", delegationRule)
	}
}
