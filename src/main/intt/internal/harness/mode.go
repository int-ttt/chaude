package harness

import "strings"

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
