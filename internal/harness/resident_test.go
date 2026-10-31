package harness

import (
	"io"
	"testing"
	"time"
)

// stubSessions counts newSessionFn calls and restores NewSession on cleanup.
func stubSessions(t *testing.T) *int {
	t.Helper()
	var n int
	newSessionFn = func(SessionOpts) (*Session, error) {
		n++
		s, _, _ := fakeSession()
		return s, nil
	}
	t.Cleanup(func() { newSessionFn = NewSession })
	return &n
}

func residentRepl(t *testing.T, preset string, maxWorkers int) *Repl {
	t.Helper()
	slots, _, err := residentPreset(preset, "haiku")
	if err != nil {
		t.Fatal(err)
	}
	return &Repl{out: io.Discard, tr: &Transcript{path: t.TempDir() + "/t.jsonl"}, tracker: newAgentTracker(),
		a: Args{Model: "haiku", Cwd: t.TempDir(), MaxWorkers: maxWorkers}, children: newChildSet(), resident: slots}
}

func TestResidentPresetFixedSpawnsEager(t *testing.T) {
	started := stubSessions(t)
	r := residentRepl(t, "fixed", 8)
	if err := r.start(); err != nil {
		t.Fatal(err)
	}
	if *started != 4 { // main + tui, coder, prompter
		t.Fatalf("spawned %d, want 4", *started)
	}
	if r.children.Len() != 3 {
		t.Fatalf("parked %d, want 3", r.children.Len())
	}
	if _, ok := r.children.Take(childKey(r.a.Cwd, "haiku", "tui")); !ok {
		t.Fatal("tui slot not parked under its role key")
	}
}

func TestResidentPresetMinSpawnsEager(t *testing.T) {
	started := stubSessions(t)
	r := residentRepl(t, "min", 6)
	if err := r.start(); err != nil {
		t.Fatal(err)
	}
	if *started != 2 { // main + coder
		t.Fatalf("spawned %d, want 2", *started)
	}
	if r.children.Len() != 1 {
		t.Fatalf("parked %d, want 1", r.children.Len())
	}
}

func TestResidentPresetCaps(t *testing.T) {
	if s, cap, err := residentPreset("fixed", "haiku"); err != nil || cap != 8 || len(s) != 8 {
		t.Fatalf("fixed: slots %d cap %d err %v", len(s), cap, err)
	}
	if s, cap, err := residentPreset("min", "haiku"); err != nil || cap != 6 || len(s) != 6 {
		t.Fatalf("min: slots %d cap %d err %v", len(s), cap, err)
	}
	if _, _, err := residentPreset("bogus", "haiku"); err == nil {
		t.Fatal("unknown preset accepted")
	}
}

func TestResidentPresetMainModelDefault(t *testing.T) {
	s, cap, err := residentPreset("fixed", "")
	if err != nil || cap != 8 || len(s) != 8 {
		t.Fatalf("fixed: slots %d cap %d err %v", len(s), cap, err)
	}
	if s[0].Role != "main" || s[0].Model != "opus" {
		t.Fatalf("fixed main: role %q model %q, want main/opus", s[0].Role, s[0].Model)
	}
	for i, role := range []string{"prompter", "tui", "coder"} {
		if s[i+1].Role != role || s[i+1].Model != "haiku" {
			t.Fatalf("fixed slot %d: role %q model %q, want %s/haiku", i+1, s[i+1].Role, s[i+1].Model, role)
		}
	}
	for i := 4; i < 8; i++ {
		if !s[i].Lazy || s[i].Model != "haiku" {
			t.Fatalf("fixed slot %d: lazy %v model %q, want lazy haiku", i, s[i].Lazy, s[i].Model)
		}
	}
	if m, _, _ := residentPreset("min", ""); m[0].Model != "haiku" {
		t.Fatalf("min main model %q, want haiku", m[0].Model)
	}
	if o, _, _ := residentPreset("fixed", "haiku"); o[0].Model != "haiku" {
		t.Fatalf("explicit override main model %q, want haiku", o[0].Model)
	}
}

func TestResidentOffSpawnsNothing(t *testing.T) {
	started := stubSessions(t)
	r := residentRepl(t, "off", 3)
	if err := r.start(); err != nil {
		t.Fatal(err)
	}
	if *started != 1 || r.children.Len() != 0 || r.residentUp {
		t.Fatalf("off: spawned %d parked %d up %v", *started, r.children.Len(), r.residentUp)
	}
}

func TestTakeOrSpawnSlotLazy(t *testing.T) {
	started := stubSessions(t)
	r := residentRepl(t, "fixed", 8)
	s, _, _ := fakeSession()
	r.sess = s
	c, err := r.takeOrSpawnSlot("coder-spare-1")
	if err != nil {
		t.Fatal(err)
	}
	if *started != 1 || !c.Lazy || c.Role != "coder-spare-1" || c.Model != "haiku" {
		t.Fatalf("lazy spawn: started %d lazy %v role %q model %q", *started, c.Lazy, c.Role, c.Model)
	}
	if r.children.Len() != 0 {
		t.Fatalf("taken child still parked: %d", r.children.Len())
	}
	if _, err := r.takeOrSpawnSlot("no-such-slot"); err == nil {
		t.Fatal("unknown slot accepted")
	}
}

func TestTakeOrSpawnSlotRespectsCap(t *testing.T) {
	started := stubSessions(t)
	r := residentRepl(t, "fixed", 1)
	s, _, _ := fakeSession()
	r.sess = s // active child fills the cap of 1; nothing parked to evict
	if _, err := r.takeOrSpawnSlot("coder-spare-2"); err == nil {
		t.Fatal("spawned past the cap")
	}
	if *started != 0 {
		t.Fatalf("spawned %d past the cap", *started)
	}
}

func TestSweepParkedRetiresLazyOnly(t *testing.T) {
	r := &Repl{out: io.Discard, a: Args{Cwd: t.TempDir()}, children: newChildSet()}
	now := time.Now()
	park := func(model, role string, idle time.Duration, lazy bool) {
		s, _, _ := fakeSession()
		c := &liveChild{Sess: s, Model: model, Role: role, Lazy: lazy}
		c.setLastAPI(now.Add(-idle))
		r.children.Park(childKey(r.a.Cwd, model, role), c)
	}
	park("haiku", "worker-2", 271*time.Second, true)
	park("haiku", "worker-3", 269*time.Second, true)
	park("haiku", "coder", 400*time.Second, false)
	r.sweepParked(now)
	if _, ok := r.children.Take(childKey(r.a.Cwd, "haiku", "worker-2")); ok {
		t.Fatal("lazy child idle 271s not retired")
	}
	if _, ok := r.children.Take(childKey(r.a.Cwd, "haiku", "worker-3")); !ok {
		t.Fatal("lazy child idle 269s retired early")
	}
	if _, ok := r.children.Take(childKey(r.a.Cwd, "haiku", "coder")); !ok {
		t.Fatal("eager child retired by sweepParked")
	}
}
