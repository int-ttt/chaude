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
	Start       time.Time // first event time (zero: Mod is used)
	Cwd, Name   string
	Cleared     bool // a user /clear is in the file: the next file starts a new group
	ClearHead   bool // the /clear is before the first turn: this file starts a new group
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
			Turns: len(imp.Turns), First: oneLine(imp.Turns[0][0]), Last: oneLine(imp.Turns[len(imp.Turns)-1][0]),
			Start: imp.Start, Cleared: imp.Cleared, ClearHead: imp.ClearHead, Cwd: imp.Cwd, Name: imp.Name})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Mod.After(out[j].Mod) })
	return out
}

func oneLine(s string) string { return strings.Join(strings.Fields(s), " ") }

func (s sessionInfo) label() string {
	return fmt.Sprintf("%s  %3d turns  %s … %s", s.Mod.Format("2006-01-02 15:04"), s.Turns, trunc(s.First, 40), trunc(s.Last, 40))
}

func (r *Repl) sessionList() ([]sessionInfo, string) {
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

// resumeTarget decides what "/resume args..." means. record=true: answer from the record (no arg, or a
// topic number/slug that exists; terms unused). Otherwise terms is the Claude Code session query
// ("/resume claude <q>" or "/resume <q>"; no terms = all groups).
func resumeTarget(cwd string, args []string) (terms []string, record bool) {
	if len(args) == 0 {
		return nil, true
	}
	if strings.EqualFold(args[0], "claude") {
		return args[1:], false
	}
	if _, _, err := recordResume(recordDir(cwd), strings.Join(args, " ")); err == nil {
		return nil, true
	}
	return args, false
}

// importSession replaces the running session with the transcript (arg: see loadImportArg).
func (r *Repl) importSession(arg string) {
	path, imp, err := loadImportArg(arg, r.a.Cwd)
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

// resumeLine handles "/resume" in the plain loop: record topics, or session groups (one match loads,
// many = numbered list, read a number, import).
func (r *Repl) resumeLine(args ...string) {
	terms, record := resumeTarget(r.a.Cwd, args)
	if record {
		r.recordResumePlain(strings.Join(args, " "))
		return
	}
	gs, msg := r.groupList(terms...)
	if msg != "" {
		r.p(msg, "\n")
		return
	}
	if len(gs) == 1 {
		r.importSession(gs[0].key())
		return
	}
	if len(gs) > 20 {
		gs = gs[:20]
	}
	for i, g := range gs {
		r.p(fmt.Sprintf("%2d) %s", i+1, g.label()), "\n")
	}
	r.p("number (empty=cancel): ", "")
	in, _ := r.in.ReadString('\n')
	var n int
	if _, err := fmt.Sscanf(strings.TrimSpace(in), "%d", &n); err != nil || n < 1 || n > len(gs) {
		r.p("cancelled", "\n")
		return
	}
	r.importSession(gs[n-1].key())
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
