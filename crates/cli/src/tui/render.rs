//! Pure rendering helpers ported from the Go TUI (tui.go, links.go, usagebar.go, input.go, repl.go).
//! Nothing here touches the terminal. Each item cites the Go source it mirrors. The event loop does not
//! call these yet, so dead-code warnings are allowed for this module until they are wired in.
#![allow(dead_code)]

use unicode_width::UnicodeWidthChar;

// R01 palette (tui.go:252-256): brand orange and grey as truecolor foreground SGR.
pub const ORANGE_ON: &str = "\x1b[38;2;217;119;87m";
pub const GREY_ON: &str = "\x1b[38;2;136;136;136m";
pub const FG_OFF: &str = "\x1b[39m";
// R18 (repl.go:25-26 dim/rst; mdBold tui.go:316 bold).
pub const DIM_ON: &str = "\x1b[2m";
pub const RESET: &str = "\x1b[0m";
pub const BOLD_ON: &str = "\x1b[1m";
pub const BOLD_OFF: &str = "\x1b[22m";
// R08 link colors (links.go:13-20).
pub const LINK_COLOR_ON: &str = "\x1b[38;2;95;175;255m";
pub const LINK_COLOR_OFF: &str = FG_OFF;
pub const LINK_URL_ON: &str = "\x1b[4m\x1b[38;2;95;175;255m";
pub const LINK_URL_OFF: &str = "\x1b[24m\x1b[39m";
// Private SGR marks (links.go:23-26): zero-width to wrap, turned into link styling by restyle.
const MARK_URL_ON: &str = "\x1b[9001m";
const MARK_PATH_ON: &str = "\x1b[9002m";
const MARK_OFF: &str = "\x1b[9003m";

type LinkFn = fn(&[u8], usize) -> Option<usize>;

// Length of the CSI escape at b[i] (ESC [ params intermediates final), or None.
// Same shape as links.go linkSGRRe: \x1b\[[0-9;:?]*[ -/]*[@-~].
fn csi_len(b: &[u8], i: usize) -> Option<usize> {
    if b.get(i) != Some(&0x1b) || b.get(i + 1) != Some(&b'[') {
        return None;
    }
    let mut j = i + 2;
    while j < b.len() && (b[j].is_ascii_digit() || matches!(b[j], b';' | b':' | b'?')) {
        j += 1;
    }
    while j < b.len() && (0x20..=0x2f).contains(&b[j]) {
        j += 1;
    }
    (j < b.len() && (0x40..=0x7e).contains(&b[j])).then_some(j + 1 - i)
}

// The char starting at byte i (i must be a char boundary).
fn cur_char(s: &str, i: usize) -> char {
    s[i..].chars().next().unwrap_or('\0')
}

// Plain text with CSI escapes removed.
pub fn strip_sgr(s: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < s.len() {
        match csi_len(s.as_bytes(), i) {
            Some(n) => i += n,
            None => {
                let c = cur_char(s, i);
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    out
}

// R13 display width in cells (unicode-width; CJK is 2, CSI escapes are 0).
pub fn cell_width(s: &str) -> usize {
    let mut w = 0;
    let mut i = 0;
    while i < s.len() {
        match csi_len(s.as_bytes(), i) {
            Some(n) => i += n,
            None => {
                let c = cur_char(s, i);
                w += c.width().unwrap_or(0);
                i += c.len_utf8();
            }
        }
    }
    w
}

// R14 ansi.Truncate(s, w, "…") (tui.go:1429): unchanged when it fits, else cut to w-1 cells plus "…".
pub fn trunc_ellipsis(s: &str, w: usize) -> String {
    if cell_width(s) <= w {
        return s.to_string();
    }
    if w == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    let mut i = 0;
    while i < s.len() {
        if let Some(n) = csi_len(s.as_bytes(), i) {
            out.push_str(&s[i..i + n]);
            i += n;
            continue;
        }
        let c = cur_char(s, i);
        let cw = c.width().unwrap_or(0);
        if used + cw > w - 1 {
            break;
        }
        out.push(c);
        used += cw;
        i += c.len_utf8();
    }
    out.push('…');
    out
}

// R12 hard wrap by cell width (input.go rows(): a row breaks before a char that would pass the width,
// unless the row is still empty). Newlines in `s` are kept. Width is at least 2 (input.go:150).
// Doubt: the Go output pane uses lipgloss word wrap; this is the input-box rule only.
pub fn wrap_cells(s: &str, width: usize) -> String {
    let w = width.max(2);
    let mut out = String::new();
    for (li, line) in s.split('\n').enumerate() {
        if li > 0 {
            out.push('\n');
        }
        let mut cur = 0;
        let mut row_empty = true;
        let mut i = 0;
        while i < line.len() {
            if let Some(n) = csi_len(line.as_bytes(), i) {
                out.push_str(&line[i..i + n]);
                i += n;
                continue;
            }
            let c = cur_char(line, i);
            let cw = c.width().unwrap_or(0);
            if cur + cw > w && !row_empty {
                out.push('\n');
                cur = 0;
            }
            out.push(c);
            cur += cw;
            row_empty = false;
            i += c.len_utf8();
        }
    }
    out
}

// R03 mdBold (tui.go:316): paired **x** on one line becomes bold; unpaired markers and ``` fences stay.
pub fn md_bold(s: &str) -> String {
    let mut fence = false;
    let mut lines: Vec<String> = Vec::new();
    for ln in s.split('\n') {
        if ln.trim().starts_with("```") {
            fence = !fence;
            lines.push(ln.to_string());
            continue;
        }
        if fence || !ln.contains("**") {
            lines.push(ln.to_string());
            continue;
        }
        let mut out = String::new();
        let mut rest = ln;
        while let Some(a) = rest.find("**") {
            let after = &rest[a + 2..];
            let Some(b) = after.find("**") else {
                break;
            };
            if b == 0 {
                break;
            }
            out.push_str(&rest[..a]);
            out.push_str(BOLD_ON);
            out.push_str(&after[..b]);
            out.push_str(BOLD_OFF);
            rest = &after[b + 2..];
        }
        out.push_str(rest);
        lines.push(out);
    }
    lines.join("\n")
}

// R04 cavemanStatusRe (tui.go:97): ^status:\s*(done|partial|fail|blocked)\b, applied to a trimmed line.
fn is_status_line(ln: &str) -> bool {
    let Some(rest) = ln.strip_prefix("status:") else {
        return false;
    };
    let rest = rest.trim_start_matches([' ', '\t', '\n', '\x0c', '\r']);
    for word in ["done", "partial", "fail", "blocked"] {
        if let Some(tail) = rest.strip_prefix(word) {
            return !tail
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        }
    }
    false
}

// R04 isCavemanBlock (tui.go:100): a status line and a reason line anywhere in the text.
pub fn is_caveman_block(s: &str) -> bool {
    let mut status = false;
    let mut reason = false;
    for ln in s.split('\n') {
        let ln = ln.trim();
        if is_status_line(ln) {
            status = true;
        }
        if ln.starts_with("reason:") {
            reason = true;
        }
    }
    status && reason
}

// R05 tool result text (toolResultMsg, tui.go:485): None when hidden as a caveman block; otherwise the first
// line cut to 160 chars, plus " … (+N lines)" when there is more.
pub fn tool_result_text(s: &str) -> Option<String> {
    let s = s.trim();
    if is_caveman_block(s) {
        return None;
    }
    let lines: Vec<&str> = s.split('\n').collect();
    let mut t: String = lines[0].chars().take(160).collect();
    if lines.len() > 1 {
        t.push_str(&format!(" … (+{} lines)", lines.len() - 1));
    }
    Some(t)
}

// R06 toolSummary (tui.go:67): the first priority key with a non-empty value, newlines turned into spaces,
// cut to 100 chars. Some("") for empty input. None when the input has keys but none of the priority keys;
// the Go fallback (JSON of the input) is not ported (no serde_json dependency in chaude-cli).
pub fn tool_summary(input: &[(&str, &str)]) -> Option<String> {
    const KEYS: [&str; 8] = [
        "command",
        "file_path",
        "pattern",
        "path",
        "url",
        "query",
        "description",
        "prompt",
    ];
    for key in KEYS {
        if let Some((_, v)) = input.iter().find(|(k, _)| *k == key)
            && !v.is_empty()
        {
            return Some(trunc_chars(&v.replace('\n', " "), 100));
        }
    }
    if input.is_empty() {
        return Some(String::new());
    }
    None
}

// Cut to n chars (repl.go trunc, rune based).
fn trunc_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

// R07 permission note (tui.go:499).
pub fn perm_note(tool: &str, arg: &str) -> String {
    format!("Allow {tool}({arg})?  [y] yes  [n] no")
}

// R16 commas (repl.go:122) for an integer.
pub fn commas(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

// R19 importBlocks (tui.go:283): (kind, text) blocks; only the last 100 turns are shown, with a note first.
pub fn import_blocks(turns: &[(String, String)]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let mut turns = turns;
    if turns.len() > 100 {
        out.push((
            "note",
            format!("({} earlier turns not shown)", turns.len() - 100),
        ));
        turns = &turns[turns.len() - 100..];
    }
    for (u, a) in turns {
        out.push(("user", u.clone()));
        out.push(("asst", a.clone()));
    }
    out
}

const BAR_CELLS: usize = 10;
// R15/R16 ctxBarMax (usagebar.go:9).
pub const CTX_BAR_MAX: i64 = 150_000;

// R15 renderBar (usagebar.go:18): "[████░░░░░░] 45%", fill clamped, " !" above 100%.
pub fn render_bar(pct: f64) -> String {
    let pct = if pct < 0.0 { 0.0 } else { pct };
    let n = ((pct / 100.0 * BAR_CELLS as f64 + 0.5) as usize).min(BAR_CELLS);
    let bar = format!("{}{}", "█".repeat(n), "░".repeat(BAR_CELLS - n));
    let mut s = format!("[{bar}] {}%", (pct + 0.5) as i64);
    if pct > 100.0 {
        s.push_str(" !");
    }
    s
}

// R15 barWidth (usagebar.go:35): display width of a rendered bar.
pub fn bar_width(pct: f64) -> usize {
    cell_width(&render_bar(pct))
}

// R17 pctRe (usagebar.go:37) `(\d+(?:\.\d+)?)%`: the value of the first percentage on a line.
pub fn pct_value(line: &str) -> Option<f64> {
    let b = line.as_bytes();
    for p in 0..b.len() {
        if !b[p].is_ascii_digit() {
            continue;
        }
        let mut q = digits_end(b, p);
        if q + 1 < b.len() && b[q] == b'.' && b[q + 1].is_ascii_digit() {
            q = digits_end(b, q + 1);
        }
        if b.get(q) == Some(&b'%') {
            return line[p..q].parse().ok();
        }
    }
    None
}

// R16 decorateUsage (usagebar.go:41): a bar after each line's percentage, then a context bar against CTX_BAR_MAX.
pub fn decorate_usage(text: &str, ctx: i64) -> String {
    let lines: Vec<String> = text
        .split('\n')
        .map(|l| match pct_value(l) {
            Some(v) => format!("{l}  {}", render_bar(v)),
            None => l.to_string(),
        })
        .collect();
    let pct = ctx as f64 / CTX_BAR_MAX as f64 * 100.0;
    format!(
        "{}\n\nContext (now): {}  ({} / {})",
        lines.join("\n"),
        render_bar(pct),
        commas(ctx),
        commas(CTX_BAR_MAX)
    )
}

// ---- Links (links.go) ----

// One link in visible text: byte range [start, end) and whether it is a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkSpan {
    pub start: usize,
    pub end: usize,
    pub url: bool,
}

fn run_while(b: &[u8], mut j: usize, ok: impl Fn(u8) -> bool) -> usize {
    while j < b.len() && ok(b[j]) {
        j += 1;
    }
    j
}

// [\w.\-+@~%]
fn is_tok(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'-' | b'+' | b'@' | b'~' | b'%')
}

// [^\s<>"'\x60\x{80}-\x{10FFFF}]
fn is_url_ch(c: u8) -> bool {
    c < 0x80
        && !matches!(
            c,
            b' ' | b'\t' | b'\n' | b'\x0c' | b'\r' | b'<' | b'>' | b'"' | b'\'' | b'`'
        )
}

// [^\s<>"'|?*\x60\x{80}-\x{10FFFF}]
fn is_win_ch(c: u8) -> bool {
    is_url_ch(c) && !matches!(c, b'|' | b'?' | b'*')
}

fn digits_end(b: &[u8], j: usize) -> usize {
    run_while(b, j, |c| c.is_ascii_digit())
}

// (?::\d+(?::\d+)?)? at j; returns j when absent.
fn line_suffix(b: &[u8], j: usize) -> usize {
    if b.get(j) != Some(&b':') || !b.get(j + 1).is_some_and(u8::is_ascii_digit) {
        return j;
    }
    let k = digits_end(b, j + 1);
    if b.get(k) == Some(&b':') && b.get(k + 1).is_some_and(u8::is_ascii_digit) {
        digits_end(b, k + 1)
    } else {
        k
    }
}

// (?::\d+(?::\d+)?|#L\d+(?:C\d+)?)? at j; returns j when absent.
fn rel_suffix(b: &[u8], j: usize) -> usize {
    let k = line_suffix(b, j);
    if k > j
        || b.get(j) != Some(&b'#')
        || b.get(j + 1) != Some(&b'L')
        || !b.get(j + 2).is_some_and(u8::is_ascii_digit)
    {
        return k;
    }
    let k = digits_end(b, j + 2);
    if b.get(k) == Some(&b'C') && b.get(k + 1).is_some_and(u8::is_ascii_digit) {
        digits_end(b, k + 1)
    } else {
        k
    }
}

// (?:/[\w.\-+@~%]+)* from j; returns the end of the last complete segment.
fn segments(b: &[u8], mut j: usize) -> usize {
    while b.get(j) == Some(&b'/') && b.get(j + 1).copied().is_some_and(is_tok) {
        j = run_while(b, j + 1, is_tok);
    }
    j
}

// https?://[^\s<>"'\x60\x{80}-\x{10FFFF}]+
fn url_end(b: &[u8], i: usize) -> Option<usize> {
    let pre = if b[i..].starts_with(b"https://") {
        8
    } else if b[i..].starts_with(b"http://") {
        7
    } else {
        return None;
    };
    let j = run_while(b, i + pre, is_url_ch);
    (j > i + pre).then_some(j)
}

// [A-Za-z]:[\\/][^\s<>"'|?*\x60\x{80}-\x{10FFFF}]+
fn win_end(b: &[u8], i: usize) -> Option<usize> {
    let drive = b.get(i).is_some_and(u8::is_ascii_alphabetic);
    let sep = matches!(b.get(i + 2).copied(), Some(b'\\' | b'/'));
    if !drive || b.get(i + 1) != Some(&b':') || !sep {
        return None;
    }
    let j = run_while(b, i + 3, is_win_ch);
    (j > i + 3).then_some(j)
}

// (?:\.{1,2}|~)/[\w.\-+@~%]+(?:/[\w.\-+@~%]+)*(?::\d+(?::\d+)?)?
fn dot_end(b: &[u8], i: usize) -> Option<usize> {
    let slash = if b[i..].starts_with(b"../") {
        i + 2
    } else if b[i..].starts_with(b"./") || b[i..].starts_with(b"~/") {
        i + 1
    } else {
        return None;
    };
    let seg = run_while(b, slash + 1, is_tok);
    if seg == slash + 1 {
        return None;
    }
    Some(line_suffix(b, segments(b, seg)))
}

// /[\w.\-+@~%]+(?:/[\w.\-+@~%]+)+(?::\d+(?::\d+)?)?
fn abs_end(b: &[u8], i: usize) -> Option<usize> {
    if b[i] != b'/' {
        return None;
    }
    let p1 = run_while(b, i + 1, is_tok);
    if p1 == i + 1 || b.get(p1) != Some(&b'/') {
        return None;
    }
    let end = segments(b, p1);
    if end == p1 {
        return None;
    }
    Some(line_suffix(b, end))
}

// [\w.\-+@~%]+(?:/[\w.\-+@~%]+)+\.[A-Za-z0-9]+(?::\d+(?::\d+)?|#L\d+(?:C\d+)?)?
// Backtracking is done by hand: the dot that ends the path is the last valid one (greedy preference).
fn rel_end(b: &[u8], i: usize) -> Option<usize> {
    let p1 = run_while(b, i, is_tok);
    if p1 == i || b.get(p1) != Some(&b'/') {
        return None;
    }
    let region = run_while(b, p1, |c| is_tok(c) || c == b'/');
    // A '/' not followed by a token char would make a segment empty, so no dot may come after it.
    let bad = (p1..region)
        .find(|&k| b[k] == b'/' && !b.get(k + 1).copied().is_some_and(is_tok))
        .unwrap_or(region);
    let mut d = bad.min(region);
    while d > p1 + 2 {
        d -= 1;
        if b[d] == b'.' && is_tok(b[d - 1]) && b.get(d + 1).is_some_and(u8::is_ascii_alphanumeric) {
            let end = run_while(b, d + 1, |c| c.is_ascii_alphanumeric());
            return Some(rel_suffix(b, end));
        }
    }
    None
}

// Leftmost non-overlapping matches of one pattern, scanning like Go FindAllStringIndex.
fn find_all(b: &[u8], f: LinkFn) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match f(b, i) {
            Some(e) if e > i => {
                out.push((i, e));
                i = e;
            }
            _ => i += 1,
        }
    }
    out
}

// linkTrimTail (links.go:34): drop sentence punctuation that closed over a link.
fn trim_tail(s: &[u8]) -> &[u8] {
    let mut n = s.len();
    while n > 0 && b".,;:!?)]}'\"".contains(&s[n - 1]) {
        n -= 1;
    }
    &s[..n]
}

// linkBoundary (links.go:43): the match starts a token, not the middle of a word or path.
fn boundary(b: &[u8], start: usize) -> bool {
    start == 0 || !(b[start - 1].is_ascii_alphanumeric() || b"_./\\-~%+@".contains(&b[start - 1]))
}

// linkSpans (links.go:59). Earlier patterns win overlaps; output is sorted by start.
fn link_spans_bytes(b: &[u8]) -> Vec<LinkSpan> {
    let pats: [(LinkFn, bool); 5] = [
        (url_end, true),
        (win_end, false),
        (dot_end, false),
        (rel_end, false),
        (abs_end, false),
    ];
    let mut out: Vec<LinkSpan> = Vec::new();
    for (f, url) in pats {
        for (m0, m1) in find_all(b, f) {
            let a = m0;
            let e = m0 + trim_tail(&b[m0..m1]).len();
            if e <= a || !boundary(b, a) || out.iter().any(|sp| a < sp.end && sp.start < e) {
                continue;
            }
            out.push(LinkSpan {
                start: a,
                end: e,
                url,
            });
        }
    }
    out.sort_by_key(|sp| sp.start);
    out
}

// linkSpans (links.go:59) on plain text.
pub fn link_spans(s: &str) -> Vec<LinkSpan> {
    link_spans_bytes(s.as_bytes())
}

// markLinks (links.go:84): wrap each link of `text` (SGR escapes allowed) in private marks.
pub fn mark_links(text: &str) -> String {
    if !text.contains(['/', '\\']) {
        return text.to_string();
    }
    let b = text.as_bytes();
    let mut vis: Vec<u8> = Vec::new();
    let mut raw: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if let Some(n) = csi_len(b, i) {
            i += n;
            continue;
        }
        vis.push(b[i]);
        raw.push(i);
        i += 1;
    }
    let spans = link_spans_bytes(&vis);
    if spans.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 16 * spans.len());
    let mut last = 0;
    for sp in spans {
        let s = raw[sp.start];
        let e = raw[sp.end - 1] + 1;
        out.push_str(&text[last..s]);
        out.push_str(if sp.url { MARK_URL_ON } else { MARK_PATH_ON });
        out.push_str(&text[s..e]);
        out.push_str(MARK_OFF);
        last = e;
    }
    out.push_str(&text[last..]);
    out
}

// restyle (links.go:128): turns marks into link styling, one line at a time. A link that continues onto the
// next line is closed at the end of the line and reopened at its start. Other SGR escapes inside a link are
// followed by the link style again.
pub fn restyle(text: &str) -> String {
    let open = |k: u8| if k == 2 { LINK_URL_ON } else { LINK_COLOR_ON };
    let close = |k: u8| if k == 2 { LINK_URL_OFF } else { LINK_COLOR_OFF };
    let mut kind = 0u8; // 0 none, 1 path, 2 URL
    let mut lines: Vec<String> = Vec::new();
    for ln in text.split('\n') {
        let b = ln.as_bytes();
        let mut out = String::new();
        if kind != 0 {
            out.push_str(open(kind));
        }
        let mut p = 0;
        while p < b.len() {
            let rest = &ln[p..];
            if rest.starts_with(MARK_URL_ON) {
                kind = 2;
                out.push_str(LINK_URL_ON);
                p += MARK_URL_ON.len();
            } else if rest.starts_with(MARK_PATH_ON) {
                kind = 1;
                out.push_str(LINK_COLOR_ON);
                p += MARK_PATH_ON.len();
            } else if rest.starts_with(MARK_OFF) {
                if kind != 0 {
                    out.push_str(close(kind));
                    kind = 0;
                }
                p += MARK_OFF.len();
            } else if let Some(n) = csi_len(b, p) {
                out.push_str(&ln[p..p + n]);
                if kind != 0 {
                    out.push_str(open(kind));
                }
                p += n;
            } else {
                let c = cur_char(ln, p);
                out.push(c);
                p += c.len_utf8();
            }
        }
        if kind != 0 {
            out.push_str(close(kind));
        }
        lines.push(out);
    }
    lines.join("\n")
}

// linkify (links.go:188): styles the links of one line without wrapping.
pub fn linkify(line: &str) -> String {
    restyle(&mark_links(line))
}

// wrapLinked (links.go:197): wraps with `wrap`, then styles links on each wrapped run outside ``` fences.
// Fenced code is wrapped but left unstyled.
pub fn wrap_linked(text: &str, wrap: impl Fn(&str) -> String) -> String {
    let mut fence = false;
    let mut runs: Vec<(String, bool)> = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    let mut cur_code = false;
    for ln in text.split('\n') {
        let marker = ln.trim().starts_with("```");
        let code = fence || marker;
        if marker {
            fence = !fence;
        }
        if !cur.is_empty() && cur_code != code {
            runs.push((cur.join("\n"), cur_code));
            cur.clear();
        }
        cur.push(ln);
        cur_code = code;
    }
    runs.push((cur.join("\n"), cur_code));
    runs.iter()
        .map(|(r, code)| {
            if *code {
                wrap(r)
            } else {
                restyle(&wrap(&mark_links(r)))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(s: &str) -> Vec<&str> {
        link_spans(s)
            .iter()
            .map(|sp| &s[sp.start..sp.end])
            .collect()
    }

    // Port of links_wrap_test.go applyColorSGR.
    fn apply_color_sgr(colored: bool, esc: &str) -> bool {
        let Some(params) = esc.strip_prefix("\x1b[").and_then(|p| p.strip_suffix('m')) else {
            return colored;
        };
        if params.is_empty() {
            return false;
        }
        let ps: Vec<&str> = params.split(';').collect();
        let mut colored = colored;
        let mut i = 0;
        while i < ps.len() {
            match ps[i] {
                "0" | "" | "39" => colored = false,
                "38" if i + 4 < ps.len() && ps[i + 1] == "2" => {
                    colored = ps[i + 2] == "95" && ps[i + 3] == "175" && ps[i + 4] == "255";
                    i += 4;
                }
                _ => {}
            }
            i += 1;
        }
        colored
    }

    // Port of links_wrap_test.go linkColoredRuns: visible text of each colored link, joined across wraps.
    fn linked_colored_runs(out: &str) -> Vec<String> {
        let mut pieces: Vec<String> = Vec::new();
        let mut carry = false;
        for ln in out.split('\n') {
            let b = ln.as_bytes();
            let mut vis = String::new();
            let mut on: Vec<bool> = Vec::new();
            let mut colored = false;
            let mut i = 0;
            while i < b.len() {
                if let Some(n) = csi_len(b, i) {
                    colored = apply_color_sgr(colored, &ln[i..i + n]);
                    i += n;
                    continue;
                }
                let c = cur_char(ln, i);
                for _ in 0..c.len_utf8() {
                    on.push(colored);
                }
                vis.push(c);
                i += c.len_utf8();
            }
            let content = vis.trim_end_matches(' ');
            let clen = content.len();
            let mut runs: Vec<String> = Vec::new();
            let mut i = 0;
            while i < clen {
                if !on[i] {
                    i += 1;
                    continue;
                }
                let mut j = i;
                while j < clen && on[j] {
                    j += 1;
                }
                runs.push(vis[i..j].to_string());
                i = j;
            }
            if carry && !runs.is_empty() && on.first() == Some(&true) {
                if let Some(last) = pieces.last_mut() {
                    last.push_str(&runs[0]);
                }
                runs.remove(0);
            }
            pieces.extend(runs);
            carry = clen > 0 && on.len() >= clen && on[clen - 1];
        }
        pieces
            .into_iter()
            .map(|p| p.trim_matches(' ').to_string())
            .filter(|p| !p.is_empty())
            .collect()
    }

    #[test]
    fn link_spans_detects_urls_paths_and_trims_tail() {
        let cases: [(&str, Vec<&str>); 8] = [
            (
                "see https://example.com/a?b=1 now",
                vec!["https://example.com/a?b=1"],
            ),
            ("(https://x.io/a).", vec!["https://x.io/a"]),
            (
                r"C:\Users\me\proj\main.go",
                vec![r"C:\Users\me\proj\main.go"],
            ),
            ("edit C:/src/app.go:42:7 today", vec!["C:/src/app.go:42:7"]),
            (
                "open ./internal/harness/tui.go:120",
                vec!["./internal/harness/tui.go:120"],
            ),
            ("see ../README.md", vec!["../README.md"]),
            (
                "file internal/harness/links.go:9 changed",
                vec!["internal/harness/links.go:9"],
            ),
            (
                "abs /usr/local/bin/chaude ok",
                vec!["/usr/local/bin/chaude"],
            ),
        ];
        for (input, want) in cases {
            assert_eq!(texts(input), want, "input {input:?}");
        }
    }

    #[test]
    fn link_spans_prose_negatives() {
        for input in [
            "and/or the 1/2 ratio",
            "ratio 3/4 and 10/20",
            "plain words only",
            "v1.2/beta mention",
            "x:12 and 1:30",
        ] {
            assert!(texts(input).is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn link_spans_overlap_and_boundary() {
        assert_eq!(texts("https://h.io/a/b/c.go").len(), 1);
        assert_eq!(texts("foo.bar/baz/qux.go"), vec!["foo.bar/baz/qux.go"]);
    }

    #[test]
    fn linkify_keeps_visible_text_and_width() {
        let input = "\x1b[1mbold\x1b[0m open https://e.com/x and C:\\a\\b.txt:3 end";
        let out = linkify(input);
        assert_ne!(out, input);
        assert_eq!(strip_sgr(&out), strip_sgr(input));
        assert_eq!(cell_width(&out), cell_width(input));
        assert!(out.contains(LINK_URL_ON));
        assert!(out.contains(LINK_COLOR_ON));
    }

    #[test]
    fn wrap_linked_skips_fenced_code() {
        let text = "see ./a/b.go\n```\n./code/c.go\n```\nafter ./d/e.go";
        let out = wrap_linked(text, |s| s.to_string());
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), 5, "{out:?}");
        assert!(lines[0].contains(LINK_COLOR_ON));
        assert!(lines[4].contains(LINK_COLOR_ON));
        assert!(!lines[2].contains('\x1b'), "{:?}", lines[2]);
        assert_eq!(strip_sgr(&out), text);
    }

    #[test]
    fn wrap_linked_colors_whole_tokens() {
        let cases: [(&str, usize, Vec<&str>); 14] = [
            (
                "see https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbbbbbbbbbbbbbb?q=1&r=%20#frag end",
                16,
                vec![
                    "https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbbbbbbbbbbbbbb?q=1&r=%20#frag",
                ],
            ),
            (
                "open https://x.io/p?a=1&b=(2)~#top, ok",
                80,
                vec!["https://x.io/p?a=1&b=(2)~#top"],
            ),
            ("https://x.com/a에서 열기", 80, vec!["https://x.com/a"]),
            ("링크는 https://x.com/a。 끝", 80, vec!["https://x.com/a"]),
            (
                "https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbb에서",
                16,
                vec!["https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbb"],
            ),
            ("cd ~/dir/sub now", 80, vec!["~/dir/sub"]),
            (
                "run /usr/local/bin/chaude now",
                80,
                vec!["/usr/local/bin/chaude"],
            ),
            (
                "see src/main/intt/x.go:12 now",
                80,
                vec!["src/main/intt/x.go:12"],
            ),
            (
                "see src/main/intt/harness/links.go:12 ok",
                20,
                vec!["src/main/intt/harness/links.go:12"],
            ),
            (
                "use `src/main/intt/x.go:12` now",
                80,
                vec!["src/main/intt/x.go:12"],
            ),
            ("**src/main/intt/x.go** now", 80, vec!["src/main/intt/x.go"]),
            (
                "[docs](https://x.com/a/b.html) done",
                80,
                vec!["https://x.com/a/b.html"],
            ),
            (
                "see (https://x.com/a). and ./a/b.go.",
                80,
                vec!["https://x.com/a", "./a/b.go"],
            ),
            (
                "https://x.com/a\x1b[0mb/c end",
                80,
                vec!["https://x.com/ab/c"],
            ),
        ];
        for (input, width, want) in cases {
            let wrap = |s: &str| wrap_cells(s, width);
            let out = wrap_linked(&md_bold(input), wrap);
            assert!(!out.contains("\x1b[900"), "marker leaked: {out:?}");
            assert_eq!(
                linked_colored_runs(&out),
                want,
                "input {input:?}\n{}",
                strip_sgr(&out)
            );
            let squash = |s: &str| s.split_whitespace().collect::<String>();
            assert_eq!(
                squash(&strip_sgr(&out)),
                squash(&strip_sgr(&md_bold(input)))
            );
        }
    }

    #[test]
    fn md_bold_pairs_and_skips_fences() {
        let cases = [
            ("**가**나", format!("{BOLD_ON}가{BOLD_OFF}나")),
            ("**가", "**가".to_string()),
            ("```\n**가**\n```", "```\n**가**\n```".to_string()),
            ("**a** x **b", format!("{BOLD_ON}a{BOLD_OFF} x **b")),
        ];
        for (input, want) in cases {
            assert_eq!(md_bold(input), want, "input {input:?}");
        }
    }

    #[test]
    fn caveman_blocks_are_detected_and_hidden() {
        let cases = [
            ("status: done\nreason: -\nclaim: a.go:1 | x", true),
            ("[harness: note]\nstatus: partial\nreason: cap", true),
            ("status: done", false),
            ("status: running\nreason: -", false),
            ("읽기 결과입니다.", false),
            ("", false),
        ];
        for (input, want) in cases {
            assert_eq!(is_caveman_block(input), want, "input {input:?}");
        }
        assert_eq!(
            tool_result_text("status: done\nreason: -\nclaim: a.go:1 | x"),
            None
        );
        assert_eq!(
            tool_result_text("ok\nmore").as_deref(),
            Some("ok … (+1 lines)")
        );
        assert_eq!(tool_result_text("  one  ").as_deref(), Some("one"));
    }

    #[test]
    fn tool_summary_uses_priority_keys() {
        assert_eq!(
            tool_summary(&[("file_path", "a.rs"), ("command", "ls -la\nx")]).as_deref(),
            Some("ls -la x")
        );
        assert_eq!(tool_summary(&[]).as_deref(), Some(""));
        assert_eq!(tool_summary(&[("other", "x")]), None);
        let long = "y".repeat(150);
        assert_eq!(
            tool_summary(&[("query", &long)]).unwrap().chars().count(),
            100
        );
    }

    #[test]
    fn permission_note_text() {
        assert_eq!(perm_note("Bash", "ls"), "Allow Bash(ls)?  [y] yes  [n] no");
    }

    #[test]
    fn ellipsis_truncation_counts_cells() {
        assert_eq!(trunc_ellipsis("abcdef", 4), "abc…");
        assert_eq!(trunc_ellipsis("abc", 4), "abc");
        assert_eq!(trunc_ellipsis("가나다", 3), "가…");
        assert_eq!(trunc_ellipsis("abc", 0), "");
    }

    #[test]
    fn wrap_cells_breaks_by_cell_width() {
        // input.go TestInputWrapWide: 3 Hangul per 6-cell row.
        assert_eq!(wrap_cells("가나다라마", 6), "가나다\n라마");
        assert_eq!(wrap_cells("ab\ncd", 10), "ab\ncd");
    }

    #[test]
    fn mixed_width_measure() {
        // input.go TestInputWidthMixed: "ab한글cd" is 8 cells, "ab글cd" is 6.
        assert_eq!(cell_width("ab한글cd"), 8);
        assert_eq!(cell_width("ab글cd"), 6);
    }

    #[test]
    fn render_bar_cases_and_width() {
        assert_eq!(render_bar(0.0), "[░░░░░░░░░░] 0%");
        assert_eq!(render_bar(50.0), "[█████░░░░░] 50%");
        assert_eq!(render_bar(100.0), "[██████████] 100%");
        assert_eq!(render_bar(130.0), "[██████████] 130% !");
        assert_eq!(bar_width(0.0), bar_width(100.0) - 2);
    }

    #[test]
    fn decorate_usage_adds_bars() {
        let out = decorate_usage("Current session: 45% used\nother", 75_000);
        assert!(out.starts_with("Current session: 45% used  ["), "{out}");
        assert!(
            out.contains("Context (now): [█████░░░░░] 50%  (75,000 / 150,000)"),
            "{out}"
        );
    }

    #[test]
    fn pct_value_finds_first_percentage() {
        assert_eq!(pct_value("Current session: 45% used"), Some(45.0));
        assert_eq!(pct_value("usage 12.5%"), Some(12.5));
        assert_eq!(pct_value("a1.2.3%"), Some(2.3));
        assert_eq!(pct_value("no number"), None);
    }

    #[test]
    fn import_shows_last_hundred_turns_with_note() {
        let turns: Vec<(String, String)> = (0..150)
            .map(|i| (format!("u{i}"), format!("a{i}")))
            .collect();
        let blocks = import_blocks(&turns);
        assert_eq!(blocks.len(), 201);
        assert_eq!(
            blocks[0],
            ("note", "(50 earlier turns not shown)".to_string())
        );
        assert_eq!(blocks[1], ("user", "u50".to_string()));
        let few = import_blocks(&turns[..2]);
        assert_eq!(few.len(), 4);
    }

    #[test]
    fn commas_groups_thousands() {
        assert_eq!(commas(150_000), "150,000");
        assert_eq!(commas(999), "999");
        assert_eq!(commas(-1234), "-1,234");
    }
}
