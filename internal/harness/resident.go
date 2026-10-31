package harness

import (
	"errors"
	"fmt"
	"strings"
	"time"
)

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
	r.children.Park(childKey(r.a.Cwd, slot.Model, slot.Role), c)
	return c, nil
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
