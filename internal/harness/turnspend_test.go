package harness

import (
	"reflect"
	"testing"
)

func subAssist(parent, msgID string, usage map[string]any) event {
	ev := event{"type": "assistant", "message": map[string]any{"id": msgID, "usage": usage}}
	if parent != "" {
		ev["parent_tool_use_id"] = parent
	}
	return ev
}

func agentUse(toolID, name, subType string) event {
	input := map[string]any{}
	if subType != "" {
		input["subagent_type"] = subType
	}
	return event{"type": "assistant", "parent_tool_use_id": nil, "message": map[string]any{"content": []any{
		map[string]any{"type": "tool_use", "id": toolID, "name": name, "input": input},
	}}}
}

func TestTurnSpendSubagentUsageMaxPerMessage(t *testing.T) {
	s := newTurnSpend()
	s.onEvent(subAssist("t1", "m1", map[string]any{"output_tokens": float64(5), "cache_creation_input_tokens": float64(100)}))
	s.onEvent(subAssist("t1", "m1", map[string]any{"output_tokens": float64(9), "cache_creation_input_tokens": float64(100)}))
	f := s.fields()
	if f["sub_msgs"] != 1 {
		t.Fatalf("sub_msgs = %v, want 1", f["sub_msgs"])
	}
	if f["sub_output_tokens"] != float64(9) {
		t.Fatalf("sub_output_tokens = %v, want 9", f["sub_output_tokens"])
	}
	if f["sub_cache_creation_input_tokens"] != float64(100) {
		t.Fatalf("sub_cache_creation_input_tokens = %v, want 100", f["sub_cache_creation_input_tokens"])
	}
}

func TestTurnSpendMainUsageIgnored(t *testing.T) {
	s := newTurnSpend()
	s.onEvent(subAssist("", "m1", map[string]any{"output_tokens": float64(7)}))
	for k := range s.fields() {
		if len(k) >= 4 && k[:4] == "sub_" {
			t.Fatalf("unexpected key %q from main event", k)
		}
	}
}

func TestTurnSpendSpawnCounting(t *testing.T) {
	s := newTurnSpend()
	s.onEvent(agentUse("a1", "Agent", "haiku-coder"))
	s.onEvent(agentUse("a1", "Agent", "haiku-coder")) // duplicate id, not counted again
	s.onEvent(agentUse("a2", "Task", "haiku-scout"))
	s.onEvent(agentUse("a3", "Agent", ""))
	f := s.fields()
	if f["spawns"] != 3 {
		t.Fatalf("spawns = %v, want 3", f["spawns"])
	}
	want := map[string]any{"haiku-coder": 1, "haiku-scout": 1, "general-purpose": 1}
	if !reflect.DeepEqual(f["spawn_types"], want) {
		t.Fatalf("spawn_types = %v, want %v", f["spawn_types"], want)
	}
}

func TestTurnSpendEmptyAndNil(t *testing.T) {
	if f := newTurnSpend().fields(); len(f) != 0 {
		t.Fatalf("empty turnSpend fields = %v, want empty", f)
	}
	var s *turnSpend
	if f := s.fields(); len(f) != 0 {
		t.Fatalf("nil turnSpend fields = %v, want empty", f)
	}
	s.onEvent(subAssist("t1", "m1", map[string]any{"output_tokens": float64(1)})) // must not panic
}
