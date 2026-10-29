package harness

import (
	"fmt"
	"sort"
	"strings"
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

const maxCandidates = 5

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

// candidates: newest first, at most maxCandidates.
func (p *workerPool) candidates() []haikuWorker {
	if p == nil {
		return nil
	}
	p.mu.Lock()
	defer p.mu.Unlock()
	var l []haikuWorker
	for _, w := range p.l {
		l = append(l, *w)
	}
	sort.Slice(l, func(i, j int) bool { return l[i].Updated.After(l[j].Updated) })
	if len(l) > maxCandidates {
		l = l[:maxCandidates]
	}
	return l
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

// candidateText is the classifier-facing list ("" when empty).
func candidateText(c []haikuWorker) string {
	if len(c) == 0 {
		return ""
	}
	var b strings.Builder
	for _, w := range c {
		fmt.Fprintf(&b, "- %s | task: %s | edited: %s\n", w.ID, w.Task, trunc(strings.Join(w.Files, ", "), 160))
	}
	return b.String()
}
