//! Shared helpers for the scripts (Go: internal/scripts/common.go).

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};

use chaude_core::lean::{self, ParseError};
use chaude_core::osenv;
use serde_json::Value;

static NULL: Value = Value::Null;

// ---- python-compatible string helpers ----

/// str.isspace() plus the 0x1c..=0x1f separators.
pub fn py_is_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1c}'..='\u{1f}')
}

pub fn py_strip(s: &str) -> &str {
    s.trim_matches(py_is_space)
}

/// str.split() with no arguments: the non-empty runs between whitespace.
pub fn py_fields(s: &str) -> Vec<&str> {
    s.split(py_is_space).filter(|p| !p.is_empty()).collect()
}

pub fn rune_len(s: &str) -> usize {
    s.chars().count()
}

/// Characters `a..b` (clamped at 0); empty when `a >= b`.
pub fn rune_slice(s: &str, a: i64, b: i64) -> String {
    let a = a.max(0) as usize;
    let b = b.max(0) as usize;
    if a >= b {
        return String::new();
    }
    s.chars().skip(a).take(b - a).collect()
}

/// Like Python str.splitlines() without keepends.
pub fn py_split_lines(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' | '\u{0b}' | '\u{0c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}'
            | '\u{2029}' => out.push(std::mem::take(&mut cur)),
            '\r' => {
                out.push(std::mem::take(&mut cur));
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Simple (one-to-one) lowercase mapping, like Go unicode.ToLower.
pub fn lower_runes(s: &str) -> String {
    s.chars()
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .collect()
}

/// Grouped integer string, e.g. "1,234" (sign and fraction are kept).
pub fn comma_str(s: &str) -> String {
    let (neg, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (int_part, frac) = match s.find('.') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    let n = int_part.len();
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    for (i, c) in int_part.chars().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.push_str(frac);
    out
}

pub fn comma_int(n: i64) -> String {
    comma_str(&n.to_string())
}

/// First 8 hex digits of the SHA-1 of the UTF-8 bytes of `s`.
pub fn sha8(s: &str) -> String {
    sha1(s.as_bytes())[..4]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for block in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 80];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (slot, v) in h.iter_mut().zip([a, b, c, d, e]) {
            *slot = slot.wrapping_add(v);
        }
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ---- textwrap.wrap(break_long_words=True, break_on_hyphens=False) ----

fn expand_tabs(s: &str) -> String {
    let mut out = String::new();
    let mut col = 0;
    for r in s.chars() {
        match r {
            '\t' => {
                let n = 8 - col % 8;
                out.extend(std::iter::repeat_n(' ', n));
                col += n;
            }
            '\n' | '\r' => {
                out.push(r);
                col = 0;
            }
            _ => {
                out.push(r);
                col += 1;
            }
        }
    }
    out
}

fn is_wrap_ws(r: char) -> bool {
    matches!(r, '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | ' ')
}

pub fn wrap_text(text: &str, width: i64) -> Vec<String> {
    let text = expand_tabs(text);
    let mut chunks: Vec<Vec<char>> = Vec::new();
    let mut cur: Vec<char> = Vec::new();
    let mut cur_ws = false;
    for r in text.chars() {
        let w = is_wrap_ws(r);
        if !cur.is_empty() && w != cur_ws {
            chunks.push(std::mem::take(&mut cur));
        }
        cur_ws = w;
        cur.push(r);
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    chunks.reverse();
    let mut lines = Vec::new();
    while !chunks.is_empty() {
        let mut line: Vec<char> = Vec::new();
        let mut cur_len: i64 = 0;
        while let Some(last) = chunks.last() {
            let l = last.len() as i64;
            if cur_len + l > width {
                break;
            }
            line.extend_from_slice(last);
            chunks.pop();
            cur_len += l;
        }
        let over = chunks.last().is_some_and(|c| c.len() as i64 > width);
        if over {
            let space_left = if width >= 1 { width - cur_len } else { 1 };
            let top = chunks.len() - 1;
            let c = std::mem::take(&mut chunks[top]);
            let end = (space_left.max(0) as usize).min(c.len());
            line.extend_from_slice(&c[..end]);
            chunks[top] = c[end..].to_vec();
        }
        if !line.is_empty() {
            lines.push(line.iter().collect());
        }
    }
    lines
}

// ---- transcript helpers ----

const NOISE_TAGS: [&str; 5] = [
    "command-name",
    "command-message",
    "command-args",
    "local-command-stdout",
    "system-reminder",
];

/// Removes `<tag>...</tag>` blocks for the noise tags (non-greedy, earliest start first).
pub fn strip_noise(s: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    loop {
        let mut best: Option<(usize, usize)> = None;
        for tag in NOISE_TAGS {
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");
            let Some(i) = s[pos..].find(&open).map(|i| i + pos) else {
                continue;
            };
            let body = i + open.len();
            let Some(j) = s[body..].find(&close) else {
                continue;
            };
            let end = body + j + close.len();
            if best.is_none_or(|(bs, _)| i < bs) {
                best = Some((i, end));
            }
        }
        let Some((start, end)) = best else {
            break;
        };
        out.push_str(&s[pos..start]);
        pos = end;
    }
    out.push_str(&s[pos..]);
    out
}

/// Text of a message `content` value: a string, or the joined "text" parts (empty if a tool_result is present).
pub fn text_of(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            let has_tool_result = items
                .iter()
                .any(|it| it.get("type").and_then(Value::as_str) == Some("tool_result"));
            if has_tool_result {
                return String::new();
            }
            let parts: Vec<&str> = items
                .iter()
                .filter(|it| it.get("type").and_then(Value::as_str) == Some("text"))
                .map(|it| sval(fget(it, "text")))
                .collect();
            parts.join("\n")
        }
        _ => String::new(),
    }
}

/// `timestamp[:16].replace("T", " ")`.
pub fn ts_of(e: &Value) -> String {
    sval(fget(e, "timestamp"))
        .chars()
        .take(16)
        .map(|c| if c == 'T' { ' ' } else { c })
        .collect()
}

/// Python-style truthiness of a JSON value.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Object field, or null when missing or `v` is not an object.
pub fn fget<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&NULL)
}

/// String value, or "" for anything that is not a string.
pub fn sval(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

// ---- files ----

fn valid_utf8(bytes: &[u8]) -> String {
    let mut s = String::new();
    for chunk in bytes.utf8_chunks() {
        s.push_str(chunk.valid());
    }
    s
}

/// Streams lines of a plain or `.gz` file. Invalid UTF-8 bytes are dropped; read errors end the stream.
pub fn each_file_line(path: &Path, mut f: impl FnMut(&str)) -> io::Result<()> {
    let file = fs::File::open(path)?;
    let reader: Box<dyn Read> = if path.to_string_lossy().ends_with(".gz") {
        Box::new(flate2::read::GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let mut br = BufReader::with_capacity(1 << 20, reader);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let Ok(n) = br.read_until(b'\n', &mut buf) else {
            break;
        };
        if n == 0 {
            break;
        }
        f(&valid_utf8(&buf));
    }
    Ok(())
}

/// A JSON object line, or None.
pub fn parse_line(line: &str) -> Option<Value> {
    serde_json::from_str::<Value>(line)
        .ok()
        .filter(|v| v.is_object())
}

/// Sorted paths in `dir` ending with `suffix`, skipping hidden names.
pub fn glob_dir(dir: &Path, suffix: &str) -> Vec<PathBuf> {
    let mut names: Vec<String> = dir_names(dir)
        .into_iter()
        .filter(|n| !n.starts_with('.') && n.ends_with(suffix))
        .collect();
    names.sort();
    names.into_iter().map(|n| dir.join(n)).collect()
}

/// Sorted entry names in `dir` matching a `*`-only pattern (like filepath.Glob on one level).
pub fn glob_names(dir: &Path, pattern: &str) -> Vec<String> {
    let mut names: Vec<String> = dir_names(dir)
        .into_iter()
        .filter(|n| wildcard(pattern, n))
        .collect();
    names.sort();
    names
}

fn dir_names(dir: &Path) -> Vec<String> {
    match fs::read_dir(dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Matches `name` against a pattern where `*` matches any run of characters.
pub fn wildcard(pat: &str, name: &str) -> bool {
    let (p, t) = (pat.as_bytes(), name.as_bytes());
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while ti < t.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            pi += 1;
            mark = ti;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

/// Lexical path cleanup (Go filepath.Clean).
pub fn clean(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(".."),
            },
            Component::Normal(_) | Component::RootDir | Component::Prefix(_) => {
                out.push(c.as_os_str())
            }
        }
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}

/// Leading "~" expanded to the home directory.
pub fn expand_user(p: &str) -> String {
    if p == "~" || p.starts_with("~/") || p.starts_with("~\\") {
        format!("{}{}", osenv::home_dir().display(), &p[1..])
    } else {
        p.to_string()
    }
}

/// Absolute, cleaned path (Go filepath.Abs after expandUser).
pub fn abs_path(p: &str) -> PathBuf {
    let e = PathBuf::from(expand_user(p));
    let full = if e.is_absolute() {
        e
    } else {
        std::env::current_dir().map(|w| w.join(&e)).unwrap_or(e)
    };
    clean(&full)
}

/// Comparison key for paths: cleaned, lowercased with '\' on Windows.
pub fn norm_case(p: &Path) -> String {
    let c = clean(p).to_string_lossy().into_owned();
    if cfg!(windows) {
        c.replace('/', "\\").to_lowercase()
    } else {
        c
    }
}

/// CLAUDE_PROJECT_DIR, else the current directory.
pub fn default_project() -> String {
    match std::env::var("CLAUDE_PROJECT_DIR") {
        Ok(p) if !p.is_empty() => p,
        _ => std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

pub fn file_name_string(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Go filepath.Ext-style trim: drops the last ".ext" (the dot included).
pub fn strip_ext(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) => &name[..i],
        None => name,
    }
}

/// filepath.ToSlash: backslashes become '/' on Windows only.
pub fn to_slash(p: &Path) -> String {
    let s = p.to_string_lossy();
    if cfg!(windows) {
        s.replace('\\', "/")
    } else {
        s.into_owned()
    }
}

// ---- flags and usage ----

/// Parsed flags: `values` keyed by flag name, `pos` the positional arguments.
pub struct Flags {
    name: String,
    values: BTreeMap<String, String>,
    pub pos: Vec<String>,
}

impl Flags {
    pub fn str(&self, name: &str, default: &str) -> String {
        self.values
            .get(name)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    pub fn int(&self, name: &str, default: i64) -> Result<i64, i32> {
        match self.values.get(name) {
            None => Ok(default),
            Some(v) => v.parse::<i64>().map_err(|_| {
                eprintln!("invalid value {v:?} for flag -{name}: parse error");
                usage(&self.name);
                2
            }),
        }
    }
}

/// Parses flags the way Go's flag.FlagSet did for these scripts. Err(code) means return that code.
/// `-h` prints usage and returns Err(0).
pub fn parse(name: &str, value_flags: &[&str], args: &[String]) -> Result<Flags, i32> {
    match lean::parse_interspersed(args, value_flags) {
        Ok(p) => Ok(Flags {
            name: name.to_string(),
            values: p.values,
            pos: p.positionals,
        }),
        Err(ParseError::Help) => {
            usage(name);
            Err(0)
        }
        Err(ParseError::Msg(m)) => {
            eprintln!("{m}");
            usage(name);
            Err(2)
        }
    }
}

pub fn usage(name: &str) {
    eprintln!("{}", help_text(name));
}

pub fn fail(msg: &str) -> i32 {
    eprintln!("{msg}");
    1
}

/// -h / usage text per script (verbatim from the Go port, Korean).
fn help_text(name: &str) -> String {
    let text = match name {
        "charcount" => {
            "usage: chaude charcount FILE [--section HEADING] [--limit N]\n\t  텍스트/마크다운 파일의 글자 수를 센다 (공백 포함·제외, 바이트). FILE 대신 - 를 주면 표준입력.\n\t  --section HEADING  해당 제목 아래 섹션만 센다\n\t  --limit N          N자 제한과 비교해 초과 여부를 출력"
        }
        "claude-history" => {
            "usage: chaude claude-history [PROJECT_DIR]\n\t  프로젝트(기본: CLAUDE_PROJECT_DIR 또는 현재 폴더)의 Claude Code 대화 기록 폴더 경로를 출력한다.\n\t  기록 폴더는 ~/.claude/projects 아래에 있다. 찾지 못하면 오류 메시지를 stderr로 출력하고 종료 코드 1."
        }
        "compress-archive" => {
            "usage: chaude compress-archive DIR [DIR...]\n\t  DIR 아래(숨김 폴더 제외)의 .jsonl 대화 기록을 .jsonl.gz로 압축하고 원본을 지운다.\n\t  완료 후 기록 개수와 용량 변화(MB)를 출력한다. 인자 없이 실행하면 사용법을 출력한다."
        }
        "export-history" => {
            "usage: chaude export-history [--project DIR] [--file F] [--out DIR] [--width N] [--split-kb N] [--hook]\n\t  프로젝트 대화 기록을 읽기 좋은 텍스트 파일로 내보낸다.\n\t  --project DIR   대상 프로젝트 (기본: CLAUDE_PROJECT_DIR 또는 현재 폴더)\n\t  --file F        특정 세션 .jsonl 파일만 내보냄\n\t  --out DIR       출력 폴더\n\t  --width N       줄 너비 (기본 120)\n\t  --split-kb N    출력 파일 분할 크기 KB (기본 500)\n\t  --hook          Claude Code 훅 JSON을 표준입력에서 읽는다"
        }
        "extract-transcript" => {
            "usage: chaude extract-transcript SOURCE [--out DIR] [--chunk N] [--reply-chars N]\n\t  SOURCE(.jsonl 파일, 프로젝트 경로, 또는 ~/.claude/projects 하위 폴더)의 대화를 조각 파일로 나눈다.\n\t  사용자/Claude 메시지만 추출하고, 긴 붙여넣기는 해시로 중복 제거한다.\n\t  --out DIR           출력 폴더 (기본 notes/distill); DIR/chunks/NNN.md 와 DIR/INDEX.md 생성\n\t  --chunk N           조각 크기 목표 글자 수 (기본 7000)\n\t  --reply-chars N     Claude 답변 자르기 글자 수 (기본 800)"
        }
        "search-history" => {
            "usage: chaude search-history \"<terms>\" [--project DIR] [--since YYYY-MM-DD] [--who user|claude|all] [--limit N] [--width N]\n\t  프로젝트 대화 기록에서 검색어가 포함된 메시지를 찾아 최신순으로 출력한다.\n\t  --project DIR      대상 프로젝트 (기본: CLAUDE_PROJECT_DIR 또는 현재 폴더)\n\t  --since DATE       이 날짜 이후만 (YYYY-MM-DD)\n\t  --who user|claude|all  발화자 필터 (기본 all)\n\t  --limit N          표시 건수 (기본 12)\n\t  --width N          스니펫 너비 (기본 300)"
        }
        "usage-report" => {
            "usage: chaude usage-report [PROJECTS_DIR] [--top N]\n\t  대화 기록(기본 ~/.claude/projects)의 모델별 요청·캐시·출력 토큰과 추정 비용을 집계한다.\n\t  --top N  비용 상위 세션 표시 개수 (기본 15)"
        }
        _ => return format!("usage: chaude {name} [args]"),
    };
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha8_matches_known_sha1_prefix() {
        // sha1("abc") = a9993e36 4706816a ba3e2571 7850c26c 9cd0d89d
        assert_eq!(sha8("abc"), "a9993e36");
    }

    #[test]
    fn comma_grouping_keeps_sign_and_fraction() {
        assert_eq!(comma_int(1234567), "1,234,567");
        assert_eq!(comma_str("-1234.50"), "-1,234.50");
    }

    #[test]
    fn splitlines_and_wrap_follow_python() {
        assert_eq!(py_split_lines("a\r\nb\n\nc"), vec!["a", "b", "", "c"]);
        assert_eq!(wrap_text("abcdefgh", 3), vec!["abc", "def", "gh"]);
    }
}
