//! The input box's text, cursor and prompt history live in core `Editor`. egui's TextEdit draws the box and
//! handles IME composition and paste; what it changes is copied back into the editor as edits.

use std::path::Path;

use chaude_core::editor::Editor;

pub struct Input {
    editor: Editor,
}

impl Input {
    // Loads the recall list from <cwd>/.chaude/history.jsonl, as the TUI does at start.
    pub fn new(cwd: &Path) -> Self {
        let mut editor = Editor::with_east_asian(false);
        editor.load_history(cwd);
        Self { editor }
    }

    pub fn text(&self) -> String {
        self.editor.text()
    }

    pub fn cursor(&self) -> usize {
        self.editor.cursor()
    }

    pub fn is_single_line(&self) -> bool {
        !self.editor.text().contains('\n')
    }

    // Copies the text the widget now shows into the editor. The span that changed is replaced with the
    // editor's own delete and insert, then the cursor takes the widget's char index.
    pub fn apply_widget(&mut self, shown: &str, cursor: usize) {
        let old: Vec<char> = self.editor.text().chars().collect();
        let new: Vec<char> = shown.chars().collect();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let limit = old.len().min(new.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(limit)
            .take_while(|(a, b)| a == b)
            .count();
        let removed = old.len() - prefix - suffix;
        if old != new {
            self.editor.set_cursor(prefix);
            for _ in 0..removed {
                self.editor.delete();
            }
            let inserted: String = new[prefix..new.len() - suffix].iter().collect();
            self.editor.insert_str(&inserted);
        }
        self.editor.set_cursor(cursor);
    }

    // Enter with a trailing backslash becomes a newline (the TUI rule). Returns false when Enter should send.
    pub fn backslash_newline(&mut self) -> bool {
        self.editor.backslash_newline()
    }

    // Trimmed text for sending; the editor clears and records it in history. Blank text is left in place.
    pub fn submit(&mut self) -> Option<String> {
        if self.editor.text().trim().is_empty() {
            return None;
        }
        Some(self.editor.take())
    }

    pub fn history_prev(&mut self) {
        self.editor.history_prev();
    }

    pub fn history_next(&mut self) {
        self.editor.history_next();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // An input with no history file, so tests never read or write the working directory.
    fn plain() -> Input {
        Input {
            editor: Editor::with_east_asian(false),
        }
    }

    #[test]
    fn widget_text_and_cursor_land_in_the_core_editor() {
        let mut input = plain();
        input.apply_widget("ab", 2);
        assert_eq!(input.text(), "ab");
        assert_eq!(input.cursor(), 2);
        input.apply_widget("aXb", 2);
        assert_eq!(input.text(), "aXb");
        assert_eq!(input.cursor(), 2);
        assert_eq!(input.editor.cursor(), 2);
    }

    #[test]
    fn hangul_ime_preedit_is_replaced_by_the_committed_syllable() {
        let mut input = plain();
        input.apply_widget("안", 1);
        // Composing: the widget shows a jamo that the commit then replaces.
        input.apply_widget("안ㅕ", 2);
        assert_eq!(input.text(), "안ㅕ");
        input.apply_widget("안녕", 2);
        assert_eq!(input.text(), "안녕");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn shift_enter_newline_and_paste_are_plain_edits() {
        let mut input = plain();
        input.apply_widget("ab", 1);
        input.apply_widget("a\nb", 2);
        assert_eq!(input.text(), "a\nb");
        assert!(!input.is_single_line());
        input.apply_widget("a\nbc d", 6);
        assert_eq!(input.text(), "a\nbc d");
        assert_eq!(input.cursor(), 6);
    }

    #[test]
    fn trailing_backslash_enter_adds_a_newline_instead_of_sending() {
        let mut input = plain();
        input.apply_widget("a\\", 2);
        assert!(input.backslash_newline());
        assert_eq!(input.text(), "a\n");
        assert_eq!(input.submit().as_deref(), Some("a"));
        assert_eq!(input.submit(), None);
    }

    #[test]
    fn submitted_prompts_are_recalled_from_the_saved_history() {
        let dir = std::env::temp_dir().join(format!("chaude-gui-input-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut input = Input::new(&dir);
        input.apply_widget("  one  ", 7);
        assert_eq!(input.submit().as_deref(), Some("one"));
        input.apply_widget("two", 3);
        assert_eq!(input.submit().as_deref(), Some("two"));
        input.history_prev();
        assert_eq!(input.text(), "two");
        input.history_prev();
        assert_eq!(input.text(), "one");
        let mut again = Input::new(&dir);
        again.history_prev();
        assert_eq!(again.text(), "two");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
