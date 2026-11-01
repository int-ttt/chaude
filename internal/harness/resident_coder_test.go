package harness

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// fakeChild is one spawned resident child: out feeds its stdout, in receives the frames sent to it.
type fakeChild struct{ out, in chan string }

// stubCoderSessions makes every spawned child a fakeSession and reports it on the returned channel.
func stubCoderSessions(t *testing.T) chan fakeChild {
	t.Helper()
	spawned := make(chan fakeChild, 16)
	newSessionFn = func(SessionOpts) (*Session, error) {
		s, out, in := fakeSession()
		spawned <- fakeChild{out, in}
		return s, nil
	}
	t.Cleanup(func() { newSessionFn = NewSession })
	return spawned
}

// spawnCoderSlot spawns slot role through spawnSlot and returns its fake child.
func spawnCoderSlot(t *testing.T, r *Repl, spawned chan fakeChild, role string) fakeChild {
	t.Helper()
	slot, ok := r.residentSlotByRole(role)
	if !ok {
		t.Fatalf("no slot %s", role)
	}
	c, err := r.spawnSlot(slot)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(c.Sess.(*Session).StopIdle)
	return <-spawned
}

// answer reads the frame the coder received and replies with a summary-bearing text and a result.
func answer(t *testing.T, fc fakeChild, text string) string {
	t.Helper()
	frame := recv(t, fc.in)
	fc.out <- fmt.Sprintf(`{"type":"assistant","message":{"content":[{"type":"text","text":%q}]}}`, text)
	fc.out <- `{"type":"result"}`
	return frame
}

type callRes struct {
	text string
	err  error
}

// callCoderAsync runs runResidentCoder in the background.
func callCoderAsync(r *Repl, slot, task string) <-chan callRes {
	done := make(chan callRes, 1)
	go func() {
		s, err := r.runResidentCoder(slot, task)
		done <- callRes{s, err}
	}()
	return done
}

func waitCall(t *testing.T, done <-chan callRes) callRes {
	t.Helper()
	select {
	case res := <-done:
		return res
	case <-time.After(2 * time.Second):
		t.Fatal("resident_coder call did not return")
	}
	return callRes{}
}

// Default slot is C0 (coder): the task reaches the coder child and the summary is returned.
func TestResidentCoderDefaultsToC0(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, "min", 6)
	fc := spawnCoderSlot(t, r, spawned, "coder")
	done := callCoderAsync(r, "", "fix parser")
	frame := answer(t, fc, "thinking\nstatus: done\nchanged: parser.go: fix nil check\nbye")
	if !strings.Contains(frame, "fix parser") || !strings.Contains(frame, "status: done|partial|fail") {
		t.Fatalf("frame = %s", frame)
	}
	res := waitCall(t, done)
	if res.err != nil || res.text != "status: done\nchanged: parser.go: fix nil check" {
		t.Fatalf("res = %+v", res)
	}
	if _, err := r.runResidentCoder("tui", "x"); err == nil {
		t.Fatal("tui is not a coder slot: want error")
	}
}

// A second call to the same slot reuses the parked child: nothing is spawned.
func TestResidentCoderSlotReuse(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, "fixed", 8)
	fc := spawnCoderSlot(t, r, spawned, "coder")
	for i, slot := range []string{"", "coder"} {
		done := callCoderAsync(r, slot, fmt.Sprintf("task %d", i))
		answer(t, fc, "status: done")
		if res := waitCall(t, done); res.err != nil {
			t.Fatalf("call %d: %v", i, res.err)
		}
	}
	if n := len(spawned); n != 0 {
		t.Fatalf("reuse spawned %d children, want 0", n)
	}
}

// A spare slot that is not running is spawned on request, then used.
func TestResidentCoderSpawnsSpare(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, "fixed", 8)
	spawnCoderSlot(t, r, spawned, "coder")
	done := callCoderAsync(r, "coder-spare-2", "parallel docs")
	fc := <-spawned
	answer(t, fc, "status: done\nchanged: docs/a.md: add section")
	if res := waitCall(t, done); res.err != nil || res.text != "status: done\nchanged: docs/a.md: add section" {
		t.Fatalf("res = %+v", res)
	}
	if _, ok := r.residentChild("coder-spare-2"); !ok {
		t.Fatal("spare must stay parked after its call")
	}
}

// Summaries are kept per coder, last 3 only, and listed by resident_coders_status.
func TestResidentCoderSummaryListedInStatus(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, "fixed", 8)
	fc := spawnCoderSlot(t, r, spawned, "coder")
	for i := 1; i <= 4; i++ {
		done := callCoderAsync(r, "", fmt.Sprintf("t%d", i))
		answer(t, fc, fmt.Sprintf("status: done\nchanged: f%d.go: step %d", i, i))
		waitCall(t, done)
	}
	st := r.coderStatus()
	if strings.Contains(st, "step 1") || !strings.Contains(st, "step 2") || !strings.Contains(st, "step 4") {
		t.Fatalf("status keeps last 3: %q", st)
	}
	if !strings.Contains(st, "coder (C0): idle") || !strings.Contains(st, "coder-spare-1 (C1): not started") {
		t.Fatalf("status rows: %q", st)
	}
}

// A busy coder refuses a second call instead of queueing it.
func TestResidentCoderBusyRefuses(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, "fixed", 8)
	fc := spawnCoderSlot(t, r, spawned, "coder")
	first := callCoderAsync(r, "", "long")
	recv(t, fc.in)
	if _, err := r.runResidentCoder("", "second"); err == nil || !strings.Contains(err.Error(), "busy") {
		t.Fatalf("second call err = %v, want busy", err)
	}
	fc.out <- `{"type":"result"}`
	waitCall(t, first)
}

// can_use_tool is approved for a busy coder only, inside cwd for Read/Edit/Write.
func TestCoderPermissionOnlyWhenBusy(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, "fixed", 8)
	fc := spawnCoderSlot(t, r, spawned, "coder")
	c, _ := r.residentChild("coder")
	edit := func(path string) string {
		return fmt.Sprintf(`{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":%q}}}`, path)
	}
	inside := filepath.Join(r.a.Cwd, "a.go")

	r.parkedIdleEvent(c, parseEvent(edit(inside)))
	if got := recv(t, fc.in); !strings.Contains(got, `"behavior":"deny"`) {
		t.Fatalf("idle coder must be denied: %s", got)
	}

	c.beginBusy("edit", time.Now())
	r.parkedIdleEvent(c, parseEvent(edit(inside)))
	if got := recv(t, fc.in); !strings.Contains(got, `"behavior":"allow"`) {
		t.Fatalf("busy coder inside cwd must be allowed: %s", got)
	}
	r.parkedIdleEvent(c, parseEvent(edit("/etc/hosts")))
	if got := recv(t, fc.in); !strings.Contains(got, `"behavior":"deny"`) {
		t.Fatalf("busy coder outside cwd must be denied: %s", got)
	}
	c.endBusy()

	if coderMayUse(c, r.a.Cwd, "Bash", nil) {
		t.Fatal("idle coder must not get Bash approval")
	}
	if coderMayUse(&liveChild{Role: "tui"}, r.a.Cwd, "Bash", nil) {
		t.Fatal("non-coder role must not get approval")
	}
}

// MCP round trip over the bridge side of the socket: initialize, tools/list, tools/call x2.
func TestMCPRoundTripFakeSession(t *testing.T) {
	spawned := stubCoderSessions(t)
	r := residentRepl(t, "fixed", 8)
	fc := spawnCoderSlot(t, r, spawned, "coder")

	srv, cli := net.Pipe()
	go r.serveMCPConn(srv)
	t.Cleanup(func() { cli.Close() })
	rd := bufio.NewReader(cli)
	call := func(req string) map[string]any {
		t.Helper()
		if _, err := fmt.Fprintln(cli, req); err != nil {
			t.Fatal(err)
		}
		line, err := rd.ReadString('\n')
		if err != nil {
			t.Fatal(err)
		}
		var m map[string]any
		if err := json.Unmarshal([]byte(line), &m); err != nil {
			t.Fatalf("bad response %q: %v", line, err)
		}
		return m
	}

	if m := call(`{"jsonrpc":"2.0","id":1,"method":"initialize"}`); m["result"] == nil {
		t.Fatalf("initialize: %v", m)
	}
	if m := call(`{"jsonrpc":"2.0","id":2,"method":"tools/list"}`); !strings.Contains(fmt.Sprint(m), "resident_coders_status") {
		t.Fatalf("tools/list: %v", m)
	}

	// tools/call resident_coder: the reply is the summary only
	got := make(chan map[string]any, 1)
	go func() {
		got <- call(`{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"resident_coder","arguments":{"task":"add docs"}}}`)
	}()
	answer(t, fc, "long prose\nstatus: done\nchanged: README.md: add usage")
	m := <-got
	if text := fmt.Sprint(m["result"]); !strings.Contains(text, "changed: README.md: add usage") || strings.Contains(text, "long prose") {
		t.Fatalf("resident_coder result: %v", m)
	}

	// tools/call resident_coders_status lists the summary
	m = call(`{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"resident_coders_status","arguments":{}}}`)
	if text := fmt.Sprint(m["result"]); !strings.Contains(text, "README.md: add usage") {
		t.Fatalf("status result: %v", m)
	}

	// notifications get no response
	if r.mcpReply([]byte(`{"jsonrpc":"2.0","method":"notifications/initialized"}`)) != nil {
		t.Fatal("notification must not be answered")
	}
}

// The --mcp-config file points the main child at the bridge command with the listener socket.
func TestResidentMCPConfigWritten(t *testing.T) {
	r := residentRepl(t, "fixed", 8)
	path, err := r.residentMCPConfig()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(filepath.Dir(path)) })
	b, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var cfg struct {
		MCPServers map[string]struct {
			Args []string `json:"args"`
		} `json:"mcpServers"`
	}
	if err := json.Unmarshal(b, &cfg); err != nil {
		t.Fatal(err)
	}
	args := cfg.MCPServers["chaude"].Args
	if len(args) != 2 || args[0] != "mcp-resident" || !strings.HasSuffix(args[1], "mcp.sock") {
		t.Fatalf("args = %v", args)
	}
	if again, _ := r.residentMCPConfig(); again != path {
		t.Fatal("config must be created once")
	}
}
