package harness

import (
	"strings"
	"testing"
)

func TestCleanEnvDropsAPIKey(t *testing.T) {
	t.Setenv("ANTHROPIC_API_KEY", "x")
	for _, kv := range cleanEnv(0) {
		if strings.HasPrefix(kv, "ANTHROPIC_API_KEY=") {
			t.Fatal("ANTHROPIC_API_KEY must be absent from child env")
		}
	}
}
