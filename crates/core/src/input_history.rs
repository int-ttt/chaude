//! Prompt history file shared by the TUI and the GUI: <cwd>/.chaude/history.jsonl, one JSON string per line.
//! Same rules as Go history.go: the newest MAX_HISTORY entries are kept and a locked append trims the file.
//! The in-memory recall list and its push rule (push_history) live in ui_state.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::file_lock::FileLock;
use crate::ui_state::{MAX_HISTORY, push_history};

// Go historyFileFor: <cwd>/.chaude/history.jsonl. An empty cwd means the process working directory.
pub fn history_file_for(cwd: &Path) -> Option<PathBuf> {
    let base = if cwd.as_os_str().is_empty() {
        std::env::current_dir().ok()?
    } else {
        cwd.to_path_buf()
    };
    Some(base.join(".chaude").join("history.jsonl"))
}

// Go loadInputHistory: one JSON string per line, oldest first. Lines that do not decode are skipped and the
// rest go through push_history. A missing file yields no entries.
pub fn read_history(path: &Path) -> Vec<String> {
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

// Appends one entry as one line (O_APPEND) while holding <path>.lock, the lock the trim rewrites under. Two TUIs in
// one project never overwrite each other, and a trim cannot drop a line appended during its read-rewrite.
// Only when the file has more than MAX_HISTORY lines is it trimmed.
pub fn append_history(path: &Path, s: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut line = encode_json_string(s);
    line.push('\n');
    let _lock = FileLock::acquire(path);
    let mut opts = fs::OpenOptions::new();
    opts.append(true).create(true);
    set_private(&mut opts);
    opts.open(path)?.write_all(line.as_bytes())?;
    if raw_line_count(path)? > MAX_HISTORY {
        trim_history(path)?;
    }
    Ok(())
}

fn raw_line_count(path: &Path) -> io::Result<usize> {
    Ok(fs::read(path)?.iter().filter(|&&b| b == b'\n').count())
}

// Trims <path> to the newest MAX_HISTORY entries. The caller holds <path>.lock (append_history).
fn trim_history(path: &Path) -> io::Result<()> {
    // Re-read the disk, so lines other TUIs appended after this one loaded are kept.
    let items = read_history(path);
    write_history(path, &items)
}

// Go saveInputHistory: writes every entry to <path>.tmp and renames it over <path>. Used only by the trim.
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

    // Unique per test run, so parallel tests do not share a history file.
    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chaude-history-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
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

    // Two TUIs in one project append at the same time: every line must survive.
    #[test]
    fn two_concurrent_writers_keep_every_line() {
        let dir = temp_dir("two-writers");
        let path = dir.join(".chaude").join("history.jsonl");
        let writers: Vec<_> = ["a", "b"]
            .into_iter()
            .map(|tag| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for i in 0..100 {
                        append_history(&path, &format!("{tag}{i}")).unwrap();
                    }
                })
            })
            .collect();
        for w in writers {
            w.join().unwrap();
        }
        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw.lines().count(), 200, "one line per append, none lost");
        let mut got: Vec<String> = read_history(&path);
        got.sort();
        let mut want: Vec<String> = (0..100)
            .flat_map(|i| [format!("a{i}"), format!("b{i}")])
            .collect();
        want.sort();
        assert_eq!(got, want);
        let _ = fs::remove_dir_all(&dir);
    }

    // Appends past MAX_HISTORY trim the file under the lock to the newest entries.
    #[test]
    fn appends_past_the_cap_trim_to_the_newest() {
        let dir = temp_dir("trim");
        let path = dir.join("history.jsonl");
        for i in 0..MAX_HISTORY + 5 {
            append_history(&path, &format!("e{i}")).unwrap();
        }
        assert_eq!(raw_line_count(&path).unwrap(), MAX_HISTORY);
        let got = read_history(&path);
        assert_eq!(got.first().map(String::as_str), Some("e5"));
        assert_eq!(
            got.last().map(String::as_str),
            Some(format!("e{}", MAX_HISTORY + 4).as_str())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // Two TUIs append while the file sits at the cap, so every append also trims. A trim's read-rewrite must not
    // drop a line another TUI appended meanwhile: all 200 new lines survive, and only prefill lines leave.
    #[test]
    fn appends_racing_a_trim_lose_no_appended_line() {
        let dir = temp_dir("trim-race");
        let path = dir.join(".chaude").join("history.jsonl");
        let prefill: Vec<String> = (0..MAX_HISTORY).map(|i| format!("pre{i}")).collect();
        write_history(&path, &prefill).unwrap();
        let writers: Vec<_> = ["a", "b"]
            .into_iter()
            .map(|tag| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for i in 0..100 {
                        append_history(&path, &format!("{tag}{i}")).unwrap();
                    }
                })
            })
            .collect();
        for w in writers {
            w.join().unwrap();
        }
        let got = read_history(&path);
        assert_eq!(got.len(), MAX_HISTORY);
        for tag in ["a", "b"] {
            for i in 0..100 {
                let line = format!("{tag}{i}");
                assert!(got.contains(&line), "{line} was lost");
            }
        }
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
