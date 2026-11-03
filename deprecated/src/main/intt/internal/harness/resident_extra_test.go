package harness

import (
	"strings"
	"testing"
	"time"
)

// TestExtraCoderPoolFromSetting: extra coders beyond the always-on coder exist only up to extraCoders.
func TestExtraCoderPoolFromSetting(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, 8)
	r.extraCoders = 1
	fc := spawnCoderSlot(t, r, spawned, "coder")
	first := callCoderAsync(r, "long")
	recv(t, fc.in)

	// coder is busy, so the next call goes to the extra coder2 (not started yet, so idle).
	if role, err := r.pickCoderRole(); err != nil || role != "coder2" {
		t.Fatalf("pick = %q, %v; want coder2", role, err)
	}
	second := callCoderAsync(r, "second")
	fc2 := <-spawned
	t.Cleanup(func() {
		if c, ok := r.residentChild("coder2"); ok {
			c.Sess.Close()
		}
	})
	answer(t, fc2, "from-extra")
	if got := waitCall(t, second); got.err != nil || !strings.Contains(got.text, "from-extra") {
		t.Fatalf("extra coder reply = %+v", got)
	}

	// extra_coders 0: coder busy means refusal, no extra coder.
	r.extraCoders = 0
	if _, err := r.pickCoderRole(); err == nil || !strings.Contains(err.Error(), "busy") {
		t.Fatalf("with extraCoders=0 err = %v, want busy", err)
	}
	fc.out <- `{"type":"result"}`
	waitCall(t, first)
}

// TestExtraCoderExpiresAt270s: an idle extra coder is ended after extraCoderTTL, the base coder is kept.
func TestExtraCoderExpiresAt270s(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, 8)
	spawnCoderSlot(t, r, spawned, "coder")
	spawnCoderSlot(t, r, spawned, "coder2")
	if _, ok := r.residentChild("coder2"); !ok {
		t.Fatal("coder2 not parked")
	}
	r.sweepIdle(time.Now().Add(extraCoderTTL - time.Second))
	if _, ok := r.residentChild("coder2"); !ok {
		t.Fatal("coder2 expired before 270s")
	}
	r.sweepIdle(time.Now().Add(extraCoderTTL + time.Second))
	if _, ok := r.residentChild("coder2"); ok {
		t.Fatal("coder2 still parked after 270s")
	}
	if _, ok := r.residentChild("coder"); !ok {
		t.Fatal("base coder expired with extra coder")
	}
}
