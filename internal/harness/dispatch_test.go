package harness

import (
	"os"
	"path/filepath"
	"sync/atomic"
	"testing"
	"time"
)

func TestPickChild(t *testing.T) {
	now := time.Now()
	const limit, deadline = 80_000, time.Hour
	dir := t.TempDir()
	reg := newClientRegistry(filepath.Join(dir, "clients.json"))
	key := childKey(dir, "sonnet", "")

	set := newChildSet()
	if got := pickChild(set, nil, key, dir, "sonnet", limit, now, deadline); got != actStartNew {
		t.Errorf("empty: got %v, want actStartNew", got)
	}

	set.Park(key, &liveChild{Sess: &fakeCloser{closed: new(int32)}, LastAPI: now, LastCtx: 100})
	if got := pickChild(set, nil, key, dir, "sonnet", limit, now, deadline); got != actUseLive {
		t.Errorf("live warm: got %v, want actUseLive", got)
	}
	set.Park(key, &liveChild{Sess: &fakeCloser{closed: new(int32)}, LastAPI: now, LastCtx: limit + 1})
	if got := pickChild(set, nil, key, dir, "sonnet", limit, now, deadline); got != actStartNew {
		t.Errorf("live over limit: got %v, want actStartNew", got)
	}
	set.CloseAll()

	reg.Upsert(clientRec{Cwd: dir, Model: "sonnet", State: clientParked, SessionID: "s1", LastAPI: now, LastCtx: 10})
	if got := pickChild(nil, reg, key, dir, "sonnet", limit, now, deadline); got != actResume {
		t.Errorf("parked warm: got %v, want actResume", got)
	}
	reg.Upsert(clientRec{Cwd: dir, Model: "sonnet", State: clientParked, SessionID: "s1", LastAPI: now.Add(-2 * time.Hour)})
	if got := pickChild(nil, reg, key, dir, "sonnet", limit, now, deadline); got != actStartNew {
		t.Errorf("parked expired: got %v, want actStartNew", got)
	}
	reg.Upsert(clientRec{Cwd: dir, Model: "sonnet", State: clientParked, SessionID: "s1", LastAPI: now, LastCtx: limit})
	if got := pickChild(nil, reg, key, dir, "sonnet", limit, now, deadline); got != actStartNew {
		t.Errorf("parked over limit: got %v, want actStartNew", got)
	}
	// Owned by another live process (pid 1 is always alive and is not this test process).
	reg.Upsert(clientRec{Cwd: dir, Model: "sonnet", State: clientParked, SessionID: "s1", LastAPI: now, OwnerPID: 1})
	if os.Getpid() != 1 {
		if got := pickChild(nil, reg, key, dir, "sonnet", limit, now, deadline); got != actStartNew {
			t.Errorf("owned by other live pid: got %v, want actStartNew", got)
		}
	}
}

func TestMaxWorkersDefault(t *testing.T) {
	t.Setenv("CHAUDE_MAX_WORKERS", "")
	got, err := resolveMaxWorkers(false, 0, os.Getenv("CHAUDE_MAX_WORKERS"))
	if err != nil || got != 3 {
		t.Errorf("default: got %d, %v; want 3, nil", got, err)
	}
}

func TestMaxWorkersEnvOverride(t *testing.T) {
	t.Setenv("CHAUDE_MAX_WORKERS", "5")
	got, err := resolveMaxWorkers(false, 0, os.Getenv("CHAUDE_MAX_WORKERS"))
	if err != nil || got != 5 {
		t.Errorf("env 5: got %d, %v; want 5, nil", got, err)
	}
	if got, err := resolveMaxWorkers(true, 2, "5"); err != nil || got != 2 {
		t.Errorf("flag beats env: got %d, %v; want 2, nil", got, err)
	}
}

func TestMaxWorkersRejectZero(t *testing.T) {
	if _, err := resolveMaxWorkers(true, 0, ""); err == nil {
		t.Errorf("flag 0: want error")
	}
	if _, err := resolveMaxWorkers(false, 0, "0"); err == nil {
		t.Errorf("env 0: want error")
	}
}

func TestMaxWorkersCapRefuses(t *testing.T) {
	now := time.Now()
	set := newChildSet()
	var closed int32
	set.Park("a", &liveChild{Sess: &fakeCloser{closed: &closed}, LastAPI: now})
	if err := checkWorkerCap(set, 2); err != nil {
		t.Fatalf("first start (1 live, cap 2): unexpected error %v", err)
	}
	set.Park("b", &liveChild{Sess: &fakeCloser{closed: &closed}, LastAPI: now})
	if err := checkWorkerCap(set, 2); err == nil {
		t.Fatalf("third start (2 live, cap 2): want refusal")
	}
	if n := atomic.LoadInt32(&closed); n != 0 {
		t.Errorf("existing children closed by refusal: %d", n)
	}
	if got := len(set.m); got != 2 {
		t.Errorf("live children after refusal: got %d, want 2", got)
	}
	set.CloseAll()
}
