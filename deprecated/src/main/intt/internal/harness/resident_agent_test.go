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

// slotArgs spawns slot role through spawnSlot with a stubbed claude and returns the argv sessionArgs builds for it.
func slotArgs(t *testing.T, r *Repl, role string) []string {
	t.Helper()
	var got SessionOpts
	newSessionFn = func(o SessionOpts) (*Session, error) {
		got = o
		s, _, _ := fakeSession()
		return s, nil
	}
	t.Cleanup(func() { newSessionFn = NewSession })
	slot, ok := r.residentSlotByRole(role)
	if !ok {
		t.Fatalf("no slot %s", role)
	}
	c, err := r.spawnSlot(slot)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(c.Sess.(*Session).StopIdle)
	return sessionArgs(got)
}

// argAfter returns the value following flag in args, and whether flag is present.
func argAfter(args []string, flag string) (string, bool) {
	for i, a := range args {
		if a == flag {
			if i+1 < len(args) {
				return args[i+1], true
			}
			return "", true
		}
	}
	return "", false
}

// The prompter is minimal: no tools, no MCP, no slash commands; its inline rule replaces the system prompt.
func TestSessionArgsMinimal(t *testing.T) {
	t.Setenv("HOME", t.TempDir())
	r := residentRepl(t, 8)
	args := slotArgs(t, r, "prompter")
	if v, ok := argAfter(args, "--tools"); !ok || v != "" {
		t.Fatalf("--tools = %q (present %v), want empty", v, ok)
	}
	for _, f := range []string{"--strict-mcp-config", "--disable-slash-commands"} {
		if _, ok := argAfter(args, f); !ok {
			t.Fatalf("missing %s in %v", f, args)
		}
	}
	if v, ok := argAfter(args, "--setting-sources"); !ok || v != "" {
		t.Fatalf("--setting-sources = %q (present %v), want empty", v, ok)
	}
	if v, ok := argAfter(args, "--system-prompt"); !ok || v != prompterRule {
		t.Fatalf("--system-prompt = %q, want the prompter rule", v)
	}
	for _, bad := range []string{"--append-system-prompt", "--bare"} {
		if _, ok := argAfter(args, bad); ok {
			t.Fatalf("unexpected %s in %v", bad, args)
		}
	}
}

// The coder keeps its tool set and agent rules, drops the MCP servers it does not name and the slash commands.
func TestSessionArgsCoder(t *testing.T) {
	t.Setenv("HOME", t.TempDir())
	r := residentRepl(t, 8)
	dir := filepath.Join(r.a.Cwd, ".claude", "agents")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "haiku-coder.md"), []byte("---\nname: haiku-coder\n---\nCODER BODY\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	args := slotArgs(t, r, "coder")
	if v, ok := argAfter(args, "--tools"); !ok || v != "Read,Edit,Write,Bash,Grep,Glob" {
		t.Fatalf("--tools = %q, want Read,Edit,Write,Bash,Grep,Glob", v)
	}
	for _, f := range []string{"--strict-mcp-config", "--disable-slash-commands"} {
		if _, ok := argAfter(args, f); !ok {
			t.Fatalf("missing %s in %v", f, args)
		}
	}
	if v, ok := argAfter(args, "--append-system-prompt"); !ok || v != "CODER BODY" {
		t.Fatalf("--append-system-prompt = %q, want CODER BODY", v)
	}
	if _, ok := argAfter(args, "--system-prompt"); ok {
		t.Fatalf("coder must not replace the system prompt: %v", args)
	}
}

// The main child gets --strict-mcp-config with its resident MCP config, and no --disable-slash-commands:
// /compact, /usage and the other passthrough commands are sent to it (repl.go passthroughCmds).
func TestMainStrictMCP(t *testing.T) {
	t.Setenv("HOME", t.TempDir())
	var mainOpts []SessionOpts
	newSessionFn = func(o SessionOpts) (*Session, error) {
		if o.Model == "haiku" && containsArg(o.Extra, "--mcp-config") {
			mainOpts = append(mainOpts, o)
		}
		s, _, _ := fakeSession()
		return s, nil
	}
	t.Cleanup(func() { newSessionFn = NewSession })
	r := residentRepl(t, 8)
	r.a.Model = "haiku"
	if err := r.start(); err != nil {
		t.Fatal(err)
	}
	if len(mainOpts) != 1 {
		t.Fatalf("main spawned with --mcp-config %d times, want 1", len(mainOpts))
	}
	args := sessionArgs(mainOpts[0])
	if _, ok := argAfter(args, "--strict-mcp-config"); !ok {
		t.Fatalf("main missing --strict-mcp-config: %v", args)
	}
	if _, ok := argAfter(args, "--mcp-config"); !ok {
		t.Fatalf("main missing --mcp-config: %v", args)
	}
	if _, ok := argAfter(args, "--disable-slash-commands"); ok {
		t.Fatalf("main must keep slash commands: %v", args)
	}
}

func containsArg(args []string, flag string) bool {
	_, ok := argAfter(args, flag)
	return ok
}
