// Stream-json events emitted by `claude -p --output-format stream-json`.
// Mirrors the event types handled by deprecated/src/main/intt/internal/harness (core.go, repl.go, livechild.go).
// Owned types only: no lifetimes. Unknown top-level types map to `Unknown`.

use serde::Deserialize;
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
}

#[derive(Debug, Deserialize)]
pub struct MessageBody {
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

#[derive(Debug, Clone, Default, Deserialize)]
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
}

#[derive(Debug, Deserialize)]
pub struct ResultEvent {
    pub subtype: Option<String>,
    pub is_error: Option<bool>,
    pub result: Option<String>,
    pub duration_ms: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub usage: Option<Usage>,
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

    // From deprecated/src/main/intt/internal/harness (repl tests): assistant text + Bash tool_use.
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
}
