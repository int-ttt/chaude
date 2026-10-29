package harness

import (
	"os"
	"sync/atomic"
	"testing"
	"time"
)

func TestChildKey(t *testing.T) {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	k := childKey("", "sonnet")
	if k != childKey(".", "sonnet") {
		t.Errorf("empty cwd and \".\" differ: %q vs %q", k, childKey(".", "sonnet"))
	}
	if k != childKey(wd, "sonnet") {
		t.Errorf("empty cwd and working dir differ: %q vs %q", k, childKey(wd, "sonnet"))
	}
	if childKey(wd, "sonnet") == childKey(wd, "haiku") {
		t.Errorf("different models share a key")
	}
}

type fakeCloser struct {
	delay  time.Duration
	closed *int32
}

func (f *fakeCloser) Close() {
	time.Sleep(f.delay)
	atomic.AddInt32(f.closed, 1)
}

func TestChildSetParkTake(t *testing.T) {
	var n int32
	s := newChildSet()
	now := time.Now()
	a := &liveChild{Sess: &fakeCloser{closed: &n}, LastAPI: now}
	s.Park("k1", a)
	s.Park("k1", &liveChild{Sess: &fakeCloser{closed: &n}, LastAPI: now})
	if n != 1 {
		t.Fatalf("replacing a parked child should close the old one once, closed=%d", n)
	}
	if len(s.m) != 1 {
		t.Fatalf("same key should keep one entry, have %d", len(s.m))
	}
	if _, ok := s.Take("k1"); !ok {
		t.Fatal("Take did not find parked child")
	}
	if _, ok := s.Take("k1"); ok {
		t.Fatal("Take should remove the child")
	}

	s.Park("old", &liveChild{Sess: &fakeCloser{closed: &n}, LastAPI: now.Add(-2 * time.Hour)})
	s.Park("new", &liveChild{Sess: &fakeCloser{closed: &n}, LastAPI: now.Add(-time.Minute)})
	if got := s.Expired(now, time.Hour); len(got) != 1 || got[0] != "old" {
		t.Errorf("Expired = %v, want [old]", got)
	}
	s.CloseAll()
	if len(s.m) != 0 {
		t.Errorf("CloseAll left %d entries", len(s.m))
	}
}

func TestCloseAllParallel(t *testing.T) {
	var n int32
	s := newChildSet()
	for _, k := range []string{"a", "b", "c"} {
		s.Park(k, &liveChild{Sess: &fakeCloser{delay: 100 * time.Millisecond, closed: &n}})
	}
	start := time.Now()
	s.CloseAll()
	elapsed := time.Since(start)
	if elapsed >= 250*time.Millisecond {
		t.Errorf("CloseAll took %v, want < 250ms (parallel)", elapsed)
	}
	if n != 3 {
		t.Errorf("closed %d children, want 3", n)
	}
}
