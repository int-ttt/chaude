package harness

import (
	"testing"
	"time"
)

func TestChildKeyRoleCoexist(t *testing.T) {
	dir := t.TempDir()
	if childKey(dir, "sonnet", "") == childKey(dir, "sonnet", "worker") {
		t.Fatal("same model with different role shares a key")
	}
	r := newMPRepl(t, "opus")
	now := time.Now()
	r.children.Park(childKey(r.a.Cwd, "sonnet", ""), &liveChild{Sess: &cntCloser{}, Model: "sonnet", LastAPI: now})
	r.children.Park(childKey(r.a.Cwd, "sonnet", "worker"), &liveChild{Sess: &cntCloser{}, Model: "sonnet", Role: "worker", LastAPI: now})
	if got := r.children.Len(); got != 2 {
		t.Fatalf("parked children = %d, want 2 (roles must coexist)", got)
	}
}

func TestAdmitEvictsOldestParked(t *testing.T) {
	r := newMPRepl(t, "opus")
	r.sess = nil
	r.a.MaxWorkers = 2
	now := time.Now()
	oldC, newC := &cntCloser{}, &cntCloser{}
	r.children.Park(childKey(r.a.Cwd, "haiku", ""), &liveChild{Sess: oldC, Model: "haiku", LastAPI: now.Add(-2 * time.Hour)})
	r.children.Park(childKey(r.a.Cwd, "sonnet", ""), &liveChild{Sess: newC, Model: "sonnet", LastAPI: now})
	if err := r.admit(); err != nil {
		t.Fatalf("admit: %v", err)
	}
	time.Sleep(50 * time.Millisecond)
	if oldC.count() != 1 || newC.count() != 0 {
		t.Fatalf("old=%d new=%d, want old closed only", oldC.count(), newC.count())
	}
	if got := r.children.Len(); got != 1 {
		t.Fatalf("parked after admit = %d, want 1", got)
	}
}

func TestAdmitRefusesWhenNothingEvictable(t *testing.T) {
	r := newMPRepl(t, "opus")
	r.a.MaxWorkers = 1
	if err := r.admit(); err == nil {
		t.Fatal("cap 1 with an active child and no parked child: want refusal")
	}
}
