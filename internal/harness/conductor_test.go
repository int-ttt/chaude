package harness

import "testing"

func TestParseRoute(t *testing.T) {
	m, task, ok := parseRoute("model: Haiku\ntask: count funcs in a.go\nreply: Korean 존댓말")
	if !ok || m != "haiku" || task != "count funcs in a.go\nreply: Korean 존댓말" {
		t.Fatal(m, task, ok)
	}
	for _, bad := range []string{"", "model: gpt\ntask: x", "model: opus", "hello"} {
		if _, _, ok := parseRoute(bad); ok {
			t.Errorf("accepted %q", bad)
		}
	}
}
