package update

import (
	"context"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

func putFile(t *testing.T, p, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

func fileGone(t *testing.T, p string) {
	t.Helper()
	if _, err := os.Lstat(p); !os.IsNotExist(err) {
		t.Errorf("%s still present (err=%v)", p, err)
	}
}

func TestNotWritableChangesNothing(t *testing.T) {
	if runtime.GOOS == "windows" || os.Geteuid() == 0 {
		t.Skip("needs a non-root unix user to make the install dir read-only")
	}
	fg := newFakeGitHub(t, "v0.9.14")
	archive := tarGzWith(t, "chaude", []byte("NEW"))
	name := "chaude_0.9.14_linux_amd64.tar.gz"
	fg.assets[name] = archive
	fg.assets["checksums.txt"] = []byte(sumLine(archive, name))
	exe := writeExe(t, "chaude")
	dir := filepath.Dir(exe)
	if err := os.Chmod(dir, 0o555); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.Chmod(dir, 0o755) })

	_, err := Run(context.Background(), "0.9.13", fg.options(exe, "linux", "amd64"))
	if err == nil || !strings.Contains(err.Error(), "run: sudo chaude update") {
		t.Fatalf("err = %v, want sudo hint", err)
	}
	if got := readFile(t, exe); got != "OLD" {
		t.Errorf("binary = %q, want OLD", got)
	}
	if len(fg.hits) != 0 {
		t.Errorf("downloaded assets %v, want none", fg.hits)
	}
}

func TestMigrateLegacyRemovesOldInstallAndPathLine(t *testing.T) {
	home := t.TempDir()
	legacy := filepath.Join(home, ".local", "bin")
	for _, n := range []string{"chaude", "chaude.exe", "chaude.0.9.12.bak", "chaude.0.9.11.bak"} {
		putFile(t, filepath.Join(legacy, n), "OLD")
	}
	putFile(t, filepath.Join(legacy, "other"), "KEEP")
	line := `export PATH="` + legacy + `:$PATH"`
	rc := filepath.Join(home, ".zshrc")
	putFile(t, rc, "export FOO=1\n# chaude\n"+line+"\nalias x=y\n")
	exe := writeExe(t, "chaude") // running binary lives elsewhere

	migrateLegacy(home, "darwin", exe)

	for _, n := range []string{"chaude", "chaude.exe", "chaude.0.9.12.bak", "chaude.0.9.11.bak"} {
		fileGone(t, filepath.Join(legacy, n))
	}
	if got := readFile(t, filepath.Join(legacy, "other")); got != "KEEP" {
		t.Errorf("unrelated file changed: %q", got)
	}
	if got, want := readFile(t, rc), "export FOO=1\n# chaude\nalias x=y\n"; got != want {
		t.Errorf("rc = %q, want %q", got, want)
	}
}

func TestMigrateLegacySkipsWhenRunningLegacyBinary(t *testing.T) {
	home := t.TempDir()
	legacy := filepath.Join(home, ".local", "bin")
	exe := filepath.Join(legacy, "chaude")
	putFile(t, exe, "RUNNING")
	line := `export PATH="` + legacy + `:$PATH"`
	rc := filepath.Join(home, ".profile")
	putFile(t, rc, line+"\n")

	migrateLegacy(home, "linux", exe)

	if got := readFile(t, exe); got != "RUNNING" {
		t.Errorf("running legacy binary changed: %q", got)
	}
	if got := readFile(t, rc); got != line+"\n" {
		t.Errorf("rc changed while running legacy binary: %q", got)
	}
}

func TestMigrateLegacyNoopWithoutLegacyFiles(t *testing.T) {
	home := t.TempDir()
	line := `export PATH="` + filepath.Join(home, ".local", "bin") + `:$PATH"`
	rc := filepath.Join(home, ".bashrc")
	putFile(t, rc, line+"\n")
	exe := writeExe(t, "chaude")

	migrateLegacy(home, "linux", exe)

	if got := readFile(t, rc); got != line+"\n" {
		t.Errorf("rc changed without legacy files: %q", got)
	}
}
