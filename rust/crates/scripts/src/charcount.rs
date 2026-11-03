//! charcount (Go: charcount.go). Counts characters of a text or markdown file.

use std::io::Read;

use crate::common::{fail, parse, py_is_space, py_strip, rune_len};

/// Go wsClass: RE2 `\s`, `\p{Z}`, U+0085 and 0x1c..=0x1f.
fn is_ws(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{0c}'
            | '\r'
            | ' '
            | '\u{85}'
            | '\u{1c}'..='\u{1f}'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
    )
}

fn ws_run(t: &[char], from: usize) -> usize {
    t[from..].iter().take_while(|&&c| is_ws(c)).count()
}

/// First line matching `^(#+)ws*SECTION ws*$` (Go multiline regexp).
/// Returns (heading level, end of the match, after trailing ws).
fn find_heading(t: &[char], section: &[char]) -> Option<(usize, usize)> {
    for p in 0..t.len() {
        if p > 0 && t[p - 1] != '\n' {
            continue;
        }
        let hashes = t[p..].iter().take_while(|&&c| c == '#').count();
        for k in (1..=hashes).rev() {
            let base = p + k;
            for s in (base..=base + ws_run(t, base)).rev() {
                if !t[s..].starts_with(section) {
                    continue;
                }
                let q = s + section.len();
                for r in (q..=q + ws_run(t, q)).rev() {
                    if r == t.len() || t[r] == '\n' {
                        return Some((k, r));
                    }
                }
            }
        }
    }
    None
}

/// First heading line at or after `from` (Go `(?m)^#{1,level}ws`).
fn find_end(t: &[char], from: usize, level: usize) -> Option<usize> {
    for p in from..t.len() {
        if p > 0 && t[p - 1] != '\n' {
            continue;
        }
        let h = t[p..].iter().take_while(|&&c| c == '#').count();
        if (1..=level).contains(&h) && p + h < t.len() && is_ws(t[p + h]) {
            return Some(p);
        }
    }
    None
}

/// Counts for `raw`, formatted like the Go output. Err carries the message for exit code 1.
pub fn count_text(raw: &str, section: &str, limit: i64) -> Result<String, String> {
    let text = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let text: String = if section.is_empty() {
        text
    } else {
        let t: Vec<char> = text.chars().collect();
        let sec: Vec<char> = section.chars().collect();
        let Some((level, from)) = find_heading(&t, &sec) else {
            return Err(format!("section not found: {section}"));
        };
        let end = find_end(&t, from, level).unwrap_or(t.len());
        t[from..end].iter().collect()
    };
    let text = py_strip(&text);

    let with_nl = rune_len(text);
    let no_nl = text.chars().filter(|&c| c != '\r' && c != '\n').count();
    let no_ws = text.chars().filter(|&c| !py_is_space(c)).count();
    let hangul2: usize = text
        .chars()
        .filter(|&c| c != '\r')
        .map(|c| if c as u32 > 127 { 2 } else { 1 })
        .sum();
    let mut out = format!(
        "공백 포함(줄바꿈 포함): {with_nl}\n공백 포함(줄바꿈 제외): {no_nl}\n공백 제외: {no_ws}\n바이트(UTF-8): {} / 바이트(한글 2byte): {hangul2}\n",
        text.len()
    );
    if limit != 0 {
        let diff = limit - no_nl as i64;
        let st = if diff < 0 { "OVER" } else { "OK" };
        out.push_str(&format!("제한 {limit}: {st} ({diff:+})\n"));
    }
    Ok(out)
}

pub fn run(args: &[String]) -> i32 {
    let flags = match parse("charcount", &["section", "limit"], args) {
        Ok(f) => f,
        Err(rc) => return rc,
    };
    let limit = match flags.int("limit", 0) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    if flags.pos.len() != 1 {
        eprintln!("usage: charcount FILE [--section HEADING] [--limit N]");
        return 2;
    }
    let raw = if flags.pos[0] == "-" {
        let mut buf = Vec::new();
        match std::io::stdin().read_to_end(&mut buf) {
            Ok(_) => buf,
            Err(e) => return fail(&e.to_string()),
        }
    } else {
        match std::fs::read(&flags.pos[0]) {
            Ok(b) => b,
            Err(e) => return fail(&e.to_string()),
        }
    };
    match count_text(
        &String::from_utf8_lossy(&raw),
        &flags.str("section", ""),
        limit,
    ) {
        Ok(out) => {
            print!("{out}");
            0
        }
        Err(msg) => fail(&msg),
    }
}
