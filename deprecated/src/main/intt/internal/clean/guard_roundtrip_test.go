package clean

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"chaude/internal/settings"
)

// TestGuardSettingsRemovedByClean: what EnsureProjectSettings creates from the embedded copies, clean --yes removes (backup kept).
func TestGuardSettingsRemovedByClean(t *testing.T) {
	cwd := t.TempDir()
	if ok, err := settings.EnsureProjectSettings(cwd, t.TempDir()); err != nil || !ok {
		t.Fatalf("ensure: %v %v", ok, err)
	}
	p, err := MakePlan(cwd)
	if err != nil {
		t.Fatal(err)
	}
	if len(p.Kept()) != 0 {
		t.Fatalf("embedded copies should match, kept: %v", p.Kept())
	}
	if _, err := p.Apply(time.Now()); err != nil {
		t.Fatal(err)
	}
	for _, rel := range []string{".claude/settings.json", ".claude/hooks/main-thread-guard.py", ".claude/hooks/_lean.py", ".claude"} {
		if _, err := os.Stat(filepath.Join(cwd, filepath.FromSlash(rel))); err == nil {
			t.Fatalf("still present after clean: %s", rel)
		}
	}
}
