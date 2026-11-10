//! Full-screen /usage window (Go agentview.go usageLines/usageKey; tui.go usageBody, usageRoom, usageScroll).
//! While open it replaces the transcript viewport. The text is one `claude -p /usage` run (core, UiEvent::Usage).
//! Nothing is written to the transcript and the session is not touched.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use chaude_core::i18n::{Msg, t, tf};

use super::bar::clip;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON, wrap_cells};

// Go usageFetch: the one-shot run failed; the error text follows.
pub fn failed(err: &str) -> String {
    // Core reports a result-less answer as "usage: no result"; Go returns "" there, shown as the empty text.
    if err == "usage: no result" {
        return t(Msg::UsageEmpty).to_string();
    }
    tf(Msg::UsageFailed, &[&err])
}

#[derive(Default)]
pub struct Usage {
    // Some while the window is open (Go m.usage != "").
    text: Option<String>,
    // True from open until the one-shot answers (Go: the window shows LOADING).
    pending: bool,
    // First body line shown (Go usageTop).
    top: usize,
}

// Go usageBody: the text wrapped to the view width, one entry per screen row.
fn body(text: &str, width: usize) -> Vec<String> {
    text.split('\n')
        .flat_map(|l| {
            wrap_cells(l, width.max(10))
                .split('\n')
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

// Go usageRoom: body rows under the header line.
fn room(body_h: usize) -> usize {
    body_h.saturating_sub(1).max(1)
}

// Go usageScroll: keeps the last row able to reach the bottom of the view.
fn clamp(top: usize, len: usize, room: usize) -> usize {
    top.min(len.saturating_sub(room))
}

impl Usage {
    pub fn is_open(&self) -> bool {
        self.text.is_some()
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    // Go openUsage: fresh open at the top, loading text until the answer arrives.
    pub fn open(&mut self) {
        self.text = Some(t(Msg::UsageLoading).to_string());
        self.pending = true;
        self.top = 0;
    }

    // Go usageMsg: a fresh open starts at the top; a refresh keeps the scroll offset and clamps it.
    pub fn set_text(&mut self, text: &str, width: usize, body_h: usize) {
        if self.text.is_none() {
            self.top = 0;
        }
        self.text = Some(text.trim_end_matches('\n').to_string());
        self.pending = false;
        self.scroll_to(self.top as isize, width, body_h);
    }

    // Go usageFetch errors and the close path (m.usage = "").
    pub fn close(&mut self) {
        self.text = None;
        self.pending = false;
    }

    // Go usageScroll: sets the top body line, clamped.
    pub fn scroll_to(&mut self, top: isize, width: usize, body_h: usize) {
        let len = self.body_len(width);
        self.top = clamp(top.max(0) as usize, len, room(body_h));
    }

    // Wheel and key scrolling: relative to the current top.
    pub fn scroll_by(&mut self, delta: isize, width: usize, body_h: usize) {
        self.scroll_to(self.top as isize + delta, width, body_h);
    }

    fn body_len(&self, width: usize) -> usize {
        body(self.text.as_deref().unwrap_or(""), width).len()
    }

    // Go usageKey. Returns true when the window closed (the caller then returns to the tail, Go GotoBottom).
    pub fn key(&mut self, key: KeyEvent, width: usize, body_h: usize) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let room = room(body_h) as isize;
        match key.code {
            KeyCode::Esc | KeyCode::Enter => self.close(),
            KeyCode::Char('c') if ctrl => self.close(),
            KeyCode::Char('q') if key.modifiers.is_empty() => self.close(),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_by(-1, width, body_h),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_by(1, width, body_h),
            KeyCode::PageUp => self.scroll_by(-room, width, body_h),
            KeyCode::PageDown => self.scroll_by(room, width, body_h),
            KeyCode::Home => self.scroll_to(0, width, body_h),
            KeyCode::End => self.scroll_to(self.body_len(width) as isize, width, body_h),
            _ => {}
        }
        !self.is_open()
    }

    // Go usageLines: exactly body_h rows, a header line then the visible body slice.
    pub fn lines(&self, width: usize, body_h: usize) -> Vec<String> {
        let Some(text) = self.text.as_deref() else {
            return vec![String::new(); body_h];
        };
        let lines = body(text, width);
        let room = room(body_h);
        let top = clamp(self.top, lines.len(), room);
        let end = lines.len().min(top + room);
        let head = format!(
            "{ORANGE_ON}Usage{GREY_ON}  {}-{end}/{} · ↑/↓ pgup/pgdn scroll · esc/q close{FG_OFF}",
            (top + 1).min(end),
            lines.len()
        );
        let mut out = vec![clip(&head, width)];
        out.extend(lines[top..end].iter().cloned());
        out.resize(body_h, String::new());
        out.truncate(body_h);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{Action, App};
    use chaude_core::api::UiEvent;

    const W: usize = 80;
    const H: usize = 30;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    fn rows(n: usize) -> String {
        (0..n)
            .map(|i| format!("row{i:02}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // Port of usageview_test.go usageModel: 60 rows in a 30-row view.
    fn open_with_rows() -> Usage {
        let mut u = Usage::default();
        u.open();
        u.set_text(&rows(60), W, H - 4);
        u
    }

    // Port of usageview_test.go TestUsageViewScrollBounds.
    #[test]
    fn scroll_bounds_match_go() {
        let body_h = H - 4;
        let mut u = open_with_rows();
        let room = room(body_h);
        let max_top = u.body_len(W) - room;
        assert!(max_top > 0);

        u.key(key(KeyCode::PageDown, KeyModifiers::NONE), W, body_h);
        assert_eq!(u.top, room.min(max_top));
        u.key(key(KeyCode::End, KeyModifiers::NONE), W, body_h);
        assert_eq!(u.top, max_top);
        u.key(key(KeyCode::Down, KeyModifiers::NONE), W, body_h);
        assert_eq!(u.top, max_top);
        for _ in 0..3 {
            u.key(key(KeyCode::PageUp, KeyModifiers::NONE), W, body_h);
        }
        assert_eq!(u.top, 0);
        u.key(key(KeyCode::Up, KeyModifiers::NONE), W, body_h);
        assert_eq!(u.top, 0);

        u.key(key(KeyCode::End, KeyModifiers::NONE), W, body_h);
        let out = u.lines(W, H);
        assert_eq!(out.len(), H);
        let joined = out.join("\n");
        assert!(joined.contains("row59"));
        assert!(!joined.contains("row00"));
    }

    // Port of usageview_test.go TestUsageViewWheelAndRefreshKeepScroll.
    #[test]
    fn wheel_and_refresh_keep_scroll() {
        let body_h = H - 4;
        let mut u = open_with_rows();
        u.key(key(KeyCode::Down, KeyModifiers::NONE), W, body_h);
        u.scroll_by(5, W, body_h);
        let top = u.top;
        assert_ne!(top, 0);
        u.set_text(&rows(60), W, body_h);
        assert_eq!(u.top, top);
        u.set_text("only\none\ntwo", W, body_h);
        assert_eq!(u.top, 0);
    }

    // Port of usageview_test.go TestUsageViewEscCloses.
    #[test]
    fn esc_closes_and_returns_true() {
        let mut u = open_with_rows();
        assert!(u.key(key(KeyCode::Esc, KeyModifiers::NONE), W, H - 4));
        assert!(!u.is_open());
    }

    // Go usageKey: ctrl+c, q and enter close too; other keys keep it open.
    #[test]
    fn close_keys_match_go() {
        for k in [
            key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            key(KeyCode::Char('q'), KeyModifiers::NONE),
            key(KeyCode::Enter, KeyModifiers::NONE),
        ] {
            let mut u = open_with_rows();
            assert!(u.key(k, W, H - 4));
        }
        let mut u = open_with_rows();
        assert!(!u.key(key(KeyCode::Char('x'), KeyModifiers::NONE), W, H - 4));
        assert!(u.is_open());
    }

    // Go openUsage: the loading text shows at the top and a fresh open resets the offset.
    #[test]
    fn open_shows_loading_at_top() {
        let mut u = open_with_rows();
        u.scroll_by(5, W, H - 4);
        u.close();
        u.open();
        assert_eq!(u.text.as_deref(), Some(t(Msg::UsageLoading)));
        assert!(u.pending());
        assert_eq!(u.top, 0);
    }

    // Go usageMsg after a close: the answer reopens the window at the top (m.usage was "").
    #[test]
    fn answer_after_close_reopens_at_top() {
        let mut u = open_with_rows();
        u.scroll_by(5, W, H - 4);
        u.close();
        u.set_text("Current session: 45% used", W, H - 4);
        assert!(u.is_open());
        assert_eq!(u.top, 0);
    }

    #[test]
    fn lines_have_header_and_exact_height() {
        let mut u = Usage::default();
        u.open();
        u.set_text("a\nb", W, H - 4);
        let out = u.lines(W, 5);
        assert_eq!(out.len(), 5);
        assert!(
            out[0].contains("Usage") && out[0].contains("1-2/2"),
            "head: {}",
            out[0]
        );
        assert_eq!(out[1], "a");
        assert_eq!(out[2], "b");
        assert_eq!(out[3], "");
    }

    // Port of usageview_test.go TestUsageOpenNotInline, at the App level: /usage opens the window,
    // asks the engine and adds no transcript row besides the echoed prompt.
    #[test]
    fn usage_command_opens_window_without_transcript_block() {
        let mut app = App::new(W, H);
        for c in "/usage".chars() {
            app.on_key(key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        let before = app.transcript.len();
        let action = app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(action, Action::Usage));
        assert!(app.usage.is_open());
        assert_eq!(app.usage.text.as_deref(), Some(t(Msg::UsageLoading)));
        // Echo of the prompt and its blank row only.
        assert_eq!(app.transcript.len(), before + 2);
    }

    // The answer is decorated with the context bar (Go usageFetch -> decorateUsage) and kept off the transcript.
    #[test]
    fn usage_event_fills_window_and_error_goes_to_window_while_pending() {
        let mut app = App::new(W, H);
        app.usage.open();
        app.apply(UiEvent::Usage {
            text: "Current session: 45% used\n".to_string(),
        });
        let text = app.usage.text.clone().unwrap_or_default();
        assert!(
            text.starts_with("Current session: 45% used  [█████░░░░░] 45%"),
            "{text}"
        );
        assert!(text.ends_with("Context (now): [░░░░░░░░░░] 0%  (0 / 150,000)"));
        assert!(app.transcript.is_empty());

        app.usage.open();
        app.apply(UiEvent::Error("usage: bad json: EOF".to_string()));
        assert_eq!(
            app.usage.text.as_deref(),
            Some("/usage 조회에 실패했습니다: usage: bad json: EOF")
        );
        assert!(app.transcript.is_empty());

        // Result-less answer: Go shows the empty-result text, not a failure.
        app.usage.open();
        app.apply(UiEvent::Error("usage: no result".to_string()));
        assert_eq!(app.usage.text.as_deref(), Some(t(Msg::UsageEmpty)));
    }
}
