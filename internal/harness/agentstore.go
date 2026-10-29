package harness

import (
	"chaude/internal/osenv"

	"encoding/json"
	"os"
	"path/filepath"
	"sort"
	"sync"
	"time"
)

// Agent states kept on disk.
const (
	stoRunning     = "running"
	stoDone        = "done"
	stoFailed      = "failed"
	stoInterrupted = "interrupted"
)

const (
	maxStoredAgents = 200
	storeMaxAge     = 7 * 24 * time.Hour
	restoreMaxAge   = 24 * time.Hour // startup restore window for interrupted agents
)

// savedAgent is the on-disk form of one tracked sub-agent.
type savedAgent struct {
	ID         string    `json:"id"`
	Model      string    `json:"model,omitempty"`
	Name       string    `json:"name,omitempty"`
	Task       string    `json:"task,omitempty"`
	Prompt     string    `json:"prompt,omitempty"`
	Status     string    `json:"status"`
	Summary    string    `json:"summary,omitempty"`
	Parent     string    `json:"parent_session,omitempty"`
	Transcript string    `json:"transcript,omitempty"` // parent Claude Code transcript (holds the sub-agent's events)
	OutPath    string    `json:"out_path,omitempty"`
	Background bool      `json:"background,omitempty"`
	Owner      int       `json:"owner_pid,omitempty"` // harness process that tracked it
	Started    time.Time `json:"started"`
	Updated    time.Time `json:"updated"`
}

// agentStore persists savedAgents as one JSON file (written atomically on every change).
// A nil *agentStore or an empty path is a no-op.
type agentStore struct {
	path string
	mu   sync.Mutex
	m    map[string]savedAgent
	last []byte
}

func newAgentStore(path string) *agentStore {
	s := &agentStore{path: path, m: map[string]savedAgent{}}
	if path == "" {
		return s
	}
	var l []savedAgent
	if b, err := os.ReadFile(path); err == nil && json.Unmarshal(b, &l) == nil {
		for _, a := range l {
			if a.ID != "" {
				s.m[a.ID] = a
			}
		}
	}
	return s
}

// put merges records (newest wins per id), prunes and writes the file if anything changed.
func (s *agentStore) put(recs []savedAgent) {
	if s == nil || s.path == "" || len(recs) == 0 {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	s.mergeDiskLocked()
	for _, a := range recs {
		a.Owner = os.Getpid()
		if old, ok := s.m[a.ID]; ok {
			a.Updated = old.Updated
			if old == a {
				continue
			}
			a.Updated = time.Now()
		} else if a.Updated.IsZero() {
			a.Updated = time.Now()
		}
		s.m[a.ID] = a
	}
	s.flushLocked()
}

// pidAlive reports whether a process exists (OS-specific: internal/osenv).
func pidAlive(pid int) bool { return osenv.PidAlive(pid) }

// interruptRunning marks stored running agents interrupted: those of this process (shutdown, own=true)
// or of dead processes (crash recovery, own=false). Agents of other live harnesses are left alone.
func (s *agentStore) interruptRunning(own bool) {
	if s == nil || s.path == "" {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	s.mergeDiskLocked()
	for id, a := range s.m {
		if a.Status == stoRunning && (own && a.Owner == os.Getpid() || !own && !pidAlive(a.Owner)) {
			a.Status, a.Updated = stoInterrupted, time.Now()
			s.m[id] = a
		}
	}
	s.flushLocked()
}

// mergeDiskLocked adds entries other harness processes wrote since we last looked (ours win by id,
// except entries another process updated later).
func (s *agentStore) mergeDiskLocked() {
	b, err := os.ReadFile(s.path)
	if err != nil {
		return
	}
	var l []savedAgent
	if json.Unmarshal(b, &l) != nil {
		return
	}
	for _, a := range l {
		if cur, ok := s.m[a.ID]; !ok || a.Updated.After(cur.Updated) && a.Owner != os.Getpid() {
			s.m[a.ID] = a
		}
	}
}

func (s *agentStore) flushLocked() {
	cut := time.Now().Add(-storeMaxAge)
	var l []savedAgent
	for _, a := range s.m {
		if a.Updated.Before(cut) {
			delete(s.m, a.ID)
			continue
		}
		l = append(l, a)
	}
	sort.Slice(l, func(i, j int) bool {
		if l[i].Updated.Equal(l[j].Updated) {
			return l[i].ID < l[j].ID
		}
		return l[i].Updated.After(l[j].Updated)
	})
	if len(l) > maxStoredAgents {
		for _, a := range l[maxStoredAgents:] {
			delete(s.m, a.ID)
		}
		l = l[:maxStoredAgents]
	}
	b, err := json.MarshalIndent(l, "", " ")
	if err != nil || string(b) == string(s.last) {
		return
	}
	if os.MkdirAll(filepath.Dir(s.path), 0o755) != nil {
		return
	}
	tmp := s.path + ".tmp"
	if os.WriteFile(tmp, b, 0o600) == nil && os.Rename(tmp, s.path) == nil {
		s.last = b
	}
}

// load returns stored agents matching parent (any when ""), oldest first. Without a parent only
// interrupted agents newer than restoreMaxAge are returned.
func (s *agentStore) load(parent string) []savedAgent {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	var l []savedAgent
	for _, a := range s.m {
		if parent != "" {
			if a.Parent != parent {
				continue
			}
		} else if a.Status != stoInterrupted || time.Since(a.Updated) > restoreMaxAge {
			continue
		}
		l = append(l, a)
	}
	sort.Slice(l, func(i, j int) bool {
		if l[i].Started.Equal(l[j].Started) {
			return l[i].ID < l[j].ID
		}
		return l[i].Started.Before(l[j].Started)
	})
	return l
}

func (a *agentRec) saved(parent, transcript string) savedAgent {
	st := stoRunning
	switch {
	case a.Interrupted:
		st = stoInterrupted
	case a.Failed:
		st = stoFailed
	case a.Done:
		st = stoDone
	}
	return savedAgent{ID: a.ID, Model: a.Model, Name: a.Name, Task: a.Task, Prompt: a.Prompt, Status: st, Summary: a.Summary,
		Parent: parent, Transcript: transcript, OutPath: a.OutPath, Background: a.Background, Started: a.Started}
}

// lastSummary: the agent's last plain text line (not a tool call / result line).
func lastSummary(log []string) string {
	for i := len(log) - 1; i >= 0; i-- {
		l := log[i]
		if l == "" || l[0] == ' ' || len(l) >= 3 && l[:3] == "●" {
			continue
		}
		return trunc(l, 200)
	}
	return ""
}

// restoreAgents seeds the current tracker with stored agents (parent "" = recent interrupted ones).
// Unfinished ones are shown as interrupted: their child process is gone.
func (r *Repl) restoreAgents(parent string) {
	if r.tracker == nil {
		return
	}
	for _, s := range r.agents.load(parent) {
		r.tracker.restore(s)
	}
}

// markInterrupted records every unfinished agent (current and draining sessions) as interrupted.
func (r *Repl) markInterrupted() {
	if r.tracker != nil {
		r.tracker.interruptRunning()
	}
	for _, d := range r.draining {
		d.tr.interruptRunning()
	}
	r.agents.interruptRunning(true)
}

func defaultAgentsFile() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}
	return filepath.Join(home, ".chaude", "agents.json")
}
