package install

import (
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

func TestMergePolicy(t *testing.T) {
	pol := "# Policy\n- rule"
	// fresh file
	got := string(MergePolicy(nil, pol))
	if got != markStart+"\n"+pol+"\n"+markEnd+"\n" {
		t.Fatalf("fresh: %q", got)
	}
	// prepend above existing content, BOM and CRLF kept
	old := bom + "# Proj\r\n\r\nbody\r\n"
	got = string(MergePolicy([]byte(old), pol))
	if !strings.HasPrefix(got, bom+markStart+"\r\n# Policy\r\n- rule\r\n"+markEnd+"\r\n\r\n# Proj\r\n") {
		t.Fatalf("prepend: %q", got)
	}
	if strings.Contains(strings.TrimPrefix(got, bom), "\n\n\n") || strings.Count(got, "\r\n") != strings.Count(got, "\n") {
		t.Fatalf("line endings: %q", got)
	}
	// re-run replaces the block only, once
	got2 := string(MergePolicy([]byte(got), "# Policy v2"))
	if strings.Count(got2, markStart) != 1 || !strings.Contains(got2, "# Policy v2\r\n"+markEnd+"\r\n\r\n# Proj") || strings.Contains(got2, "- rule") {
		t.Fatalf("replace: %q", got2)
	}
	if !strings.HasPrefix(got2, bom) {
		t.Fatal("BOM lost")
	}
}

const pkgSettings = `{
  "model": "opus",
  "effortLevel": "low",
  "env": {"A": "1", "B": "2"},
  "permissions": {"allow": ["Bash(bin/chaude search-history:*)"]},
  "hooks": {
    "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/style-adapter.py\"", "timeout": 5}]}],
    "SessionStart": [{"matcher": "startup", "hooks": [{"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/session-context.py\"", "timeout": 5}]}]
  }
}`

func TestMergeSettings(t *testing.T) {
	hooks := map[string]bool{"style-adapter.py": true, "session-context.py": true}
	existing := `{
  "model": "sonnet", "effortLevel": "medium",
  "env": {"A": "9", "C": "3"},
  "permissions": {"allow": ["Bash(bin/chaude search-history:*)", "Bash(git status:*)"]},
  "hooks": {
    "UserPromptSubmit": [
      {"hooks": [{"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/style-adapter.py\"", "timeout": 5}]},
      {"hooks": [{"type": "command", "command": "echo mine"}]}
    ],
    "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "py", "args": ["-3", "${CLAUDE_PROJECT_DIR}/.claude/hooks/session-context.py"]}]}]
  },
  "other": true
}`
	out, err := MergeSettings([]byte(existing), []byte(pkgSettings), "", hooks)
	if err != nil {
		t.Fatal(err)
	}
	var m map[string]any
	if err := json.Unmarshal(out, &m); err != nil {
		t.Fatalf("invalid json: %v\n%s", err, out)
	}
	if m["model"] != "sonnet" || m["effortLevel"] != "medium" || m["other"] != true {
		t.Errorf("user scalars not kept: %s", out)
	}
	env := m["env"].(map[string]any)
	if env["A"] != "9" || env["B"] != "2" || env["C"] != "3" {
		t.Errorf("env merge: %v", env)
	}
	allow := m["permissions"].(map[string]any)["allow"].([]any)
	if len(allow) != 2 {
		t.Errorf("allow union: %v", allow)
	}
	hk := m["hooks"].(map[string]any)
	ups := hk["UserPromptSubmit"].([]any)
	if len(ups) != 2 || !strings.Contains(string(out), "echo mine") {
		t.Errorf("UserPromptSubmit: user hook lost or package hook duplicated: %v", ups)
	}
	if pre, ok := hk["PreToolUse"]; ok { // windows exec-form package hook recognised and removed (package has none for PreToolUse): event dropped
		t.Errorf("exec-form package hook not recognised: %v", pre)
	}
	if ss := hk["SessionStart"].([]any); len(ss) != 1 {
		t.Errorf("SessionStart: %v", ss)
	}
	if !strings.HasPrefix(string(out), "{\n  \"model\": \"sonnet\",\n  \"effortLevel\": \"medium\"") {
		t.Errorf("key order: %s", out)
	}
	// fresh install with --main
	out, err = MergeSettings(nil, []byte(pkgSettings), "fable", hooks)
	if err != nil || !strings.Contains(string(out), `"model": "fable"`) {
		t.Errorf("main override: %v %s", err, out)
	}
	if _, err := MergeSettings([]byte("{bad"), []byte(pkgSettings), "", hooks); err == nil {
		t.Error("invalid existing settings must be an error, not overwritten")
	}
}

func TestRunWindowsFreshAndUpgrade(t *testing.T) {
	dir := t.TempDir()
	exe := filepath.Join(t.TempDir(), "fake.exe")
	if err := os.WriteFile(exe, []byte("binary"), 0o755); err != nil {
		t.Fatal(err)
	}
	md := bom + "# TTTT\r\n\r\nUnity project.\r\n"
	if err := os.WriteFile(filepath.Join(dir, "CLAUDE.md"), []byte(md), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, ".gitignore"), []byte("Library/\r\nTemp/\r\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	o := Options{Dir: dir, Version: "9.9.9", Exe: exe, Windows: true, Python: "py -3", Main: "opus"}
	if err := Run(o); err != nil {
		t.Fatal(err)
	}
	for _, rel := range []string{".claude/agents/haiku-scout.md", ".claude/agents/fable-architect.md", ".claude/hooks/_lean.py",
		".claude/hooks/style-adapter.py", ".claude/skills/check/SKILL.md", ".claude/settings.json", ".claude/claude-lean.version",
		"notes/FACTS.md", "notes/DECISIONS.md", "notes/runs", "logs", "bin/chaude.exe", "chaude-start.bat"} {
		if _, err := os.Stat(filepath.Join(dir, filepath.FromSlash(rel))); err != nil {
			t.Errorf("missing %s", rel)
		}
	}
	if b, _ := os.ReadFile(filepath.Join(dir, "bin", "chaude.exe")); string(b) != "binary" {
		t.Error("binary not copied")
	}
	if b, _ := os.ReadFile(filepath.Join(dir, "chaude-start.bat")); !strings.HasPrefix(string(b), "@echo off\r\n") || !strings.Contains(string(b), `start "" wt -d "%PROJDIR%" cmd /k "bin\chaude.exe"`) {
		t.Errorf("launcher bat: %q", b)
	}
	set, _ := os.ReadFile(filepath.Join(dir, ".claude", "settings.json"))
	var m map[string]any
	if err := json.Unmarshal(set, &m); err != nil {
		t.Fatalf("settings invalid: %v", err)
	}
	if !strings.Contains(string(set), `"command": "py"`) || !strings.Contains(string(set), `"${CLAUDE_PROJECT_DIR}/.claude/hooks/style-adapter.py"`) ||
		strings.Contains(string(set), "python3") || !strings.Contains(string(set), `"PowerShell(bin/chaude.exe search-history:*)"`) {
		t.Errorf("windows form: %s", set)
	}
	if m["model"] != "opus" || m["env"].(map[string]any)["CLAUDE_CODE_SUBAGENT_MODEL"] != "haiku" {
		t.Errorf("model/env: %s", set)
	}
	got, _ := os.ReadFile(filepath.Join(dir, "CLAUDE.md"))
	s := string(got)
	if !strings.HasPrefix(s, bom+markStart+"\r\n") || !strings.HasSuffix(s, markEnd+"\r\n\r\n# TTTT\r\n\r\nUnity project.\r\n") || strings.Contains(s, "\n\n\n") {
		t.Errorf("CLAUDE.md merge: %q", s[:80])
	}
	if strings.Count(s, "\n") != strings.Count(s, "\r\n") {
		t.Error("CLAUDE.md: mixed line endings")
	}
	if gi, _ := os.ReadFile(filepath.Join(dir, ".gitignore")); !strings.Contains(string(gi), "Library/\r\n") || !strings.Contains(string(gi), "\r\nlogs/\r\n") {
		t.Errorf("gitignore: %q", gi)
	}
	if v, _ := os.ReadFile(filepath.Join(dir, ".claude", "claude-lean.version")); strings.TrimSpace(string(v)) != "9.9.9" {
		t.Error("version marker")
	}
	backups, _ := filepath.Glob(filepath.Join(dir, ".claude-backup-*", "CLAUDE.md"))
	if len(backups) != 1 {
		t.Errorf("backup of CLAUDE.md: %v", backups)
	}

	// upgrade: user tuning and user hook survive, nothing duplicated, policy replaced once
	m["effortLevel"] = "medium"
	m["hooks"].(map[string]any)["Stop"] = []any{map[string]any{"hooks": []any{map[string]any{"type": "command", "command": "echo mine"}}}}
	set2, _ := json.Marshal(m)
	if err := os.WriteFile(filepath.Join(dir, ".claude", "settings.json"), set2, 0o644); err != nil {
		t.Fatal(err)
	}
	o.Main = ""
	if err := Run(o); err != nil {
		t.Fatal(err)
	}
	set, _ = os.ReadFile(filepath.Join(dir, ".claude", "settings.json"))
	var m2 map[string]any
	if err := json.Unmarshal(set, &m2); err != nil {
		t.Fatalf("settings invalid after upgrade: %v", err)
	}
	if m2["effortLevel"] != "medium" || m2["model"] != "opus" || !strings.Contains(string(set), "echo mine") {
		t.Errorf("upgrade lost user settings: %s", set)
	}
	if n := strings.Count(string(set), "style-adapter.py"); n != 1 {
		t.Errorf("package hook duplicated %d times after upgrade", n)
	}
	got, _ = os.ReadFile(filepath.Join(dir, "CLAUDE.md"))
	if strings.Count(string(got), markStart) != 1 || !strings.HasSuffix(string(got), "# TTTT\r\n\r\nUnity project.\r\n") {
		t.Error("policy block duplicated or content lost on upgrade")
	}
	if gi, _ := os.ReadFile(filepath.Join(dir, ".gitignore")); strings.Count(string(gi), "logs/") != 1 {
		t.Error("gitignore lines duplicated")
	}
}

func TestRunPosix(t *testing.T) {
	dir := t.TempDir()
	if err := Run(Options{Dir: dir, Version: "1", Exe: "-", Windows: false}); err != nil {
		t.Fatal(err)
	}
	set, _ := os.ReadFile(filepath.Join(dir, ".claude", "settings.json"))
	if !strings.Contains(string(set), `\"$CLAUDE_PROJECT_DIR/.claude/hooks/pyrun\" \"$CLAUDE_PROJECT_DIR/.claude/hooks/style-adapter.py\"`) || strings.Contains(string(set), `"args"`) {
		t.Errorf("posix form: %s", set)
	}
	if _, err := os.Stat(filepath.Join(dir, "bin")); err == nil {
		t.Error("Exe \"-\" must skip the binary")
	}
	if st, err := os.Stat(filepath.Join(dir, ".claude", "hooks", "pyrun")); err != nil || (runtime.GOOS != "windows" && st.Mode()&0o111 == 0) {
		t.Errorf("pyrun missing or not executable: %v", err)
	}
	if st, err := os.Stat(filepath.Join(dir, "chaude-start.command")); err != nil || (runtime.GOOS != "windows" && st.Mode()&0o111 == 0) {
		t.Errorf("launcher .command missing or not executable: %v", err)
	}
	if _, err := os.Stat(filepath.Join(dir, "chaude-start.bat")); err == nil {
		t.Error("posix install must not write the .bat launcher")
	}
	if _, err := os.Stat(filepath.Join(dir, ".gitignore")); err == nil {
		t.Error(".gitignore must not be created")
	}
	if err := Run(Options{Dir: filepath.Join(dir, "nope"), Exe: "-"}); err == nil {
		t.Error("missing folder must fail")
	}
}
