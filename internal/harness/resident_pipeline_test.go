package harness

import (
	"bytes"
	"strings"
	"testing"
	"time"
)

// Stage (b): a prompter that fails to answer (no session, empty summary) shows a notice and sends the input unchanged.
func TestViaPrompterFailureNotice(t *testing.T) {
	r := residentRepl(t, 8)
	var out bytes.Buffer
	r.out = &out
	if got := r.viaPrompter("plain"); got != "plain" {
		t.Fatalf("no prompter: got %q", got)
	}
	if !strings.Contains(out.String(), "prompter 실패(오류") {
		t.Fatalf("no prompter: missing notice, out = %q", out.String())
	}
	_, prOut, prIn := parkResident(t, r, "prompter")
	out.Reset()
	done := make(chan string, 1)
	go func() { done <- r.viaPrompter("plain") }()
	recv(t, prIn)
	prOut <- `{"type":"result"}`
	select {
	case got := <-done:
		if got != "plain" {
			t.Fatalf("empty summary: got %q", got)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("viaPrompter did not return")
	}
	if !strings.Contains(out.String(), "prompter 실패(빈 응답)") {
		t.Fatalf("empty summary: missing notice, out = %q", out.String())
	}
}

// Stage (c): the tui's Korean answer is what the user sees.
func TestViaTUITranslates(t *testing.T) {
	r := residentRepl(t, 8)
	_, tuOut, tuIn := parkResident(t, r, "tui")
	done := make(chan string, 1)
	go func() { done <- r.viaTUI("Fix the parser bug.") }()
	if got := recv(t, tuIn); !strings.Contains(got, "Fix the parser bug.") {
		t.Fatalf("tui did not get the main reply: %s", got)
	}
	tuOut <- `{"type":"assistant","message":{"content":[{"type":"text","text":"번역"}]}}`
	tuOut <- `{"type":"result"}`
	select {
	case got := <-done:
		if got != "번역" {
			t.Fatalf("viaTUI = %q, want 번역", got)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("viaTUI did not return")
	}
}

// Stage (c): without a live tui, or when it does not answer in time, the main reply is shown with a notice.
func TestViaTUIFailureNotice(t *testing.T) {
	r := residentRepl(t, 8)
	var out bytes.Buffer
	r.out = &out
	if got := r.viaTUI("raw reply"); got != "raw reply" {
		t.Fatalf("no tui: got %q", got)
	}
	if !strings.Contains(out.String(), "haiku-tui 실패(오류") {
		t.Fatalf("no tui: missing notice, out = %q", out.String())
	}
	parkResident(t, r, "tui")
	old := tuiTimeout
	tuiTimeout = 10 * time.Millisecond
	t.Cleanup(func() { tuiTimeout = old })
	out.Reset()
	if got := r.viaTUI("raw reply"); got != "raw reply" {
		t.Fatalf("silent tui: got %q", got)
	}
	if !strings.Contains(out.String(), "haiku-tui 실패(") || !strings.Contains(out.String(), "시간초과") {
		t.Fatalf("silent tui: missing timeout notice, out = %q", out.String())
	}
}

// Stage (c): with residents, the pane gets only the tui's answer, not the raw main deltas; runTurn returns the raw main text.
func TestRunTurnShowsTUIOutputOnly(t *testing.T) {
	r := residentRepl(t, 8)
	var out bytes.Buffer
	r.out = &out
	main, mainOut, mainIn := fakeSession()
	r.sess = main
	_, tuOut, tuIn := parkResident(t, r, "tui")
	type turn struct {
		reply string
		usage map[string]any
	}
	done := make(chan turn, 1)
	go func() {
		reply, _, usage := r.runTurn("hi", true)
		done <- turn{reply, usage}
	}()
	recv(t, mainIn)
	mainOut <- `{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"RAW-DELTA"}}}`
	mainOut <- `{"type":"result"}`
	if got := recv(t, tuIn); !strings.Contains(got, "RAW-DELTA") {
		t.Fatalf("tui did not get the main reply: %s", got)
	}
	tuOut <- `{"type":"assistant","message":{"content":[{"type":"text","text":"번역 답변"}]}}`
	tuOut <- `{"type":"result"}`
	select {
	case got := <-done:
		if got.reply != "RAW-DELTA" {
			t.Fatalf("runTurn reply = %q, want raw main text", got.reply)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("runTurn did not return")
	}
	pane := out.String()
	if !strings.Contains(pane, "번역 답변") {
		t.Fatalf("pane missing tui output: %q", pane)
	}
	if strings.Contains(pane, "RAW-DELTA") {
		t.Fatalf("pane shows raw main deltas: %q", pane)
	}
}
