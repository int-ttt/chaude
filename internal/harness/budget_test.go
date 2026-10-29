package harness

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestBudgetDeltaPerProcess(t *testing.T) {
	b := newBudget(1)
	s1, s2 := &Session{}, &Session{}
	b.charge(s1, 0.10)
	b.charge(s1, 0.25) // +0.15
	b.charge(s2, 0.05) // other process starts at its own baseline
	if got := b.Spent(); got < 0.2999 || got > 0.3001 {
		t.Fatalf("spent %v, want 0.30", got)
	}
	b.charge(s1, 0.01) // counter dropped: whole total is new spend
	if got := b.Spent(); got < 0.3099 || got > 0.3101 {
		t.Fatalf("after drop spent %v, want 0.31", got)
	}
}

func TestBudgetOffAllows(t *testing.T) {
	var nb *budget
	nb.reset()
	nb.charge(nil, 5)
	nb.chargeOneShot(5)
	if !nb.allow(100) || nb.over(100) || nb.Spent() != 0 {
		t.Fatal("nil budget must be a no-op that allows")
	}
	z := newBudget(0)
	z.chargeOneShot(5)
	if !z.allow(100) || z.over(100) {
		t.Fatal("cap 0 must allow")
	}
}

func TestBudgetResetPerPrompt(t *testing.T) {
	b := newBudget(1)
	s := &Session{}
	b.charge(s, 0.9)
	b.reset()
	if b.Spent() != 0 || !b.allow(0.5) {
		t.Fatal("reset must clear spent")
	}
	b.charge(s, 1.0) // baseline kept: only +0.1
	if got := b.Spent(); got > 0.1001 {
		t.Fatalf("baseline lost: %v", got)
	}
}

func TestBudgetResumeBaseline(t *testing.T) {
	if resumeBaseline() != 0 {
		t.Fatal("resume cost must be 0")
	}
	// a resumed process (new *Session) books its whole first total, never seeded from a prior SID
	b := newBudget(1)
	b.charge(&Session{SessionID: "old"}, 0.2)
	if b.Spent() != 0.2 {
		t.Fatalf("spent %v", b.Spent())
	}
}

func TestBudgetGateBeforeSend(t *testing.T) {
	s, _, in := fakeSession()
	r := &Repl{out: os.Stderr, a: Args{Model: "opus", TaskBudgetUSD: 0.01}, sess: s, tracker: newAgentTracker()}
	r.out = &strings.Builder{}
	r.budget = newBudget(0.01)
	r.budget.chargeOneShot(0.02)
	reply, res, _ := r.runTurn("hi", false)
	if reply != "" || res != nil {
		t.Fatal("gate must refuse")
	}
	select {
	case m := <-in:
		t.Fatalf("message sent despite gate: %q", m)
	case <-time.After(100 * time.Millisecond):
	}
}

func TestBudgetInterruptMidTurn(t *testing.T) {
	s, out, in := fakeSession()
	r := &Repl{out: &strings.Builder{}, a: Args{Model: "opus", TaskBudgetUSD: 0.1}, sess: s, tracker: newAgentTracker(),
		intr: make(chan struct{}, 1)}
	r.budget = newBudget(0.1)
	out <- `{"type":"assistant","message":{"content":[],"usage":{"output_tokens":100000}}}`
	out <- `{"type":"result","total_cost_usd":0.5}`
	r.runTurn("hi", false)
	var all string
	for i := 0; i < 2; i++ {
		all += recv(t, in)
	}
	if !strings.Contains(all, "interrupt") {
		t.Fatalf("no interrupt sent: %s", all)
	}
	if r.budget.Spent() < 0.49 {
		t.Fatalf("result not charged: %v", r.budget.Spent())
	}
}

func TestBudgetExtraAppends(t *testing.T) {
	var got []string
	newSessionFn = func(o SessionOpts) (*Session, error) {
		got = o.Extra
		s, _, _ := fakeSession()
		return s, nil
	}
	defer func() { newSessionFn = NewSession }()
	r := newMPRepl(t, "opus")
	r.sess = nil
	r.cur = nil
	r.a.TaskBudgetUSD = 0.5
	if err := r.start(); err != nil {
		t.Fatal(err)
	}
	j := strings.Join(got, " ")
	if !strings.Contains(j, "--append-system-prompt") || !strings.Contains(j, "--max-budget-usd 0.5") {
		t.Fatalf("extra %q", j)
	}
	r.a.TaskBudgetUSD = 0
	_ = r.start()
	if strings.Contains(strings.Join(got, " "), "--max-budget-usd") {
		t.Fatal("flag present with budget off")
	}
}

func TestBudgetDisabledByDefault(t *testing.T) {
	r := &Repl{a: Args{ClientsFile: ""}}
	r.wireClients()
	if r.budget != nil {
		t.Fatal("budget must be off by default")
	}
	r.a.TaskBudgetUSD = 0.5
	r.wireClients()
	if r.budget == nil || r.budget.cap != 0.5 {
		t.Fatal("budget not wired when flag set")
	}
}

// BUG-1: the child pid is recorded on park.
func TestChildPIDRecordedOnPark(t *testing.T) {
	if childPID(&cntCloser{}) != 0 || childPID(nil) != 0 {
		t.Fatal("fake/nil child pid must be 0")
	}
	r := newMPRepl(t, "opus")
	s, _, _ := fakeSession()
	s.cmd = fakeCmdWithPID(4242)
	r.sess, r.cur = s, &liveChild{Sess: s, Tracker: r.tracker, Model: "opus"}
	r.setClock(time.Now())
	r.parkOld()
	rec, ok := r.reg.Get(r.a.Cwd, "opus")
	if !ok || rec.ChildPID != 4242 || rec.State != clientParked {
		t.Fatalf("row %+v", rec)
	}
}

// BUG-2: a parked row whose child is still alive is not resumed; it becomes an orphan notice.
func TestReconcileParkedAliveChild(t *testing.T) {
	reg := newClientRegistry(filepath.Join(t.TempDir(), "clients.json"))
	now := time.Now()
	reg.Upsert(clientRec{Cwd: "/w/a", Model: "opus", SessionID: "s1", State: clientParked, LastAPI: now, ChildPID: os.Getpid()})
	reg.Upsert(clientRec{Cwd: "/w/b", Model: "opus", SessionID: "s2", State: clientParked, LastAPI: now, ChildPID: 0})
	orph := reg.reconcile(now, time.Hour)
	if len(orph) != 1 || orph[0].Cwd != "/w/a" {
		t.Fatalf("orphans %+v", orph)
	}
	if got, _ := reg.Get("/w/a", "opus"); got.State != clientExpired || got.EndedReason != "orphan-alive" || got.ChildPID != 0 {
		t.Fatalf("alive-child row %+v", got)
	}
	if got, _ := reg.Get("/w/b", "opus"); got.State != clientParked {
		t.Fatalf("dead-child parked row changed %+v", got)
	}
}

func fakeCmdWithPID(pid int) *exec.Cmd { return &exec.Cmd{Process: &os.Process{Pid: pid}} }
