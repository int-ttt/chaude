// Enter inside a burst of keys is a newline, not a submit. A terminal without bracketed paste sends a pasted
// "a\nb" as plain key events, so the Enter in it has a key right before it or right after it, within a few ms.
// A lone Enter, typed by a person, has neither and keeps its submit meaning.
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Gap under which two key events count as one burst.
pub const BURST: Duration = Duration::from_millis(10);

#[derive(Default)]
pub struct Burst {
    last: Option<Instant>,
}

impl Burst {
    // `next_pending` says whether the next event is already queued. Only an Enter can change: a burst Enter
    // gets Alt, which the input reads as a newline (same as Alt+Enter typed by hand).
    pub fn filter(
        &mut self,
        mut key: KeyEvent,
        now: Instant,
        next_pending: impl FnOnce() -> bool,
    ) -> KeyEvent {
        let near = self
            .last
            .is_some_and(|t| now.saturating_duration_since(t) < BURST);
        self.last = Some(now);
        if key.code == KeyCode::Enter && (near || next_pending()) {
            key.modifiers |= KeyModifiers::ALT;
        }
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
    fn enter_right_after_a_char_is_a_burst() {
        let mut b = Burst::default();
        let t = Instant::now();
        b.filter(press(KeyCode::Char('a')), t, no);
        let k = b.filter(press(KeyCode::Enter), t + Duration::from_millis(2), no);
        assert!(k.modifiers.contains(KeyModifiers::ALT));
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
