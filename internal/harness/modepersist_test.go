package harness

import (
	"os"
	"testing"
)

// Shift+Tab / "/mode" changes are remembered per folder and restored on the next start.
func TestModePersistence(t *testing.T) {
	dir := t.TempDir()
	if m := loadMode(dir); m != modeDefault {
		t.Fatalf("fresh folder: %v", m)
	}
	saveMode(dir, modeAuto)
	if m := loadMode(dir); m != modeAuto {
		t.Fatalf("round trip: %v", m)
	}
	saveMode(dir, modeAcceptEdits)
	if m := loadMode(dir); m != modeAcceptEdits {
		t.Fatalf("overwrite: %v", m)
	}
	if err := os.WriteFile(modeFile(dir), []byte("garbage"), 0o644); err != nil {
		t.Fatal(err)
	}
	if m := loadMode(dir); m != modeDefault {
		t.Fatalf("garbage must fall back to default: %v", m)
	}
	// the TUI and the plain loop both go through saveMode; the status line shows a non-default mode
	r := &Repl{a: Args{Cwd: dir}, out: os.Stdout}
	r.modeLine([]string{"plan"})
	if m := loadMode(dir); m != modePlan {
		t.Fatalf("modeLine did not persist: %v", m)
	}
}
