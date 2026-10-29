package harness

import (
	"os"
	"path/filepath"
	"strings"
)

// permMode is a harness-side permission mode applied to the child's permission prompts
// (no child restart needed). default asks; acceptEdits auto-allows file edits; plan allows
// read-only tools and denies everything that changes state; auto allows everything.
type permMode int

const (
	modeDefault permMode = iota
	modeAcceptEdits
	modePlan
	modeAuto
	numModes
)

var modeNames = [numModes]string{"default", "acceptEdits", "plan", "auto"}

func (m permMode) String() string { return modeNames[m] }

func (m permMode) Next() permMode { return (m + 1) % numModes }

func parseMode(s string) (permMode, bool) {
	s = strings.ToLower(strings.TrimSpace(s))
	if s == "accept" || s == "acceptedits" {
		return modeAcceptEdits, true
	}
	for i, n := range modeNames {
		if strings.ToLower(n) == s {
			return permMode(i), true
		}
	}
	return modeDefault, false
}

var editTools = map[string]bool{"Edit": true, "Write": true, "MultiEdit": true, "NotebookEdit": true}
var readTools = map[string]bool{"Read": true, "Grep": true, "Glob": true, "LS": true, "WebFetch": true,
	"WebSearch": true, "TodoWrite": true, "Agent": true, "Task": true, "NotebookRead": true}

// decideMode returns verdict "allow", "deny" or "ask" for a tool under the mode.
func decideMode(m permMode, tool string) (verdict, msg string) {
	switch m {
	case modeAuto:
		return "allow", ""
	case modeAcceptEdits:
		if editTools[tool] {
			return "allow", ""
		}
	case modePlan:
		if readTools[tool] {
			return "allow", ""
		}
		return "deny", "plan mode: do not modify anything or run commands; research read-only and present a plan for the user to approve"
	}
	return "ask", ""
}

// modeFile is where the permission mode of a project is remembered (per folder, outside git):
// Shift+Tab / "/mode" changes are restored on the next start.
func modeFile(cwd string) string {
	if cwd == "" {
		cwd, _ = os.Getwd()
	}
	return filepath.Join(cwd, ".claude", "chaude-mode")
}

// loadMode returns the mode saved for cwd (default when none or unreadable).
func loadMode(cwd string) permMode {
	b, err := os.ReadFile(modeFile(cwd))
	if err != nil {
		return modeDefault
	}
	m, ok := parseMode(string(b))
	if !ok {
		return modeDefault
	}
	return m
}

// saveMode remembers m for cwd (best effort; the .claude folder is created when missing).
func saveMode(cwd string, m permMode) {
	p := modeFile(cwd)
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		return
	}
	_ = os.WriteFile(p, []byte(m.String()+"\n"), 0o644)
}
