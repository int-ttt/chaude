package harness

import (
	"strings"
	"testing"
	"time"
)

// runQuietTurn runs one turn on a fake session with a small idle notice and returns the reply and the output.
func runQuietTurn(t *testing.T, s *Session, out *strings.Builder) <-chan string {
	t.Helper()
	r := &Repl{out: out, a: Args{IdleNotice: 20 * time.Millisecond}, sess: s, tracker: newAgentTracker(),
		intr: make(chan struct{}, 1)}
	done := make(chan string, 1)
	go func() {
		reply, _, _ := r.runTurn("hi", false)
		done <- reply
	}()
	return done
}

// assertNoInterrupt fails when an interrupt control_request was written to the child's stdin.
func assertNoInterrupt(t *testing.T, in chan string) {
	t.Helper()
	for {
		select {
		case m := <-in:
			if strings.Contains(m, "interrupt") {
				t.Fatalf("turn interrupted: %s", m)
			}
		case <-time.After(50 * time.Millisecond):
			return
		}
	}
}

// An Agent tool_use whose result arrives after several idle notices: the late reply is kept, no interrupt.
func TestTurnWaitsForOpenAgentToolResult(t *testing.T) {
	s, out, in := fakeSession()
	var buf strings.Builder
	done := runQuietTurn(t, s, &buf)
	recv(t, in) // the user frame
	out <- `{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_agent","name":"Agent","input":{"subagent_type":"x","prompt":"p"}}]}}`
	time.Sleep(200 * time.Millisecond) // about ten idle notices pass while the Agent runs
	out <- `{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_agent","content":"done"}]}}`
	out <- `{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"LATE REPLY"}}}`
	out <- `{"type":"result","session_id":"x"}`
	select {
	case reply := <-done:
		if !strings.Contains(reply, "LATE REPLY") {
			t.Fatalf("late reply lost: %q", reply)
		}
	case <-time.After(5 * time.Second):
		t.Fatal("turn did not end")
	}
	if strings.Contains(buf.String(), "[interrupted]") {
		t.Fatalf("interrupted while tool open: %q", buf.String())
	}
	assertNoInterrupt(t, in)
}

// With no open tool call, a quiet stretch shows the notice once and keeps waiting; the reply still arrives.
func TestTurnQuietNoticeKeepsWaiting(t *testing.T) {
	s, out, in := fakeSession()
	var buf strings.Builder
	done := runQuietTurn(t, s, &buf)
	recv(t, in)
	time.Sleep(100 * time.Millisecond)
	out <- `{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"SLOW REPLY"}}}`
	out <- `{"type":"result","session_id":"x"}`
	select {
	case reply := <-done:
		if !strings.Contains(reply, "SLOW REPLY") {
			t.Fatalf("slow reply lost: %q", reply)
		}
	case <-time.After(5 * time.Second):
		t.Fatal("turn did not end")
	}
	if n := strings.Count(buf.String(), "still waiting"); n != 1 {
		t.Fatalf("want one notice, got %d: %q", n, buf.String())
	}
	if strings.Contains(buf.String(), "[interrupted]") {
		t.Fatalf("interrupted on quiet: %q", buf.String())
	}
	assertNoInterrupt(t, in)
}
