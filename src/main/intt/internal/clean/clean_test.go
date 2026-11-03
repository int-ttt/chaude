package clean

import (
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
	"testing"
	"time"

	"chaude/internal/settings"
)

// installFixture writes a project as the old install step left it: package files, CLAUDE.md policy block
// above user content, settings.json with user keys and a user hook next to the package hook, plus notes/ and record/.
func installFixture(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	for _, rel := range pkgFiles {
		body := "pkg " + rel
		if strings.HasPrefix(rel, hooksDir) {
			b, ok := settings.EmbeddedHook(path.Base(rel))
			if !ok {
				t.Fatalf("no embedded copy of %s", rel)
			}
			body = string(b) // package copy: clean removes it
		}
		writeTest(t, filepath.Join(dir, filepath.FromSlash(rel)), body)
	}
	writeTest(t, filepath.Join(dir, "CLAUDE.md"), markStart+"\npolicy\n"+markEnd+"\n\n# my project\nkeep me\n")
	writeTest(t, filepath.Join(dir, ".claude", "settings.json"), `{
  "model": "opus",
  "env": {"X": "1"},
  "hooks": {
    "PreToolUse": [
      {"matcher": "Bash", "hooks": [
        {"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/main-thread-guard.py"},
        {"type": "command", "command": "/usr/local/bin/my-own-hook.sh"}
      ]}
    ],
    "Stop": [
      {"hooks": [{"type": "command", "command": "python3 .claude/hooks/ledger.py"}]}
    ]
  }
}`)
	writeTest(t, filepath.Join(dir, "notes", "keep.md"), "notes")
	writeTest(t, filepath.Join(dir, "record", "keep.jsonl"), "record")
	writeTest(t, filepath.Join(dir, ".claude", "user-file.txt"), "user")
	return dir
}

// useHookFn replaces the package hook lookup for one test (nil-returning fn: no package copies).
func useHookFn(t *testing.T, fn func(string) ([]byte, bool)) {
	t.Helper()
	old := pkgHookFn
	pkgHookFn = fn
	t.Cleanup(func() { pkgHookFn = old })
}

func noPkgHook(string) ([]byte, bool) { return nil, false }

// TestApplyKeepsUserModifiedHook: a hook file that differs from the package copy is left alone; identical ones go.
func TestApplyKeepsUserModifiedHook(t *testing.T) {
	dir := installFixture(t)
	const edited = ".claude/hooks/main-thread-guard.py"
	writeTest(t, filepath.Join(dir, filepath.FromSlash(edited)), "my own guard edit")
	p, err := MakePlan(dir)
	if err != nil {
		t.Fatal(err)
	}
	if !containsStr(p.Kept(), edited) {
		t.Fatalf("modified hook not reported as kept: %v", p.Kept())
	}
	if _, err := p.Apply(time.Date(2026, 11, 1, 9, 30, 0, 0, time.UTC)); err != nil {
		t.Fatal(err)
	}
	b, err := os.ReadFile(filepath.Join(dir, filepath.FromSlash(edited)))
	if err != nil || string(b) != "my own guard edit" {
		t.Fatalf("modified hook changed or removed: %q %v", b, err)
	}
	if exists(filepath.Join(dir, ".claude", "hooks", "ledger.py")) {
		t.Error("unmodified package hook was not removed")
	}
	s, _ := os.ReadFile(filepath.Join(dir, ".claude", "settings.json"))
	if !strings.Contains(string(s), "main-thread-guard.py") {
		t.Errorf("settings entry of the kept hook was removed:\n%s", s)
	}
	if strings.Contains(string(s), "ledger.py") {
		t.Errorf("settings entry of the removed hook stayed:\n%s", s)
	}
}

// TestApplyKeepsHooksWithoutPackageCopy: with no package hook folder nothing can be proven identical, so hooks stay.
func TestApplyKeepsHooksWithoutPackageCopy(t *testing.T) {
	dir := installFixture(t)
	useHookFn(t, noPkgHook)
	p, err := MakePlan(dir)
	if err != nil {
		t.Fatal(err)
	}
	var hooks int
	for _, rel := range pkgFiles {
		if strings.HasPrefix(rel, hooksDir) {
			hooks++
		}
	}
	if len(p.Kept()) != hooks || hooks == 0 {
		t.Fatalf("kept=%d want all %d hook files", len(p.Kept()), hooks)
	}
	if _, err := p.Apply(time.Date(2026, 11, 1, 9, 30, 0, 0, time.UTC)); err != nil {
		t.Fatal(err)
	}
	for _, rel := range p.Kept() {
		if !exists(filepath.Join(dir, filepath.FromSlash(rel))) {
			t.Errorf("kept hook removed: %s", rel)
		}
	}
	for _, rel := range pkgFiles {
		if !strings.HasPrefix(rel, hooksDir) && exists(filepath.Join(dir, filepath.FromSlash(rel))) {
			t.Errorf("non-hook manifest file still present: %s", rel)
		}
	}
}

func containsStr(list []string, s string) bool {
	for _, v := range list {
		if v == s {
			return true
		}
	}
	return false
}

func writeTest(t *testing.T, p, body string) {
	t.Helper()
	if err := writeFile(p, []byte(body)); err != nil {
		t.Fatal(err)
	}
}

func exists(p string) bool {
	_, err := os.Stat(p)
	return err == nil
}

func TestDryRunChangesNothing(t *testing.T) {
	dir := installFixture(t)
	before := snapshot(t, dir)
	p, err := MakePlan(dir)
	if err != nil {
		t.Fatal(err)
	}
	if len(p.Lines()) == 0 {
		t.Fatal("expected targets in a fixture with package files")
	}
	after := snapshot(t, dir)
	if strings.Join(before, "\n") != strings.Join(after, "\n") {
		t.Fatalf("dry-run changed the tree")
	}
}

func TestApplyRemovesManifestAndBacksUp(t *testing.T) {
	dir := installFixture(t)
	p, err := MakePlan(dir)
	if err != nil {
		t.Fatal(err)
	}
	bdir, err := p.Apply(time.Date(2026, 11, 1, 9, 30, 0, 0, time.UTC))
	if err != nil {
		t.Fatal(err)
	}
	if filepath.Base(bdir) != ".chaude-backup-20261101-093000" {
		t.Fatalf("backup name = %s", filepath.Base(bdir))
	}
	for _, rel := range pkgFiles {
		if exists(filepath.Join(dir, filepath.FromSlash(rel))) {
			t.Errorf("manifest file still present: %s", rel)
		}
		if !exists(filepath.Join(bdir, filepath.FromSlash(rel))) {
			t.Errorf("manifest file not backed up: %s", rel)
		}
	}
	for _, rel := range []string{"CLAUDE.md", ".claude/settings.json"} {
		if !exists(filepath.Join(bdir, filepath.FromSlash(rel))) {
			t.Errorf("%s not in backup", rel)
		}
	}
	if !exists(filepath.Join(dir, ".claude", "user-file.txt")) {
		t.Error("file outside manifest was removed")
	}
	if !exists(filepath.Join(dir, ".claude")) {
		t.Error(".claude removed although a user file is in it")
	}
}

func TestApplyKeepsNotesRecordAndUserSettings(t *testing.T) {
	dir := installFixture(t)
	p, err := MakePlan(dir)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := p.Apply(time.Now()); err != nil {
		t.Fatal(err)
	}
	for _, rel := range []string{"notes/keep.md", "record/keep.jsonl"} {
		if !exists(filepath.Join(dir, filepath.FromSlash(rel))) {
			t.Errorf("%s was removed", rel)
		}
	}
	md, _ := os.ReadFile(filepath.Join(dir, "CLAUDE.md"))
	if string(md) != "# my project\nkeep me\n" {
		t.Errorf("CLAUDE.md after clean = %q", md)
	}
	sb, err := os.ReadFile(filepath.Join(dir, ".claude", "settings.json"))
	if err != nil {
		t.Fatal(err)
	}
	s := string(sb)
	for _, want := range []string{`"model": "opus"`, `"X": "1"`, "my-own-hook.sh"} {
		if !strings.Contains(s, want) {
			t.Errorf("settings lost %q:\n%s", want, s)
		}
	}
	for _, gone := range []string{"main-thread-guard.py", "ledger.py"} {
		if strings.Contains(s, gone) {
			t.Errorf("package hook %s still in settings", gone)
		}
	}
}

func TestClaudeMdWithoutBlockUntouched(t *testing.T) {
	dir := t.TempDir()
	writeTest(t, filepath.Join(dir, "CLAUDE.md"), "plain user file\n")
	p, err := MakePlan(dir)
	if err != nil {
		t.Fatal(err)
	}
	if len(p.Lines()) != 0 {
		t.Fatalf("expected no targets, got %v", p.Lines())
	}
}

func TestClaudeMdOnlyBlockIsDeleted(t *testing.T) {
	dir := t.TempDir()
	writeTest(t, filepath.Join(dir, "CLAUDE.md"), "\xef\xbb\xbf"+markStart+"\r\npolicy\r\n"+markEnd+"\r\n")
	p, err := MakePlan(dir)
	if err != nil {
		t.Fatal(err)
	}
	if !p.mdDelete {
		t.Fatal("expected CLAUDE.md to be deleted when only the block remains")
	}
	if _, err := p.Apply(time.Now()); err != nil {
		t.Fatal(err)
	}
	if exists(filepath.Join(dir, "CLAUDE.md")) {
		t.Error("CLAUDE.md still exists")
	}
}

func TestBrokenMarkersFailBeforeAnyChange(t *testing.T) {
	dir := t.TempDir()
	writeTest(t, filepath.Join(dir, "CLAUDE.md"), markStart+"\nno end\n")
	if _, err := MakePlan(dir); err == nil {
		t.Fatal("expected error for start marker without end marker")
	}
}

// TestManifestMatchesRepo keeps pkgFiles equal to the tracked .claude/{agents,hooks,skills} tree of this repo.
func TestManifestMatchesRepo(t *testing.T) {
	root := filepath.Join("..", "..", "..", "..", "..")
	var got []string
	for _, sub := range []string{"agents", "hooks", "skills"} {
		base := filepath.Join(root, ".claude", sub)
		err := filepath.Walk(base, func(p string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if info.IsDir() || strings.Contains(p, "__pycache__") {
				return nil
			}
			rel, err := filepath.Rel(root, p)
			if err != nil {
				return err
			}
			got = append(got, filepath.ToSlash(rel))
			return nil
		})
		if err != nil {
			t.Fatal(err)
		}
	}
	want := append([]string(nil), pkgFiles...)
	sort.Strings(got)
	sort.Strings(want)
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("manifest drift\nrepo:\n%s\nmanifest:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

// snapshot lists every path and its contents under dir, for before/after comparison.
func snapshot(t *testing.T, dir string) []string {
	t.Helper()
	var out []string
	err := filepath.Walk(dir, func(p string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			return nil
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		out = append(out, p+"\x00"+string(b))
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return out
}
