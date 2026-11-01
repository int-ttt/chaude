package harness

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// Agent rules: the body of <cwd>/.claude/agents/<name>.md (frontmatter dropped) is passed as --append-system-prompt.
func TestAgentPromptLoaded(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	cwd := t.TempDir()
	dir := filepath.Join(cwd, ".claude", "agents")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "haiku-prompter.md"), []byte("---\nname: haiku-prompter\n---\nBODY LINE\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	args := sessionArgs(SessionOpts{Model: "haiku", Cwd: cwd, Agent: "haiku-prompter"})
	for i, a := range args {
		if a == "--append-system-prompt" {
			if i+1 >= len(args) || args[i+1] != "BODY LINE" {
				t.Fatalf("append value = %q, want %q", args[i+1], "BODY LINE")
			}
			return
		}
	}
	t.Fatalf("no --append-system-prompt in argv: %v", args)
}

// A missing agent file: the notice is shown and the child is spawned without --append-system-prompt.
func TestAgentPromptMissingNotice(t *testing.T) {
	t.Setenv("HOME", t.TempDir())
	r := residentRepl(t, 8)
	var out bytes.Buffer
	r.out = &out
	var gotAgent string
	newSessionFn = func(o SessionOpts) (*Session, error) {
		gotAgent = o.Agent
		s, _, _ := fakeSession()
		return s, nil
	}
	t.Cleanup(func() { newSessionFn = NewSession })
	if _, err := r.spawnSlot(residentSlot{Role: "prompter", Model: "haiku", Agent: "haiku-missing"}); err != nil {
		t.Fatal(err)
	}
	if want := "에이전트 규칙 없음(haiku-missing): 기본 세션으로 시작"; !strings.Contains(out.String(), want) {
		t.Fatalf("missing notice: out = %q, want %q", out.String(), want)
	}
	if gotAgent != "" {
		t.Fatalf("spawned with agent %q, want none", gotAgent)
	}
}

// Stage lines come in pipeline order: prompter, then tui.
func TestStageLinesOrder(t *testing.T) {
	t.Setenv("HOME", t.TempDir())
	r := residentRepl(t, 8)
	var out bytes.Buffer
	r.out = &out
	_, prOut, prIn := parkResident(t, r, "prompter")
	_, tuOut, tuIn := parkResident(t, r, "tui")

	done := make(chan string, 1)
	go func() { done <- r.viaPrompter("plain") }()
	recv(t, prIn)
	prOut <- `{"type":"assistant","message":{"content":[{"type":"text","text":"summary"}]}}`
	prOut <- `{"type":"result"}`
	select {
	case <-done:
	case <-time.After(2 * time.Second):
		t.Fatal("viaPrompter did not return")
	}

	go func() { done <- r.viaTUI("summary") }()
	recv(t, tuIn)
	tuOut <- `{"type":"assistant","message":{"content":[{"type":"text","text":"번역"}]}}`
	tuOut <- `{"type":"result"}`
	select {
	case <-done:
	case <-time.After(2 * time.Second):
		t.Fatal("viaTUI did not return")
	}

	lines := []string{"· prompter 처리중", "· prompter 완료", "· tui 번역중", "· tui 완료"}
	last := -1
	for _, l := range lines {
		i := strings.Index(out.String(), l)
		if i < 0 {
			t.Fatalf("missing stage line %q in %q", l, out.String())
		}
		if i <= last {
			t.Fatalf("stage line %q out of order in %q", l, out.String())
		}
		last = i
	}
}
