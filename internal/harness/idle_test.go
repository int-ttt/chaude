package harness

import (
	"bufio"
	"io"
	"strings"
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
