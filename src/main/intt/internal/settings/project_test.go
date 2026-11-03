package settings

import (
	"bytes"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// TestEnsureProjectSettingsCreatesGuard: no hook folder anywhere on disk; settings.json and the embedded hook scripts appear.
func TestEnsureProjectSettingsCreatesGuard(t *testing.T) {
	cwd := t.TempDir()
	home := t.TempDir()
	created, err := EnsureProjectSettings(cwd, home)
	if err != nil || !created {
		t.Fatalf("created=%v err=%v", created, err)
	}
	b, err := os.ReadFile(filepath.Join(cwd, ".claude", "settings.json"))
	if err != nil {
		t.Fatal(err)
	}
	s := string(b)
	if !strings.Contains(s, `"PreToolUse"`) || !strings.Contains(s, `python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\"`) {
		t.Fatalf("guard hook missing:\n%s", s)
	}
	for _, f := range guardFiles {
		got, err := os.ReadFile(filepath.Join(cwd, ".claude", "hooks", f))
		if err != nil {
			t.Fatalf("hook script not written: %s: %v", f, err)
		}
		want, ok := EmbeddedHook(f)
		if !ok || !bytes.Equal(got, want) {
			t.Fatalf("hook %s differs from the embedded copy", f)
		}
	}
}

// TestEmbeddedHooksMatchRepo: the embedded copies equal the repo's .claude/hooks (skipped when run outside the repo).
func TestEmbeddedHooksMatchRepo(t *testing.T) {
	repoHooks := filepath.Join("..", "..", "..", "..", "..", ".claude", "hooks")
	if _, err := os.Stat(repoHooks); err != nil {
		t.Skip("repo .claude/hooks not reachable from here")
	}
	ents, err := hookFS.ReadDir("hooks")
	if err != nil || len(ents) == 0 {
		t.Fatalf("no embedded hooks: %v", err)
	}
	for _, e := range ents {
		emb, _ := EmbeddedHook(e.Name())
		repo, err := os.ReadFile(filepath.Join(repoHooks, e.Name()))
		if err != nil {
			t.Errorf("%s: repo copy missing: %v", e.Name(), err)
			continue
		}
		if !bytes.Equal(emb, repo) {
			t.Errorf("%s: embedded copy differs from .claude/hooks; copy it again into internal/settings/hooks/", e.Name())
		}
	}
}

func TestEnsureProjectSettingsKeepsExisting(t *testing.T) {
	cwd := t.TempDir()
	home := t.TempDir()
	existing := filepath.Join(cwd, ".claude", "settings.json")
	if err := os.MkdirAll(filepath.Dir(existing), 0o755); err != nil {
		t.Fatal(err)
	}
	const user = `{"model":"sonnet"}`
	if err := os.WriteFile(existing, []byte(user), 0o644); err != nil {
		t.Fatal(err)
	}
	created, err := EnsureProjectSettings(cwd, home)
	if err != nil || created {
		t.Fatalf("created=%v err=%v", created, err)
	}
	b, _ := os.ReadFile(existing)
	if string(b) != user {
		t.Fatalf("existing settings changed: %s", b)
	}
	if _, err := os.Stat(filepath.Join(cwd, ".claude", "hooks", "main-thread-guard.py")); err == nil {
		t.Fatal("hook scripts copied into a project that already has settings.json")
	}
}

func TestEnsureProjectSettingsSkipsHome(t *testing.T) {
	home := t.TempDir()
	created, err := EnsureProjectSettings(home, home)
	if err != nil || created {
		t.Fatalf("created=%v err=%v", created, err)
	}
	if _, err := os.Stat(filepath.Join(home, ".claude")); err == nil {
		t.Fatal("home was written")
	}
}

// hookPython: python3, then python, then py -3, first found on PATH.
func TestHookPythonOrder(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("LookPath needs PATHEXT names on windows")
	}
	cases := []struct {
		have []string
		want string
	}{
		{[]string{"python3", "python", "py"}, "python3"},
		{[]string{"python", "py"}, "python"},
		{[]string{"py"}, "py -3"},
		{nil, "python"},
	}
	for _, c := range cases {
		dir := t.TempDir()
		for _, n := range c.have {
			if err := os.WriteFile(filepath.Join(dir, n), []byte("#!/bin/sh\n"), 0o755); err != nil {
				t.Fatal(err)
			}
		}
		t.Setenv("PATH", dir)
		if got := hookPython(); got != c.want {
			t.Errorf("have %v: got %q want %q", c.have, got, c.want)
		}
	}
}
