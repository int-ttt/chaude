package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// /model X is remembered in the project's .claude/settings.json and is the REPL's default next time.
func TestMainModelPersistence(t *testing.T) {
	dir := t.TempDir()
	if m := loadMainModel(dir); m != "" {
		t.Fatalf("fresh folder: %q", m)
	}
	if err := saveMainModel(dir, "fable"); err != nil {
		t.Fatal(err)
	}
	if m := loadMainModel(dir); m != "fable" {
		t.Fatalf("round trip: %q", m)
	}
	// an existing settings.json keeps its other keys
	p := filepath.Join(dir, ".claude", "settings.json")
	if err := os.WriteFile(p, []byte("{\n  \"model\": \"opus\",\n  \"effortLevel\": \"medium\",\n  \"env\": {\"K\": \"v\"}\n}\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := saveMainModel(dir, "fable"); err != nil {
		t.Fatal(err)
	}
	b, _ := os.ReadFile(p)
	if !strings.Contains(string(b), `"model": "fable"`) || !strings.Contains(string(b), `"effortLevel": "medium"`) || !strings.Contains(string(b), `"K": "v"`) {
		t.Fatalf("settings after save:\n%s", b)
	}
	r := &Repl{a: Args{Cwd: dir, Model: "fable"}}
	if txt := r.modelText(); !strings.Contains(txt, "주력 모델: fable") || !strings.Contains(txt, "/model opus | fable") {
		t.Fatalf("modelText: %s", txt)
	}
}
