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
