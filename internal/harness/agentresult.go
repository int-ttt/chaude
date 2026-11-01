package harness

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// agentResultRel is the durable result file of one agent, relative to the project dir.
func agentResultRel(id string, started time.Time) string {
	return "notes/runs/" + started.Format("060102") + "-agent-" + id + ".md"
}

// appendAgentResult appends one result block for a finished background agent to a.ResultPath under cwd.
// text is the completion notice (fallback: the agent's last summary line).
func appendAgentResult(cwd string, a *agentRec, text string, now time.Time) error {
	if cwd == "" || a.ResultPath == "" {
		return nil
	}
	body := strings.TrimSpace(text)
	if body == "" {
		body = a.Summary
	}
	if body == "" {
		body = lastSummary(a.Log)
	}
	p := filepath.Join(cwd, a.ResultPath)
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		return err
	}
	f, err := os.OpenFile(p, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return err
	}
	defer f.Close()
	var b strings.Builder
	fmt.Fprintf(&b, "\n## %s · agent %s\n", now.Format("2006-01-02 15:04:05"), a.ID)
	fmt.Fprintf(&b, "- task: %s\n- name: %s · model: %s\n", a.Task, a.Name, a.Model)
	if a.OutPath != "" {
		fmt.Fprintf(&b, "- agent output file: %s\n", a.OutPath)
	}
	if body != "" {
		fmt.Fprintf(&b, "\n%s\n", trunc(body, 4000))
	}
	_, err = f.WriteString(b.String())
	return err
}

// writeResults appends the result blocks of finished background agents (called without t.mu held),
// then marks each one written so a repeated notice does not duplicate it.
func (t *agentTracker) writeResults(list []*agentRec, text string) {
	if t.cwd == "" {
		return
	}
	for _, a := range list {
		if err := appendAgentResult(t.cwd, a, text, time.Now()); err != nil {
			continue
		}
		t.mu.Lock()
		a.ResultDone = true
		t.mu.Unlock()
	}
}
