package harness

import (
	"bufio"
	"bytes"
	"strings"
	"testing"
)

// /help, /status, /exit in the plain REPL are answered locally: the child session must receive nothing.
func TestPlainLocalCmdsNoSend(t *testing.T) {
	sess, _, in := fakeSession()
	newSessionFn = func(SessionOpts) (*Session, error) { return sess, nil }
	defer func() { newSessionFn = NewSession }()
	var out bytes.Buffer
	r := &Repl{out: &out, in: bufio.NewReader(strings.NewReader("/help\n/status\n/exit\n")),
		a: Args{Model: "opus"}, tr: &Transcript{path: t.TempDir() + "/t.jsonl"}, tracker: newAgentTracker(), lastCtx: 1234}
	if code := r.loop(); code != 0 {
		t.Fatalf("loop exit code %d", code)
	}
	select {
	case l := <-in:
		t.Fatalf("local command sent to child: %.80s", l)
	default:
	}
	s := out.String()
	for _, w := range []string{"/help", "show commands and keys", "model opus", "ctx 1,234/"} {
		if !strings.Contains(s, w) {
			t.Fatalf("output missing %q: %.200s", w, s)
		}
	}
}
