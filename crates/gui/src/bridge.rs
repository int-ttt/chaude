//! The webview bridge's wire format. The page posts a JSON `Command` through `window.ipc.postMessage`, and Rust answers
//! with a JSON `Event` run as a script call on `window.chaude.onEvent`. The shapes are core's (chaude_core::gui_bridge).

use chaude_core::gui_bridge::{Command, Event};

// Reads one page message. Err carries the parse failure text (unknown type or bad JSON).
pub fn parse(body: &str) -> Result<Command, String> {
    serde_json::from_str(body).map_err(|e| e.to_string())
}

// The script that delivers one event to the page.
pub fn script(ev: &Event) -> String {
    // serde_json cannot fail on these plain types.
    let json = serde_json::to_string(ev).unwrap_or_default();
    format!("window.chaude && window.chaude.onEvent({json});")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chaude_core::gui_bridge::{EntryView, ToolEnd};

    #[test]
    fn ping_from_page_parses() {
        assert_eq!(parse(r#"{"type":"ping"}"#), Ok(Command::Ping));
    }

    #[test]
    fn submit_from_page_parses() {
        assert_eq!(
            parse(r#"{"type":"submit","text":"hi"}"#),
            Ok(Command::Submit { text: "hi".into() })
        );
    }

    #[test]
    fn unknown_type_is_rejected() {
        assert!(parse(r#"{"type":"no_such_message"}"#).is_err());
    }

    #[test]
    fn invalid_json_is_rejected() {
        assert!(parse("{not json").is_err());
    }

    #[test]
    fn pong_script_calls_the_page_handler() {
        assert_eq!(
            script(&Event::Pong),
            r#"window.chaude && window.chaude.onEvent({"type":"pong"});"#
        );
    }

    #[test]
    fn entry_script_carries_the_event_json() {
        let ev = Event::EntryPush {
            index: 3,
            entry: EntryView::Tool {
                id: "t".into(),
                name: "Read".into(),
                summary: String::new(),
                ok: None,
                start: 1,
                end: Some(ToolEnd { at: 2, secs: 1 }),
            },
        };
        assert_eq!(
            script(&ev),
            r#"window.chaude && window.chaude.onEvent({"type":"entry_push","index":3,"entry":{"kind":"tool","id":"t","name":"Read","summary":"","ok":null,"start":1,"end":{"at":2,"secs":1}}});"#
        );
    }
}
