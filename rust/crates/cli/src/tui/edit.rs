// Single input line for the inline TUI. The buffer is a Vec<char>, so the cursor moves by whole characters:
// a Korean syllable is one char but two columns, and the cursor column is the sum of display widths.

use unicode_width::UnicodeWidthStr;

// A newline (Ctrl+J) is shown on the single input row as this marker.
const NEWLINE_MARK: &str = "⏎";

#[derive(Debug, Default)]
pub struct Editor {
    buf: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    // Some(i) while browsing history entry i; `draft` keeps the unsent text from before browsing started.
    browse: Option<usize>,
    draft: String,
}

impl Editor {
    pub fn text(&self) -> String {
        self.buf.iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn insert_char(&mut self, c: char) {
        if c == '\r' {
            return;
        }
        self.buf.insert(self.cursor, c);
        self.cursor += 1;
    }

    // Pasted or committed text. Carriage returns are dropped; newlines are kept as line breaks.
    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.insert_char(c);
        }
    }

    pub fn newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buf.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.buf.len() {
            self.buf.remove(self.cursor);
        }
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        if self.cursor < self.buf.len() {
            self.cursor += 1;
        }
    }

    // Start of the current logical line.
    pub fn home(&mut self) {
        while self.cursor > 0 && self.buf[self.cursor - 1] != '\n' {
            self.cursor -= 1;
        }
    }

    // End of the current logical line.
    pub fn end(&mut self) {
        while self.cursor < self.buf.len() && self.buf[self.cursor] != '\n' {
            self.cursor += 1;
        }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.cursor = 0;
        self.browse = None;
    }

    // Takes the text for submission and records it in history. Blank text is not recorded, and an
    // entry equal to the previous one is not recorded twice.
    pub fn take(&mut self) -> String {
        let text = self.text();
        if !text.trim().is_empty() && self.history.last() != Some(&text) {
            self.history.push(text.clone());
        }
        self.clear();
        self.draft.clear();
        text
    }

    pub fn history_prev(&mut self) {
        let next = match self.browse {
            None => {
                if self.history.is_empty() {
                    return;
                }
                self.draft = self.text();
                self.history.len() - 1
            }
            Some(0) => return,
            Some(i) => i - 1,
        };
        self.browse = Some(next);
        self.set_text(self.history[next].clone());
    }

    pub fn history_next(&mut self) {
        match self.browse {
            None => {}
            Some(i) if i + 1 < self.history.len() => {
                self.browse = Some(i + 1);
                self.set_text(self.history[i + 1].clone());
            }
            Some(_) => {
                self.browse = None;
                let draft = std::mem::take(&mut self.draft);
                self.set_text(draft);
            }
        }
    }

    fn set_text(&mut self, s: String) {
        self.buf = s.chars().collect();
        self.cursor = self.buf.len();
    }

    // The part of the line to draw in `width` columns, chosen so the cursor stays visible, and the
    // cursor's column inside that part. The cursor column is always below `width`.
    pub fn view(&self, width: usize) -> (String, usize) {
        let cells: Vec<(String, usize)> = self
            .buf
            .iter()
            .map(|&c| {
                let s = if c == '\n' {
                    NEWLINE_MARK.to_string()
                } else {
                    c.to_string()
                };
                let w = UnicodeWidthStr::width(s.as_str());
                (s, w)
            })
            .collect();
        // starts[i] is the column where cell i begins; starts[len] is the total width.
        let mut starts = Vec::with_capacity(cells.len() + 1);
        let mut col = 0;
        for (_, w) in &cells {
            starts.push(col);
            col += w;
        }
        starts.push(col);
        let cur = starts[self.cursor];
        let window_start = if col <= width {
            0
        } else {
            cur.saturating_sub(width.saturating_sub(1))
        };
        let first = starts
            .iter()
            .position(|&s| s >= window_start)
            .unwrap_or(cells.len());
        let base = starts[first];
        let mut out = String::new();
        let mut used = 0;
        for (s, w) in cells.iter().skip(first) {
            if used + w > width {
                break;
            }
            out.push_str(s);
            used += w;
        }
        (out, cur - base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> Editor {
        let mut e = Editor::default();
        e.insert_str(s);
        e
    }

    #[test]
    fn korean_input_is_utf8_and_cursor_counts_double_width() {
        let mut e = typed("한국어");
        assert_eq!(e.text(), "한국어");
        assert_eq!(e.view(20), ("한국어".to_string(), 6));
        e.left();
        assert_eq!(e.view(20).1, 4);
        e.insert_char('가');
        assert_eq!(e.text(), "한국가어");
        assert_eq!(e.view(20).1, 6);
    }

    #[test]
    fn backspace_and_delete_remove_whole_syllables() {
        let mut e = typed("가나다");
        e.backspace();
        assert_eq!(e.text(), "가나");
        e.left();
        e.delete();
        assert_eq!(e.text(), "가");
        e.backspace();
        e.backspace();
        assert!(e.is_empty());
    }

    #[test]
    fn newline_is_a_marker_column_and_home_end_stay_on_the_line() {
        let mut e = typed("ab");
        e.newline();
        e.insert_str("가");
        let marker = UnicodeWidthStr::width(NEWLINE_MARK);
        assert_eq!(e.view(40), (format!("ab{NEWLINE_MARK}가"), 2 + marker + 2));
        e.home();
        assert_eq!(e.view(40).1, 2 + marker);
        e.insert_char('X');
        assert_eq!(e.text(), "ab\nX가");
        e.end();
        assert_eq!(
            e.view(40).1,
            UnicodeWidthStr::width(format!("ab{NEWLINE_MARK}X가").as_str())
        );
    }

    #[test]
    fn long_line_scrolls_so_the_cursor_stays_inside_the_width() {
        let e = typed(&"가".repeat(20));
        let (view, cur) = e.view(10);
        assert!(UnicodeWidthStr::width(view.as_str()) <= 10, "{view}");
        assert!(cur < 10, "cursor {cur}");
        assert_eq!(cur, 8);
        let mut front = typed(&"가".repeat(20));
        for _ in 0..40 {
            front.left();
        }
        assert_eq!(front.view(10).1, 0);
    }

    #[test]
    fn history_walks_back_and_forth_and_restores_the_draft() {
        let mut e = typed("one");
        assert_eq!(e.take(), "one");
        e.insert_str("두");
        assert_eq!(e.take(), "두");
        e.insert_str("   ");
        assert_eq!(e.take(), "   ", "blank text is returned but not recorded");
        e.insert_str("cur");
        e.history_prev();
        assert_eq!(e.text(), "두");
        e.history_prev();
        assert_eq!(e.text(), "one");
        e.history_prev();
        assert_eq!(e.text(), "one", "stops at the oldest entry");
        e.history_next();
        assert_eq!(e.text(), "두");
        e.history_next();
        assert_eq!(e.text(), "cur", "leaving history restores the draft");
    }
}
