//! Esc Esc rewind picker model, shared by the TUI (tui/rewind.rs keys and draw) and the GUI window.
//! Lists the real prompts of a transcript (slash commands excluded), newest selected first.
//! Confirm returns the cut index (blocks[..cut] are kept) and the prompt text to put back in the input.
//! Pure over `(kind, text)` pairs: the caller owns the transcript and applies the cut.

// I28: shown when Esc Esc finds no prompt to rewind to.
pub const NOTHING: &str = "nothing to rewind to";
// Go tui.go rewind confirm: note appended after the cut.
pub const REWOUND: &str = "rewound to this prompt (view + input only; the child session keeps its context. Use /clear or /rotate to reset it)";

// Go history.go rewindState.
#[derive(Debug, Default)]
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

    // The block indices of the listed prompts, oldest first.
    pub fn turns(&self) -> &[usize] {
        &self.turns
    }

    // The selected row (an index into turns).
    pub fn sel(&self) -> usize {
        self.sel
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

    // Selects row i (a click on a row); out-of-range rows are ignored.
    pub fn select(&mut self, i: usize) {
        if i < self.turns.len() {
            self.sel = i;
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
        assert_eq!(s.turns(), &[2, 4]);
        assert_eq!(s.sel(), 1);
        s.move_by(-1);
        assert_eq!(s.sel(), 0);
        s.move_by(-1); // wraps
        assert_eq!(s.sel(), 1);
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
    fn select_ignores_out_of_range_rows() {
        let b = bl();
        let mut s = Rewind::default();
        s.open(&b);
        s.select(0);
        assert_eq!(s.sel(), 0);
        s.select(5);
        assert_eq!(s.sel(), 0, "row 5 does not exist");
        assert_eq!(s.confirm(&b).map(|c| c.cut), Some(2));
    }
}
