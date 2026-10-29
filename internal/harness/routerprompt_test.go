package harness

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

// sonnetPromptSHA256 freezes classifyPromptSonnet; change it only with a deliberate prompt change.
const sonnetPromptSHA256 = "bd3790c78a32bcadd964a0bd57431bfb483dbbd7f17a122579f312a51b2c6e62"

func promptSHA256(s string) string {
	sum := sha256.Sum256([]byte(s))
	return hex.EncodeToString(sum[:])
}

// TestRouterPromptDump writes both router prompts and their hashes to $CHAUDE_DUMP_PROMPTS.
func TestRouterPromptDump(t *testing.T) {
	dir := os.Getenv("CHAUDE_DUMP_PROMPTS")
	if dir == "" {
		t.Skip("set CHAUDE_DUMP_PROMPTS=<dir> to dump router prompts")
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "sonnet.txt"), []byte(classifyPromptSonnet), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "haiku.txt"), []byte(classifyPromptHaiku), 0o644); err != nil {
		t.Fatal(err)
	}
	hashes := fmt.Sprintf("sonnet %s\nhaiku %s\n", promptSHA256(classifyPromptSonnet), promptSHA256(classifyPromptHaiku))
	if err := os.WriteFile(filepath.Join(dir, "hashes.txt"), []byte(hashes), 0o644); err != nil {
		t.Fatal(err)
	}
}

// TestSonnetPromptFrozen fails if the Sonnet router prompt text changes.
func TestSonnetPromptFrozen(t *testing.T) {
	if got := promptSHA256(classifyPromptSonnet); got != sonnetPromptSHA256 {
		t.Fatalf("classifyPromptSonnet sha256 = %s, want %s", got, sonnetPromptSHA256)
	}
}

// TestRouterModelEnvCases checks CHAUDE_ROUTER_MODEL parsing (trim, lowercase, fallback to sonnet).
func TestRouterModelEnvCases(t *testing.T) {
	cases := []struct {
		name  string
		env   string
		unset bool
		want  string
	}{
		{name: "unset", unset: true, want: "haiku"},
		{name: "sonnet", env: "sonnet", want: "sonnet"},
		{name: "SONNET with space", env: "SONNET ", want: "sonnet"},
		{name: "haiku", env: "haiku", want: "haiku"},
		{name: "Haiku with leading space", env: " Haiku", want: "haiku"},
		{name: "opus falls back", env: "opus", want: "haiku"},
		{name: "x falls back", env: "x", want: "haiku"},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			t.Setenv(routerModelEnv, c.env)
			if c.unset {
				os.Unsetenv(routerModelEnv)
			}
			if got := routerModel(); got != c.want {
				t.Errorf("CHAUDE_ROUTER_MODEL=%q (unset=%v): got %q, want %q", c.env, c.unset, got, c.want)
			}
		})
	}
}
