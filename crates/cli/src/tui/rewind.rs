//! Esc Esc rewind picker (Go history.go rewindState; tui.go key() rewind branch; tui.go rewind draw).
//! Lists the real prompts of the transcript (slash commands excluded), newest selected first.
//! Confirm returns the cut index (blocks[..cut] are kept) and the prompt text to put back in the input.
//! Pure over `(kind, text)` pairs: the caller owns the transcript and applies the cut.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::bar::clip;
use super::render::{DIM_ON, FG_OFF, ORANGE_ON};

// I28: shown when Esc Esc finds no prompt to rewind to.
pub const NOTHING: &str = "nothing to rewind to";
// Go tui.go rewind confirm: note appended after the cut.
pub const REWOUND: &str = "rewound to this prompt (view + input only; the child session keeps its context. Use /clear or /rotate to reset it)";
// Go tui.go rewind draw: header line.
pub const HEADER: &str = "Rewind to a prompt  (↑/↓ select · enter rewind · esc cancel)";

// Go history.go rewindState.
#[derive(Default)]
pub struct Rewind {
    open: bool,
    turns: Vec<usize>, // indices into the blocks of "user" blocks that are real prompts
    sel: usize,
}

// Result of Confirm: keep blocks[..cut], put `text` back in the input.
#[derive(Debug, PartialEq)]
pub struct Cut {
    pub cut: usize,
    pub text: String,
}

impl Rewind {
    pub fn is_open(&self) -> bool {
        self.open
    }

    // Go Open: lists prompt turns (slash commands excluded). Returns false when there is nothing to rewind to.
    pub fn open(&mut self, blocks: &[(&str, &str)]) -> bool {
        self.turns.clear();
        self.sel = 0;
        for (i, (kind, text)) in blocks.iter().enumerate() {
            if *kind == "user" && !text.is_empty() && !text.starts_with('/') {
                self.turns.push(i);
            }
        }
        self.open = !self.turns.is_empty();
        self.sel = self.turns.len().saturating_sub(1);
        self.open
    }

    // Go Move: wraps around the list.
    pub fn move_by(&mut self, d: isize) {
        let n = self.turns.len() as isize;
        if n > 0 {
            self.sel = (self.sel as isize + d).rem_euclid(n) as usize;
        }
    }

    // Go Cancel.
    pub fn cancel(&mut self) {
        self.open = false;
    }

    // Go Confirm: closes the picker and returns the cut. None when closed or the selection is out of range.
    pub fn confirm(&mut self, blocks: &[(&str, &str)]) -> Option<Cut> {
        if !self.open || self.sel >= self.turns.len() {
            return None;
        }
        self.open = false;
        let cut = self.turns[self.sel];
        Some(Cut {
            cut,
            text: blocks[cut].1.to_string(),
        })
    }

    // Go tui.go key() rewind branch: up/k, down/j move; esc, ctrl+c, q cancel; enter confirms.
    // While open every key is consumed by the picker. Returns the cut only on enter.
    pub fn key(&mut self, key: KeyEvent, blocks: &[(&str, &str)]) -> Option<Cut> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::Esc => self.cancel(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.cancel(),
            KeyCode::Char('q') => self.cancel(),
            KeyCode::Enter => return self.confirm(blocks),
            _ => {}
        }
        None
    }

    // Go rewind draw: header, then up to `body_h - 1` prompt rows around the selection (newest last).
    // Row text is "%2d) <prompt>" with newlines folded and the prompt cut at 100 chars; the selected row is
    // orange with "> ", the others dim with "  ".
    pub fn lines(&self, blocks: &[(&str, &str)], width: usize, body_h: usize) -> Vec<String> {
        let mut out = vec![format!("{ORANGE_ON}{}{FG_OFF}", clip(HEADER, width))];
        let n = self.turns.len() as isize;
        let h = body_h as isize;
        let from = (self.sel as isize - h / 2).min(n - (h - 2)).max(0);
        let mut i = from;
        while i < n && out.len() + 1 < body_h {
            let idx = i as usize;
            let text: String = blocks[self.turns[idx]]
                .1
                .replace('\n', " ")
                .chars()
                .take(100)
                .collect();
            let row = format!("{:>2}) {}", idx + 1, text);
            let row = if idx == self.sel {
                format!(
                    "{ORANGE_ON}> {}{FG_OFF}",
                    clip(&row, width.saturating_sub(2))
                )
            } else {
                format!("{DIM_ON}  {}{FG_OFF}", clip(&row, width.saturating_sub(2)))
            };
            out.push(row);
            i += 1;
        }
        while out.len() < body_h {
            out.push(String::new());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bl() -> Vec<(&'static str, &'static str)> {
        vec![
            ("user", "/help"),
            ("note", "n"),
            ("user", "one"),
            ("asst", "r1"),
            ("user", "two"),
            ("asst", "r2"),
        ]
    }

    // Go TestRewindState (history_test.go:55).
    #[test]
    fn rewind_state_follows_go() {
        let b = bl();
        let mut s = Rewind::default();
        assert!(!s.open(&[]), "nothing to rewind");
        assert!(s.open(&b));
        assert_eq!(s.turns, vec![2, 4]);
        assert_eq!(s.sel, 1);
        s.move_by(-1);
        assert_eq!(s.sel, 0);
        s.move_by(-1); // wraps
        assert_eq!(s.sel, 1);
        s.move_by(-1);
        assert_eq!(
            s.confirm(&b),
            Some(Cut {
                cut: 2,
                text: "one".into()
            })
        );
        assert!(!s.is_open());
        assert_eq!(s.confirm(&b), None, "confirm after close");
        s.open(&b);
        s.cancel();
        assert!(!s.is_open(), "cancel");
    }

    #[test]
    fn empty_transcript_reports_nothing_to_rewind() {
        let mut s = Rewind::default();
        assert!(!s.open(&[("user", "/clear"), ("asst", "x")]));
        assert_eq!(NOTHING, "nothing to rewind to");
    }

    #[test]
    fn keys_move_confirm_and_cancel() {
        let b = bl();
        let mut s = Rewind::default();
        s.open(&b);
        let press = |c| KeyEvent::new(c, KeyModifiers::NONE);
        assert_eq!(s.key(press(KeyCode::Up), &b), None);
        assert_eq!(s.sel, 0);
        assert_eq!(s.key(press(KeyCode::Char('j')), &b), None);
        assert_eq!(s.sel, 1);
        let got = s.key(press(KeyCode::Enter), &b);
        assert_eq!(got.map(|c| c.cut), Some(4));
        s.open(&b);
        s.key(press(KeyCode::Esc), &b);
        assert!(!s.is_open());
        s.open(&b);
        s.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), &b);
        assert!(!s.is_open());
    }

    #[test]
    fn draw_fills_body_and_marks_selection() {
        let b = bl();
        let mut s = Rewind::default();
        s.open(&b);
        let rows = s.lines(&b, 80, 4);
        assert_eq!(rows.len(), 4);
        assert!(rows[0].contains(HEADER));
        assert!(rows[2].starts_with(ORANGE_ON));
        assert!(rows[2].contains(" 2) two"));
        assert!(rows[1].contains(" 1) one"));
    }
}
