//! Helpers for the chaude GUI (eframe, see main.rs). Uses the chaude-core public API only.

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
}
