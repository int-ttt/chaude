//! Helpers for the chaude GUI (tao + wry window, see main.rs). Uses the chaude-core public API only.

pub mod bridge;
pub mod commands;
pub mod engine;
pub mod front;
pub mod guard;
pub mod input;
pub mod page;
pub mod state;

/// Kind of one stream-json line from the Claude CLI, as the shared core names it.
/// Returns None when the line is not valid JSON.
pub fn event_kind(line: &str) -> Option<&'static str> {
    chaude_core::event::parse_line(line)
        .ok()
        .map(|ev| chaude_core::event::type_name(&ev))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_type_is_unknown() {
        assert_eq!(event_kind(r#"{"type":"no_such_event"}"#), Some("unknown"));
    }

    #[test]
    fn invalid_json_is_none() {
        assert_eq!(event_kind("{not json"), None);
    }

    #[test]
    fn known_types_map_to_their_names() {
        assert_eq!(
            event_kind(r#"{"type":"rate_limit_event"}"#),
            Some("rate_limit_event")
        );
        assert_eq!(event_kind(r#"{"type":"result"}"#), Some("result"));
    }
}
