package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestSettingsDefaultsWhenMissing(t *testing.T) {
	s := loadChaudeSettings(filepath.Join(t.TempDir(), "none.json"))
	want := chaudeSettings{SessionRetentionDays: 30, ExtraCoders: 4, MainModel: "opus"}
	if s != want {
		t.Fatalf("defaults %+v, want %+v", s, want)
	}
	if got := loadChaudeSettings(""); got != want {
		t.Fatalf("empty path %+v, want defaults", got)
	}
}

func TestSettingsClamp(t *testing.T) {
	s := chaudeSettings{SessionRetentionDays: -5, ExtraCoders: 99, MainModel: "gpt"}.clamp()
	if s.SessionRetentionDays != 0 || s.ExtraCoders != 8 || s.MainModel != "opus" {
		t.Fatalf("clamp %+v", s)
	}
	if got := (chaudeSettings{ExtraCoders: -1, MainModel: "sonnet"}).clamp(); got.ExtraCoders != 0 || got.MainModel != "sonnet" {
		t.Fatalf("clamp low %+v", got)
	}
}

func TestSettingsSaveLoadRoundTrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), ".chaude", "settings.json")
	in := chaudeSettings{SessionRetentionDays: 0, ExtraCoders: 2, MainModel: "fable"}
	if err := saveChaudeSettings(path, in); err != nil {
		t.Fatal(err)
	}
	if out := loadChaudeSettings(path); out != in {
		t.Fatalf("round trip %+v, want %+v", out, in)
	}
	if _, err := os.Stat(path + ".tmp"); err == nil {
		t.Fatal("tmp file left behind")
	}
}

func TestSettingsPartialFileKeepsDefaults(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s.json")
	if err := os.WriteFile(path, []byte(`{"coder_slots": 6}`), 0o600); err != nil {
		t.Fatal(err)
	}
	s := loadChaudeSettings(path)
	if s.ExtraCoders != 6 || s.SessionRetentionDays != 30 || s.MainModel != "opus" {
		t.Fatalf("partial %+v", s)
	}
}

func TestSettingsModelID(t *testing.T) {
	cases := map[string]string{"opus": "claude-opus-5-5", "sonnet": "claude-sonnet-5-5", "fable": "claude-fable-5-1"}
	for alias, id := range cases {
		if got := (chaudeSettings{MainModel: alias}).modelID(); got != id {
			t.Fatalf("%s -> %s, want %s", alias, got, id)
		}
	}
}

func TestPruneRecordFiles(t *testing.T) {
	root := t.TempDir()
	proj := filepath.Join(root, "proj")
	if err := os.MkdirAll(proj, 0o755); err != nil {
		t.Fatal(err)
	}
	old := filepath.Join(proj, "old.md")
	fresh := filepath.Join(proj, "fresh.md")
	keep := filepath.Join(proj, "note.txt")
	for _, f := range []string{old, fresh, keep} {
		if err := os.WriteFile(f, []byte("x"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	now := time.Now()
	past := now.Add(-40 * 24 * time.Hour)
	if err := os.Chtimes(old, past, past); err != nil {
		t.Fatal(err)
	}
	if n, _ := pruneRecordFiles(root, 0, now); n != 0 {
		t.Fatalf("days=0 removed %d", n)
	}
	if n, _ := pruneRecordFiles(root, 30, now); n != 1 {
		t.Fatalf("removed %d, want 1", n)
	}
	if _, err := os.Stat(old); err == nil {
		t.Fatal("old record kept")
	}
	for _, f := range []string{fresh, keep} {
		if _, err := os.Stat(f); err != nil {
			t.Fatalf("%s removed: %v", f, err)
		}
	}
	// an emptied project folder is removed
	empty := filepath.Join(root, "empty")
	_ = os.MkdirAll(empty, 0o755)
	emptyOld := filepath.Join(empty, "a.md")
	_ = os.WriteFile(emptyOld, []byte("x"), 0o644)
	_ = os.Chtimes(emptyOld, past, past)
	if _, err := pruneRecordFiles(root, 30, now); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(empty); err == nil {
		t.Fatal("empty project folder kept")
	}
}

func TestExitHandoffWriteRead(t *testing.T) {
	path := filepath.Join(t.TempDir(), "proj", exitHandoffFile)
	if _, ok := readExitHandoff(path); ok {
		t.Fatal("read before write succeeded")
	}
	in := exitHandoff{Cwd: "/work/x", MainSession: "m1", ResidentSessions: map[string]string{"coder": "c1", "tui": "t1"}, OwnedSessions: []string{"m1", "c1", "t1"}}
	if err := writeExitHandoff(path, in); err != nil {
		t.Fatal(err)
	}
	out, ok := readExitHandoff(path)
	if !ok || out.Version != 1 || out.Cwd != "/work/x" || out.MainSession != "m1" || out.ResidentSessions["coder"] != "c1" || len(out.OwnedSessions) != 3 || out.SavedAt.IsZero() {
		t.Fatalf("read %+v ok=%v", out, ok)
	}
	if got := exitHandoffPath(t.TempDir(), "/work/x"); filepath.Base(got) != exitHandoffFile {
		t.Fatalf("path %s", got)
	}
}

func TestResidentSlotsFixedFour(t *testing.T) {
	var got []string
	for _, x := range residentSlots("opus") {
		got = append(got, x.Role)
	}
	want := []string{"main", "prompter", "tui", "coder"}
	if strings.Join(got, ",") != strings.Join(want, ",") {
		t.Fatalf("roles %v, want %v", got, want)
	}
}

func TestLegacyCoderSlotsKey(t *testing.T) {
	dir := t.TempDir()
	legacy := filepath.Join(dir, "old.json")
	if err := os.WriteFile(legacy, []byte(`{"coder_slots": 2}`), 0o600); err != nil {
		t.Fatal(err)
	}
	if s := loadChaudeSettings(legacy); s.ExtraCoders != 2 {
		t.Fatalf("legacy coder_slots gave ExtraCoders %d, want 2", s.ExtraCoders)
	}
	both := filepath.Join(dir, "both.json")
	if err := os.WriteFile(both, []byte(`{"coder_slots": 2, "extra_coders": 5}`), 0o600); err != nil {
		t.Fatal(err)
	}
	if s := loadChaudeSettings(both); s.ExtraCoders != 5 {
		t.Fatalf("extra_coders should win, got %d", s.ExtraCoders)
	}
}

func TestSettingsFileIsPerProject(t *testing.T) {
	projA, projB := t.TempDir(), t.TempDir()
	pathA, pathB := settingsFileFor(projA), settingsFileFor(projB)
	if want := filepath.Join(projA, ".chaude", "settings.json"); pathA != want {
		t.Fatalf("path %s, want %s", pathA, want)
	}
	inA := chaudeSettings{SessionRetentionDays: 7, ExtraCoders: 2, MainModel: "sonnet"}
	if err := saveChaudeSettings(pathA, inA); err != nil {
		t.Fatal(err)
	}
	if got := loadChaudeSettings(pathB); got != defaultChaudeSettings() {
		t.Fatalf("project B %+v, want defaults (independent of A)", got)
	}
	if got := loadChaudeSettings(pathA); got != inA {
		t.Fatalf("project A %+v, want %+v", got, inA)
	}
}

func TestSettingsFileEmptyCwdUsesWorkingDir(t *testing.T) {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	if got, want := settingsFileFor(""), filepath.Join(wd, ".chaude", "settings.json"); got != want {
		t.Fatalf("empty cwd %s, want %s", got, want)
	}
}
