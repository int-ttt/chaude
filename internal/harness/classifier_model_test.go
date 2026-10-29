package harness

import (
	"os"
	"testing"
)

func TestRouterModelDefault(t *testing.T) {
	t.Setenv(routerModelEnv, "")
	os.Unsetenv(routerModelEnv)
	if m := routerModel(); m != "sonnet" {
		t.Fatalf("unset: got %q, want sonnet", m)
	}
	if routerPrompt(routerModel()) != classifyPromptSonnet {
		t.Error("unset: prompt must be the Sonnet prompt")
	}
}

func TestRouterModelHaiku(t *testing.T) {
	t.Setenv(routerModelEnv, "haiku")
	if m := routerModel(); m != "haiku" {
		t.Fatalf("haiku: got %q, want haiku", m)
	}
	if routerPrompt(routerModel()) != classifyPromptHaiku {
		t.Error("haiku: prompt must be the Haiku prompt")
	}
}

func TestRouterModelInvalidFallsBack(t *testing.T) {
	for _, v := range []string{"opus", "sonet", "bogus"} {
		t.Setenv(routerModelEnv, v)
		if m := routerModel(); m != "sonnet" {
			t.Errorf("%q: got %q, want sonnet", v, m)
		}
		if routerPrompt(routerModel()) != classifyPromptSonnet {
			t.Errorf("%q: prompt must fall back to the Sonnet prompt", v)
		}
	}
}
