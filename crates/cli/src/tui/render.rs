//! Pure rendering helpers ported from the Go TUI (tui.go, links.go, usagebar.go, input.go, repl.go).
//! Nothing here touches the terminal. Each item cites the Go source it mirrors. The event loop does not
//! call these yet, so the items not wired are marked with a targeted #[allow(dead_code)] and a reason.

use std::sync::OnceLock;

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
pub fn csi_len(b: &[u8], i: usize) -> Option<usize> {
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

// ---- Display width: go-runewidth v0.0.16 StringWidth/RuneWidth, which the Go TUI uses ----

// runewidth.go handleEnv: RUNEWIDTH_EASTASIAN ("1" = on) wins when set, else the locale decides.
pub fn east_asian() -> bool {
    static EA: OnceLock<bool> = OnceLock::new();
    *EA.get_or_init(|| match std::env::var("RUNEWIDTH_EASTASIAN") {
        Ok(v) if !v.is_empty() => v == "1",
        _ => {
            let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
                .iter()
                .filter_map(|k| std::env::var(k).ok())
                .find(|v| !v.is_empty())
                .unwrap_or_default();
            is_east_asian_locale(&locale)
        }
    })
}

// reLoc in runewidth_posix.go: ^[a-z][a-z][a-z]?(?:_[A-Z][A-Z])?\.(.+) ; returns the charset part.
fn locale_charset(locale: &str) -> Option<&str> {
    let b = locale.as_bytes();
    let mut i = 0;
    while i < b.len() && i < 3 && b[i].is_ascii_lowercase() {
        i += 1;
    }
    if !(2..=3).contains(&i) {
        return None;
    }
    if b.len() >= i + 3
        && b[i] == b'_'
        && b[i + 1].is_ascii_uppercase()
        && b[i + 2].is_ascii_uppercase()
    {
        i += 3;
    }
    if b.get(i) != Some(&b'.') || i + 1 >= b.len() {
        return None;
    }
    Some(&locale[i + 1..])
}

// runewidth_posix.go IsEastAsian/isEastAsian: C/POSIX are off; a legacy CJK charset (mblenTable) is on;
// utf-8 is on only for ja/ko/zh locales. "euc-kr" (with a dash) is not in the table, so it stays off.
pub fn is_east_asian_locale(locale: &str) -> bool {
    if locale == "POSIX" || locale == "C" {
        return false;
    }
    let b = locale.as_bytes();
    if b.len() > 1 && b[0] == b'C' && (b[1] == b'.' || b[1] == b'-') {
        return false;
    }
    let mut charset = match locale_charset(locale) {
        Some(c) => c.to_ascii_lowercase(),
        None => locale.to_ascii_lowercase(),
    };
    if charset.ends_with("@cjk_narrow") {
        return false;
    }
    if let Some(p) = charset.find('@') {
        charset.truncate(p);
    }
    const MBLEN_CJK: [&str; 15] = [
        "utf-8", "utf8", "jis", "eucjp", "euckr", "euccn", "sjis", "cp932", "cp51932", "cp936",
        "cp949", "cp950", "big5", "gbk", "gb2312",
    ];
    MBLEN_CJK.contains(&charset.as_str())
        && (!charset.starts_with('u') || ["ja", "ko", "zh"].iter().any(|p| locale.starts_with(p)))
}

// RuneWidth for one rune. Zero-width marks are 0 as in unicode-width. Go gives the Hangul jamo block
// U+1160..U+11FF width 1 (unicode-width says 0), and U+FE0F width 2 under EastAsianWidth, else 1.
pub fn char_width_ea(c: char, ea: bool) -> usize {
    use super::runewidth_table::{AMBIGUOUS, COMBINING, DOUBLEWIDTH, NARROW, NONPRINT};
    let r = c as u32;
    match r {
        0x1160..=0x11FF => return 1,
        0xFE0F => return if ea { 2 } else { 1 },
        _ => {}
    }
    // go-runewidth v0.0.16 RuneWidth with StrictEmojiNeutral=true (the default).
    if !ea {
        return if r < 0x20 || (0x7F..=0x9F).contains(&r) || r == 0xAD {
            0
        } else if r < 0x300 || in_table(r, NARROW) {
            1
        } else if in_table(r, NONPRINT) || in_table(r, COMBINING) {
            0
        } else if in_table(r, DOUBLEWIDTH) {
            2
        } else {
            1
        };
    }
    if in_table(r, NONPRINT) || in_table(r, COMBINING) {
        0
    } else if in_table(r, NARROW) {
        1
    } else if in_table(r, AMBIGUOUS) || in_table(r, DOUBLEWIDTH) {
        2
    } else {
        1
    }
}

// Binary search over sorted inclusive (first, last) ranges (go-runewidth inTable).
fn in_table(r: u32, t: &[(u32, u32)]) -> bool {
    t.binary_search_by(|&(lo, hi)| {
        if hi < r {
            std::cmp::Ordering::Less
        } else if lo > r {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    })
    .is_ok()
}

// Width of one rune under the process EastAsianWidth setting (input.go rw).
pub fn char_width(c: char) -> usize {
    char_width_ea(c, east_asian())
}

fn is_ri(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}

fn hangul_syllable(c: char) -> bool {
    ('\u{AC00}'..='\u{D7A3}').contains(&c)
}

// Hangul syllable with no trailing consonant (LV). Such a syllable takes V or T after it.
fn hangul_lv(c: char) -> bool {
    hangul_syllable(c) && (c as u32 - 0xAC00).is_multiple_of(28)
}

// UAX29 GB6-GB8: L x (L|V|LV|LVT); (LV|V) x (V|T); (LVT|T) x T. Decomposed jamo are L 1100-115F, V 1160-11A7, T 11A8-11FF.
fn hangul_joins(prev: char, next: char) -> bool {
    let (p, n) = (prev as u32, next as u32);
    let l = matches!(p, 0x1100..=0x115F | 0xA960..=0xA97C);
    let v = matches!(p, 0x1160..=0x11A7 | 0xD7B0..=0xD7C6);
    let t = matches!(p, 0x11A8..=0x11FF | 0xD7CB..=0xD7FB);
    let nl = matches!(n, 0x1100..=0x115F | 0xA960..=0xA97C);
    let nv = matches!(n, 0x1160..=0x11A7 | 0xD7B0..=0xD7C6);
    let nt = matches!(n, 0x11A8..=0x11FF | 0xD7CB..=0xD7FB);
    (l && (nl || nv || hangul_syllable(next)))
        || ((hangul_lv(prev) || v) && (nv || nt))
        || (((hangul_syllable(prev) && !hangul_lv(prev)) || t) && nt)
}

// Chars that attach to the cluster before them (Extend/SpacingMark, approximated by zero width).
// ZWJ, the Hangul jamo block and the format controls are handled elsewhere.
fn is_extend(c: char) -> bool {
    match c as u32 {
        0xFE0E | 0xFE0F | 0x1F3FB..=0x1F3FF => true,
        0x1160..=0x11FF | 0xD7B0..=0xD7FB => false,
        0x200B..=0x200F | 0x2028..=0x202E | 0x2060..=0x206F | 0xFEFF => false,
        _ => c.width() == Some(0) && !c.is_control(),
    }
}

// Splits s into extended grapheme clusters, approximating UAX29 (go-runewidth uses uniseg for StringWidth).
pub fn grapheme_clusters(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut prev: Option<char> = None;
    let mut ri_run = 0usize;
    for (i, c) in s.char_indices() {
        if let Some(p) = prev {
            let join = c == '\u{200D}'
                || p == '\u{200D}'
                || is_extend(c)
                || hangul_joins(p, c)
                || (is_ri(p) && is_ri(c) && ri_run % 2 == 1);
            if !join {
                out.push(&s[start..i]);
                start = i;
            }
        }
        ri_run = if is_ri(c) { ri_run + 1 } else { 0 };
        prev = Some(c);
    }
    if prev.is_some() {
        out.push(&s[start..]);
    }
    out
}

// StringWidth per cluster: the first rune with a nonzero width decides (runewidth.go:177-190).
pub fn cluster_width_ea(cluster: &str, ea: bool) -> usize {
    cluster
        .chars()
        .map(|c| char_width_ea(c, ea))
        .find(|&w| w > 0)
        .unwrap_or(0)
}

pub fn cluster_width(cluster: &str) -> usize {
    cluster_width_ea(cluster, east_asian())
}

pub fn display_width_ea(s: &str, ea: bool) -> usize {
    grapheme_clusters(s)
        .iter()
        .map(|g| cluster_width_ea(g, ea))
        .sum()
}

// Display width of s in cells (go-runewidth StringWidth). Escapes are not counted.
pub fn display_width(s: &str) -> usize {
    display_width_ea(s, east_asian())
}

// Pieces of s: each CSI escape is one piece of width 0, plain text is split into clusters.
// The pieces cover s in order, so their byte lengths add up to s.len().
fn width_pieces(s: &str) -> Vec<(&str, usize)> {
    let mut out = Vec::new();
    let mut run = 0;
    let mut i = 0;
    while i < s.len() {
        match csi_len(s.as_bytes(), i) {
            Some(n) => {
                out.extend(
                    grapheme_clusters(&s[run..i])
                        .into_iter()
                        .map(|g| (g, cluster_width(g))),
                );
                out.push((&s[i..i + n], 0));
                i += n;
                run = i;
            }
            None => i += cur_char(s, i).len_utf8(),
        }
    }
    out.extend(
        grapheme_clusters(&s[run..])
            .into_iter()
            .map(|g| (g, cluster_width(g))),
    );
    out
}

// R13 display width in cells: CSI escapes are 0, the rest is the StringWidth of the plain text.
pub fn cell_width(s: &str) -> usize {
    display_width(&strip_sgr(s))
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
    for (piece, pw) in width_pieces(s) {
        if pw > 0 && used + pw > w - 1 {
            break;
        }
        out.push_str(piece);
        used += pw;
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
            let cw = char_width(c);
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

// R06 toolSummary (tui.go:67): the first priority key whose string value is non-empty, newlines turned
// into spaces, cut to 100 chars. Otherwise the compact JSON of the input (Go jsonStr), cut to 100 chars.
// Empty for no input.
pub fn tool_summary(input: &serde_json::Value) -> String {
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
        if let Some(v) = input.get(key).and_then(serde_json::Value::as_str)
            && !v.is_empty()
        {
            return trunc_chars(&v.replace('\n', " "), 100);
        }
    }
    let Some(obj) = input.as_object().filter(|m| !m.is_empty()) else {
        return String::new();
    };
    // Go encodes a map with sorted keys; a BTreeMap gives the same order whatever serde_json preserves.
    let sorted: std::collections::BTreeMap<&String, &serde_json::Value> = obj.iter().collect();
    trunc_chars(&serde_json::to_string(&sorted).unwrap_or_default(), 100)
}

// Cut to n chars (repl.go trunc, rune based).
pub fn trunc_chars(s: &str, n: usize) -> String {
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
#[allow(dead_code)] // unwired: usage bar width (usagebar.go barWidth) not drawn on this path
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
#[allow(dead_code)] // blocked: assistant text is linkified before wrapping; raw text not kept
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

// ---- Selection (selection.go) ----

// Splits s at cell column `at`. The first part holds escapes and cells before `at`; the second holds the rest.
fn split_cells(s: &str, at: usize) -> (String, String) {
    let mut col = 0;
    let mut i = 0;
    for (piece, pw) in width_pieces(s) {
        if col >= at {
            break;
        }
        col += pw;
        i += piece.len();
    }
    (s[..i].to_string(), s[i..].to_string())
}

// Visible text of cells [from, to) of s, without escapes (selection.go Cut + Strip).
fn cell_slice(s: &str, from: usize, to: usize) -> String {
    let (_, rest) = split_cells(s, from);
    let (mid, _) = split_cells(&rest, to - from);
    strip_sgr(&mid)
}

// R20 textPos/textSel (selection.go:18-25): content coordinates, so a selection stays on its text while
// output streams in and the view scrolls. `col` is a cell column.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextPos {
    pub line: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextSel {
    pub set: bool,
    pub anchor: TextPos,
    pub head: TextPos,
}

impl TextSel {
    // ordered (selection.go:28): start and end, in either drag direction.
    fn ordered(&self) -> (TextPos, TextPos) {
        let (a, b) = (self.anchor, self.head);
        if b.line < a.line || (b.line == a.line && b.col < a.col) {
            (b, a)
        } else {
            (a, b)
        }
    }

    // empty (selection.go:37): no selection, or press and release on the same cell.
    pub fn empty(&self) -> bool {
        let (a, b) = self.ordered();
        !self.set || a == b
    }

    // span (selection.go:44): selected cell range [from, to) on content line i of cell width w.
    pub fn span(&self, i: usize, w: usize) -> Option<(usize, usize)> {
        if self.empty() {
            return None;
        }
        let (a, b) = self.ordered();
        if i < a.line || i > b.line {
            return None;
        }
        let from = if i == a.line { a.col.min(w) } else { 0 };
        let to = if i == b.line { b.col.min(w) } else { w };
        (from < to).then_some((from, to))
    }

    // text (selection.go:63): plain text under the selection, one line per content line.
    pub fn text(&self, lines: &[String]) -> String {
        if self.empty() {
            return String::new();
        }
        let (a, b) = self.ordered();
        let mut out = Vec::new();
        for i in a.line..=b.line {
            let Some(line) = lines.get(i) else {
                break;
            };
            out.push(match self.span(i, cell_width(line)) {
                Some((from, to)) => cell_slice(line, from, to),
                None => String::new(),
            });
        }
        out.join("\n")
    }
}

// highlightLine (selection.go:81): cells [from, to) in reverse video; styling outside the range is kept.
pub fn highlight_line(line: &str, from: usize, to: usize) -> String {
    let w = cell_width(line);
    if from >= to || from >= w {
        return line.to_string();
    }
    let to = to.min(w);
    let (pre, rest) = split_cells(line, from);
    let (mid, post) = split_cells(&rest, to - from);
    format!("{pre}\x1b[7m{}\x1b[27m{post}", strip_sgr(&mid))
}

// highlightView (selection.go:92): screen row r shows content line y_offset + r, so the highlight is
// recomputed from content coordinates on every frame.
#[allow(dead_code)] // unwired: mod.rs highlights rows itself; this view-level helper is not called
pub fn highlight_view(frame: &str, lines: &[String], y_offset: usize, s: &TextSel) -> String {
    if s.empty() {
        return frame.to_string();
    }
    frame
        .split('\n')
        .enumerate()
        .map(|(r, row)| {
            let i = y_offset + r;
            match lines.get(i).and_then(|l| s.span(i, cell_width(l))) {
                Some((from, to)) => highlight_line(row, from, to),
                None => row.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---- Input box (input.go, tui.go) ----

// I32 placeholder (tui.go:879): idle hint shown in the empty input box.
pub fn input_placeholder(busy: bool) -> &'static str {
    if busy {
        "생각중 - 추가 입력은 한 번에 합쳐 처리"
    } else {
        "대기 - 프롬프트 입력"
    }
}

// I30 cells (input.go:24): cell column of byte index `pos` within one visual row.
// Only tests call it: no terminal cursor placement uses the cell column yet.
#[allow(dead_code)]
pub fn input_cursor_cells(row: &str, pos: usize) -> usize {
    cell_width(&row[..pos])
}

// I33 View (input.go:211-248): the row with a reverse-video cell at the cursor. A cursor past the end
// draws a reverse space after the text. `None` means the cursor is on another row.
pub fn input_view_row(row: &str, cursor: Option<usize>) -> String {
    let Some(pos) = cursor else {
        return row.to_string();
    };
    if pos >= row.len() {
        return format!("{row}\x1b[7m \x1b[27m");
    }
    let c = cur_char(row, pos);
    let next = pos + c.len_utf8();
    format!("{}\x1b[7m{c}\x1b[27m{}", &row[..pos], &row[next..])
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
        let out = restyle(&mark_links(input));
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
        use serde_json::json;
        assert_eq!(
            tool_summary(&json!({"file_path": "a.rs", "command": "ls -la\nx"})),
            "ls -la x"
        );
        assert_eq!(tool_summary(&json!({})), "");
        // No priority key: Go falls back to the compact JSON of the input (sorted keys, no HTML escaping).
        assert_eq!(
            tool_summary(&json!({"other": "x<a>", "n": 1})),
            r#"{"n":1,"other":"x<a>"}"#
        );
        // A non-string priority key does not count as a match.
        assert_eq!(tool_summary(&json!({"command": 5})), r#"{"command":5}"#);
        let long = "y".repeat(150);
        assert_eq!(tool_summary(&json!({"query": long})).chars().count(), 100);
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
    fn width_matches_go_runewidth() {
        // Expected (EA off, EA on) from go-runewidth v0.0.16 StringWidth, probe /private/tmp/gowidth (en2.txt, ea2.txt).
        let cases: &[(&str, usize, usize)] = &[
            ("가", 2, 2),
            ("힣", 2, 2),
            ("ㄱ", 2, 2),
            ("ᄀ", 2, 2),
            ("ᅡ", 1, 1),
            ("ᆨ", 1, 1),
            ("한글 abc", 8, 8),
            ("。", 2, 2),
            ("、", 2, 2),
            ("「」", 4, 4),
            ("「", 2, 2),
            ("─", 1, 2),
            ("│", 1, 2),
            ("┌", 1, 2),
            ("█", 1, 2),
            ("░", 1, 1),
            ("·", 1, 2),
            ("※", 1, 2),
            ("①", 1, 2),
            ("●", 1, 2),
            ("→", 1, 2),
            ("°", 1, 2),
            ("✻", 1, 1),
            ("✶", 1, 1),
            ("⠋", 1, 1),
            ("😀", 2, 2),
            ("👍🏽", 2, 2),
            ("👨\u{200d}👩\u{200d}👧", 2, 2),
            ("❤\u{fe0f}", 1, 1),
            ("✔\u{fe0f}", 1, 1),
            ("☀", 1, 1),
            ("☀\u{fe0f}", 1, 1),
            ("🇰🇷", 1, 1),
            ("🇰🇷🇺", 2, 2),
            ("🇰", 1, 1),
            ("\u{e9}", 1, 2),
            ("e\u{301}", 1, 1),
            ("e\u{301}\u{302}", 1, 1),
            ("\u{200b}", 0, 0),
            ("\u{200d}", 0, 0),
            ("a\u{200b}b", 2, 2),
            ("🏽", 2, 2),
            ("\u{fe0f}", 1, 2),
            ("\u{1100}\u{1161}", 2, 2),
            ("\u{1100}\u{1161}\u{11a8}", 2, 2),
            ("\u{1112}\u{1161}\u{11ab}", 2, 2),
            ("\u{ac00}\u{11a8}", 2, 2),
            ("\u{1161}\u{1100}", 3, 3),
            ("❤\u{fe0f}\u{200d}🔥", 1, 1),
            ("❤", 1, 1),
            ("✔", 1, 1),
        ];
        for &(s, off, on) in cases {
            assert_eq!(display_width_ea(s, false), off, "EA off: {s:?}");
            assert_eq!(display_width_ea(s, true), on, "EA on: {s:?}");
        }
        // Per-rune width (input.go rw) for the lone FE0F and the jamo block.
        assert_eq!(char_width_ea('\u{fe0f}', false), 1);
        assert_eq!(char_width_ea('\u{fe0f}', true), 2);
        assert_eq!(char_width_ea('\u{11a8}', false), 1);
    }

    #[test]
    fn width_matches_go_runewidth_sentences() {
        // Expected (EA off, EA on) from go-runewidth v0.0.16 StringWidth, probe /tmp/rwgo/linesbin (UTF-8 lines in).
        let cases: &[(&str, usize, usize)] = &[
            ("한국어 ─ 테스트 ①", 17, 19),
            ("🇰🇷🇺🇸", 2, 2),
            ("👍🏽x", 3, 3),
            ("❤️\u{200d}🔥", 1, 1),
            ("e\u{fe0f}", 1, 1),
            ("a\u{301}", 1, 1),
            ("漢字テスト", 10, 10),
            ("ＡＢＣ", 6, 6),
            ("ｶﾀｶﾅ", 4, 4),
            ("→ ● ✓ ✗ ⚠", 9, 11),
            ("⚠️ 경고", 6, 6),
            ("⏳ 대기", 7, 7),
            ("✅ 완료", 7, 7),
            ("🔥🔥", 4, 4),
            ("é中", 3, 4),
            ("日本語ひらがな", 14, 14),
            ("‼️", 1, 1),
            ("•°±×÷", 5, 10),
            ("“quoted”", 8, 10),
        ];
        for &(s, off, on) in cases {
            assert_eq!(display_width_ea(s, false), off, "EA off: {s:?}");
            assert_eq!(display_width_ea(s, true), on, "EA on: {s:?}");
        }
    }

    #[test]
    fn east_asian_locale_rule() {
        // runewidth_posix.go isEastAsian/IsEastAsian (the RUNEWIDTH_EASTASIAN override is not read here).
        assert!(!is_east_asian_locale(""));
        assert!(!is_east_asian_locale("C"));
        assert!(!is_east_asian_locale("POSIX"));
        assert!(!is_east_asian_locale("C.UTF-8"));
        assert!(!is_east_asian_locale("en_US.UTF-8"));
        assert!(!is_east_asian_locale("en_US.utf8"));
        assert!(!is_east_asian_locale("UTF-8"));
        assert!(!is_east_asian_locale("ko_KR"));
        assert!(!is_east_asian_locale("ko_KR.UTF-8@cjk_narrow"));
        // "euc-kr" has a dash, so it is not in mblenTable and stays off.
        assert!(!is_east_asian_locale("ko_KR.EUC-KR"));
        assert!(is_east_asian_locale("ko_KR.UTF-8"));
        assert!(is_east_asian_locale("ko_KR.utf8"));
        assert!(is_east_asian_locale("ja_JP.eucJP"));
        assert!(is_east_asian_locale("ko_KR.euckr"));
        assert!(is_east_asian_locale("zh_TW.Big5"));
        assert!(is_east_asian_locale("ja_JP.SJIS"));
        assert!(is_east_asian_locale("ko_KR.UTF-8@cjk"));
        assert!(!is_east_asian_locale("de_DE.ISO-8859-1"));
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
    fn sel(a: (usize, usize), h: (usize, usize)) -> TextSel {
        TextSel {
            set: true,
            anchor: TextPos {
                line: a.0,
                col: a.1,
            },
            head: TextPos {
                line: h.0,
                col: h.1,
            },
        }
    }
    fn lines_of(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn selection_text_multi_line_and_reversed() {
        // selection_test.go TestSelectionMultiLineAndReversedOrder
        let lines = lines_of(&["one", "two", "three"]);
        assert_eq!(sel((2, 2), (0, 1)).text(&lines), "ne\ntwo\nth");
        assert_eq!(sel((0, 1), (0, 1)).text(&lines), "");
    }
    #[test]
    fn selection_anchor_stays_on_content_after_append() {
        // selection_test.go TestSelectionAnchorStaysOnContentAfterAppend
        let mut lines = lines_of(&["alpha", "beta gamma", "delta"]);
        let s = sel((1, 0), (1, 4));
        assert_eq!(s.text(&lines), "beta");
        lines.extend(lines_of(&[
            "streamed one",
            "streamed two",
            "streamed three",
        ]));
        assert_eq!(s.text(&lines), "beta");
        let out = highlight_view(&lines[1..3].join("\n"), &lines, 1, &s);
        let rows: Vec<&str> = out.split('\n').collect();
        assert!(
            rows[0].contains("\x1b[7m") && !rows[1].contains("\x1b[7m"),
            "{out:?}"
        );
        let out = highlight_view(&lines[0..2].join("\n"), &lines, 0, &s);
        let rows: Vec<&str> = out.split('\n').collect();
        assert!(
            !rows[0].contains("\x1b[7m") && rows[1].contains("\x1b[7m"),
            "{out:?}"
        );
    }
    #[test]
    fn input_cursor_cells_and_view() {
        // input_test.go TestInputWidthMixed: cursor before 글 in "ab한글cd" is cell 4.
        assert_eq!(input_cursor_cells("ab한글cd", 5), 4);
        assert_eq!(input_cursor_cells("ab글cd", 2), 2);
        assert_eq!(input_view_row("abc", Some(1)), "a\x1b[7mb\x1b[27mc");
        assert_eq!(input_view_row("abc", Some(3)), "abc\x1b[7m \x1b[27m");
        assert_eq!(input_view_row("abc", None), "abc");
        assert_eq!(input_placeholder(false), "대기 - 프롬프트 입력");
        assert_eq!(
            input_placeholder(true),
            "생각중 - 추가 입력은 한 번에 합쳐 처리"
        );
    }
}
