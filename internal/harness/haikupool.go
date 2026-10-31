package harness

import (
	"sync"
	"time"
)

// haikuWorker is a Haiku session that already edited files: a candidate for the next edit-type prompt.
type haikuWorker struct {
	ID      string
	Task    string // first/last task it handled (terse)
	Files   []string
	Updated time.Time
}

// workerPool remembers Haiku sessions with edit history (in memory, per chaude process).
type workerPool struct {
	mu sync.Mutex
	l  map[string]*haikuWorker
}

func newWorkerPool() *workerPool { return &workerPool{l: map[string]*haikuWorker{}} }

// noteEdit records that Haiku session id edited file while handling task.
func (p *workerPool) noteEdit(id, task, file string) {
	if p == nil || id == "" || file == "" {
		return
	}
	p.mu.Lock()
	defer p.mu.Unlock()
	w := p.l[id]
	if w == nil {
		w = &haikuWorker{ID: id}
		p.l[id] = w
	}
	if task != "" {
		w.Task = trunc(task, 80)
	}
	if !contains(w.Files, file) {
		w.Files = append(w.Files, file)
	}
	w.Updated = time.Now()
}

func (p *workerPool) has(id string) bool {
	if p == nil {
		return false
	}
	p.mu.Lock()
	defer p.mu.Unlock()
	_, ok := p.l[id]
	return ok
}
