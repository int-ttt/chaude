//! The page the window loads: web/index.html with its style, the vendored libraries and the app script inlined into
//! one string (no network, no build step). The vendored files keep their license banners.

const HTML: &str = include_str!("../web/index.html");
const STYLE: &str = include_str!("../web/style.css");
const APP: &str = include_str!("../web/app.js");
const MARKED: &str = include_str!("../web/vendor/marked.min.js");
const PURIFY: &str = include_str!("../web/vendor/purify.min.js");
const HIGHLIGHT: &str = include_str!("../web/vendor/highlight.min.js");

// The whole page, ready for wry's with_html.
pub fn html() -> String {
    let vendor: String = [MARKED, PURIFY, HIGHLIGHT]
        .iter()
        .map(|js| format!("<script>{js}</script>"))
        .collect();
    HTML.replace("<!--style-->", &format!("<style>{STYLE}</style>"))
        .replace("<!--vendor-->", &vendor)
        .replace("<!--app-->", &format!("<script>{APP}</script>"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge;
    use chaude_core::gui_bridge::{Command, EntryView, Event};

    #[test]
    fn index_html_embeds_style_vendor_and_app() {
        let page = html();
        assert!(page.starts_with("<!DOCTYPE html>"));
        for placeholder in ["<!--style-->", "<!--vendor-->", "<!--app-->"] {
            assert!(!page.contains(placeholder), "{placeholder} not replaced");
        }
        // The license banners of the vendored libraries show the embed worked.
        assert!(page.contains("marked v15.0.12"));
        assert!(page.contains("DOMPurify 3.2.6"));
        assert!(page.contains("Highlight.js v11.11.1"));
        assert!(page.contains("id=\"column\""));
        assert!(page.contains("id=\"jump-bottom\""));
        assert!(page.contains("window.ipc.postMessage"));
    }

    #[test]
    fn embedded_scripts_cannot_close_their_tag_early() {
        for js in [MARKED, PURIFY, HIGHLIGHT, APP] {
            assert!(!js.contains("</script"));
        }
    }

    #[test]
    fn page_commands_parse_in_rust() {
        // The shapes app.js posts; keep in step with its send() calls.
        assert_eq!(bridge::parse(r#"{"type":"ping"}"#), Ok(Command::Ping));
        assert_eq!(
            bridge::parse(r#"{"type":"submit","text":"hi"}"#),
            Ok(Command::Submit { text: "hi".into() })
        );
        assert_eq!(
            bridge::parse(r#"{"type":"interrupt"}"#),
            Ok(Command::Interrupt)
        );
        assert_eq!(
            bridge::parse(r#"{"type":"permission_answer","allow":false,"interrupt":false}"#),
            Ok(Command::PermissionAnswer {
                allow: false,
                interrupt: false
            })
        );
        assert_eq!(
            bridge::parse(r#"{"type":"copy","text":"x"}"#),
            Ok(Command::Copy { text: "x".into() })
        );
    }

    #[test]
    fn core_events_have_a_page_handler() {
        let events = [
            Event::Pong,
            Event::EntryPush {
                index: 0,
                entry: EntryView::Blank,
            },
            Event::EntryUpdate {
                index: 0,
                entry: EntryView::Blank,
            },
            Event::EntryCut { len: 0 },
            Event::PickerOpen {
                picker: "lang".into(),
            },
            Event::PickerClose {
                picker: "lang".into(),
            },
            Event::Note { text: "n".into() },
        ];
        for ev in &events {
            let handler = format!("{}(", ev.kind());
            assert!(APP.contains(&handler), "no page handler for {}", ev.kind());
        }
    }
}
