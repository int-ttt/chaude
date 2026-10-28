package harness

import (
	"testing"
	"time"
)

func TestInputHistory(t *testing.T) {
	var h inputHistory
	if _, ok := h.Prev("x"); ok {
		t.Fatal("empty history must not recall")
	}
	h.Add("a")
	h.Add("a") // dup
	h.Add("")
	h.Add("b")
	if len(h.items) != 2 {
		t.Fatalf("items=%v", h.items)
	}
	if s, _ := h.Prev("draft"); s != "b" {
		t.Fatal(s)
	}
	if s, _ := h.Prev("ignored"); s != "a" {
		t.Fatal(s)
	}
	if _, ok := h.Prev(""); ok {
		t.Fatal("must stop at oldest")
	}
	if s, _ := h.Next(); s != "b" {
		t.Fatal(s)
	}
	if s, ok := h.Next(); !ok || s != "draft" || h.Browsing() {
		t.Fatalf("draft restore: %q %v", s, ok)
	}
	if _, ok := h.Next(); ok {
		t.Fatal("next past end")
	}
	h.Prev("z")
	h.Add("c")
	if h.Browsing() || h.idx != 3 {
		t.Fatal("Add must reset browsing")
	}
}

func TestInputHistoryCap(t *testing.T) {
	var h inputHistory
	for i := 0; i < maxHistory+5; i++ {
		h.Add(string(rune('a'+i%20)) + string(rune('A'+i/20)))
	}
	if len(h.items) != maxHistory {
		t.Fatal(len(h.items))
	}
}

func TestRewindState(t *testing.T) {
	bl := []block{{"user", "/help"}, {"note", "n"}, {"user", "one"}, {"asst", "r1"}, {"user", "two"}, {"asst", "r2"}}
	var s rewindState
	if s.Open(nil) {
		t.Fatal("nothing to rewind")
	}
	if !s.Open(bl) || len(s.turns) != 2 || s.sel != 1 {
		t.Fatalf("%+v", s)
	}
	s.Move(-1)
	if s.sel != 0 {
		t.Fatal(s.sel)
	}
	s.Move(-1) // wraps
	if s.sel != 1 {
		t.Fatal(s.sel)
	}
	s.Move(-1)
	cut, text, ok := s.Confirm(bl)
	if !ok || cut != 2 || text != "one" || s.open {
		t.Fatalf("%d %q %v", cut, text, ok)
	}
	if _, _, ok := s.Confirm(bl); ok {
		t.Fatal("confirm after close")
	}
	s.Open(bl)
	s.Cancel()
	if s.open {
		t.Fatal("cancel")
	}
}

func TestAgentCounts(t *testing.T) {
	tr := newAgentTracker()
	now := time.Now()
	tr.onToolUse("a", "Agent", map[string]any{"description": "x"}, now)
	tr.onToolUse("b", "Agent", map[string]any{"description": "y", "run_in_background": true}, now)
	if fg, bg := tr.counts(); fg != 1 || bg != 1 {
		t.Fatal(fg, bg)
	}
	tr.onToolResult("a")
	if fg, bg := tr.counts(); fg != 0 || bg != 1 {
		t.Fatal(fg, bg)
	}
}
