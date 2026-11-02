package harness

import (
	"context"
	"fmt"
	"strings"

	"chaude/internal/update"
)

// localCmd answers the plain REPL's local commands (/help, /status) without sending anything to the child.
// ok=false means the line is not a local command.
func localCmd(r *Repl, line string) (string, bool) {
	f := strings.Fields(line)
	if len(f) == 0 {
		return "", false
	}
	switch f[0] {
	case "/help":
		return localHelp(), true
	case "/status":
		return r.localStatus(), true
	case "/update":
		return update.Text(context.Background(), ChaudeVersion), true
	}
	return "", false
}

// localHelp lists the TUI slash commands (same table as /help in the TUI).
func localHelp() string {
	var l []string
	for _, c := range slashCmds {
		l = append(l, fmt.Sprintf("%-8s %s", c.name, c.desc))
	}
	return strings.Join(l, "\n")
}

// localStatus mirrors the TUI /status line.
func (r *Repl) localStatus() string {
	sid := "-"
	if r.sess != nil && r.sess.ID() != "" {
		sid = trunc(r.sess.ID(), 8)
	}
	return fmt.Sprintf("model %s · ctx %s/%s · rotations %d · session %s", r.a.Model, commas(r.lastCtx), commas(r.effectiveThreshold()), r.rotations, sid)
}
