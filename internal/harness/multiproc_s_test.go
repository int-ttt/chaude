package harness

import (
	"io"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
)

type cntCloser struct {
	mu sync.Mutex
	n  int
}

func (c *cntCloser) Close() { c.mu.Lock(); c.n++; c.mu.Unlock() }
func (c *cntCloser) count() int {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.n
}

type deadCloser struct{ cntCloser }

func (d *deadCloser) Dead() bool { return true }

func newMPRepl(t *testing.T, model string) *Repl {
	t.Helper()
	dir := t.TempDir()
	s, _, _ := fakeSession()
	r := &Repl{out: io.Discard, a: Args{Model: model, Cwd: dir}, tr: &Transcript{path: dir + "/t.jsonl"},
		tracker: newAgentTracker(), sess: s, parked: map[string]string{}}
	r.reg = newClientRegistry(dir + "/clients.json")
	r.children = newChildSet()
	r.idleRotate = time.Hour
	r.ttl = "1h"
	r.cur = &liveChild{Sess: s, Tracker: r.tracker, Model: model}
	return r
}

func withFakeSessions(t *testing.T, started *int) {
	t.Helper()
	newSessionFn = func(SessionOpts) (*Session, error) {
		*started++
		s, _, _ := fakeSession()
		return s, nil
	}
	t.Cleanup(func() { newSessionFn = NewSession })
}

func mainEv(typ string, extra map[string]any) event {
	ev := event{"type": typ}
	for k, v := range extra {
		ev[k] = v
	}
	return ev
}

// ---- U5a

func TestTouchMainOnly(t *testing.T) {
	r := &Repl{}
	now := time.Now()
	r.touchMain(event{"type": "stream_event", "parent_tool_use_id": "x", "event": map[string]any{"type": "message_start"}}, now)
	if !r.lastAPI.IsZero() {
		t.Fatal("subagent event moved the clock")
	}
	r.touchMain(event{"type": "stream_event", "event": map[string]any{"type": "message_start"}}, now)
	if !r.lastAPI.Equal(now) {
		t.Fatal("main message_start did not move the clock")
	}
}

func TestIdleEventFromParkedChild(t *testing.T) {
	r := newMPRepl(t, "opus")
	s, _, _ := fakeSession()
	c := &liveChild{Sess: s, Tracker: newAgentTracker(), Model: "haiku"}
	c.parked.Store(true)
	before := time.Now().Add(-time.Hour)
	r.lastAPI = before
	r.parkedIdleEvent(c, event{"type": "stream_event", "event": map[string]any{"type": "message_start"}})
	if !r.lastAPI.Equal(before) {
		t.Fatal("parked event moved the active clock")
	}
	if c.lastAPI().IsZero() {
		t.Fatal("parked child's own clock not updated")
	}
}

func parkedReply(r *Repl, c *liveChild, txt string) {
	r.parkedIdleEvent(c, event{"type": "stream_event", "event": map[string]any{"type": "content_block_delta",
		"delta": map[string]any{"type": "text_delta", "text": txt}}})
	r.parkedIdleEvent(c, event{"type": "result"})
}

func TestIdleEventParkedNoRender(t *testing.T) {
	r := newMPRepl(t, "opus")
	var sb strings.Builder
	r.out = &sb
	s, _, _ := fakeSession()
	c := &liveChild{Sess: s, Tracker: newAgentTracker(), Model: "haiku"}
	parkedReply(r, c, "hello")
	if sb.Len() != 0 {
		t.Fatalf("parked reply rendered: %q", sb.String())
	}
	n := r.takeNotices()
	if len(n) != 1 || !strings.Contains(n[0], "parked haiku background reply") || !strings.Contains(n[0], "hello") {
		t.Fatalf("notice: %q", n)
	}
}

func TestIdleEventParkedPermDenied(t *testing.T) {
	r := newMPRepl(t, "opus")
	s, _, in := fakeSession()
	c := &liveChild{Sess: s, Tracker: newAgentTracker(), Model: "haiku"}
	r.parkedIdleEvent(c, event{"type": "control_request", "request_id": "r1",
		"request": map[string]any{"subtype": "can_use_tool", "tool_name": "Bash", "input": map[string]any{}}})
	got := recv(t, in)
	if !strings.Contains(got, "denied: session parked") || false {
		t.Fatalf("not denied: %s", got)
	}
}

func TestParkedReplyNoUnseen(t *testing.T) {
	r := newMPRepl(t, "opus")
	s, _, _ := fakeSession()
	c := &liveChild{Sess: s, Tracker: newAgentTracker(), Model: "haiku"}
	parkedReply(r, c, "x")
	if r.idle.active || r.idle.streamed {
		t.Fatal("parked reply touched idle render state")
	}
}

// ---- U5b

func TestBindUnbindRoundTrip(t *testing.T) {
	r := newMPRepl(t, "opus")
	old := r.sess
	r.pendingSnapshot = "SNAP"
	r.lastCtx = 42
	r.lastAPI = time.Now()
	r.ttlSeen = "1h"
	r.cat = "x"
	c := r.unbindActive()
	if r.sess != nil || r.pendingSnapshot != "" || r.cur != nil || r.tracker == nil || !c.parked.Load() {
		t.Fatal("unbind state")
	}
	r.bindChild(c)
	if r.sess != old || r.pendingSnapshot != "SNAP" || r.lastCtx != 42 || r.ttlSeen != "1h" || r.cat != "x" || r.cur != c || c.parked.Load() {
		t.Fatalf("bind state: %+v", r.pendingSnapshot)
	}
}

func TestBindAddsParkedDelta(t *testing.T) {
	r := newMPRepl(t, "opus")
	c := r.unbindActive()
	r.tr.turns = append(r.tr.turns, [2]string{"q1", "a1"})
	r.bindChild(c)
	if !strings.Contains(r.pendingSnapshot, "While this session was parked") || !strings.Contains(r.pendingSnapshot, "q1") {
		t.Fatalf("delta: %q", r.pendingSnapshot)
	}
}

func TestBindNoDeltaKeepsEmpty(t *testing.T) {
	r := newMPRepl(t, "opus")
	c := r.unbindActive()
	r.bindChild(c)
	if r.pendingSnapshot != "" {
		t.Fatalf("snapshot: %q", r.pendingSnapshot)
	}
}

func TestTrackerNeverNil(t *testing.T) {
	r := newMPRepl(t, "opus")
	r.tracker = nil
	r.cur = nil
	c := r.unbindActive()
	if r.tracker == nil {
		t.Fatal("tracker nil after unbind")
	}
	c.Tracker = nil
	r.bindChild(c)
	if r.tracker == nil {
		t.Fatal("tracker nil after bind")
	}
}

func TestUnbindParkedRace(t *testing.T) {
	r := newMPRepl(t, "opus")
	var wg sync.WaitGroup
	stop := make(chan struct{})
	c := r.cur
	wg.Add(1)
	cc := c
	go func() {
		defer wg.Done()
		for {
			select {
			case <-stop:
				return
			default:
			}
			r.touchMain(event{"type": "stream_event", "event": map[string]any{"type": "message_start"}}, time.Now())
			if cc.parked.Load() {
				r.parkedIdleEvent(cc, event{"type": "stream_event", "event": map[string]any{"type": "message_start"}})
			}
		}
	}()
	for i := 0; i < 200; i++ {
		r.bindChild(r.unbindActive())
	}
	close(stop)
	wg.Wait()
}

// ---- U5d

func TestMakeSnapshotStates(t *testing.T) {
	mk := func(turns bool, noValidate bool) *Repl {
		r := newMPRepl(t, "opus")
		r.a.NoValidate = noValidate
		s, out, _ := fakeSession()
		close(out) // the child answers nothing: empty note
		r.sess = s
		if turns {
			r.tr.turns = [][2]string{{"q", "a"}}
		}
		return r
	}
	if _, st := mk(false, false).makeSnapshot("t"); st != snapHeld {
		t.Fatalf("no turns, no note: st=%v want held", st)
	}
	if _, st := mk(true, false).makeSnapshot("t"); st != snapInvalid {
		t.Fatalf("invalid note: st=%v", st)
	}
	if _, st := mk(true, true).makeSnapshot("t"); st != snapOK {
		t.Fatalf("validation off: st=%v want ok", st)
	}
}

// ---- U6b

func TestParkReplaceClosesOnce(t *testing.T) {
	cs := newChildSet()
	a, b := &cntCloser{}, &cntCloser{}
	cs.Park("k", &liveChild{Sess: a})
	cs.Park("k", &liveChild{Sess: b})
	if a.count() != 1 || b.count() != 0 {
		t.Fatalf("a=%d b=%d", a.count(), b.count())
	}
}

func seedTurn(r *Repl) {
	r.tr.turns = [][2]string{{"q", "a"}}
	r.pendingSnapshot = "SNAP" // avoid a handoff turn against the fake child
	r.setClock(time.Now())
}

func (r *Repl) setClock(t time.Time) {
	r.clockMu.Lock()
	r.lastAPI = t
	r.clockMu.Unlock()
}

func TestRouteSameModelWorkerUsesOldPath(t *testing.T) {
	var got []string
	newSessionFn = func(o SessionOpts) (*Session, error) {
		got = append(got, o.Resume)
		s, _, _ := fakeSession()
		return s, nil
	}
	defer func() { newSessionFn = NewSession }()
	r := newMPRepl(t, "haiku")
	seedTurn(r)
	r.routeChildren("haiku", "task", "worker-9")
	if len(got) != 1 || got[0] != "worker-9" {
		t.Fatalf("resume opts: %v", got)
	}
	if r.children.m != nil && len(r.children.m) != 0 {
		t.Fatal("old path must not park into the child set")
	}
}

// ---- U7b

func TestIdleDue(t *testing.T) {
	r := &Repl{idleRotate: time.Hour, ttl: "1h"}
	t0 := time.Now()
	r.lastAPI = t0
	d := idleDeadline(r.idleRotate, "1h")
	if r.idleDue(t0.Add(d - time.Second)) {
		t.Fatal("fired before the deadline")
	}
	if !r.idleDue(t0.Add(d)) {
		t.Fatal("did not fire at the deadline")
	}
	if r.idleDue(t0.Add(d + time.Minute)) {
		t.Fatal("fired twice for one lastAPI")
	}
	r.lastAPI = t0.Add(time.Hour) // a new turn re-arms
	if !r.idleDue(t0.Add(time.Hour + d)) {
		t.Fatal("not re-armed")
	}
	r2 := &Repl{idleRotate: time.Hour, ttl: "1h"}
	if r2.idleDue(time.Now().Add(10 * time.Hour)) {
		t.Fatal("zero lastAPI fired")
	}
}

func TestIdleDueDisabled(t *testing.T) {
	r := &Repl{idleRotate: 0, ttl: ""}
	r.lastAPI = time.Now().Add(-100 * time.Hour)
	if r.idleDue(time.Now()) {
		t.Fatal("fired while disabled")
	}
}

// ---- U8

func TestSweepIdleClosesParkedOnly(t *testing.T) {
	r := newMPRepl(t, "opus")
	cold, warm := &cntCloser{}, &cntCloser{}
	now := time.Now()
	r.children.Park(childKey(r.a.Cwd, "haiku", ""), &liveChild{Sess: cold, Model: "haiku", LastAPI: now.Add(-2 * time.Hour)})
	r.children.Park(childKey(r.a.Cwd, "sonnet", ""), &liveChild{Sess: warm, Model: "sonnet", LastAPI: now})
	r.setClock(now.Add(-2 * time.Hour))
	if !r.sweepIdle(now) {
		t.Fatal("active not due")
	}
	time.Sleep(50 * time.Millisecond)
	if cold.count() != 1 || warm.count() != 0 {
		t.Fatalf("cold=%d warm=%d", cold.count(), warm.count())
	}
}

func TestSweepDrainsBusyParked(t *testing.T) {
	r := newMPRepl(t, "opus")
	tk := newAgentTracker()
	tk.onToolUse("a1", "Agent", map[string]any{"description": "d", "name": "w", "prompt": "p"}, time.Now())
	c := &cntCloser{}
	r.children.Park(childKey(r.a.Cwd, "haiku", ""), &liveChild{Sess: c, Tracker: tk, Model: "haiku", LastAPI: time.Now().Add(-2 * time.Hour)})
	r.sweepIdle(time.Now())
	time.Sleep(50 * time.Millisecond)
	if c.count() != 0 {
		t.Fatal("busy parked child closed")
	}
}

func TestSweepSkippedWhileBusy(t *testing.T) {
	m, r := avModel(t)
	r.idleRotate, r.ttl = time.Hour, "1h"
	s, _, _ := fakeSession()
	r.sess = s
	r.setClock(time.Now().Add(-2 * time.Hour))
	m.busy = true
	m.Update(spinner.TickMsg{})
	select {
	case l := <-m.submit:
		t.Fatalf("idle rotate submitted while busy: %s", l)
	default:
	}
}

func TestSweepSkippedDuringPermPrompt(t *testing.T) {
	m, r := avModel(t)
	r.idleRotate, r.ttl = time.Hour, "1h"
	s, _, _ := fakeSession()
	r.sess = s
	r.setClock(time.Now().Add(-2 * time.Hour))
	m.perm = &permMsg{}
	m.Update(spinner.TickMsg{})
	select {
	case l := <-m.submit:
		t.Fatalf("idle rotate submitted during prompt: %s", l)
	default:
	}
}

func TestIdleRotateKeepsPendingSnapshot(t *testing.T) {
	var n int
	withFakeSessions(t, &n)
	r := newMPRepl(t, "opus")
	r.pendingSnapshot = "KEEP"
	r.tr.turns = [][2]string{{"q", "a"}}
	r.idleRotateNow()
	if r.pendingSnapshot != "KEEP" || n != 1 {
		t.Fatalf("snap=%q n=%d", r.pendingSnapshot, n)
	}
}

func TestIdleNoticeKeepsHeldOffset(t *testing.T) {
	m, r := avModel(t)
	for i := 0; i < 60; i++ {
		m.Update(tea.MouseMsg{Action: tea.MouseActionPress, Button: tea.MouseButtonWheelUp})
	}
	r.addNotice("[parked haiku background reply] hi")
	m.Update(spinner.TickMsg{})
	_ = m.View()
}

func TestQuitCallsCloseAll(t *testing.T) {
	r := newMPRepl(t, "opus")
	c := &cntCloser{}
	r.children.Park("k", &liveChild{Sess: c})
	r.closeAllChildren()
	if c.count() != 1 {
		t.Fatal("not closed")
	}
}

func TestPlainLoopCallsCloseAll(t *testing.T) { TestQuitCallsCloseAll(t) }

func TestQuitCloseAllWithNilSess(t *testing.T) {
	r := newMPRepl(t, "opus")
	r.sess = nil
	c := &cntCloser{}
	r.children.Park("k", &liveChild{Sess: c})
	r.closeAllChildren()
	if c.count() != 1 {
		t.Fatal("not closed with nil sess")
	}
}
