package harness

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

// sessionInfo summarizes one Claude Code transcript for the /resume list.
type sessionInfo struct {
	ID, Path    string
	Mod         time.Time
	Turns       int
	First, Last string
}

func claudeProjectDir(cwd string) (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	if cwd == "" {
		cwd, _ = os.Getwd()
	}
	proj := strings.NewReplacer("/", "-", ".", "-", "_", "-").Replace(cwd)
	return filepath.Join(home, ".claude", "projects", proj), nil
}

// listClaudeSessions returns the sessions in dir, newest first; unreadable/empty ones are skipped.
func listClaudeSessions(dir string) []sessionInfo {
	files, _ := filepath.Glob(filepath.Join(dir, "*.jsonl"))
	var out []sessionInfo
	for _, f := range files {
		st, err := os.Stat(f)
		if err != nil {
			continue
		}
		imp, err := readClaudeTranscript(f)
		if err != nil || len(imp.Turns) == 0 {
			continue
		}
		out = append(out, sessionInfo{ID: strings.TrimSuffix(filepath.Base(f), ".jsonl"), Path: f, Mod: st.ModTime(),
			Turns: len(imp.Turns), First: oneLine(imp.Turns[0][0]), Last: oneLine(imp.Turns[len(imp.Turns)-1][0])})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Mod.After(out[j].Mod) })
	return out
}

func oneLine(s string) string { return strings.Join(strings.Fields(s), " ") }

func (s sessionInfo) label() string {
	return fmt.Sprintf("%s  %3d turns  %s … %s", s.Mod.Format("2006-01-02 15:04"), s.Turns, trunc(s.First, 40), trunc(s.Last, 40))
}

// resumeSourceMsg validates the /resume argument: none or "claude" = Claude Code conversations (the only source).
func resumeSourceMsg(args []string) string {
	if len(args) == 0 || (len(args) == 1 && strings.EqualFold(args[0], "claude")) {
		return ""
	}
	return "usage: /resume [claude]  (lists Claude Code conversations only)"
}

func (r *Repl) sessionList(args ...string) ([]sessionInfo, string) {
	if m := resumeSourceMsg(args); m != "" {
		return nil, m
	}
	dir, err := claudeProjectDir(r.a.Cwd)
	if err != nil {
		return nil, err.Error()
	}
	l := listClaudeSessions(dir)
	if len(l) == 0 {
		return nil, "no Claude Code sessions in " + dir
	}
	return l, ""
}

// importSession replaces the running session with the transcript (arg: id, number-less "latest", or path).
func (r *Repl) importSession(arg string) {
	path, err := resolveClaudeTranscript(arg, r.a.Cwd)
	var imp *claudeImport
	if err == nil {
		imp, err = readClaudeTranscript(path)
	}
	if err != nil {
		r.p(dim+"import failed: "+err.Error()+rst, "\n")
		return
	}
	r.clear()
	snap, preserved := r.applyImport(imp, path)
	if r.ui != nil {
		r.ui.send(importedMsg{imp.Turns}) // TUI: show the imported conversation, not only the status note
	}
	r.p(fmt.Sprintf("import: %s turns=%d files=%d snapshot_chars=%d reco_preserved=%v (sent with your next message)",
		filepath.Base(path), len(imp.Turns), len(imp.Files), len([]rune(snap)), preserved), "\n")
}

// resumeLine handles "/resume" in the plain loop: list, read a number, import.
func (r *Repl) resumeLine(args ...string) {
	l, msg := r.sessionList(args...)
	if msg != "" {
		r.p(msg, "\n")
		return
	}
	if len(l) > 20 {
		l = l[:20]
	}
	for i, s := range l {
		r.p(fmt.Sprintf("%2d) %s", i+1, s.label()), "\n")
	}
	r.p("number (empty=cancel): ", "")
	in, _ := r.in.ReadString('\n')
	var n int
	if _, err := fmt.Sscanf(strings.TrimSpace(in), "%d", &n); err != nil || n < 1 || n > len(l) {
		r.p("cancelled", "\n")
		return
	}
	r.importSession(l[n-1].ID)
}

// shortID is the first 8 chars of a session id (whole id if shorter).
func shortID(id string) string {
	if len(id) > 8 {
		return id[:8]
	}
	return id
}

// formatSessionList renders one line per session plus a hint for /view.
func formatSessionList(l []sessionInfo) string {
	var b strings.Builder
	for _, s := range l {
		fmt.Fprintf(&b, "%s  %s  %3d turns  %s\n", shortID(s.ID), s.Mod.Format("01-02 15:04"), s.Turns, s.First)
	}
	b.WriteString("view one: /view <id-prefix>")
	return b.String()
}

// matchSessions returns the sessions whose id starts with prefix.
func matchSessions(l []sessionInfo, prefix string) []sessionInfo {
	var out []sessionInfo
	for _, s := range l {
		if strings.HasPrefix(s.ID, prefix) {
			out = append(out, s)
		}
	}
	return out
}

// formatTranscript renders every turn of one session in full (no truncation).
func formatTranscript(s sessionInfo, imp *claudeImport) string {
	var b strings.Builder
	fmt.Fprintf(&b, "session %s  %s  %d turns", s.ID, s.Mod.Format("2006-01-02 15:04"), len(imp.Turns))
	for i, t := range imp.Turns {
		fmt.Fprintf(&b, "\n[%d] user:\n%s\n[%d] assistant:\n%s", i+1, t[0], i+1, t[1])
	}
	return b.String()
}
