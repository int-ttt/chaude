package harness

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestClientRegistryRoundTrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "clients.json")
	now := time.Now()
	newClientRegistry(path).Upsert(clientRec{Cwd: "/w/a", Model: "haiku", SessionID: "s1", State: clientLive, LastAPI: now, LastCtx: 42, OwnerPID: os.Getpid(), ChildPID: 7})

	got, ok := newClientRegistry(path).Get("/w/a", "haiku")
	if !ok || got.SessionID != "s1" || got.State != clientLive || got.LastCtx != 42 || got.ChildPID != 7 || got.OwnerPID != os.Getpid() {
		t.Fatalf("round trip mismatch: ok=%v rec=%+v", ok, got)
	}
	if _, ok := newClientRegistry(path).Get("/w/a", "sonnet"); ok {
		t.Fatal("unexpected row for other model")
	}

	reg := newClientRegistry(path)
	reg.MarkEnded("/w/a", "haiku", "idle-rotate", now)
	got, _ = newClientRegistry(path).Get("/w/a", "haiku")
	if got.State != clientExpired || got.EndedReason != "idle-rotate" || got.ChildPID != 0 {
		t.Fatalf("MarkEnded mismatch: %+v", got)
	}

	var nilReg *clientRegistry
	nilReg.Upsert(clientRec{Cwd: "/x", Model: "m"})
	if _, ok := nilReg.Get("/x", "m"); ok {
		t.Fatal("nil registry should return no row")
	}
}

func TestClientRegistryBrokenJSON(t *testing.T) {
	path := filepath.Join(t.TempDir(), "clients.json")
	if err := os.WriteFile(path, []byte("{not json"), 0o600); err != nil {
		t.Fatal(err)
	}
	reg := newClientRegistry(path)
	if _, ok := reg.Get("/w/a", "haiku"); ok {
		t.Fatal("broken file should read as empty")
	}
	reg.reconcile(time.Now(), time.Minute) // must not panic
	reg.Upsert(clientRec{Cwd: "/w/a", Model: "haiku", State: clientLive})
	if _, ok := newClientRegistry(path).Get("/w/a", "haiku"); !ok {
		t.Fatal("upsert after broken file should write a fresh row")
	}
}

func TestClientRegistryAtomicWrite(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "clients.json")
	reg := newClientRegistry(path)
	reg.Upsert(clientRec{Cwd: "/w/a", Model: "haiku", State: clientLive})
	reg.Upsert(clientRec{Cwd: "/w/a", Model: "haiku", State: clientParked})
	if _, err := os.Stat(path + ".tmp"); !os.IsNotExist(err) {
		t.Fatalf("tmp file left behind: %v", err)
	}
	if got, _ := reg.Get("/w/a", "haiku"); got.State != clientParked {
		t.Fatalf("second upsert not visible: %+v", got)
	}
}

func TestClientRegistryKeyedByCwdModel(t *testing.T) {
	path := filepath.Join(t.TempDir(), "clients.json")
	reg := newClientRegistry(path)
	reg.Upsert(clientRec{Cwd: "/w/a", Model: "haiku", SessionID: "a-h", State: clientLive})
	reg.Upsert(clientRec{Cwd: "/w/a", Model: "sonnet", SessionID: "a-s", State: clientLive})
	reg.Upsert(clientRec{Cwd: "/w/b", Model: "haiku", SessionID: "b-h", State: clientLive})
	reg.Upsert(clientRec{Cwd: "/w/a", Model: "haiku", SessionID: "a-h2", State: clientParked})

	for _, c := range []struct{ cwd, model, sid string }{
		{"/w/a", "haiku", "a-h2"}, {"/w/a", "sonnet", "a-s"}, {"/w/b", "haiku", "b-h"},
	} {
		got, ok := newClientRegistry(path).Get(c.cwd, c.model)
		if !ok || got.SessionID != c.sid {
			t.Fatalf("%s/%s: want session %q, got ok=%v %+v", c.cwd, c.model, c.sid, ok, got)
		}
	}
}

func TestIsWarm(t *testing.T) {
	now := time.Now()
	const limit, deadline = 100000, 5 * time.Minute
	base := clientRec{Cwd: "/w/a", Model: "haiku", State: clientLive, LastAPI: now.Add(-time.Minute), LastCtx: 1000}
	cases := []struct {
		name   string
		mutate func(*clientRec)
		want   bool
	}{
		{"live recent small", func(*clientRec) {}, true},
		{"parked recent small", func(r *clientRec) { r.State = clientParked }, true},
		{"expired", func(r *clientRec) { r.State = clientExpired }, false},
		{"api older than deadline", func(r *clientRec) { r.LastAPI = now.Add(-deadline) }, false},
		{"api just inside deadline", func(r *clientRec) { r.LastAPI = now.Add(-deadline + time.Second) }, true},
		{"context at limit", func(r *clientRec) { r.LastCtx = limit }, false},
		{"context below limit", func(r *clientRec) { r.LastCtx = limit - 1 }, true},
	}
	for _, c := range cases {
		rec := base
		c.mutate(&rec)
		if got := isWarm(rec, limit, now, deadline); got != c.want {
			t.Errorf("%s: isWarm=%v want %v (rec=%+v)", c.name, got, c.want, rec)
		}
	}
}

func TestReconcileKilledOwner(t *testing.T) {
	path := filepath.Join(t.TempDir(), "clients.json")
	now := time.Now()
	reg := newClientRegistry(path)
	// dead owner (pid 0), recent API, has session -> parked
	reg.Upsert(clientRec{Cwd: "/w/a", Model: "haiku", SessionID: "s1", State: clientLive, LastAPI: now.Add(-time.Minute)})
	// dead owner, old API -> expired
	reg.Upsert(clientRec{Cwd: "/w/b", Model: "haiku", SessionID: "s2", State: clientLive, LastAPI: now.Add(-time.Hour)})
	// live owner (this process) -> untouched
	reg.Upsert(clientRec{Cwd: "/w/c", Model: "haiku", SessionID: "s3", State: clientLive, LastAPI: now.Add(-time.Minute), OwnerPID: os.Getpid()})

	reg.reconcile(now, 10*time.Minute)

	reg2 := newClientRegistry(path)
	if got, _ := reg2.Get("/w/a", "haiku"); got.State != clientParked || got.EndedReason != "harness-killed" {
		t.Fatalf("recent dead-owner row: %+v", got)
	}
	if got, _ := reg2.Get("/w/b", "haiku"); got.State != clientExpired || got.EndedReason != "harness-killed" {
		t.Fatalf("old dead-owner row: %+v", got)
	}
	if got, _ := reg2.Get("/w/c", "haiku"); got.State != clientLive || got.EndedReason != "" {
		t.Fatalf("live-owner row changed: %+v", got)
	}
}

func TestIdleDeadline(t *testing.T) {
	h := time.Hour
	cases := []struct {
		rot  time.Duration
		ttl  string
		want time.Duration
	}{
		{h, "1h", h},
		{h, "5m", 290 * time.Second},
		{h, "auto", 290 * time.Second},
		{100 * time.Second, "auto", 100 * time.Second},
		{0, "1h", 0},
		{0, "5m", 0},
	}
	for _, c := range cases {
		if got := idleDeadline(c.rot, c.ttl); got != c.want {
			t.Errorf("idleDeadline(%v, %q) = %v, want %v", c.rot, c.ttl, got, c.want)
		}
	}
}

func TestExpiredZeroDeadline(t *testing.T) {
	cases := []struct {
		deadline time.Duration
		want     bool
	}{
		{0, false},
		{-time.Second, false},
		{time.Second, true},
		{3599 * time.Second, true},
	}
	for _, c := range cases {
		if got := expiredZero(c.deadline); got != c.want {
			t.Errorf("expiredZero(%v) = %v, want %v", c.deadline, got, c.want)
		}
	}
}
