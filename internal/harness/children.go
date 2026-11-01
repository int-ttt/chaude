package harness

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

// childKey identifies a live child session by working directory, model and role.
// An empty cwd means the process working directory; symlinks are resolved so
// /var and /private/var on macOS map to the same key. An empty role gives the
// role-less key; a non-empty role appends one more NUL-separated field.
func childKey(cwd, model, role string) string {
	if cwd == "" {
		if wd, err := os.Getwd(); err == nil {
			cwd = wd
		}
	}
	if abs, err := filepath.Abs(cwd); err == nil {
		cwd = abs
	}
	if real, err := filepath.EvalSymlinks(cwd); err == nil {
		cwd = real
	}
	if role == "" {
		return cwd + "\x00" + model
	}
	return cwd + "\x00" + model + "\x00" + role
}

// closer is the part of a child session that childSet needs; *Session satisfies it.
type closer interface{ Close() }

// liveChild is a claude -p child kept alive outside the active slot (parked-live).
type liveChild struct {
	Sess     closer
	Tracker  *agentTracker
	Model    string
	Role     string
	Lazy     bool // resident lazy slot: retired by sweepParked after residentLazyIdle parked
	LastAPI  time.Time
	LastCtx  int
	Files    []string
	TurnFrom int

	// Repl state with no other home while the child is parked (saved by unbindActive, restored by bindChild).
	PendingSnapshot string
	LastRotFail     time.Time
	TTLSeen         string
	Cat             string

	mu       sync.Mutex  // guards LastAPI and the parked reply buffer
	parked   atomic.Bool // true from unbindActive until bindChild; the idle callback branches on it
	reply    strings.Builder
	stream   bool        // the parked reply arrived as text_delta events
	sawNotif bool        // a <task-notification> was seen since the last result
	busy     *agentRec   // command in flight: one running-list row while set (nil = idle); guarded by mu
	await    chan string // reply sink of the command in flight (sendResident); nil = nobody waits; guarded by mu
}

func (c *liveChild) lastAPI() time.Time {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.LastAPI
}

func (c *liveChild) setLastAPI(t time.Time) {
	c.mu.Lock()
	c.LastAPI = t
	c.mu.Unlock()
}

// childSet holds the parked-live children of one harness process, keyed by childKey.
type childSet struct {
	mu sync.Mutex
	m  map[string]*liveChild
}

func newChildSet() *childSet {
	return &childSet{m: map[string]*liveChild{}}
}

// Park stores c under key, replacing and closing any child already stored there.
func (s *childSet) Park(key string, c *liveChild) {
	s.mu.Lock()
	old := s.m[key]
	s.m[key] = c
	s.mu.Unlock()
	if old != nil && old.Sess != nil && old.Sess != c.Sess {
		old.Sess.Close()
	}
}

// Len returns the number of parked children.
func (s *childSet) Len() int {
	s.mu.Lock()
	defer s.mu.Unlock()
	return len(s.m)
}

// EvictLRU removes and returns the parked child with the oldest LastAPI. The caller retires it.
func (s *childSet) EvictLRU() (*liveChild, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	var oldestKey string
	var oldest *liveChild
	var oldestAt time.Time
	for k, c := range s.m {
		at := c.lastAPI()
		if oldest == nil || at.Before(oldestAt) || (at.Equal(oldestAt) && k < oldestKey) {
			oldestKey, oldest, oldestAt = k, c, at
		}
	}
	if oldest == nil {
		return nil, false
	}
	delete(s.m, oldestKey)
	return oldest, true
}

// Take removes and returns the child stored under key.
func (s *childSet) Take(key string) (*liveChild, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	c, ok := s.m[key]
	if ok {
		delete(s.m, key)
	}
	return c, ok
}

// Expired returns the keys, sorted, of children whose last API call is at least deadline old.
// It does not remove them; the caller Takes and closes the ones it wants to end.
func (s *childSet) Expired(now time.Time, deadline time.Duration) []string {
	s.mu.Lock()
	defer s.mu.Unlock()
	var keys []string
	for k, c := range s.m {
		if now.Sub(c.lastAPI()) >= deadline {
			keys = append(keys, k)
		}
	}
	sort.Strings(keys)
	return keys
}

// CloseAll ends every stored child in parallel and empties the set.
// Each Close waits for its process, so serial closing would add up to the sum of the waits.
func (s *childSet) CloseAll() {
	s.mu.Lock()
	list := make([]*liveChild, 0, len(s.m))
	for k, c := range s.m {
		list = append(list, c)
		delete(s.m, k)
	}
	s.mu.Unlock()
	var wg sync.WaitGroup
	for _, c := range list {
		if c.Sess == nil {
			continue
		}
		wg.Add(1)
		go func(sess closer) {
			defer wg.Done()
			sess.Close()
		}(c.Sess)
	}
	wg.Wait()
}
