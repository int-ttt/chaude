package harness

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// exitHandoffFile sits in the project record folder; listRecordFiles ignores it (not .md).
const exitHandoffFile = ".exit-handoff.json"

// exitHandoff is what the next chaude start in the same cwd reads: the session ids of this run, so
// /view and /resume can find them again.
type exitHandoff struct {
	Version          int               `json:"version"`
	Cwd              string            `json:"cwd"`
	MainSession      string            `json:"main_session,omitempty"`
	ResidentSessions map[string]string `json:"resident_sessions,omitempty"` // role -> session id
	OwnedSessions    []string          `json:"owned_sessions,omitempty"`
	SavedAt          time.Time         `json:"saved_at"`
}

// exitHandoffPath is <record root>/<project folder>/.exit-handoff.json for cwd.
func exitHandoffPath(root, cwd string) string {
	return filepath.Join(recordProjectDir(root, cwd), exitHandoffFile)
}

// writeExitHandoff stores the handoff for this run's sessions. Called from both exit paths before the
// sessions close.
func writeExitHandoff(path string, h exitHandoff) error {
	h.Version = 1
	h.SavedAt = time.Now()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	b, err := json.MarshalIndent(h, "", "  ")
	if err != nil {
		return err
	}
	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, append(b, '\n'), 0o600); err != nil {
		return err
	}
	return os.Rename(tmp, path)
}

// readExitHandoff returns the saved handoff, ok=false when there is none or it does not parse.
func readExitHandoff(path string) (exitHandoff, bool) {
	var h exitHandoff
	b, err := os.ReadFile(path)
	if err != nil {
		return h, false
	}
	if json.Unmarshal(b, &h) != nil {
		return h, false
	}
	return h, true
}

// handoffSnapshot collects the session ids this process owns: main, resident slots, and every owned child.
func (r *Repl) handoffSnapshot() exitHandoff {
	h := exitHandoff{Cwd: r.a.Cwd, ResidentSessions: map[string]string{}}
	if r.sess != nil {
		h.MainSession = r.sess.ID()
	}
	if r.children != nil {
		r.children.mu.Lock()
		for _, c := range r.children.m {
			if c == nil || c.Role == "" {
				continue
			}
			if s, ok := c.Sess.(interface{ ID() string }); ok && s.ID() != "" {
				h.ResidentSessions[c.Role] = s.ID()
			}
		}
		r.children.mu.Unlock()
	}
	r.ownMu.Lock()
	seen := map[string]bool{}
	for _, s := range r.owned {
		if id := s.ID(); id != "" && !seen[id] {
			seen[id] = true
			h.OwnedSessions = append(h.OwnedSessions, id)
		}
	}
	r.ownMu.Unlock()
	return h
}

// saveExitHandoff writes the handoff for this run; no-op without a cwd.
func (r *Repl) saveExitHandoff() {
	if r.a.Cwd == "" {
		return
	}
	_ = writeExitHandoff(exitHandoffPath(recordDir(r.a.Cwd), r.a.Cwd), r.handoffSnapshot())
}

// loadPrevOwned reads the previous run's owned session ids for this cwd.
func loadPrevOwned(root, cwd string) map[string]bool {
	ids := map[string]bool{}
	if h, ok := readExitHandoff(exitHandoffPath(root, cwd)); ok {
		for _, id := range h.OwnedSessions {
			ids[id] = true
		}
		if h.MainSession != "" {
			ids[h.MainSession] = true
		}
	}
	return ids
}

// pruneRecordFiles deletes auto-saved .md records in project folders under root whose mtime is older
// than days. days <= 0 keeps everything. Empty project folders are removed too.
func pruneRecordFiles(root string, days int, now time.Time) (int, error) {
	if days <= 0 {
		return 0, nil
	}
	cutoff := now.Add(-time.Duration(days) * 24 * time.Hour)
	dirs, err := os.ReadDir(root)
	if err != nil {
		return 0, nil
	}
	removed := 0
	for _, d := range dirs {
		if !d.IsDir() || strings.HasPrefix(d.Name(), ".") {
			continue
		}
		dir := filepath.Join(root, d.Name())
		files, err := os.ReadDir(dir)
		if err != nil {
			continue
		}
		for _, f := range files {
			if f.IsDir() || !strings.HasSuffix(f.Name(), ".md") {
				continue
			}
			info, err := f.Info()
			if err == nil && info.ModTime().Before(cutoff) && os.Remove(filepath.Join(dir, f.Name())) == nil {
				removed++
			}
		}
		if left, _ := os.ReadDir(dir); len(left) == 0 {
			_ = os.Remove(dir)
		}
	}
	return removed, nil
}
