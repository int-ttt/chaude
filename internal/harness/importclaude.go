package harness

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// claudeImport is the result of reading a plain Claude Code transcript.
type claudeImport struct {
	Path  string
	Turns [][2]string // user text, assistant text (final text blocks only)
	Files []string
	Reco  string
}

// resolveClaudeTranscript maps "latest" / a session id / a path to a .jsonl file.
func resolveClaudeTranscript(arg, cwd string) (string, error) {
	if strings.HasSuffix(arg, ".jsonl") || strings.Contains(arg, string(filepath.Separator)) {
		return arg, nil
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	if cwd == "" {
		cwd, _ = os.Getwd()
	}
	proj := strings.NewReplacer("/", "-", ".", "-", "_", "-").Replace(cwd)
	dir := filepath.Join(home, ".claude", "projects", proj)
	if arg != "latest" {
		return filepath.Join(dir, arg+".jsonl"), nil
	}
	ents, err := filepath.Glob(filepath.Join(dir, "*.jsonl"))
	if err != nil || len(ents) == 0 {
		return "", fmt.Errorf("no transcripts in %s", dir)
	}
	var best string
	var bt time.Time
	for _, e := range ents {
		if st, err := os.Stat(e); err == nil && st.ModTime().After(bt) {
			best, bt = e, st.ModTime()
		}
	}
	return best, nil
}

// readClaudeTranscript streams the file (lines can be >100KB) and extracts turns, edited files and the last 추천.
func readClaudeTranscript(path string) (*claudeImport, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	imp := &claudeImport{Path: path}
	br := bufio.NewReaderSize(f, 1<<20)
	var curUser string
	var asst []string
	flush := func() {
		if curUser != "" {
			imp.Turns = append(imp.Turns, [2]string{curUser, strings.Join(asst, "\n")})
		}
		curUser, asst = "", nil
	}
	for {
		line, rerr := br.ReadString('\n')
		if strings.TrimSpace(line) != "" {
			ev := parseEvent(line)
			t := str(ev["type"])
			side, _ := ev["isSidechain"].(bool)
			meta, _ := ev["isMeta"].(bool)
			m := asMap(ev["message"])
			if !side && !meta && (t == "user" || t == "assistant") {
				if t == "user" {
					if s, ok := m["content"].(string); ok && strings.TrimSpace(s) != "" && !strings.HasPrefix(s, "<") {
						flush()
						curUser = s
					} else {
						for _, bi := range asList(m["content"]) {
							if b := asMap(bi); str(b["type"]) == "text" && curUser == "" && !strings.HasPrefix(str(b["text"]), "<") {
								curUser = str(b["text"])
							}
						}
					}
				} else {
					for _, bi := range asList(m["content"]) {
						b := asMap(bi)
						switch str(b["type"]) {
						case "text":
							if tx := strings.TrimSpace(str(b["text"])); tx != "" {
								asst = append(asst, tx)
								if rc := extractReco(tx); rc != "" {
									imp.Reco = rc
								}
							}
						case "tool_use":
							switch str(b["name"]) {
							case "Write", "Edit", "NotebookEdit":
								if fp := str(asMap(b["input"])["file_path"]); fp != "" && !contains(imp.Files, fp) {
									imp.Files = append(imp.Files, fp)
								}
							}
						}
					}
				}
			}
		}
		if rerr != nil {
			if rerr != io.EOF {
				return nil, rerr
			}
			break
		}
	}
	flush()
	if len(imp.Turns) == 0 {
		return nil, fmt.Errorf("no user/assistant turns in %s", path)
	}
	return imp, nil
}

// importFromClaude loads the transcript into r and returns (exit code, done).
// done=false means continue into the normal REPL with pendingSnapshot set.
func (r *Repl) importFromClaude() (int, bool) {
	path, err := resolveClaudeTranscript(r.a.FromClaude, r.a.Cwd)
	var imp *claudeImport
	if err == nil {
		imp, err = readClaudeTranscript(path)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "chaude: import failed:", err)
		return 1, true
	}
	snap, preserved := r.applyImport(imp, path)
	if r.a.ImportDry {
		fmt.Printf("import dry-run: %s\nturns=%d files=%d snapshot_chars=%d snapshot_bytes=%d\nreco_found=%v reco_preserved=%v reco_bytes=%d\n",
			path, len(imp.Turns), len(imp.Files), len([]rune(snap)), len(snap), imp.Reco != "", preserved, len(imp.Reco))
		if !preserved {
			return 1, true
		}
		return 0, true
	}
	return 0, false
}

// applyImport installs imp into r (turns, files, pending snapshot) and logs it.
func (r *Repl) applyImport(imp *claudeImport, path string) (string, bool) {
	r.tr.turns, r.files, r.lastReco = imp.Turns, imp.Files, imp.Reco
	note := "(imported from a plain Claude Code session " + strings.TrimSuffix(filepath.Base(path), ".jsonl") +
		"; no handoff note was written by it. Goal, Decisions, In-progress work, Open questions, User preferences " +
		"and exact values are in the last turns below; the files it changed are listed. " +
		"Re-read notes/ (FACTS.md, DECISIONS.md) for durable state.)"
	snap := withReco(imp.Reco, r.buildSnapshot(note))
	r.pendingSnapshot = snap
	preserved := recoPreserved(imp.Reco, snap)
	_ = r.metrics.Log(map[string]any{"stage": "import", "source": path, "turns": len(imp.Turns), "files": len(imp.Files),
		"snapshot_chars": len([]rune(snap)), "reco_found": imp.Reco != "", "reco_preserved": preserved, "dry_run": r.a.ImportDry})
	return snap, preserved
}
