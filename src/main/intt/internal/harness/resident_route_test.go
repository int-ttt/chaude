package harness

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"
	"time"
)

// parkResident parks a fake resident child of role under its key, with the idle reader on
// (as spawnSlot sets it). out feeds its stdout, in receives the frames the harness writes to it.
func parkResident(t *testing.T, r *Repl, role string) (*liveChild, chan string, chan string) {
	t.Helper()
	s, out, in := fakeSession()
	c := &liveChild{Sess: s, Tracker: newAgentTracker(), Model: "haiku", Role: role}
	c.setLastAPI(time.Now())
	c.parked.Store(true)
	s.StartIdle(func(ev event) { r.parkedIdleEvent(c, ev) })
	t.Cleanup(s.StopIdle)
	r.children.Park(childKey(r.a.Cwd, "haiku", role), c)
	return c, out, in
}

// Stage (a): a user message reaches the resident child in the Session.Send frame; it is busy until its result.
func TestSendResidentBusyUntilResult(t *testing.T) {
	r := residentRepl(t, 8)
	c, out, in := parkResident(t, r, "coder")
	ch, err := r.sendResident("coder", "hello", "hello")
	if err != nil {
		t.Fatal(err)
	}
	if got := recv(t, in); !strings.Contains(got, `"type":"user"`) || !strings.Contains(got, `"content":"hello"`) {
		t.Fatalf("frame = %s", got)
	}
	if c.busyRec() == nil {
		t.Fatal("resident child must be busy while its command runs")
	}
	out <- `{"type":"assistant","message":{"content":[{"type":"text","text":"done"}]}}`
	out <- `{"type":"result"}`
	select {
	case reply := <-ch:
		if reply != "done" {
			t.Fatalf("reply = %q, want done", reply)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("no reply from resident child")
	}
	if c.busyRec() != nil {
		t.Fatal("busy must end with the result")
	}
}

// Stage (a): spawned resident children are parked with their stdout read, so results are seen.
func TestSpawnSlotParksReadingChild(t *testing.T) {
	stubSessions(t)
	r := residentRepl(t, 8)
	slot, ok := r.residentSlotByRole("coder")
	if !ok {
		t.Fatal("no coder slot")
	}
	c, err := r.spawnSlot(slot)
	if err != nil {
		t.Fatal(err)
	}
	s := c.Sess.(*Session)
	t.Cleanup(s.StopIdle)
	if !c.parked.Load() || s.idleStop == nil {
		t.Fatal("spawned resident child must be parked and have its idle reader running")
	}
}

// runViaPrompter sends input through viaPrompter with a fake prompter that answers reply; it returns what main gets.
func runViaPrompter(t *testing.T, input, reply string) string {
	t.Helper()
	r := residentRepl(t, 8)
	_, out, in := parkResident(t, r, "prompter")
	done := make(chan string, 1)
	go func() { done <- r.viaPrompter(input) }()
	if got := recv(t, in); !strings.Contains(got, input) {
		t.Fatalf("prompter did not get the input as-is: %s", got)
	}
	out <- `{"type":"assistant","message":{"content":[{"type":"text","text":` + jsonString(reply) + `}]}}`
	out <- `{"type":"result"}`
	select {
	case got := <-done:
		return got
	case <-time.After(2 * time.Second):
		t.Fatal("viaPrompter did not return")
		return ""
	}
}

func jsonString(s string) string {
	b, _ := json.Marshal(s)
	return string(b)
}

// Stage (b): a plain summary goes to main; the original is added only when needOriginal says so.
func TestViaPrompterSummaryOnly(t *testing.T) {
	got := runViaPrompter(t, "버그 고쳐줘", "Fix the parser bug.")
	if want := "[prompter summary]\nFix the parser bug."; got != want {
		t.Fatalf("routed input = %q, want %q", got, want)
	}
}

// The prompter's OK line is dropped; a plain input gets no original.
func TestViaPrompterOKNoOriginal(t *testing.T) {
	got := runViaPrompter(t, "로그인 버그 고쳐줘", "OK\nfix login")
	if want := "[prompter summary]\nfix login"; got != want {
		t.Fatalf("routed input = %q, want %q", got, want)
	}
}

// NEED_ORIGINAL from the prompter adds the original even for plain input.
func TestViaPrompterNeedOriginalFlag(t *testing.T) {
	got := runViaPrompter(t, "로그인 버그 고쳐줘", "NEED_ORIGINAL\nfix login")
	if want := "[prompter summary]\nfix login\n\n[original]\n로그인 버그 고쳐줘"; got != want {
		t.Fatalf("routed input = %q, want %q", got, want)
	}
}

// Without a flag line the heuristic decides: a file path in the input adds the original.
func TestViaPrompterNoFlagHeuristic(t *testing.T) {
	got := runViaPrompter(t, "main.go 봐줘", "Review main.go.")
	if want := "[prompter summary]\nReview main.go.\n\n[original]\nmain.go 봐줘"; got != want {
		t.Fatalf("routed input = %q, want %q", got, want)
	}
}

// Only a flag line with an empty summary falls back to the input unchanged.
func TestViaPrompterEmptySummaryReturnsText(t *testing.T) {
	if got := runViaPrompter(t, "로그인 버그 고쳐줘", "OK"); got != "로그인 버그 고쳐줘" {
		t.Fatalf("routed input = %q, want input unchanged", got)
	}
}

func TestNeedOriginal(t *testing.T) {
	cases := []struct {
		in   string
		want bool
	}{
		{"로그인 버그 고쳐줘", false},
		{"아니 그거 말고", true},
		{"main.go 봐줘", true},
		{`"안녕" 번역해줘`, true},
		{"3번 반복", true},
		{"a\nb\nc\nd", true},
		{"ok", false},
		{"translate this", true},
		{"```go\nx\n```", true},
		{"~/x/y 확인", true},
	}
	for _, c := range cases {
		if got := needOriginal(c.in); got != c.want {
			t.Errorf("needOriginal(%q) = %v, want %v", c.in, got, c.want)
		}
	}
	if !needOriginal(strings.Repeat("가", 401)) {
		t.Error("needOriginal(401 runes) = false, want true")
	}
}

// Stage (b): without a live prompter, or when it does not answer in time, the input is unchanged and a notice says why.
func TestViaPrompterFallsBack(t *testing.T) {
	r := residentRepl(t, 8)
	var out bytes.Buffer
	r.out = &out
	if got := r.viaPrompter("plain"); got != "plain" {
		t.Fatalf("no prompter: got %q", got)
	}
	if !strings.Contains(out.String(), "prompter 실패(") {
		t.Fatalf("no prompter: missing notice, out = %q", out.String())
	}
	parkResident(t, r, "prompter")
	old := prompterTimeout
	prompterTimeout = 10 * time.Millisecond
	t.Cleanup(func() { prompterTimeout = old })
	out.Reset()
	if got := r.viaPrompter("plain"); got != "plain" {
		t.Fatalf("silent prompter: got %q", got)
	}
	if !strings.Contains(out.String(), "시간초과") {
		t.Fatalf("silent prompter: missing timeout notice, out = %q", out.String())
	}
}

// Stage (d): resident usage fills its P/T/C slot, including events that carry a parent_tool_use_id.
func TestResidentUsageFillsRoleSlot(t *testing.T) {
	r := residentRepl(t, 8)
	coder, _, _ := parkResident(t, r, "coder")
	prompter, _, _ := parkResident(t, r, "prompter")
	r.parkedIdleEvent(coder, parseEvent(`{"type":"assistant","parent_tool_use_id":"toolu_1","message":{"usage":{"input_tokens":10,"cache_read_input_tokens":90}}}`))
	r.parkedIdleEvent(prompter, parseEvent(`{"type":"assistant","message":{"usage":{"input_tokens":40}}}`))
	snap := r.roleCtxSnap()
	if snap["C"] != 100 || snap["P"] != 40 {
		t.Fatalf("role ctx = %v, want C=100 P=40", snap)
	}
}
