//! Per-turn subagent spend and spawn counts (Go harness turnspend.go), and this turn's cost from the
//! cumulative total_cost_usd (Go harness usagepoll.go turnCostUSD). Pure state over raw stream JSON; the
//! caller keeps one `TurnSpend` per turn and one `TurnCost` per process.

use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

// Go spendKeys: the usage fields summed over subagent messages.
const SPEND_KEYS: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
];

// Go isMainEvent: no parent_tool_use_id, or null, or "".
fn is_main_event(ev: &Value) -> bool {
    match ev.get("parent_tool_use_id") {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.is_empty(),
        Some(_) => false,
    }
}

#[derive(Debug, Default)]
pub struct TurnSpend {
    // message.id -> usage field -> max value seen (a message streams several partial usages).
    sub: HashMap<String, HashMap<&'static str, f64>>,
    // tool_use ids already counted as spawns.
    tools: HashSet<String>,
    // subagent_type -> spawns ("general-purpose" when unset).
    types: HashMap<String, u64>,
}

impl TurnSpend {
    pub fn new() -> Self {
        Self::default()
    }

    // Records one stream event. Only "assistant" events count. Usage counts only from subagent events;
    // Agent/Task tool_use blocks count once per tool id.
    pub fn on_event(&mut self, ev: &Value) {
        if ev.get("type").and_then(Value::as_str) != Some("assistant") {
            return;
        }
        let empty = Map::new();
        let m = ev.get("message").and_then(Value::as_object).unwrap_or(&empty);
        let id = m.get("id").and_then(Value::as_str).unwrap_or("");
        let usage = m.get("usage").and_then(Value::as_object).filter(|u| !u.is_empty());
        if let Some(u) = usage.filter(|_| !is_main_event(ev) && !id.is_empty()) {
            let row = self.sub.entry(id.to_string()).or_default();
            for k in SPEND_KEYS {
                if let Some(v) = u.get(k).and_then(Value::as_f64) {
                    let slot = row.entry(k).or_insert(0.0);
                    if v > *slot {
                        *slot = v;
                    }
                }
            }
        }
        let blocks = m.get("content").and_then(Value::as_array);
        for b in blocks.into_iter().flatten() {
            if b.get("type").and_then(Value::as_str) != Some("tool_use") {
                continue;
            }
            let name = b.get("name").and_then(Value::as_str).unwrap_or("");
            if name != "Agent" && name != "Task" {
                continue;
            }
            let id = b.get("id").and_then(Value::as_str).unwrap_or("");
            if id.is_empty() || self.tools.contains(id) {
                continue;
            }
            self.tools.insert(id.to_string());
            let t = b
                .get("input")
                .and_then(|i| i.get("subagent_type"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let t = if t.is_empty() { "general-purpose" } else { t };
            *self.types.entry(t.to_string()).or_insert(0) += 1;
        }
    }

    // Turn-row fields (Go fields()). Empty object when nothing was seen.
    pub fn fields(&self) -> Value {
        let mut out = Map::new();
        if !self.sub.is_empty() {
            out.insert("sub_msgs".into(), json!(self.sub.len()));
            for k in SPEND_KEYS {
                let sum: f64 = self
                    .sub
                    .values()
                    .map(|row| row.get(k).copied().unwrap_or(0.0))
                    .sum();
                out.insert(format!("sub_{k}"), json!(sum));
            }
        }
        if !self.tools.is_empty() {
            out.insert("spawns".into(), json!(self.tools.len()));
            let types: Map<String, Value> = self
                .types
                .iter()
                .map(|(k, v)| (k.clone(), json!(v)))
                .collect();
            out.insert("spawn_types".into(), Value::Object(types));
        }
        Value::Object(out)
    }
}

// Go turnCostUSD state: the session and cumulative cost last seen by this process.
#[derive(Debug, Default)]
pub struct TurnCost {
    sess: String,
    last: f64,
}

impl TurnCost {
    pub fn new() -> Self {
        Self::default()
    }

    // This turn's cost from the cumulative total_cost_usd of `sess`. A new session, or a cumulative cost
    // that went down, starts again from 0. None when the cost is not a number (state unchanged).
    pub fn delta(&mut self, sess: &str, cost: Option<f64>) -> Option<f64> {
        let c = cost?;
        let mut prev = self.last;
        if sess != self.sess || c < prev {
            prev = 0.0;
        }
        self.sess = sess.to_string();
        self.last = c;
        Some(c - prev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub_assist(parent: Option<&str>, msg_id: &str, usage: Value) -> Value {
        let mut ev = json!({"type": "assistant", "message": {"id": msg_id, "usage": usage}});
        if let Some(p) = parent {
            ev["parent_tool_use_id"] = json!(p);
        }
        ev
    }

    fn agent_use(tool_id: &str, name: &str, sub_type: &str) -> Value {
        let mut input = json!({});
        if !sub_type.is_empty() {
            input["subagent_type"] = json!(sub_type);
        }
        json!({"type": "assistant", "parent_tool_use_id": null, "message": {"content": [
            {"type": "tool_use", "id": tool_id, "name": name, "input": input}
        ]}})
    }

    // Go TestTurnSpendSubagentUsageMaxPerMessage
    #[test]
    fn subagent_usage_takes_max_per_message() {
        let mut s = TurnSpend::new();
        s.on_event(&sub_assist(
            Some("t1"),
            "m1",
            json!({"output_tokens": 5.0, "cache_creation_input_tokens": 100.0}),
        ));
        s.on_event(&sub_assist(
            Some("t1"),
            "m1",
            json!({"output_tokens": 9.0, "cache_creation_input_tokens": 100.0}),
        ));
        let f = s.fields();
        assert_eq!(f["sub_msgs"], 1);
        assert_eq!(f["sub_output_tokens"].as_f64(), Some(9.0));
        assert_eq!(f["sub_cache_creation_input_tokens"].as_f64(), Some(100.0));
    }

    // Go TestTurnSpendMainUsageIgnored
    #[test]
    fn main_usage_is_ignored() {
        let mut s = TurnSpend::new();
        s.on_event(&sub_assist(None, "m1", json!({"output_tokens": 7.0})));
        let f = s.fields();
        assert!(f.as_object().unwrap().keys().all(|k| !k.starts_with("sub_")));
    }

    // Go TestTurnSpendSpawnCounting
    #[test]
    fn spawns_count_once_per_tool_id_with_type() {
        let mut s = TurnSpend::new();
        s.on_event(&agent_use("a1", "Agent", "haiku-coder"));
        s.on_event(&agent_use("a1", "Agent", "haiku-coder"));
        s.on_event(&agent_use("a2", "Task", "haiku-scout"));
        s.on_event(&agent_use("a3", "Agent", ""));
        let f = s.fields();
        assert_eq!(f["spawns"], 3);
        assert_eq!(
            f["spawn_types"],
            json!({"haiku-coder": 1, "haiku-scout": 1, "general-purpose": 1})
        );
    }

    // Go TestTurnSpendEmptyAndNil. The Rust side has no nil receiver; a fresh value is the empty case.
    #[test]
    fn empty_turn_spend_has_no_fields() {
        let mut s = TurnSpend::new();
        assert_eq!(s.fields(), json!({}));
        s.on_event(&sub_assist(Some("t1"), "m1", json!({"output_tokens": 1.0})));
        assert_eq!(s.fields()["sub_msgs"], 1);
    }

    // Go TestTurnCostUSDDelta
    #[test]
    fn turn_cost_delta_per_session() {
        let mut c = TurnCost::new();
        let check = |c: &mut TurnCost, sess: &str, cost: Option<f64>, want: f64| {
            let got = c.delta(sess, cost).expect("numeric cost");
            assert!((got - want).abs() < 1e-9, "delta({sess}) = {got}, want {want}");
        };
        check(&mut c, "a", Some(0.438), 0.438);
        check(&mut c, "a", Some(0.963), 0.525);
        check(&mut c, "b", Some(0.100), 0.100);
        check(&mut c, "b", Some(0.050), 0.050);
        assert_eq!(c.delta("b", None), None);
    }
}
