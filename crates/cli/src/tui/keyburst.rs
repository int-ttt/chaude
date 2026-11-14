// Enter inside a burst of keys is a newline, not a submit. A terminal without bracketed paste sends a pasted
// "a\nb" as plain key events, so the Enter in it has two quick keys right before it, or the next key queued.
// A lone Enter, typed by a person or committed with one IME key, keeps its submit meaning.
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Gap under which two key events count as one burst.
pub const BURST: Duration = Duration::from_millis(10);

#[derive(Default)]
pub struct Burst {
    last: Option<Instant>,
    // The previous key was itself within BURST of the key before it, so two quick keys end here.
    run: bool,
}

impl Burst {
    // `next_pending` says whether the next event is already queued. Only an Enter can change: a burst Enter
    // gets Alt, which the input reads as a newline (same as Alt+Enter typed by hand). An Enter right after a
    // single key is not a burst: that is an IME commit or a person, not a paste.
    pub fn filter(
        &mut self,
        mut key: KeyEvent,
        now: Instant,
        next_pending: impl FnOnce() -> bool,
    ) -> KeyEvent {
        let near = self
            .last
            .is_some_and(|t| now.saturating_duration_since(t) < BURST);
        if key.code == KeyCode::Enter && ((near && self.run) || next_pending()) {
            key.modifiers |= KeyModifiers::ALT;
        }
        self.run = near;
        self.last = Some(now);
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn no() -> bool {
        false
    }

    #[test]
    fn lone_enter_keeps_submit() {
        let mut b = Burst::default();
        let k = b.filter(press(KeyCode::Enter), Instant::now(), no);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn enter_after_two_quick_keys_is_a_burst() {
        let mut b = Burst::default();
        let t = Instant::now();
        b.filter(press(KeyCode::Char('a')), t, no);
        b.filter(press(KeyCode::Char('b')), t + Duration::from_millis(1), no);
        let k = b.filter(press(KeyCode::Enter), t + Duration::from_millis(2), no);
        assert!(k.modifiers.contains(KeyModifiers::ALT));
    }

    // macOS Korean IME: the commit of the last syllable and the Enter arrive within a millisecond of each
    // other, with one key before the Enter. That is one key, not a paste, so the Enter submits.
    #[test]
    fn enter_right_after_one_char_is_a_lone_enter() {
        let mut b = Burst::default();
        let t = Instant::now();
        b.filter(press(KeyCode::Char('한')), t, no);
        let k = b.filter(press(KeyCode::Enter), t + Duration::from_millis(1), no);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn enter_after_a_pause_is_a_lone_enter() {
        let mut b = Burst::default();
        let t = Instant::now();
        b.filter(press(KeyCode::Char('a')), t, no);
        let k = b.filter(press(KeyCode::Enter), t + Duration::from_millis(200), no);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn enter_with_next_key_already_pending_is_a_burst() {
        let mut b = Burst::default();
        let k = b.filter(press(KeyCode::Enter), Instant::now(), || true);
        assert!(k.modifiers.contains(KeyModifiers::ALT));
    }

    #[test]
    fn other_keys_are_never_changed() {
        let mut b = Burst::default();
        let t = Instant::now();
        let k = b.filter(press(KeyCode::Char('x')), t, || true);
        assert_eq!(k, press(KeyCode::Char('x')));
    }
}
