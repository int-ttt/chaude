package harness

// turnSpend accumulates subagent token usage and Agent/Task spawns seen in one turn's stream.
type turnSpend struct {
	sub   map[string]map[string]float64 // message.id -> usage field -> max value
	tools map[string]bool               // tool_use id already counted
	types map[string]int                // subagent_type -> spawns
}

func newTurnSpend() *turnSpend {
	return &turnSpend{sub: map[string]map[string]float64{}, tools: map[string]bool{}, types: map[string]int{}}
}

var spendKeys = []string{"input_tokens", "output_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"}

// onEvent records one stream event; nil receiver is a no-op.
func (s *turnSpend) onEvent(ev event) {
	if s == nil || str(ev["type"]) != "assistant" {
		return
	}
	m := asMap(ev["message"])
	if !isMainEvent(ev) {
		if id := str(m["id"]); id != "" {
			if u := asMap(m["usage"]); len(u) > 0 {
				row := s.sub[id]
				if row == nil {
					row = map[string]float64{}
					s.sub[id] = row
				}
				for _, k := range spendKeys {
					if v, ok := u[k].(float64); ok && v > row[k] {
						row[k] = v
					}
				}
			}
		}
	}
	for _, bi := range asList(m["content"]) {
		b := asMap(bi)
		if str(b["type"]) != "tool_use" {
			continue
		}
		if n := str(b["name"]); n != "Agent" && n != "Task" {
			continue
		}
		id := str(b["id"])
		if id == "" || s.tools[id] {
			continue
		}
		s.tools[id] = true
		t := str(asMap(b["input"])["subagent_type"])
		if t == "" {
			t = "general-purpose"
		}
		s.types[t]++
	}
}

// fields returns turn-row keys; empty map when nothing was seen.
func (s *turnSpend) fields() map[string]any {
	out := map[string]any{}
	if s == nil {
		return out
	}
	if len(s.sub) > 0 {
		out["sub_msgs"] = len(s.sub)
		for _, k := range spendKeys {
			var sum float64
			for _, row := range s.sub {
				sum += row[k]
			}
			out["sub_"+k] = sum
		}
	}
	if len(s.tools) > 0 {
		out["spawns"] = len(s.tools)
		types := map[string]any{}
		for k, v := range s.types {
			types[k] = v
		}
		out["spawn_types"] = types
	}
	return out
}
