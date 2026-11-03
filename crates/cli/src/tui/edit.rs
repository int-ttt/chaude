// Single input line for the inline TUI. The buffer is a Vec<char>, so the cursor moves by whole characters:
// a Korean syllable is one char but two columns, and the cursor column is the sum of display widths.
// Editing rules follow the Go inputBox (deprecated/src/main/intt/internal/harness/input.go); prompt history
// follows history.go (cap, consecutive-duplicate rule, <cwd>/.chaude/history.jsonl).

// This is a binary crate, and mod.rs does not call every method here yet (delete_word, kill_to_start,
// kill_to_end, move_row, backslash_newline, load_history are waiting for key wiring in mod.rs).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use unicode_width::UnicodeWidthChar;

// history.go maxHistory
const MAX_HISTORY: usize = 500;

#[derive(Debug, Default)]
pub struct Editor {
    buf: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    // Some(i) while browsing history entry i; `draft` keeps the unsent text from before browsing started.
    browse: Option<usize>,
    draft: String,
    // Set by load_history; every submitted prompt is saved here.
    history_file: Option<PathBuf>,
}

impl Editor {
    pub fn text(&self) -> String {
        self.buf.iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    // Index into the buffer's chars where the cursor sits.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    // Go inputBox.Insert for one rune: tab becomes a space, a lone CR becomes a newline, other control
    // characters and DEL are dropped.
    pub fn insert_char(&mut self, c: char) {
        if let Some(c) = filter_char(c) {
            self.buf.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    // Pasted or committed text. A CR followed by LF is dropped; the other rules are those of insert_char.
    pub fn insert_str(&mut self, s: &str) {
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\r' && chars.peek() == Some(&'\n') {
                continue;
            }
            self.insert_char(c);
        }
    }

    pub fn newline(&mut self) {
        self.insert_char('\n');
    }

    // Go tui.go enter: a trailing "\" at the cursor is replaced by a newline instead of submitting.
    pub fn backslash_newline(&mut self) -> bool {
        if self.cursor == self.buf.len() && self.buf.last() == Some(&'\\') {
            self.backspace();
            self.newline();
            return true;
        }
        false
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

    // Go DeleteWord (opt+delete, ctrl+w): deletes the previous word; a newline before the cursor goes alone.
    pub fn delete_word(&mut self) {
        if self.cursor == 0 {
            return;
        }
        if self.buf[self.cursor - 1] == '\n' {
            self.backspace();
            return;
        }
        let mut i = self.cursor;
        while i > 0 && self.buf[i - 1] != '\n' && !is_word_char(self.buf[i - 1]) {
            i -= 1;
        }
        while i > 0 && self.buf[i - 1] != '\n' && is_word_char(self.buf[i - 1]) {
            i -= 1;
        }
        self.buf.drain(i..self.cursor);
        self.cursor = i;
    }

    // Go KillToStart (cmd+delete, ctrl+u): deletes from the cursor to the start of the logical line.
    pub fn kill_to_start(&mut self) {
        if self.cursor == 0 {
            return;
        }
        if self.buf[self.cursor - 1] == '\n' {
            self.backspace();
            return;
        }
        let start = self.line_start(self.cursor);
        self.buf.drain(start..self.cursor);
        self.cursor = start;
    }

    // Go KillToEnd (ctrl+k): deletes from the cursor to the end of the logical line.
    pub fn kill_to_end(&mut self) {
        let end = self.line_end(self.cursor);
        self.buf.drain(self.cursor..end);
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
        self.cursor = self.line_start(self.cursor);
    }

    // End of the current logical line.
    pub fn end(&mut self) {
        self.cursor = self.line_end(self.cursor);
    }

    // Go inputBox.MoveRow over logical lines (this row does not soft-wrap): moves to the line above (-1)
    // or below (+1) and keeps the cell column. Reports false when there is no such line.
    pub fn move_row(&mut self, d: i32) -> bool {
        let ls = self.line_start(self.cursor);
        let col = cells(&self.buf[ls..self.cursor]);
        let (start, end) = if d < 0 {
            if ls == 0 {
                return false;
            }
            let end = ls - 1;
            (self.line_start(end), end)
        } else {
            let end = self.line_end(self.cursor);
            if end == self.buf.len() {
                return false;
            }
            (end + 1, self.line_end(end + 1))
        };
        let mut p = start;
        let mut c = 0;
        while p < end && c + cell_width(self.buf[p]) <= col {
            c += cell_width(self.buf[p]);
            p += 1;
        }
        self.cursor = p;
        true
    }

    fn line_start(&self, mut i: usize) -> usize {
        while i > 0 && self.buf[i - 1] != '\n' {
            i -= 1;
        }
        i
    }

    fn line_end(&self, mut i: usize) -> usize {
        while i < self.buf.len() && self.buf[i] != '\n' {
            i += 1;
        }
        i
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.cursor = 0;
        self.browse = None;
    }

    // Takes the submitted text: trimmed, like the Go submit line. Blank text returns "" and is not recorded.
    // Other text is recorded in history (and saved when a history file is loaded), skipping an entry equal
    // to the previous one.
    pub fn take(&mut self) -> String {
        let text = self.text().trim().to_string();
        if !text.is_empty() {
            push_history(&mut self.history, text.clone());
            if let Some(path) = &self.history_file {
                // Best effort, like Go recordHistory: recall still works in memory.
                let _ = write_history(path, &self.history);
            }
        }
        self.clear();
        self.draft.clear();
        text
    }

    // Go loadHistory: remembers <cwd>/.chaude/history.jsonl for saving and restores the recall list from it.
    pub fn load_history(&mut self, cwd: &Path) {
        self.history_file = history_file_for(cwd);
        self.history = match &self.history_file {
            Some(path) => read_history(path),
            None => Vec::new(),
        };
        self.browse = None;
        self.draft.clear();
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
}

// Go inputBox.Insert, per rune. None means the rune is dropped.
fn filter_char(c: char) -> Option<char> {
    match c {
        '\n' | '\r' => Some('\n'),
        '\t' => Some(' '),
        c if c >= ' ' && c != '\x7f' => Some(c),
        _ => None,
    }
}

// Go isWordRune: letters (Hangul included), digits and '_'.
fn is_word_char(c: char) -> bool {
    c.is_alphabetic() || c.is_numeric() || c == '_'
}

fn cell_width(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

fn cells(cs: &[char]) -> usize {
    cs.iter().map(|&c| cell_width(c)).sum()
}

// Go historyFileFor: <cwd>/.chaude/history.jsonl. An empty cwd means the process working directory.
pub fn history_file_for(cwd: &Path) -> Option<PathBuf> {
    let base = if cwd.as_os_str().is_empty() {
        std::env::current_dir().ok()?
    } else {
        cwd.to_path_buf()
    };
    Some(base.join(".chaude").join("history.jsonl"))
}

// Go inputHistory.Add: blanks and a repeat of the last entry are skipped; only the newest MAX_HISTORY are kept.
fn push_history(items: &mut Vec<String>, s: String) {
    if !s.is_empty() && items.last() != Some(&s) {
        items.push(s);
        if items.len() > MAX_HISTORY {
            items.drain(..items.len() - MAX_HISTORY);
        }
    }
}

// Go loadInputHistory: one JSON string per line, oldest first. Lines that do not decode are skipped and the
// rest go through push_history. A missing file yields no entries.
fn read_history(path: &Path) -> Vec<String> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        if let Some(s) = decode_json_string(line.trim()) {
            push_history(&mut items, s);
        }
    }
    items
}

// Go saveInputHistory: writes every entry to <path>.tmp and renames it over <path>.
fn write_history(path: &Path, items: &[String]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut body = String::new();
    for s in items {
        body.push_str(&encode_json_string(s));
        body.push('\n');
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    set_private(&mut opts);
    opts.open(&tmp)?.write_all(body.as_bytes())?;
    fs::rename(&tmp, path)
}

// Go writes the history file with mode 0600.
#[cfg(unix)]
fn set_private(opts: &mut fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    opts.mode(0o600);
}

#[cfg(not(unix))]
fn set_private(_opts: &mut fs::OpenOptions) {}

// JSON string literal for one history entry: quotes, backslashes and control characters are escaped.
fn encode_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// The string held by one trimmed line, or None unless the whole line is a single valid JSON string.
// Unpaired surrogates decode to U+FFFD, as in Go.
fn decode_json_string(line: &str) -> Option<String> {
    let inner = line.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        match c {
            '"' => return None,
            c if (c as u32) < 0x20 => return None,
            '\\' => match it.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let hi = hex4(&mut it)?;
                    if (0xD800..0xDC00).contains(&hi) {
                        let mut ahead = it.clone();
                        if ahead.next() == Some('\\') && ahead.next() == Some('u') {
                            let lo = hex4(&mut ahead)?;
                            if (0xDC00..0xE000).contains(&lo) {
                                it = ahead;
                                let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                                out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                                continue;
                            }
                        }
                        out.push('\u{FFFD}');
                    } else {
                        out.push(char::from_u32(hi).unwrap_or('\u{FFFD}'));
                    }
                }
                _ => return None,
            },
            c => out.push(c),
        }
    }
    Some(out)
}

fn hex4(it: &mut impl Iterator<Item = char>) -> Option<u32> {
    let mut v = 0;
    for _ in 0..4 {
        v = v * 16 + it.next()?.to_digit(16)?;
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> Editor {
        let mut e = Editor::default();
        e.insert_str(s);
        e
    }

    // Unique per test run, so parallel tests do not share a history file.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chaude-edit-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
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

    // Go insert rules (input.go:40-62): CR LF collapses, lone CR is a newline, tab is a space,
    // other control characters and DEL are dropped, C1 controls are kept.
    #[test]
    fn insert_filters_control_characters_and_converts_tab_and_cr() {
        let mut e = Editor::default();
        e.insert_str("a\u{1}b\u{7f}\tc\u{85}");
        assert_eq!(e.text(), "ab c\u{85}");
        let mut e = Editor::default();
        e.insert_char('\r');
        e.insert_char('\t');
        e.insert_char('\u{1b}');
        assert_eq!(e.text(), "\n ");
    }

    #[test]
    fn space_key_inserts_a_space() {
        let mut e = typed("a");
        e.insert_char(' ');
        assert_eq!(e.text(), "a ");
    }

    // Go TestInputNewline (input_test.go:52), adapted to the single-row view.
    #[test]
    fn newline_is_a_marker_column_and_home_end_stay_on_the_line() {
        let mut e = Editor::default();
        e.insert_str("안녕\r\nhi\n");
        assert_eq!(e.text(), "안녕\nhi\n");
        e.backspace();
        assert_eq!(e.text(), "안녕\nhi");
        e.insert_char('\n');
        e.home();
        assert_eq!(e.cursor, 6, "home on an empty line after the newline");
        e.end();
        assert_eq!(e.text(), "안녕\nhi\n");
        assert_eq!(e.cursor, 6);
    }

    #[test]
    fn paste_keeps_newlines_and_drops_cr_before_lf() {
        let mut e = Editor::default();
        e.insert_str("a\r\nb\rc\td");
        assert_eq!(e.text(), "a\nb\nc d");
        assert_eq!(e.take(), "a\nb\nc d", "a paste is not submitted by itself");
    }

    // Go TestInputKillToStart (input_test.go:72)
    #[test]
    fn kill_to_start_cases() {
        let cases = [
            ("hello world", "world", 6), // mid-line: removes "hello "
            ("ab\ncd", "ab\n", 5),       // end of 2nd line: removes "cd"
            ("ab\ncd", "abcd", 3),       // line start: joins lines
            ("abc", "abc", 0),           // pos 0: no-op
        ];
        for (input, want, pos) in cases {
            let mut e = typed(input);
            e.cursor = pos;
            e.kill_to_start();
            assert_eq!(e.text(), want, "in {input:?} pos {pos}");
        }
    }

    #[test]
    fn kill_to_end_stays_on_the_logical_line() {
        let mut e = typed("ab\ncd");
        e.cursor = 1;
        e.kill_to_end();
        assert_eq!(e.text(), "a\ncd");
        let mut e = typed("ab\ncd");
        e.cursor = 2;
        e.kill_to_end();
        assert_eq!(e.text(), "ab\ncd", "at a line end nothing is removed");
    }

    // Go TestInputDeleteWordPunct (input_test.go:91)
    #[test]
    fn delete_word_is_punctuation_aware() {
        let cases = [
            ("hello, world", "hello, "),
            ("hello, ", ""),
            ("a.b.c", "a.b."),
            ("안녕 하세요", "안녕 "),
            ("x\nfoo bar", "x\nfoo "),
            ("x\n", "x"),
        ];
        for (input, want) in cases {
            let mut e = typed(input);
            e.delete_word();
            assert_eq!(e.text(), want, "in {input:?}");
        }
    }

    // Go TestTUIAltBackspaceAndCtrlU (input_test.go:109), editor part: alt+backspace then ctrl+u on one line.
    #[test]
    fn alt_backspace_then_ctrl_u_on_a_single_line() {
        let mut e = typed("hello, world");
        e.delete_word();
        assert_eq!(e.text(), "hello, ");
        e.kill_to_start();
        assert_eq!(e.text(), "");
    }

    #[test]
    fn move_row_keeps_the_cell_column_across_logical_lines() {
        // "ab\ncde", cursor after 'e' (cell column 3 on the second line)
        let mut e = typed("ab\ncde");
        assert!(e.move_row(-1));
        assert_eq!(
            e.cursor, 2,
            "column 3 is past the end of 'ab', so the cursor lands at its end"
        );
        assert!(e.move_row(1));
        assert_eq!(e.cursor, 5, "column 2 on 'cde' is after 'd'");
        assert!(!e.move_row(1), "no line below");
    }

    #[test]
    fn move_row_reports_false_at_the_first_line() {
        // cursor lands after the wide char 가 (cell column 2), so 'b' at column 2 is not reached
        let mut e = typed("가b\nab");
        assert!(e.move_row(-1));
        assert_eq!(e.cursor, 1);
        let mut first = typed("ab");
        assert!(!first.move_row(-1));
    }

    #[test]
    fn backslash_enter_turns_a_trailing_backslash_into_a_newline() {
        let mut e = typed("a\\");
        assert!(e.backslash_newline());
        assert_eq!(e.text(), "a\n");
        let mut e = typed("a\\b");
        assert!(!e.backslash_newline());
        assert_eq!(e.text(), "a\\b");
    }

    #[test]
    fn history_walks_back_and_forth_and_restores_the_draft() {
        let mut e = typed("one");
        assert_eq!(e.take(), "one");
        e.insert_str("두");
        assert_eq!(e.take(), "두");
        e.insert_str("   ");
        assert_eq!(e.take(), "", "blank text is not returned or recorded");
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

    // Go TestHistoryDedupeConsecutive (history_file_test.go:127), with the file part.
    #[test]
    fn history_dedupes_consecutive_entries_across_restart() {
        let dir = temp_dir("dedupe");
        let mut e = Editor::default();
        e.load_history(&dir);
        for s in ["p", "p", "q", "p"] {
            e.insert_str(s);
            e.take();
        }
        assert_eq!(e.history, ["p", "q", "p"]);
        let mut e2 = Editor::default();
        e2.load_history(&dir);
        e2.insert_str("p");
        e2.take();
        assert_eq!(
            e2.history,
            ["p", "q", "p"],
            "last loaded entry is p: skipped across restart"
        );
        assert_eq!(
            read_history(&history_file_for(&dir).unwrap()),
            ["p", "q", "p"]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // Go TestRecordHistoryPersistsAndNewModelLoads (history_file_test.go:54)
    #[test]
    fn recorded_history_persists_and_a_new_editor_loads_it() {
        let dir = temp_dir("persist");
        let mut e = Editor::default();
        e.load_history(&dir);
        assert_eq!(
            e.history_file.as_deref(),
            Some(dir.join(".chaude").join("history.jsonl").as_path())
        );
        e.insert_str("first run");
        e.take();
        e.insert_str("first run");
        e.take();
        let mut e2 = Editor::default();
        e2.load_history(&dir);
        assert_eq!(e2.history, ["first run"]);
        e2.history_prev();
        assert_eq!(e2.text(), "first run", "Up recall after restart");
        let _ = fs::remove_dir_all(&dir);
    }

    // Go TestHistoryFileFor (history_file_test.go:48)
    #[test]
    fn history_file_path_is_under_chaude() {
        assert_eq!(
            history_file_for(Path::new("/proj")),
            Some(Path::new("/proj").join(".chaude").join("history.jsonl"))
        );
    }

    // Go TestHistoryFileRoundTrip (history_file_test.go:12)
    #[test]
    fn history_file_round_trip() {
        let dir = temp_dir("roundtrip");
        let path = dir.join(".chaude").join("history.jsonl");
        assert!(read_history(&path).is_empty(), "missing file");
        let input: Vec<String> = ["one", "two\nlines", "/usage", "한글", "q\"\\\t"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        write_history(&path, &input).unwrap();
        assert_eq!(read_history(&path), input);
        let _ = fs::remove_dir_all(&dir);
    }

    // Go TestHistoryFileLoadRules (history_file_test.go:26)
    #[test]
    fn history_file_load_rules_skip_bad_lines_and_cap() {
        let dir = temp_dir("rules");
        let path = dir.join("history.jsonl");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "\"a\"\nnot json\n\"a\"\n\"\"\n\"b\"\n").unwrap();
        assert_eq!(read_history(&path), ["a", "b"]);
        let many: Vec<String> = (0..MAX_HISTORY + 7)
            .map(|i| {
                format!(
                    "{}{}",
                    (b'a' + (i % 26) as u8) as char,
                    (b'A' + (i / 26) as u8) as char
                )
            })
            .collect();
        write_history(&path, &many).unwrap();
        let got = read_history(&path);
        assert_eq!(got.len(), MAX_HISTORY);
        assert_eq!(got.last(), many.last());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_string_escapes_decode_like_go() {
        assert_eq!(decode_json_string(r#""aA😀\/""#), Some("aA😀/".to_string()));
        assert_eq!(
            decode_json_string(r#""\ud800x""#),
            Some("\u{FFFD}x".to_string())
        );
        assert_eq!(
            decode_json_string("\"a\tb\""),
            None,
            "raw control characters are invalid"
        );
        assert_eq!(decode_json_string(r#""a" x"#), None);
        assert_eq!(
            decode_json_string(r#""a\""#),
            None,
            "trailing backslash escapes the quote"
        );
        assert_eq!(decode_json_string("not json"), None);
        assert_eq!(
            decode_json_string(&encode_json_string("q\"\\\n\u{1}가")),
            Some("q\"\\\n\u{1}가".to_string())
        );
    }
}
