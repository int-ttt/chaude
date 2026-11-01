package harness

import (
	"errors"
	"strings"
	"testing"
	"time"
)

// A parked prompter that died is respawned by sendResident and the message still reaches it.
func TestSendResidentRespawnsDeadSlot(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, 8)
	slot, _ := r.residentSlotByRole("prompter")
	dead, err := r.spawnSlot(slot)
	if err != nil {
		t.Fatal(err)
	}
	<-spawned
	dead.Sess.(*Session).dead.Store(true)

	ch, err := r.sendResident("prompter", "hello", "hello")
	if err != nil {
		t.Fatalf("send after death: %v", err)
	}
	fc := <-spawned
	if c, ok := r.residentChild("prompter"); !ok {
		t.Fatal("prompter not live after respawn")
	} else {
		t.Cleanup(c.Sess.(*Session).StopIdle)
	}
	if got := answer(t, fc, "summary"); !strings.Contains(got, "hello") {
		t.Fatalf("respawned prompter got %q, want the message", got)
	}
	select {
	case got := <-ch:
		if got != "summary" {
			t.Fatalf("reply %q, want summary", got)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("no reply from respawned prompter")
	}
}

// When the respawn fails, viaPrompter keeps the raw input (the fallback).
func TestViaPrompterRawWhenRespawnFails(t *testing.T) {
	newSessionFn = func(SessionOpts) (*Session, error) { return nil, errors.New("spawn boom") }
	t.Cleanup(func() { newSessionFn = NewSession })
	r := residentRepl(t, 8)
	if got := r.viaPrompter("원문"); got != "원문" {
		t.Fatalf("viaPrompter = %q, want raw input", got)
	}
}

// failRespawn makes every spawn fail, for tests of the no-live-slot fallback: a missing slot is not revived.
func failRespawn(t *testing.T) {
	t.Helper()
	newSessionFn = func(SessionOpts) (*Session, error) { return nil, errors.New("spawn boom") }
	t.Cleanup(func() { newSessionFn = NewSession })
}
