package harness

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sort"
	"sync"
	"time"
)

// Client states kept on disk.
const (
	clientLive    = "live"
	clientParked  = "parked"
	clientExpired = "expired"
)

// clientRec is one row of clients.json: a model client (child session) bound to a cwd.
// Rows are keyed by (Cwd, Model).
type clientRec struct {
	Cwd         string    `json:"cwd"`
	Model       string    `json:"model"`
	Role        string    `json:"role,omitempty"` // design|exec
	SessionID   string    `json:"session_id,omitempty"`
	State       string    `json:"state"` // live|parked|expired
	LastAPI     time.Time `json:"last_api"`
	LastCtx     int       `json:"last_ctx,omitempty"`
	EndedReason string    `json:"ended_reason,omitempty"`
	DesignPath  string    `json:"design_path,omitempty"`
	OwnerPID    int       `json:"owner_pid,omitempty"` // harness process that owns the row
	ChildPID    int       `json:"child_pid,omitempty"` // claude -p process; 0 when none
}

// clientRegistry persists clientRecs as one JSON file. Every write re-reads the file and replaces
// only the caller's (cwd, model) row, so rows written by other harness processes survive.
// A nil *clientRegistry or an empty path is a no-op.
type clientRegistry struct {
	path string
	mu   sync.Mutex
}

func newClientRegistry(path string) *clientRegistry {
	return &clientRegistry{path: path}
}

// defaultClientsFile is ~/.chaude/clients.json ("" when the home directory is unknown).
func defaultClientsFile() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}
	return filepath.Join(home, ".chaude", "clients.json")
}

// readLocked returns the rows on disk; a missing or broken file reads as empty.
func (r *clientRegistry) readLocked() []clientRec {
	var l []clientRec
	if b, err := os.ReadFile(r.path); err == nil {
		if json.Unmarshal(b, &l) != nil {
			return nil
		}
	}
	return l
}

// writeLocked writes rows sorted by (cwd, model) through a tmp file and rename.
func (r *clientRegistry) writeLocked(l []clientRec) {
	sort.Slice(l, func(i, j int) bool {
		if l[i].Cwd == l[j].Cwd {
			return l[i].Model < l[j].Model
		}
		return l[i].Cwd < l[j].Cwd
	})
	b, err := json.MarshalIndent(l, "", " ")
	if err != nil {
		return
	}
	if os.MkdirAll(filepath.Dir(r.path), 0o755) != nil {
		return
	}
	tmp := r.path + ".tmp"
	if os.WriteFile(tmp, b, 0o600) == nil {
		_ = os.Rename(tmp, r.path)
	}
}

// Get returns the row for (cwd, model) from disk.
func (r *clientRegistry) Get(cwd, model string) (clientRec, bool) {
	if r == nil || r.path == "" {
		return clientRec{}, false
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	for _, c := range r.readLocked() {
		if c.Cwd == cwd && c.Model == model {
			return c, true
		}
	}
	return clientRec{}, false
}

// Upsert replaces the (rec.Cwd, rec.Model) row on disk and keeps every other row.
func (r *clientRegistry) Upsert(rec clientRec) {
	if r == nil || r.path == "" {
		return
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	l := r.readLocked()
	for i := range l {
		if l[i].Cwd == rec.Cwd && l[i].Model == rec.Model {
			l = append(l[:i], l[i+1:]...)
			break
		}
	}
	r.writeLocked(append(l, rec))
}

// MarkEnded sets the (cwd, model) row to expired with reason. A missing row is created with LastAPI=now.
func (r *clientRegistry) MarkEnded(cwd, model, reason string, now time.Time) {
	if r == nil || r.path == "" {
		return
	}
	rec, ok := r.Get(cwd, model)
	if !ok {
		rec = clientRec{Cwd: cwd, Model: model, LastAPI: now}
	}
	rec.State, rec.EndedReason, rec.ChildPID = clientExpired, reason, 0
	r.Upsert(rec)
}

// isWarm reports whether a row can be resumed with its prompt cache still warm: it is live or parked,
// its last API call is within deadline, and its context is below limit tokens.
func isWarm(rec clientRec, limit int, now time.Time, deadline time.Duration) bool {
	if rec.State != clientLive && rec.State != clientParked {
		return false
	}
	return now.Sub(rec.LastAPI) < deadline && rec.LastCtx < limit
}

// idleDeadline is how long a client may sit idle before its prompt cache is assumed cold.
// ttl is the observed cache TTL when known, else the --cache-ttl flag value. "1h" gives idleRotate;
// "5m" and an unobserved "auto" give 290s, capped by idleRotate. idleRotate 0 disables it (returns 0).
func idleDeadline(idleRotate time.Duration, ttl string) time.Duration {
	if idleRotate <= 0 {
		return 0
	}
	if ttl == "1h" {
		return idleRotate
	}
	const short = 290 * time.Second
	if idleRotate < short {
		return idleRotate
	}
	return short
}

// expiredZero reports whether a deadline can expire at all: a deadline <= 0 means never expires.
func expiredZero(deadline time.Duration) bool {
	return deadline > 0
}

// reconcile fixes rows whose owner harness died without ending them: live rows with a dead OwnerPID
// become parked when they have a SessionID and last called the API within deadline, else expired.
// Rows of live owners are left alone.
func (r *clientRegistry) reconcile(now time.Time, deadline time.Duration) {
	if r == nil || r.path == "" {
		return
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	l := r.readLocked()
	changed := false
	for i := range l {
		c := &l[i]
		if c.State != clientLive || pidAlive(c.OwnerPID) {
			continue
		}
		c.State = clientExpired
		if c.SessionID != "" && now.Sub(c.LastAPI) < deadline {
			c.State = clientParked
		}
		c.EndedReason, c.ChildPID, changed = "harness-killed", 0, true
	}
	if changed {
		r.writeLocked(l)
	}
}
