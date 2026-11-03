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

func TestForwardRuleFor(t *testing.T) {
	if forwardRuleFor("haiku") != forwardRule {
		t.Errorf("forwardRuleFor(haiku) must return forwardRule")
	}
	if forwardRuleFor("") != forwardRule {
		t.Errorf("forwardRuleFor(empty) must return forwardRule")
	}
	if forwardRuleFor("opus") != "" {
		t.Errorf("forwardRuleFor(opus) must be empty")
	}
	if forwardRuleFor("claude-opus-4") != "" {
		t.Errorf("forwardRuleFor(claude-opus-4) must be empty")
	}
	if !strings.Contains(forwardRule, "opus-architect") {
		t.Errorf("forwardRule must name the opus-architect agent")
	}
}
