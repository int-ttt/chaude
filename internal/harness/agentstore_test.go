package harness

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func newTestTracker(st *agentStore, parent string) *agentTracker {
	tk := newAgentTracker()
	tk.store = st
	tk.parent = func() string { return parent }
	tk.transcript = func() string { return "/t/" + parent + ".jsonl" }
	return tk
}

func TestAgentStorePersistsEveryChange(t *testing.T) {
	path := filepath.Join(t.TempDir(), "sub", "agents.json")
	st := newAgentStore(path)
	tk := newTestTracker(st, "sess1")
	tk.onToolUse("a", "Agent", map[string]any{"description": "d", "name": "w", "model": "haiku", "run_in_background": true}, time.Now())
	b, err := os.ReadFile(path)
	if err != nil || !strings.Contains(string(b), `"status": "running"`) || !strings.Contains(string(b), `"parent_session": "sess1"`) {
		t.Fatalf("not saved on start: %v %s", err, b)
	}
	tk.onSubEvent(event{"type": "assistant", "parent_tool_use_id": "a", "message": map[string]any{"content": []any{map[string]any{"type": "text", "text": "result line"}}}})
	tk.onNotification("<task-notification><tool-use-id>a</tool-use-id></task-notification>")
	got := newAgentStore(path).load("sess1")
	if len(got) != 1 || got[0].Status != stoDone || got[0].Summary != "result line" || got[0].Model != "haiku" ||
		got[0].Name != "w" || got[0].Transcript != "/t/sess1.jsonl" {
		t.Fatalf("after done: %+v", got)
	}
}

func TestAgentRestoreMarksInterrupted(t *testing.T) {
	path := filepath.Join(t.TempDir(), "agents.json")
	st := newAgentStore(path)
	tk := newTestTracker(st, "s1")
	tk.onToolUse("a", "Agent", map[string]any{"description": "bg", "run_in_background": true}, time.Now())
	tk.onToolUse("b", "Agent", map[string]any{"description": "fg"}, time.Now())
	tk.onToolResult("b")
	// shutdown: unfinished agents become interrupted, finished ones stay done
	tk.interruptRunning()
	st.interruptRunning(true)
	st2 := newAgentStore(path) // restart
	byID := map[string]savedAgent{}
	for _, a := range st2.load("s1") {
		byID[a.ID] = a
	}
	if byID["a"].Status != stoInterrupted || byID["b"].Status != stoDone {
		t.Fatalf("states: %+v", byID)
	}
	// startup restore (no parent): only interrupted ones
	r := &Repl{agents: st2, tracker: newAgentTracker()}
	r.restoreAgents("")
	if n := len(r.tracker.agents); n != 1 || !r.tracker.agents["a"].Interrupted || r.tracker.agents["a"].Done != true {
		t.Fatalf("restored: %+v", r.tracker.agents)
	}
	if len(r.tracker.running()) != 0 {
		t.Fatal("restored agent must not count as running")
	}
	if rs := r.tracker.sortedRestored(); len(rs) != 1 || agentState(rs[0]) != stInterr {
		t.Fatalf("panel rows: %v", rs)
	}
	r.tracker.dismissRestored()
	if len(r.tracker.sortedRestored()) != 0 || len(r.tracker.sortedAll()) != 1 {
		t.Fatal("dismissed agent leaves the panel but stays in the agent view")
	}
	// /resume restores everything of that parent session
	r2 := &Repl{agents: st2, tracker: newAgentTracker()}
	r2.restoreAgents("s1")
	if len(r2.tracker.agents) != 2 {
		t.Fatalf("resume restore: %+v", r2.tracker.agents)
	}
	// restored agents are not rewritten under another parent
	r2.tracker.store, r2.tracker.parent = st2, func() string { return "other" }
	r2.tracker.onToolUse("c", "Agent", map[string]any{"description": "n"}, time.Now())
	if len(newAgentStore(path).load("other")) != 1 {
		t.Fatal("restored agents re-saved under a new parent")
	}
}

func TestAgentStoreCrashRecoveryAndOwners(t *testing.T) {
	path := filepath.Join(t.TempDir(), "agents.json")
	st := newAgentStore(path)
	st.put([]savedAgent{{ID: "x", Status: stoRunning, Started: time.Now()}})
	// own process is alive: startup recovery leaves it; a dead owner's agent is interrupted
	st.interruptRunning(false)
	if st.load("") != nil {
		t.Fatalf("live owner's agent must stay running: %+v", st.load(""))
	}
	st.mu.Lock()
	a := st.m["x"]
	a.Owner = 1 << 30 // no such pid
	st.m["x"] = a
	st.mu.Unlock()
	st.interruptRunning(false)
	if l := st.load(""); len(l) != 1 || l[0].Status != stoInterrupted {
		t.Fatalf("crash recovery: %+v", l)
	}
}

func TestAgentStoreOffAndNil(t *testing.T) {
	var st *agentStore
	st.put([]savedAgent{{ID: "a"}})
	st.interruptRunning(true)
	if st.load("") != nil {
		t.Fatal("nil store")
	}
	off := newAgentStore("")
	off.put([]savedAgent{{ID: "a"}})
	if len(off.m) != 0 {
		t.Fatal("empty path must not store")
	}
}
