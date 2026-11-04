package settings

import (
	"bytes"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// TestEnsureProjectSettingsCreatesGuard: no hook folder anywhere on disk; settings.json, the agent definitions and the embedded hook scripts appear.
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
	if !strings.Contains(s, `"Agent|Task"`) {
		t.Fatalf("Agent|Task guard group missing:\n%s", s)
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
	ents, err := agentFS.ReadDir("agents")
	if err != nil || len(ents) == 0 {
		t.Fatalf("no embedded agents: %v", err)
	}
	for _, e := range ents {
		got, err := os.ReadFile(filepath.Join(cwd, ".claude", "agents", e.Name()))
		if err != nil {
			t.Fatalf("agent not written: %s: %v", e.Name(), err)
		}
		want, _ := agentFS.ReadFile("agents/" + e.Name())
		if !bytes.Equal(got, want) {
			t.Fatalf("agent %s differs from the embedded copy", e.Name())
		}
	}
}

// TestEnsureProjectAgentsKeepExisting: an agent file the user already has is never overwritten; the rest are written.
func TestEnsureProjectAgentsKeepExisting(t *testing.T) {
	cwd := t.TempDir()
	mine := filepath.Join(cwd, ".claude", "agents", "haiku-coder.md")
	if err := os.MkdirAll(filepath.Dir(mine), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(mine, []byte("mine"), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := EnsureProjectSettings(cwd, t.TempDir()); err != nil {
		t.Fatal(err)
	}
	if b, _ := os.ReadFile(mine); string(b) != "mine" {
		t.Fatalf("existing agent overwritten: %s", b)
	}
	if _, err := os.Stat(filepath.Join(cwd, ".claude", "agents", "opus-architect.md")); err != nil {
		t.Fatalf("opus-architect.md not written: %v", err)
	}
}

// TestEnsureProjectSettingsMergesExisting: user keys and user hook groups stay; the guard groups are appended once.
func TestEnsureProjectSettingsMergesExisting(t *testing.T) {
	cwd := t.TempDir()
	home := t.TempDir()
	existing := filepath.Join(cwd, ".claude", "settings.json")
	if err := os.MkdirAll(filepath.Dir(existing), 0o755); err != nil {
		t.Fatal(err)
	}
	const user = `{
  "model": "sonnet",
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [{"type": "command", "command": "echo mine"}]
      }
    ]
  }
}
`
	if err := os.WriteFile(existing, []byte(user), 0o644); err != nil {
		t.Fatal(err)
	}
	created, err := EnsureProjectSettings(cwd, home)
	if err != nil || created {
		t.Fatalf("created=%v err=%v", created, err)
	}
	first, _ := os.ReadFile(existing)
	s := string(first)
	for _, want := range []string{`"model": "sonnet"`, `"echo mine"`, `"Agent|Task"`, `main-thread-guard.py`} {
		if !strings.Contains(s, want) {
			t.Fatalf("merged settings lack %s:\n%s", want, s)
		}
	}
	if strings.Index(s, `"echo mine"`) > strings.Index(s, `"Agent|Task"`) {
		t.Fatalf("user hook group should stay before the guard groups:\n%s", s)
	}
	// second run: nothing to add, file unchanged
	if _, err := EnsureProjectSettings(cwd, home); err != nil {
		t.Fatal(err)
	}
	second, _ := os.ReadFile(existing)
	if !bytes.Equal(first, second) {
		t.Fatalf("second run changed settings:\n%s\n---\n%s", first, second)
	}
}

// TestMergeGuardRejectsBadJSON: a settings file that is not an object is an error and the caller leaves it alone.
func TestMergeGuardRejectsBadJSON(t *testing.T) {
	cwd := t.TempDir()
	existing := filepath.Join(cwd, ".claude", "settings.json")
	if err := os.MkdirAll(filepath.Dir(existing), 0o755); err != nil {
		t.Fatal(err)
	}
	const bad = `[1, 2]`
	if err := os.WriteFile(existing, []byte(bad), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := EnsureProjectSettings(cwd, t.TempDir()); err == nil {
		t.Fatal("expected an error for a non-object settings.json")
	}
	if b, _ := os.ReadFile(existing); string(b) != bad {
		t.Fatalf("bad settings changed: %s", b)
	}
}

// TestEnsureProjectSettingsSkipsHome: cwd == home writes nothing (the user-global .claude).
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

// TestEmbeddedHooksMatchRepo: the embedded copies equal the repo's .claude/hooks (skipped when run outside the repo).
func TestEmbeddedHooksMatchRepo(t *testing.T) {
	repoHooks := filepath.Join("..", "..", "..", "..", "..", "..", ".claude", "hooks")
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

// TestEmbeddedAgentsMatchRepo: the embedded agent definitions equal the repo's .claude/agents (skipped outside the repo).
func TestEmbeddedAgentsMatchRepo(t *testing.T) {
	repoAgents := filepath.Join("..", "..", "..", "..", "..", "..", ".claude", "agents")
	if _, err := os.Stat(repoAgents); err != nil {
		t.Skip("repo .claude/agents not reachable from here")
	}
	ents, err := agentFS.ReadDir("agents")
	if err != nil || len(ents) == 0 {
		t.Fatalf("no embedded agents: %v", err)
	}
	for _, e := range ents {
		emb, _ := agentFS.ReadFile("agents/" + e.Name())
		repo, err := os.ReadFile(filepath.Join(repoAgents, e.Name()))
		if err != nil {
			t.Errorf("%s: repo copy missing: %v", e.Name(), err)
			continue
		}
		if !bytes.Equal(emb, repo) {
			t.Errorf("%s: embedded copy differs from .claude/agents; copy it again into internal/settings/agents/", e.Name())
		}
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
