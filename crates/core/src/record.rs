//! Record files (Go record.go): every turn is saved as markdown blocks, each under a
//! `<!-- rec #NNNN KIND ISO cat=TOPIC -->` header, in `<record dir>/NNNNNN-HHMMSS-NNNN.md` files.
//! Local-time conversion stays with the caller: it passes a `Stamp`.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MAX_BYTES: usize = 200 << 10; // a new file starts once a turn would exceed this
pub const MAX_ANSWERS: usize = 100; // answers (asst blocks) per file
const LOCK_NAME: &str = ".record.lock";
const TAIL_BYTES: u64 = 8 << 10; // bytes read from the end of a file to find its last rec number
const HEADER_OPEN: &str = "<!-- rec #";

// One saved output block of a turn. Kind: user | asst | note | tool | result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub kind: String,
    pub text: String,
}

// One block parsed back from a record file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub num: u64,
    pub kind: String,
    pub ts: String,
    pub topic: String,
    pub text: String,
}

// One record file with the rec number of its last turn (0 when it has none).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordFile {
    pub name: String,
    pub last: u64,
}

// Local wall-clock time in the three layouts Go uses: file name "060102-150405", heading "15:04",
// header "2006-01-02T15:04".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub file: String,
    pub hm: String,
    pub iso: String,
}

impl Stamp {
    pub fn new(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> Self {
        Stamp {
            file: format!("{:02}{mo:02}{d:02}-{h:02}{mi:02}{s:02}", y.rem_euclid(100)),
            hm: format!("{h:02}:{mi:02}"),
            iso: format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}"),
        }
    }
}

// Go recordDir: HARNESS_RECORD_DIR if set, else <main checkout root>/record (the same root for every
// stacked worktree), else <cwd>/record.
pub fn dir_for(cwd: &Path) -> PathBuf {
    if let Some(d) = std::env::var_os("HARNESS_RECORD_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(d);
    }
    let out = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(cwd)
        .output();
    if let Ok(out) = out
        && out.status.success()
    {
        let gc = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !gc.is_empty() {
            let gc = if Path::new(&gc).is_absolute() {
                PathBuf::from(&gc)
            } else {
                cwd.join(&gc)
            };
            if let Some(root) = gc.parent() {
                return root.join("record");
            }
        }
    }
    cwd.join("record")
}

// Go recordCat: whitespace collapsed, and "-->" cannot end the comment.
pub fn cat_label(topic: &str) -> String {
    topic
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("-->", "->")
}

// Go slugify: whitespace, control characters and /:*?<>|"\ become '-'; at most 30 chars; empty is "misc".
pub fn slugify(topic: &str) -> String {
    let s: String = topic
        .trim()
        .chars()
        .take(30)
        .map(|r| {
            if r.is_whitespace() || r.is_control() || "/:*?<>|\"\\".contains(r) {
                '-'
            } else {
                r
            }
        })
        .collect();
    if s.is_empty() { "misc".to_string() } else { s }
}

// Go recordRender: per block, a header comment, a heading, a blank line, the text and a blank line.
// Text loses its trailing newlines so parse_entries returns it unchanged.
pub fn render(num: u64, ts: &Stamp, topic: &str, turn: &[Block]) -> String {
    let cat = cat_label(topic);
    let mut out = String::new();
    for bl in turn {
        out += &format!("<!-- rec #{num:04} {} {} cat={cat} -->\n", bl.kind, ts.iso);
        out += &format!("## #{num:04} · {} · {cat}\n\n", ts.hm);
        out += &escape_lines(bl.text.trim_end_matches('\n'));
        out += "\n\n";
    }
    out
}

// Go recordEscapeRe: a body line that looks like a header gets one more backslash.
fn escape_lines(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| {
            if line.trim_start_matches('\\').starts_with(HEADER_OPEN) {
                format!("\\{line}")
            } else {
                line.to_string()
            }
        })
        .collect()
}

// Go recordUnescapeRe: the inverse of escape_lines, one backslash less when read back.
fn unescape_lines(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| match line.strip_prefix('\\') {
            Some(rest) if rest.trim_start_matches('\\').starts_with(HEADER_OPEN) => {
                rest.to_string()
            }
            _ => line.to_string(),
        })
        .collect()
}

struct Header<'a> {
    num: u64,
    kind: &'a str,
    ts: &'a str,
    topic: &'a str,
}

// Go recordHeaderRe for one line (without its newline): `<!-- rec #(\d+) (\w+) (\S+) cat=(.*) -->`.
fn parse_header(line: &str) -> Option<Header<'_>> {
    let rest = line.strip_prefix(HEADER_OPEN)?;
    let (num, rest) = rest.split_once(' ')?;
    if num.is_empty() || !num.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (kind, rest) = rest.split_once(' ')?;
    if kind.is_empty() || !kind.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    let (ts, rest) = rest.split_once(' ')?;
    if ts.is_empty() || ts.chars().any(char::is_whitespace) {
        return None;
    }
    let topic = rest.strip_prefix("cat=")?.strip_suffix(" -->")?;
    Some(Header {
        num: num.parse().ok()?,
        kind,
        ts,
        topic,
    })
}

// Header lines of data with their byte range (the header line includes its newline). The last line
// must end in '\n' to count, as in the Go regex.
fn headers(data: &str) -> Vec<(usize, usize, Header<'_>)> {
    let mut out = Vec::new();
    let mut pos = 0;
    for line in data.split_inclusive('\n') {
        let start = pos;
        pos += line.len();
        if let Some(body) = line.strip_suffix('\n')
            && let Some(h) = parse_header(body)
        {
            out.push((start, pos, h));
        }
    }
    out
}

// Go parseRecordEntries: blocks read back from record file content, in file order.
pub fn parse_entries(data: &str) -> Vec<Entry> {
    let hs = headers(data);
    hs.iter()
        .enumerate()
        .map(|(i, (_, body_start, h))| {
            let end = hs.get(i + 1).map_or(data.len(), |next| next.0);
            let seg = &data[*body_start..end];
            // drop the heading line, then the blank line after it, and the blank line before the next header
            let seg = seg.find('\n').map_or(seg, |j| &seg[j + 1..]);
            let seg = seg.strip_prefix('\n').unwrap_or(seg);
            let seg = seg.strip_suffix("\n\n").unwrap_or(seg);
            Entry {
                num: h.num,
                kind: h.kind.to_string(),
                ts: h.ts.to_string(),
                topic: h.topic.to_string(),
                text: unescape_lines(seg),
            }
        })
        .collect()
}

// Go category.go catRe for one line: a line ending in `[cat: X]` or `[cat-switch: X]`, with optional
// spaces, `*`, `>` or backticks around it. Returns (is cat-switch, X).
fn cat_mark_span(line: &str) -> Option<(usize, bool, &str)> {
    let t = line.trim_end_matches(|c: char| c.is_whitespace() || c == '*' || c == '`');
    let body = t.strip_suffix(']')?;
    for (p, _) in body.match_indices('[') {
        let inner = &body[p + 1..];
        if inner.contains(']') {
            continue; // an earlier bracket group; the mark starts at a later '['
        }
        let before = body[..p].trim_end_matches(['*', '>', '`']);
        if !(before.is_empty() || before.ends_with(' ') || before.ends_with('\t')) {
            continue;
        }
        // the regex consumes the one space/tab before the marker, or starts at line start
        let start = before.len().saturating_sub(1);
        if let Some(label) = inner.strip_prefix("cat-switch:") {
            return Some((start, true, label.trim()));
        }
        if let Some(label) = inner.strip_prefix("cat:") {
            return Some((start, false, label.trim()));
        }
    }
    None
}

fn cat_mark(line: &str) -> Option<(bool, &str)> {
    cat_mark_span(line).map(|(_, switch, label)| (switch, label))
}

// Go catRe.ReplaceAllString(reply, ""): each line loses its trailing [cat: X] / [cat-switch: X] mark.
pub fn strip_cat_marks(reply: &str) -> String {
    reply
        .split('\n')
        .map(|line| match cat_mark_span(line) {
            Some((start, _, _)) => &line[..start],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Go recordTopicOf: the last [cat-switch: X], else the last [cat: X], else prev.
pub fn topic_of(reply: &str, prev: &str) -> String {
    let (mut switch, mut cat) = (String::new(), String::new());
    for line in reply.lines() {
        match cat_mark(line) {
            Some((_, "")) | None => {}
            Some((true, label)) => switch = label.to_string(),
            Some((false, label)) => cat = label.to_string(),
        }
    }
    if !switch.is_empty() {
        switch
    } else if !cat.is_empty() {
        cat
    } else {
        prev.to_string()
    }
}

// Go lastNumIn: rec number of the last header in data, 0 if none.
fn last_num_in(data: &str) -> u64 {
    headers(data).last().map_or(0, |h| h.2.num)
}

// Go lastRecordNum: reads only the file tail unless the last turn is longer than the tail.
pub fn last_record_num(path: &Path) -> io::Result<u64> {
    let mut f = fs::File::open(path)?;
    let size = f.metadata()?.len();
    let off = size.saturating_sub(TAIL_BYTES);
    f.seek(SeekFrom::Start(off))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let mut data = String::from_utf8_lossy(&buf).into_owned();
    if off > 0 {
        // drop the partial first line
        data = data
            .find('\n')
            .map_or(String::new(), |i| data[i + 1..].to_string());
    }
    let n = last_num_in(&data);
    if n > 0 || off == 0 {
        return Ok(n);
    }
    Ok(last_num_in(&String::from_utf8_lossy(&fs::read(path)?)))
}

// Go listRecordFiles: the *.md files in dir, oldest rec number first (ties by name). A missing dir is empty.
pub fn list_files(dir: &Path) -> io::Result<Vec<RecordFile>> {
    let ents = match fs::read_dir(dir) {
        Ok(ents) => ents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for ent in ents {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if ent.file_type()?.is_dir() || !name.ends_with(".md") {
            continue;
        }
        let last = last_record_num(&ent.path())?;
        out.push(RecordFile { name, last });
    }
    out.sort_by(|a, b| a.last.cmp(&b.last).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

// Go nextRecordNumber: max last rec number over all files, plus one (1 for an empty dir).
pub fn next_number(dir: &Path) -> io::Result<u64> {
    let max = list_files(dir)?.iter().map(|f| f.last).max().unwrap_or(0);
    Ok(max + 1)
}

// Go recordAppendTurn: appends one turn (all blocks share one rec number) with a single append write
// under an exclusive lock on <dir>/.record.lock. The turn goes to the newest file unless split is set
// (a /clear) or that file is at a cap; then a new file named by stamp starts. A turn larger than the
// cap stays in one file.
pub fn append_turn(
    dir: &Path,
    stamp: &Stamp,
    topic: &str,
    turn: &[Block],
    split: bool,
) -> io::Result<()> {
    if turn.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(dir.join(LOCK_NAME))?;
    lock.lock()?; // released when the handle closes

    let files = list_files(dir)?;
    let num = files.iter().map(|f| f.last + 1).max().unwrap_or(1);
    // newest = the first file with the highest rec number, as in Go
    let newest = files
        .iter()
        .fold(None::<&RecordFile>, |best, f| match best {
            Some(b) if f.last <= b.last => Some(b),
            _ => Some(f),
        });
    let payload = render(num, stamp, topic, turn);
    let answers = turn.iter().filter(|b| b.kind == "asst").count();

    let mut name = format!("{}-{num:04}.md", stamp.file);
    if let Some(newest) = newest
        && !split
    {
        let data = fs::read_to_string(dir.join(&newest.name))?;
        let have = parse_entries(&data)
            .iter()
            .filter(|e| e.kind == "asst")
            .count();
        if data.len() + payload.len() <= MAX_BYTES && have + answers <= MAX_ANSWERS {
            name = newest.name.clone();
        }
    }

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(name))?;
    f.write_all(payload.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn stamp() -> Stamp {
        Stamp::new(2026, 11, 4, 9, 5, 7)
    }

    fn blk(kind: &str, text: &str) -> Block {
        Block {
            kind: kind.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn stamp_layouts_match_go() {
        let s = stamp();
        assert_eq!(s.file, "261104-090507");
        assert_eq!(s.hm, "09:05");
        assert_eq!(s.iso, "2026-11-04T09:05");
    }

    #[test]
    fn render_then_parse_round_trips_blocks() {
        let turn = [blk("user", "hi\n"), blk("asst", "line one\n\nline two\n")];
        let out = render(7, &stamp(), "rename  -->  x", &turn);
        assert!(out.starts_with(
            "<!-- rec #0007 user 2026-11-04T09:05 cat=rename -> x -->\n## #0007 · 09:05 · rename -> x\n\n"
        ));
        let got = parse_entries(&out);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].num, 7);
        assert_eq!(got[0].kind, "user");
        assert_eq!(got[0].ts, "2026-11-04T09:05");
        assert_eq!(got[0].topic, "rename -> x");
        assert_eq!(got[0].text, "hi");
        assert_eq!(got[1].kind, "asst");
        assert_eq!(got[1].text, "line one\n\nline two");
    }

    #[test]
    fn body_line_that_looks_like_a_header_survives() {
        let tricky = "<!-- rec #0009 asst x cat=y -->\n\\<!-- rec #1 -->";
        let out = render(1, &stamp(), "t", &[blk("asst", tricky)]);
        let got = parse_entries(&out);
        assert_eq!(got.len(), 1, "escaped body must not start a block");
        assert_eq!(got[0].text, tricky);
    }

    #[test]
    fn header_needs_its_newline_and_exact_shape() {
        assert!(parse_entries("<!-- rec #0001 asst t cat=x -->").is_empty());
        assert!(parse_entries("<!-- rec #x asst t cat=x -->\n").is_empty());
        assert_eq!(
            parse_entries("<!-- rec #0001 asst t cat=x -->\n")[0].topic,
            "x"
        );
    }

    #[test]
    fn topic_prefers_last_switch_then_last_cat_then_prev() {
        assert_eq!(topic_of("done [cat: a]", "p"), "a");
        assert_eq!(topic_of("[cat: a]\nmore\n[cat: b]", "p"), "b");
        assert_eq!(topic_of("[cat: a]\n[cat-switch: s]\n[cat: c]", "p"), "s");
        assert_eq!(topic_of("**[cat: bold]**", "p"), "bold");
        assert_eq!(topic_of("x [cat: a] [cat: b]", "p"), "b");
        assert_eq!(topic_of("[cat: ]", "p"), "p");
        assert_eq!(topic_of("no mark", "p"), "p");
        assert_eq!(
            topic_of("word[cat: no]", "p"),
            "p",
            "needs a space or line start before the mark"
        );
    }

    #[test]
    fn slugify_keeps_korean_and_bounds_length() {
        assert_eq!(slugify("작업 a/b:c"), "작업-a-b-c");
        assert_eq!(slugify("   "), "misc");
        assert_eq!(slugify(&"가".repeat(40)).chars().count(), 30);
    }

    #[test]
    fn append_fills_one_file_then_splits_on_request() {
        let tmp = TempDir::new("record-append");
        let dir = tmp.path().join("record");
        let turn = [blk("user", "q"), blk("asst", "a")];
        append_turn(&dir, &stamp(), "t", &turn, false).unwrap();
        assert_eq!(next_number(&dir).unwrap(), 2);
        let second = Stamp::new(2026, 11, 4, 9, 6, 0);
        append_turn(&dir, &second, "t", &turn, false).unwrap();
        let files = list_files(&dir).unwrap();
        assert_eq!(files.len(), 1, "same turn file while under caps");
        assert_eq!(files[0].last, 2);
        append_turn(&dir, &second, "t", &turn, true).unwrap();
        assert_eq!(
            list_files(&dir).unwrap().len(),
            2,
            "split starts a new file"
        );
        assert_eq!(next_number(&dir).unwrap(), 4);
        let all: Vec<Entry> = list_files(&dir)
            .unwrap()
            .iter()
            .flat_map(|f| parse_entries(&fs::read_to_string(dir.join(&f.name)).unwrap()))
            .collect();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn empty_turn_and_missing_dir_are_noops() {
        let tmp = TempDir::new("record-empty");
        let dir = tmp.path().join("nope");
        append_turn(&dir, &stamp(), "t", &[], false).unwrap();
        assert!(!dir.exists());
        assert_eq!(next_number(&dir).unwrap(), 1);
    }

    #[test]
    fn last_number_reads_the_tail_of_a_big_file() {
        let tmp = TempDir::new("record-tail");
        let path = tmp.path().join("big.md");
        let mut body = String::new();
        for n in 1..=3 {
            body += &render(n, &stamp(), "t", &[blk("note", &"x".repeat(5000))]);
        }
        fs::write(&path, &body).unwrap();
        assert_eq!(last_record_num(&path).unwrap(), 3);
    }
}
