package harness

import (
	"bufio"
	"io"
	"os"
	"strings"
	"sync"
	"testing"
	"time"
)

// fakeSession returns a Session whose stdout is fed via out and whose stdin lines arrive on in.
func fakeSession() (s *Session, out chan string, in chan string) {
	pr, pw := io.Pipe()
	out, in = make(chan string, 16), make(chan string, 16)
	s = &Session{stdin: pw, lines: out}
	go func() {
		sc := bufio.NewScanner(pr)
		for sc.Scan() {
			in <- sc.Text()
		}
	}()
	return
}

func recv(t *testing.T, ch chan string) string {
	t.Helper()
	select {
	case v := <-ch:
		return v
	case <-time.After(2 * time.Second):
		t.Fatal("timeout")
		return ""
	}
}

const canUse = `{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"ls"}}}`

func TestIdleControlRequestAnswered(t *testing.T) {
	s, out, in := fakeSession()
	r := &Repl{out: io.Discard, a: Args{}, tr: &Transcript{path: t.TempDir() + "/t.jsonl"}, tracker: newAgentTracker()}
	r.sess = s
	r.mode.Store(int32(modeAuto))
	s.StartIdle(func(ev event) { r.idleEvent(s, r.tracker, ev) })
	defer s.StopIdle()
	out <- `{"type":"result","session_id":"x"}`
	out <- canUse // between turns: no Events call active
	got := recv(t, in)
	if !strings.Contains(got, `"request_id":"r1"`) || !strings.Contains(got, `"allow"`) {
		t.Fatal(got)
	}
}

func TestIdleDeniesWhenNoPrompt(t *testing.T) {
	s, out, in := fakeSession()
	r := &Repl{out: io.Discard, a: Args{}, tr: &Transcript{path: t.TempDir() + "/t.jsonl"}, tracker: newAgentTracker()}
	r.sess = s
	s.StartIdle(func(ev event) { r.idleEvent(s, r.tracker, ev) })
	defer s.StopIdle()
	out <- canUse
	if got := recv(t, in); !strings.Contains(got, `"deny"`) {
		t.Fatal(got)
	}
}

func TestStrayLateResultSkipped(t *testing.T) {
	s, out, _ := fakeSession()
	s.StartIdle(func(event) {})
	defer s.StopIdle()
	// a background turn starts by itself, then the user sends before it ends
	out <- `{"type":"assistant","message":{"content":[]}}`
	deadline := time.Now().Add(2 * time.Second)
	for {
		s.mu.Lock()
		bg := s.bgTurn
		s.mu.Unlock()
		if bg {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("bg turn not noticed")
		}
		time.Sleep(5 * time.Millisecond)
	}
	_ = s.Send("hi")
	out <- `{"type":"result","result":"stray"}`
	out <- `{"type":"result","result":"mine"}`
	var res []string
	s.Events(2*time.Second, nil, func(ev event) {
		if ev["type"] == "result" {
			res = append(res, str(ev["result"]))
		}
	})
	if len(res) != 1 || res[0] != "mine" {
		t.Fatal(res)
	}
}

func TestIdleBackgroundFollowupRendered(t *testing.T) {
	s, out, _ := fakeSession()
	buf := &lockBuf{}
	r := &Repl{out: buf, a: Args{}, tr: &Transcript{path: t.TempDir() + "/t.jsonl"}, tracker: newAgentTracker()}
	r.sess = s
	r.tracker.onToolUse("tu1", "Agent", map[string]any{"description": "d", "run_in_background": true}, time.Now())
	if _, bg := r.tracker.counts(); bg != 1 {
		t.Fatal("setup")
	}
	s.StartIdle(func(ev event) { r.idleEvent(s, r.tracker, ev) })
	defer s.StopIdle()
	out <- `{"type":"result","session_id":"x"}`
	out <- `{"type":"user","message":{"content":"<task-notification><tool-use-id>tu1</tool-use-id></task-notification>"}}`
	out <- `{"type":"assistant","message":{"content":[{"type":"text","text":"에이전트 완료"}],"usage":{"input_tokens":3}}}`
	out <- `{"type":"result","session_id":"x","total_cost_usd":0.01}`
	deadline := time.Now().Add(2 * time.Second)
	for !strings.Contains(buf.String(), "[ctx") { // usage line is printed last
		if time.Now().After(deadline) {
			t.Fatal("no usage line")
		}
		time.Sleep(10 * time.Millisecond)
	}
	if b, _ := os.ReadFile(r.tr.path); !strings.Contains(string(b), "background") {
		t.Fatal("no transcript record")
	}
	if !strings.Contains(buf.String(), "에이전트 완료") {
		t.Fatalf("not rendered: %q", buf.String())
	}
	if fg, bg := r.tracker.counts(); fg+bg != 0 {
		t.Fatal("counter not cleared")
	}
	// a result with no preceding output is ignored (late-result guard stays quiet)
	out <- `{"type":"result","session_id":"x"}`
}

type lockBuf struct {
	mu sync.Mutex
	b  strings.Builder
}

func (l *lockBuf) Write(p []byte) (int, error) {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.b.Write(p)
}

func (l *lockBuf) String() string {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.b.String()
}
