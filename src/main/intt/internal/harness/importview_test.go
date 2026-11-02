package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// readClaudeTranscript must keep Turns text-only (snapshot input) and put the full transcript into View:
// thinking, tool calls and results, and the subagent transcript right after the Agent call that spawned it.
func TestReadClaudeTranscriptFullView(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sid1.jsonl")
	main := strings.Join([]string{
		`{"type":"user","message":{"role":"user","content":"make the thing"},"cwd":"/x","timestamp":"2026-01-01T00:00:00Z"}`,
		`{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"plan first"},{"type":"text","text":"Checking."},{"type":"tool_use","id":"toolu_bash","name":"Bash","input":{"command":"ls"}}]}}`,
		`{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_bash","content":"a.go\nb.go"}]}}`,
		`{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_agent","name":"Agent","input":{"description":"scan"}}]}}`,
		`{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_agent","content":[{"type":"text","text":"scan done"}]}]}}`,
		`{"type":"assistant","message":{"content":[{"type":"text","text":"All done."}]}}`,
		`{"type":"user","isMeta":true,"message":{"content":"skill body"}}`,
		`{"type":"user","message":{"content":"second prompt"}}`,
		`{"type":"assistant","message":{"content":[{"type":"text","text":"Second answer."}]}}`,
	}, "\n") + "\n"
	sub := strings.Join([]string{
		`{"type":"user","isSidechain":true,"uuid":"u1","message":{"role":"user","content":"find things"}}`,
		`{"type":"assistant","isSidechain":true,"uuid":"u2","message":{"content":[{"type":"tool_use","id":"toolu_grep","name":"Grep","input":{"pattern":"x"}}]}}`,
		`{"type":"user","isSidechain":true,"uuid":"u3","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_grep","content":"hit1"}]}}`,
		`{"type":"assistant","isSidechain":true,"uuid":"u4","message":{"content":[{"type":"text","text":"scan finished"}]}}`,
	}, "\n") + "\n"
	subDir := filepath.Join(dir, "sid1", "subagents")
	if err := os.MkdirAll(subDir, 0o755); err != nil {
		t.Fatal(err)
	}
	must := func(p, s string) {
		if err := os.WriteFile(p, []byte(s), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	must(path, main)
	must(filepath.Join(subDir, "agent-a1.jsonl"), sub)
	must(filepath.Join(subDir, "agent-a1.meta.json"), `{"agentType":"haiku-scout","toolUseId":"toolu_agent"}`)

	imp, err := readClaudeTranscript(path)
	if err != nil {
		t.Fatal(err)
	}

	// snapshot input: text only, no tool lines, no thinking
	if len(imp.Turns) != 2 || imp.Turns[0][0] != "make the thing" || imp.Turns[0][1] != "Checking.\nAll done." {
		t.Fatalf("Turns changed: %q", imp.Turns)
	}

	if len(imp.View) != 2 {
		t.Fatalf("View turns = %d, want 2: %q", len(imp.View), imp.View)
	}
	if imp.Subagents != 1 {
		t.Fatalf("Subagents = %d, want 1", imp.Subagents)
	}
	if imp.View[1][0] != "second prompt" || imp.View[1][1] != "Second answer." {
		t.Fatalf("second turn = %q", imp.View[1])
	}
	first := imp.View[0][1]
	if imp.View[0][0] != "make the thing" {
		t.Fatalf("first user = %q", imp.View[0][0])
	}
	// every item appears, in transcript order
	order := []string{
		"[thinking] plan first",
		"Checking.",
		"▸ Bash",
		"← a.go",
		"▸ Agent",
		"┆ task: find things",
		"┆ ▸ Grep",
		"┆ ← hit1",
		"┆ scan finished",
		"← scan done",
		"All done.",
	}
	at := -1
	for _, want := range order {
		i := strings.Index(first, want)
		if i < 0 {
			t.Fatalf("View missing %q in:\n%s", want, first)
		}
		if i < at {
			t.Fatalf("%q out of order in:\n%s", want, first)
		}
		at = i
	}
	if strings.Contains(imp.View[0][1]+imp.View[1][0], "skill body") {
		t.Fatal("meta (injected) user content leaked into View")
	}
}
