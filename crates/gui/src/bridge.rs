//! The webview bridge's message shapes. Plain JSON, tagged by "type": the page posts a `FromPage` string through
//! `window.ipc.postMessage`, and Rust answers with a `ToPage` run as a script call on `window.chaude.onEvent`.

use serde::{Deserialize, Serialize};

// A message from the page (JS -> Rust).
#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromPage {
    Ping,
}

// A message to the page (Rust -> JS).
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToPage {
    Pong,
}

// Reads one page message. Err carries the parse failure text (unknown type or bad JSON).
pub fn parse(body: &str) -> Result<FromPage, String> {
    serde_json::from_str(body).map_err(|e| e.to_string())
}

// The script that delivers one message to the page.
pub fn script(ev: &ToPage) -> String {
    // serde_json cannot fail on these plain enums.
    let json = serde_json::to_string(ev).unwrap_or_default();
    format!("window.chaude && window.chaude.onEvent({json});")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_from_page_parses() {
        assert_eq!(parse(r#"{"type":"ping"}"#), Ok(FromPage::Ping));
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
    fn pong_serializes_with_its_type() {
        assert_eq!(
            serde_json::to_string(&ToPage::Pong).unwrap(),
            r#"{"type":"pong"}"#
        );
    }

    #[test]
    fn pong_script_calls_the_page_handler() {
        assert_eq!(
            script(&ToPage::Pong),
            r#"window.chaude && window.chaude.onEvent({"type":"pong"});"#
        );
    }
}
