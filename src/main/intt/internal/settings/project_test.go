package settings

import (
	"bytes"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// fakeHooks is a .claude/hooks folder with the guard hook scripts (the layout HookSource looks for).
func fakeHooks(t *testing.T) string {
	t.Helper()
	dir := filepath.Join(t.TempDir(), ".claude", "hooks")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	for _, f := range guardFiles {
		if err := os.WriteFile(filepath.Join(dir, f), []byte("# "+f+"\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return dir
}

// captureWarn replaces warnOut for one test.
func captureWarn(t *testing.T) *bytes.Buffer {
	t.Helper()
	var buf bytes.Buffer
	old := warnOut
	warnOut = &buf
	t.Cleanup(func() { warnOut = old })
	return &buf
}

func TestEnsureProjectSettingsCreatesGuard(t *testing.T) {
	src := fakeHooks(t)
	cwd := t.TempDir()
	home := t.TempDir()
	created, err := EnsureProjectSettings(cwd, home, src)
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
		if _, err := os.Stat(filepath.Join(cwd, ".claude", "hooks", f)); err != nil {
			t.Fatalf("hook script not copied: %s: %v", f, err)
		}
	}
}

func TestEnsureProjectSettingsKeepsExisting(t *testing.T) {
	src := fakeHooks(t)
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
	created, err := EnsureProjectSettings(cwd, home, src)
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
	src := fakeHooks(t)
	home := t.TempDir()
	created, err := EnsureProjectSettings(home, home, src)
	if err != nil || created {
		t.Fatalf("created=%v err=%v", created, err)
	}
	if _, err := os.Stat(filepath.Join(home, ".claude")); err == nil {
		t.Fatal("home was written")
	}
}

// no hook source (empty srcDir, or a folder missing a guard script): one warning line, nothing written, no error.
func TestEnsureProjectSettingsNoSourceWarnsAndSkips(t *testing.T) {
	partial := fakeHooks(t)
	if err := os.Remove(filepath.Join(partial, "_lean.py")); err != nil {
		t.Fatal(err)
	}
	for _, src := range []string{"", partial, t.TempDir()} {
		warn := captureWarn(t)
		cwd := t.TempDir()
		created, err := EnsureProjectSettings(cwd, t.TempDir(), src)
		if err != nil || created {
			t.Fatalf("src=%q: want no error and no creation, got created=%v err=%v", src, created, err)
		}
		if _, err := os.Stat(filepath.Join(cwd, ".claude")); err == nil {
			t.Fatalf("src=%q: .claude created despite no hook source", src)
		}
		if n := strings.Count(warn.String(), "\n"); n != 1 {
			t.Fatalf("src=%q: want one warning line, got %d:\n%s", src, n, warn.String())
		}
	}
}

func TestHookSourceCandidates(t *testing.T) {
	// bin/ layout: <root>/bin/chaude with <root>/.claude/hooks
	root := t.TempDir()
	hooks := filepath.Join(root, ".claude", "hooks")
	if err := os.MkdirAll(filepath.Join(root, "bin"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(hooks, 0o755); err != nil {
		t.Fatal(err)
	}
	for _, f := range guardFiles {
		if err := os.WriteFile(filepath.Join(hooks, f), []byte("x"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	if got := hookSource(filepath.Join(root, "bin", "chaude")); got != filepath.Join(root, "bin", "..", ".claude", "hooks") {
		t.Fatalf("bin layout: got %q want %q", got, hooks)
	}
	// flat layout: <root>/chaude with <root>/.claude/hooks (found through the second candidate)
	if got := hookSource(filepath.Join(root, "chaude")); got != filepath.Join(root, ".claude", "hooks") {
		t.Fatalf("flat layout: got %q", got)
	}
	// nothing next to the binary (what the release installers leave behind)
	if got := hookSource(filepath.Join(t.TempDir(), "usr", "local", "bin", "chaude")); got != "" {
		t.Fatalf("no source: got %q", got)
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
