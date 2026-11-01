package harness

import (
	"errors"
	"fmt"
	"sort"
	"strings"
	"time"
)

// beginBusy marks a resident child as handling a command: it shows as one running-list row until endBusy.
func (c *liveChild) beginBusy(prompt string, now time.Time) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.busy = &agentRec{ID: c.Role, Name: c.Role, Model: c.Model, Prompt: firstLine(prompt), Started: now}
}

// endBusy clears the command marker; the child drops out of the running list.
func (c *liveChild) endBusy() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.busy = nil
}

// busyRec returns the running-list row of a busy child, nil when idle.
func (c *liveChild) busyRec() *agentRec {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.busy
}

// residentBusy: parked resident children that are handling a command, oldest first. Idle ones are not listed.
func (r *Repl) residentBusy() []*agentRec {
	if r.children == nil {
		return nil
	}
	var l []*agentRec
	r.children.mu.Lock()
	for _, c := range r.children.m {
		if c == nil {
			continue
		}
		if b := c.busyRec(); b != nil {
			l = append(l, b)
		}
	}
	r.children.mu.Unlock()
	sort.Slice(l, func(i, j int) bool {
		if l[i].Started.Equal(l[j].Started) {
			return l[i].Name < l[j].Name
		}
		return l[i].Started.Before(l[j].Started)
	})
	return l
}

// residentLazyIdle is how long a lazy resident child may stay parked unused before it is retired.
const residentLazyIdle = 270 * time.Second

// residentSlot is one named child of a resident preset. Eager slots are spawned with the REPL;
// lazy slots are spawned by takeOrSpawnSlot on first use. The "main" slot is the active child.
type residentSlot struct {
	Role  string
	Model string
	Lazy  bool
}

// residentPreset returns the slots and the worker cap of a preset. "" and "off" give no slots.
// mainModel is the model of the "main" slot; "" means the preset default (fixed: opus, min: haiku).
func residentPreset(name, mainModel string) ([]residentSlot, int, error) {
	switch strings.TrimSpace(name) {
	case "", "off":
		return nil, 0, nil
	case "fixed":
		if mainModel == "" {
			mainModel = "opus"
		}
		return []residentSlot{
			{Role: "main", Model: mainModel},
			{Role: "prompter", Model: "haiku"},
			{Role: "tui", Model: "haiku"},
			{Role: "coder", Model: "haiku"},
			{Role: "coder-spare-1", Model: "haiku", Lazy: true},
			{Role: "coder-spare-2", Model: "haiku", Lazy: true},
			{Role: "coder-spare-3", Model: "haiku", Lazy: true},
			{Role: "coder-spare-4", Model: "haiku", Lazy: true},
		}, 8, nil
	case "min":
		if mainModel == "" {
			mainModel = "haiku"
		}
		return []residentSlot{
			{Role: "main", Model: mainModel},
			{Role: "coder", Model: "haiku"},
			{Role: "worker-2", Model: "haiku", Lazy: true},
			{Role: "worker-3", Model: "haiku", Lazy: true},
			{Role: "worker-4", Model: "haiku", Lazy: true},
			{Role: "opus-1", Model: "opus", Lazy: true},
		}, 6, nil
	}
	return nil, 0, fmt.Errorf("--resident must be off, min or fixed (got %q)", name)
}

// residentSlotByRole finds the slot with role in r.resident.
func (r *Repl) residentSlotByRole(role string) (residentSlot, bool) {
	for _, s := range r.resident {
		if s.Role == role {
			return s, true
		}
	}
	return residentSlot{}, false
}

// spawnSlot starts a claude child for slot through admit() and newSessionFn, and parks it under
// its (cwd, model, role) key. The child is stamped with now so an unused lazy child idles from its spawn.
func (r *Repl) spawnSlot(slot residentSlot) (*liveChild, error) {
	if r.children == nil {
		return nil, errors.New("resident slots need the client registry (--clients-file)")
	}
	if err := r.admit(); err != nil {
		return nil, err
	}
	s, err := newSessionFn(SessionOpts{Model: slot.Model, Cwd: r.a.Cwd, CompactWindow: r.a.CompactWindow, CacheTTL: r.a.CacheTTL})
	if err != nil {
		return nil, err
	}
	c := &liveChild{Sess: s, Tracker: newAgentTracker(), Model: slot.Model, Role: slot.Role, Lazy: slot.Lazy}
	c.setLastAPI(time.Now())
	c.parked.Store(true)
	s.StartIdle(func(ev event) { r.parkedIdleEvent(c, ev) }) // parked children have no turn: read stdout here
	r.children.Park(childKey(r.a.Cwd, slot.Model, slot.Role), c)
	return c, nil
}

// residentRoleKey maps a resident role to its status-line slot: prompter P, tui T, coder C0, coder-spare-i Ci.
// "" for roles without a slot (main, worker-N, opus-N).
func residentRoleKey(role string) string {
	switch {
	case role == "prompter":
		return "P"
	case role == "tui":
		return "T"
	case role == "coder":
		return "C0"
	case strings.HasPrefix(role, "coder-spare-"):
		return "C" + strings.TrimPrefix(role, "coder-spare-")
	}
	return ""
}

// residentChild returns the parked child of resident slot role, left in the set. False when absent or dead.
func (r *Repl) residentChild(role string) (*liveChild, bool) {
	slot, ok := r.residentSlotByRole(role)
	if !ok || r.children == nil {
		return nil, false
	}
	r.children.mu.Lock()
	c := r.children.m[childKey(r.a.Cwd, slot.Model, role)]
	r.children.mu.Unlock()
	return c, c != nil && !childDead(c)
}

// sendResident writes msg to the parked resident child of role as one user message and marks it busy
// with label until its result. The returned channel gets the reply text of that result (buffered, one value).
func (r *Repl) sendResident(role, msg, label string) (<-chan string, error) {
	c, ok := r.residentChild(role)
	if !ok {
		return nil, fmt.Errorf("no live resident %q", role)
	}
	s, ok := c.Sess.(*Session)
	if !ok || s == nil {
		return nil, fmt.Errorf("resident %q has no session", role)
	}
	ch := make(chan string, 1)
	c.mu.Lock()
	c.await = ch
	c.mu.Unlock()
	c.beginBusy(label, time.Now()) // before the write: the result may arrive right after it
	if err := s.sendFrame(msg); err != nil {
		c.endBusy()
		c.mu.Lock()
		c.await = nil
		c.mu.Unlock()
		return nil, err
	}
	return ch, nil
}

// prompterTimeout bounds the wait for the resident prompter's summary; on timeout the input goes unchanged.
var prompterTimeout = 60 * time.Second

// viaPrompter returns the text Opus main receives: the resident prompter's English summary, then the
// original input. Without a live prompter, or when it does not answer in time, text is returned unchanged.
func (r *Repl) viaPrompter(text string) string {
	ask := "Summarize the following user input in English in a few sentences. Reply with the summary only.\n\n" + text
	ch, err := r.sendResident("prompter", ask, text)
	if err != nil {
		return text
	}
	select {
	case sum := <-ch:
		if sum == "" {
			return text
		}
		return "[prompter summary EN]\n" + sum + "\n\n[original input]\n" + text
	case <-time.After(prompterTimeout):
		return text
	}
}

// spawnResident starts the eager non-main slots of the preset and parks them. Lazy slots wait for
// takeOrSpawnSlot. Failures are collected; the other slots are still tried.
func (r *Repl) spawnResident() error {
	var errs []error
	for _, slot := range r.resident {
		if slot.Role == "main" || slot.Lazy {
			continue
		}
		if _, err := r.spawnSlot(slot); err != nil {
			errs = append(errs, fmt.Errorf("%s: %w", slot.Role, err))
		}
	}
	return errors.Join(errs...)
}

// takeOrSpawnSlot returns the child of slot role, removed from the parked set so the caller can bind it.
// A parked child is taken as is; otherwise the slot is spawned (admit() makes room first).
func (r *Repl) takeOrSpawnSlot(role string) (*liveChild, error) {
	slot, ok := r.residentSlotByRole(role)
	if !ok {
		return nil, fmt.Errorf("no resident slot %q", role)
	}
	if r.children == nil {
		return nil, errors.New("resident slots need the client registry (--clients-file)")
	}
	key := childKey(r.a.Cwd, slot.Model, role)
	if c, ok := r.children.Take(key); ok && c != nil && !childDead(c) {
		return c, nil
	}
	c, err := r.spawnSlot(slot)
	if err != nil {
		return nil, err
	}
	if taken, ok := r.children.Take(key); ok && taken == c {
		return taken, nil
	}
	return c, nil
}

// sweepParked retires lazy parked children idle for residentLazyIdle or more. Eager children and
// children with delegated agents still running are kept.
func (r *Repl) sweepParked(now time.Time) {
	if r.children == nil {
		return
	}
	for _, k := range r.children.Expired(now, residentLazyIdle) {
		r.children.mu.Lock()
		c := r.children.m[k]
		skip := c == nil || !c.Lazy || (c.Tracker != nil && len(c.Tracker.running()) > 0)
		r.children.mu.Unlock()
		if skip {
			continue
		}
		if c, ok := r.children.Take(k); ok {
			r.retire(c)
			r.reg.MarkEnded(r.a.Cwd, c.Model, "lazy-idle", now)
		}
	}
}
