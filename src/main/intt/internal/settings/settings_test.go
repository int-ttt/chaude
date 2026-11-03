package settings

import (
	"os"
	"path/filepath"
	"testing"
)

func writeSettings(t *testing.T, dir string) string {
	t.Helper()
	p := filepath.Join(dir, ".claude", "settings.json")
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, []byte("{}"), 0o644); err != nil {
		t.Fatal(err)
	}
	return p
}

func TestDefaultSrcPrecedence(t *testing.T) {
	root := t.TempDir()
	proj := filepath.Join(root, "proj")
	cwd := filepath.Join(root, "elsewhere")
	exe := filepath.Join(proj, "bin", "chaude")
	projSrc := writeSettings(t, proj)
	cwdSrc := writeSettings(t, cwd)

	// 1. CLAUDE_PROJECT_DIR wins over everything.
	if got := defaultSrc(proj, cwd, exe); got != filepath.Join(proj, ".claude", "settings.json") {
		t.Fatalf("env: %s", got)
	}
	// 2. no env: exe-relative <project>/bin/chaude -> <project>/.claude when it exists.
	if got := defaultSrc("", cwd, exe); got != projSrc {
		t.Fatalf("exe-relative: got %s want %s", got, projSrc)
	}
	// 3. binary elsewhere (no bin/ layout): fall back to cwd/.claude.
	foreignExe := filepath.Join(root, "tmp", "go-build", "exe", "chaude")
	if got := defaultSrc("", cwd, foreignExe); got != cwdSrc {
		t.Fatalf("cwd fallback: got %s want %s", got, cwdSrc)
	}
	// 4. nothing exists: exe form returned so the read error names a concrete path.
	if got := defaultSrc("", filepath.Join(root, "none"), foreignExe); got != filepath.Join(root, "tmp", "go-build", "exe", "..", ".claude", "settings.json") {
		t.Fatalf("none: %s", got)
	}
}

// TestGenSettingsCreatesSettingsFolder: in a fresh temp dir, --out into a missing .claude folder creates .claude/,
// .claude/hooks/ and the output file.
func TestGenSettingsCreatesSettingsFolder(t *testing.T) {
	root := t.TempDir()
	src := filepath.Join(root, "src.json")
	if err := os.WriteFile(src, []byte("{}"), 0o644); err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(root, "fresh", ".claude", "settings.json")
	if code := Main([]string{"mac", "--src", src, "--out", out}); code != 0 {
		t.Fatalf("Main exit %d", code)
	}
	for _, d := range []string{filepath.Join(root, "fresh", ".claude"), filepath.Join(root, "fresh", ".claude", "hooks")} {
		if st, err := os.Stat(d); err != nil || !st.IsDir() {
			t.Fatalf("folder not created: %s: %v", d, err)
		}
	}
	if _, err := os.Stat(out); err != nil {
		t.Fatalf("output not written: %v", err)
	}
}
