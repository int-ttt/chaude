// Stream-json events emitted by `claude -p --output-format stream-json`.
// Mirrors the event types handled by Go tree (v0.11.2) internal/harness (core.go, repl.go, livechild.go).
// Owned types only: no lifetimes. Unknown top-level types map to `Unknown`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    System(SystemEvent),
    StreamEvent(StreamEventWrap),
    Assistant(MessageEnvelope),
    User(MessageEnvelope),
    Result(ResultEvent),
    ControlRequest(ControlRequest),
    RateLimitEvent(RateLimitEvent),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct SystemEvent {
    pub subtype: Option<String>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    // Background task events (task_started, task_updated, task_notification). An owned task belongs to a running
    // agent: parent_task_id names that agent's task_id (not its tool_use_id); api.rs maps it via agent_task_ids.
    pub task_id: Option<String>,
    pub tool_use_id: Option<String>,
    pub task_type: Option<String>,
    pub status: Option<String>,
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub owned_by_subagent: bool,
    pub patch: Option<Value>,
    // task_notification: the agent's report ({"text": ...}) when it delivered one (handback_report).
    pub handback_report: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct StreamEventWrap {
    pub event: StreamInner,
    pub parent_tool_use_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamInner {
    MessageStart,
    MessageDelta,
    MessageStop,
    ContentBlockStart {
        content_block: Option<Value>,
    },
    ContentBlockDelta {
        delta: Delta,
    },
    ContentBlockStop,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Delta {
    TextDelta {
        text: String,
    },
    ThinkingDelta {
        thinking: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    SignatureDelta,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub struct MessageEnvelope {
    pub message: MessageBody,
    // Set on events produced inside a subagent (Agent/Task tool); None for the main conversation.
    pub parent_tool_use_id: Option<String>,
    // User events carrying a tool result: that tool's own result object (Agent/Task: totalTokens, totalDurationMs).
    #[serde(default)]
    pub tool_use_result: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct MessageBody {
    // Message id: a streamed reply repeats the same id and usage on each content block.
    pub id: Option<String>,
    pub content: Option<Content>,
    pub model: Option<String>,
    pub usage: Option<Usage>,
}

// A message body is either a plain string (user prompts) or a list of blocks.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Blocks(Vec<Block>),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Option<Value>,
    },
    ToolResult {
        tool_use_id: String,
        content: Option<Value>,
        is_error: Option<bool>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

impl Usage {
    // Tokens currently occupying the context window: fresh input plus both cache buckets.
    pub fn context_tokens(&self) -> u64 {
        self.input_tokens.unwrap_or(0)
            + self.cache_creation_input_tokens.unwrap_or(0)
            + self.cache_read_input_tokens.unwrap_or(0)
    }

    // Field-wise maximum: a streamed reply repeats its usage on each block, and the counts only grow.
    pub fn max_with(&mut self, o: &Usage) {
        self.input_tokens = self.input_tokens.max(o.input_tokens);
        self.cache_creation_input_tokens = self
            .cache_creation_input_tokens
            .max(o.cache_creation_input_tokens);
        self.cache_read_input_tokens = self.cache_read_input_tokens.max(o.cache_read_input_tokens);
        self.output_tokens = self.output_tokens.max(o.output_tokens);
    }

    // (input, output, cache) for the turn line: cache = cache read + cache creation. None when all are zero.
    pub fn turn_counts(&self) -> Option<(u64, u64, u64)> {
        let input = self.input_tokens.unwrap_or(0);
        let output = self.output_tokens.unwrap_or(0);
        let cache = self.cache_read_input_tokens.unwrap_or(0)
            + self.cache_creation_input_tokens.unwrap_or(0);
        (input + output + cache > 0).then_some((input, output, cache))
    }
}

// Per-agent figures from a Task/Agent tool result (its top-level tool_use_result). Either may be absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolStats {
    pub tokens: Option<u64>,
    pub duration_ms: Option<u64>,
}

// totalTokens, else the sum of the usage buckets; totalDurationMs. None when neither is present.
pub fn tool_stats(v: &Value) -> Option<ToolStats> {
    let num = |k: &str| v.get(k).and_then(Value::as_u64);
    let tokens = num("totalTokens").or_else(|| {
        let u = v.get("usage")?;
        let sum: u64 = [
            "input_tokens",
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
            "output_tokens",
        ]
        .iter()
        .filter_map(|k| u.get(*k).and_then(Value::as_u64))
        .sum();
        (sum > 0).then_some(sum)
    });
    let duration_ms = num("totalDurationMs");
    (tokens.is_some() || duration_ms.is_some()).then_some(ToolStats {
        tokens,
        duration_ms,
    })
}

#[derive(Debug, Deserialize)]
pub struct ResultEvent {
    pub subtype: Option<String>,
    pub is_error: Option<bool>,
    pub result: Option<String>,
    pub session_id: Option<String>,
    pub duration_ms: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub usage: Option<Usage>,
    // Go result["modelUsage"]: per-model usage of the turn, written to the turn row when non-empty.
    #[serde(rename = "modelUsage")]
    pub model_usage: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ControlRequest {
    pub request_id: String,
    pub request: ControlBody,
}

#[derive(Debug, Deserialize)]
pub struct ControlBody {
    pub subtype: String,
    pub tool_name: Option<String>,
    pub input: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitEvent {
    pub rate_limit_info: Option<Value>,
}

// Parse one stdout line. Errors are returned to the caller, which reports them as Error.
pub fn parse_line(line: &str) -> Result<Event, serde_json::Error> {
    serde_json::from_str(line)
}

// Top-level type name as the Go side sees it (used for counting and tests).
pub fn type_name(ev: &Event) -> &'static str {
    match ev {
        Event::System(_) => "system",
        Event::StreamEvent(_) => "stream_event",
        Event::Assistant(_) => "assistant",
        Event::User(_) => "user",
        Event::Result(_) => "result",
        Event::ControlRequest(_) => "control_request",
        Event::RateLimitEvent(_) => "rate_limit_event",
        Event::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real claude output captured for the Go harness (90 lines).
    const FIXTURE: &str = include_str!("../tests/fixtures/bg_turn_live.jsonl");

    #[test]
    fn fixture_every_line_parses() {
        let mut n = 0;
        for line in FIXTURE.lines().filter(|l| !l.trim().is_empty()) {
            parse_line(line).unwrap_or_else(|e| panic!("parse failed: {e}: {line}"));
            n += 1;
        }
        assert_eq!(n, 90);
    }

    #[test]
    fn fixture_top_level_types_covered() {
        let mut seen = std::collections::BTreeSet::new();
        for line in FIXTURE.lines().filter(|l| !l.trim().is_empty()) {
            let ev = parse_line(line).unwrap();
            seen.insert(type_name(&ev));
        }
        for want in [
            "system",
            "stream_event",
            "assistant",
            "user",
            "result",
            "rate_limit_event",
        ] {
            assert!(seen.contains(want), "missing {want}, saw {seen:?}");
        }
        assert!(
            !seen.contains("unknown"),
            "fixture should have no unknown top-level types"
        );
    }

    #[test]
    fn text_delta_is_main_stream() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hi"}},"session_id":"s","parent_tool_use_id":null}"#;
        match parse_line(line).unwrap() {
            Event::StreamEvent(w) => {
                assert!(w.parent_tool_use_id.is_none());
                match w.event {
                    StreamInner::ContentBlockDelta {
                        delta: Delta::TextDelta { text },
                    } => assert_eq!(text, "hi"),
                    other => panic!("wrong inner: {other:?}"),
                }
            }
            other => panic!("wrong event: {other:?}"),
        }
    }

    // From Go tree (v0.11.2) internal/harness (repl tests): assistant text + Bash tool_use.
    #[test]
    fn assistant_text_and_tool_use() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"plan first"},{"type":"text","text":"Checking."},{"type":"tool_use","id":"toolu_bash","name":"Bash","input":{"command":"ls"}}]}}"#;
        match parse_line(line).unwrap() {
            Event::Assistant(env) => match env.message.content {
                Some(Content::Blocks(bs)) => {
                    assert_eq!(bs.len(), 3);
                    assert!(matches!(bs[0], Block::Thinking { .. }));
                    assert!(matches!(&bs[1], Block::Text { text } if text == "Checking."));
                    match &bs[2] {
                        Block::ToolUse { name, input, .. } => {
                            assert_eq!(name, "Bash");
                            assert_eq!(input.as_ref().unwrap()["command"], "ls");
                        }
                        other => panic!("wrong block: {other:?}"),
                    }
                }
                other => panic!("wrong content: {other:?}"),
            },
            other => panic!("wrong event: {other:?}"),
        }
    }

    // Plain-string user prompt (the shape chaude writes on stdin, echoed back by claude).
    #[test]
    fn user_string_content() {
        let line = r#"{"type":"user","message":{"role":"user","content":"hello"}}"#;
        match parse_line(line).unwrap() {
            Event::User(env) => {
                assert!(matches!(env.message.content, Some(Content::Text(ref s)) if s == "hello"))
            }
            other => panic!("wrong event: {other:?}"),
        }
    }

    #[test]
    fn user_tool_result_blocks() {
        let line = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_bash","content":"a.txt","is_error":false}]}}"#;
        match parse_line(line).unwrap() {
            Event::User(env) => match env.message.content {
                Some(Content::Blocks(bs)) => assert!(
                    matches!(&bs[0], Block::ToolResult { tool_use_id, .. } if tool_use_id == "toolu_bash")
                ),
                other => panic!("wrong content: {other:?}"),
            },
            other => panic!("wrong event: {other:?}"),
        }
    }

    #[test]
    fn subagent_envelope_carries_parent_id() {
        let line =
            r#"{"type":"assistant","message":{"content":[]},"parent_tool_use_id":"toolu_agent"}"#;
        match parse_line(line).unwrap() {
            Event::Assistant(env) => {
                assert_eq!(env.parent_tool_use_id.as_deref(), Some("toolu_agent"))
            }
            other => panic!("wrong event: {other:?}"),
        }
    }

    #[test]
    fn usage_context_tokens_sums_input_and_cache() {
        let line = r#"{"type":"assistant","message":{"content":[],"usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000,"output_tokens":5}}}"#;
        match parse_line(line).unwrap() {
            Event::Assistant(env) => {
                let u = env.message.usage.expect("usage");
                assert_eq!(u.context_tokens(), 1110);
            }
            other => panic!("wrong event: {other:?}"),
        }
    }

    // From the Go permission tests: can_use_tool control_request.
    #[test]
    fn control_request_can_use_tool() {
        let line = r#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"ls"}}}"#;
        match parse_line(line).unwrap() {
            Event::ControlRequest(c) => {
                assert_eq!(c.request_id, "r1");
                assert_eq!(c.request.subtype, "can_use_tool");
                assert_eq!(c.request.tool_name.as_deref(), Some("Bash"));
            }
            other => panic!("wrong event: {other:?}"),
        }
    }

    #[test]
    fn result_event() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"result":"done","session_id":"s","duration_ms":12,"total_cost_usd":0.25}"#;
        match parse_line(line).unwrap() {
            Event::Result(r) => {
                assert_eq!(r.result.as_deref(), Some("done"));
                assert_eq!(r.duration_ms, Some(12));
                assert_eq!(r.total_cost_usd, Some(0.25));
            }
            other => panic!("wrong event: {other:?}"),
        }
    }

    #[test]
    fn unknown_top_level_type_is_tolerated() {
        let ev = parse_line(r#"{"type":"brand_new_thing","x":1}"#).unwrap();
        assert_eq!(type_name(&ev), "unknown");
    }

    #[test]
    fn unknown_inner_types_are_tolerated() {
        let line = r#"{"type":"stream_event","event":{"type":"ping"},"parent_tool_use_id":null}"#;
        assert!(parse_line(line).is_ok());
    }

    #[test]
    fn non_json_is_an_error() {
        assert!(parse_line("not json").is_err());
    }

    #[test]
    fn user_tool_use_result_is_read_and_optional() {
        let with = r#"{"type":"user","message":{"content":[]},"tool_use_result":{"totalTokens":1500,"totalDurationMs":45000}}"#;
        match parse_line(with).unwrap() {
            Event::User(env) => {
                let s = tool_stats(env.tool_use_result.as_ref().unwrap()).unwrap();
                assert_eq!(s.tokens, Some(1500));
                assert_eq!(s.duration_ms, Some(45000));
            }
            _ => panic!("not a user event"),
        }
        let without = r#"{"type":"user","message":{"content":[]}}"#;
        match parse_line(without).unwrap() {
            Event::User(env) => assert!(env.tool_use_result.is_none()),
            _ => panic!("not a user event"),
        }
    }

    #[test]
    fn tool_stats_falls_back_to_usage_sum_and_ignores_empty() {
        let usage = serde_json::json!({"usage": {"input_tokens": 100, "output_tokens": 50,
            "cache_read_input_tokens": 1000, "cache_creation_input_tokens": 0}});
        let s = tool_stats(&usage).unwrap();
        assert_eq!(s.tokens, Some(1150));
        assert_eq!(s.duration_ms, None);
        assert!(
            tool_stats(&serde_json::json!({"isAsync": true, "status": "async_launched"})).is_none()
        );
        assert!(tool_stats(&serde_json::json!({"usage": {}})).is_none());
    }

    #[test]
    fn turn_counts_splits_cache_and_hides_zero() {
        let u = Usage {
            input_tokens: Some(12),
            output_tokens: Some(3),
            cache_read_input_tokens: Some(80_000),
            cache_creation_input_tokens: Some(5),
        };
        assert_eq!(u.turn_counts(), Some((12, 3, 80_005)));
        assert_eq!(Usage::default().turn_counts(), None);
    }

    #[test]
    fn max_with_keeps_largest_per_field() {
        let mut a = Usage {
            input_tokens: Some(10),
            output_tokens: Some(1),
            ..Default::default()
        };
        a.max_with(&Usage {
            input_tokens: Some(10),
            output_tokens: Some(7),
            cache_read_input_tokens: Some(4),
            ..Default::default()
        });
        assert_eq!(a.input_tokens, Some(10));
        assert_eq!(a.output_tokens, Some(7));
        assert_eq!(a.cache_read_input_tokens, Some(4));
    }
}
