//! Topic tracking for the main session (Go category.go): the category rule text, the `[cat: X]` and
//! `[cat-switch: X]` marker parser, the rotation decision, and a stream filter that hides marker lines from
//! the displayed text. The Go regexes are replaced by line matching: a marker counts only when it ends a
//! line, and each line holds at most one.

// Go categoryRule: appended to the main session's system prompt (<=500 tokens).
pub const CATEGORY_RULE: &str = concat!(
    "Topic tracking: end EVERY reply with one last line, exactly `[cat: X]`, where X is a 1-3 word label of the ",
    "task category the user is working in (e.g. chaude-metrics, resume-writing). Keep the label stable while the user continues work ",
    "within the same category. If the user's latest prompt starts work in a clearly different category, end with `[cat-switch: X]` instead. ",
    "Follow-ups, fixes, questions or small asides about the current work are NOT a switch. Short acknowledgements or replies ",
    "(ㅇㅇ, ok, 응, 좋아, 계속, 네, thanks) NEVER switch. Decide quickly and briefly; do no extra analysis for this.",
);

// Go ackWords: replies that never switch the category.
const ACK_WORDS: &[&str] = &[
    "ㅇㅇ", "ㅇㅋ", "응", "네", "넵", "예", "좋아", "계속", "ok", "okay", "yes", "y", "go",
    "thanks", "ㄱㄱ", "고",
];

// Go isAck: an empty, short (<=2 characters) or listed acknowledgement.
pub fn is_ack(prompt: &str) -> bool {
    let p = prompt
        .trim()
        .trim_matches(['.', '!', '?', '~', ' '])
        .to_lowercase();
    p.is_empty() || ACK_WORDS.contains(&p.as_str()) || p.chars().count() <= 2
}

// RE2 \s: tab, newline, form feed, carriage return, space.
fn is_re_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

// Characters allowed after a marker: \s, '*' and '`'.
fn is_tail(c: char) -> bool {
    is_re_space(c) || c == '*' || c == '`'
}

// Characters allowed before '[' in the inline strip: [ \t*`].
fn is_lead(c: char) -> bool {
    matches!(c, ' ' | '\t' | '*' | '`')
}

// Characters allowed before '[' at the start of a marker line: [\s*>`].
fn is_line_lead(c: char) -> bool {
    is_re_space(c) || matches!(c, '*' | '>' | '`')
}

// Finds the marker that ends the line (Go catRe / catInlineRe body). Returns the byte index of its '[',
// whether it is cat-switch, and the trimmed label. The label may not contain ']'.
fn tail_marker(line: &str) -> Option<(usize, bool, &str)> {
    let body = line.trim_end_matches(is_tail).strip_suffix(']')?;
    let lo = body.rfind(']').map_or(0, |p| p + 1);
    for (i, _) in body[lo..].match_indices('[') {
        let open = lo + i;
        let rest = &body[open + 1..];
        if let Some(label) = rest.strip_prefix("cat-switch:") {
            return Some((open, true, label.trim_matches(is_re_space)));
        }
        if let Some(label) = rest.strip_prefix("cat:") {
            return Some((open, false, label.trim_matches(is_re_space)));
        }
    }
    None
}

// The catRe boundary: the marker's '[' (after optional *>`) starts the line or follows a space or tab.
fn at_boundary(line: &str, open: usize) -> bool {
    let b = line.as_bytes();
    let mut q = open;
    while q > 0 && matches!(b[q - 1], b'*' | b'>' | b'`') {
        q -= 1;
    }
    q == 0 || matches!(b[q - 1], b' ' | b'\t')
}

// Go parseCategory: the label of the last marker line of reply, and whether it announces a switch.
// An empty label is returned as an empty string.
pub fn parse_category(reply: &str) -> (String, bool) {
    let mut found = (String::new(), false);
    for line in reply.split('\n') {
        if let Some((_, sw, label)) = tail_marker(line).filter(|m| at_boundary(line, m.0)) {
            found = (label.to_string(), sw);
        }
    }
    found
}

// Go categoryRotation: the new category label and whether to rotate. Disabled mode, short acks, a missing
// label, or a switch to the same label (case-insensitive) never rotate.
pub fn category_rotation(prompt: &str, reply: &str, cur: &str, disabled: bool) -> (String, bool) {
    let (cat, sw) = parse_category(reply);
    if cat.is_empty() {
        return (cur.to_string(), false);
    }
    let same = cat.to_lowercase() == cur.to_lowercase();
    if disabled || !sw || is_ack(prompt) || same {
        return (cat, false);
    }
    (cat, true)
}

// Go catTailRe: t is exactly one marker, with only tail characters after its ']'.
fn is_cat_tail(t: &str) -> bool {
    let Some(rest) = t
        .strip_prefix("[cat:")
        .or_else(|| t.strip_prefix("[cat-switch:"))
    else {
        return false;
    };
    rest.find(']')
        .is_some_and(|i| rest[i + 1..].chars().all(is_tail))
}

// Go markerPrefix: t is the start of an unfinished [cat:..] or [cat-switch:..] marker.
fn marker_prefix(t: &str) -> bool {
    if t.contains([']', '\n']) {
        return false;
    }
    for m in ["[cat:", "[cat-switch:"] {
        if m.starts_with(t) || t.starts_with(m) {
            return true;
        }
    }
    false
}

// Go stripInline: removes a marker at the end of a line, with the spaces before it. Keeps the line ending.
fn strip_inline(ln: &str) -> String {
    let body = ln.trim_end_matches(['\r', '\n']);
    let nl = &ln[body.len()..];
    let kept = match tail_marker(body) {
        Some((open, _, _)) => {
            let b = body.as_bytes();
            let mut q = open;
            while q > 0 && is_lead(b[q - 1] as char) {
                q -= 1;
            }
            &body[..q]
        }
        None => body,
    };
    format!("{kept}{nl}")
}

// Go stripMarker: drops a whole marker line, otherwise strips an inline marker.
fn strip_marker(ln: &str) -> String {
    let body = ln.trim_end_matches(['\r', '\n']);
    if is_cat_tail(body.trim_start_matches(is_line_lead)) {
        return String::new();
    }
    strip_inline(ln)
}

// Go markerFilter: hides [cat: X] / [cat-switch: X] from streamed text. A trailing partial line that may
// still become a marker is held back until it is complete. mid: text of the current line was already shown.
#[derive(Debug, Default)]
pub struct MarkerFilter {
    pend: String,
    mid: bool,
}

impl MarkerFilter {
    // Go Feed: returns the text that is safe to display after adding chunk.
    pub fn feed(&mut self, chunk: &str) -> String {
        self.pend.push_str(chunk);
        let mut out = String::new();
        if let Some(i) = self.pend.rfind('\n') {
            let tail = self.pend.split_off(i + 1);
            let complete = std::mem::replace(&mut self.pend, tail);
            for ln in complete.split_inclusive('\n') {
                let shown = if self.mid {
                    strip_inline(ln)
                } else {
                    strip_marker(ln)
                };
                out.push_str(&shown);
                self.mid = false;
            }
        }
        let hold = self
            .pend
            .rfind('[')
            .filter(|&j| marker_prefix(&self.pend[j..]) || is_cat_tail(&self.pend[j..]));
        match hold {
            Some(mut j) => {
                let b = self.pend.as_bytes();
                while j > 0 && is_lead(b[j - 1] as char) {
                    j -= 1;
                }
                out.push_str(&self.pend[..j]);
                self.pend = self.pend[j..].to_string();
            }
            None => {
                out.push_str(&self.pend);
                self.pend.clear();
            }
        }
        if !out.is_empty() {
            self.mid = !out.ends_with('\n');
        }
        out
    }

    // Go Flush: the held remainder, dropped when it is a marker line.
    pub fn flush(&mut self) -> String {
        let p = std::mem::take(&mut self.pend);
        if self.mid {
            strip_inline(&p)
        } else {
            strip_marker(&p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go TestParseCategory
    #[test]
    fn parse_category_takes_last_marker_line() {
        assert_eq!(
            parse_category("답변입니다\n\n[cat-switch: resume]\n"),
            ("resume".to_string(), true)
        );
        assert_eq!(
            parse_category("x\n[cat: chaude]"),
            ("chaude".to_string(), false)
        );
        assert_eq!(parse_category("no marker").0, "");
    }

    // Go TestCategoryRotation
    #[test]
    fn category_rotation_rules() {
        let reply = "done\n[cat-switch: resume]";
        assert_eq!(
            category_rotation("이력서 써줘", reply, "chaude", false),
            ("resume".to_string(), true)
        );
        assert!(
            !category_rotation("ㅇㅇ", reply, "chaude", false).1,
            "ack must not rotate"
        );
        assert!(
            !category_rotation("이력서 써줘", reply, "chaude", true).1,
            "disabled must not rotate"
        );
        assert!(
            !category_rotation("계속 해", "ok\n[cat: chaude]", "chaude", false).1,
            "same category must not rotate"
        );
        assert!(
            !category_rotation("이력서 써줘", reply, "resume", false).1,
            "same label must not rotate"
        );
    }

    // Go TestCategoryRuleSize (~500 tokens upper bound)
    #[test]
    fn category_rule_fits_budget() {
        assert!(
            CATEGORY_RULE.len() <= 1500,
            "rule too long: {}",
            CATEGORY_RULE.len()
        );
    }

    // Go TestMarkerFilter
    #[test]
    fn marker_filter_hides_split_marker_line() {
        let mut f = MarkerFilter::default();
        let mut got = String::new();
        for c in ["안녕", "하세요\n\n[ca", "t-switch: re", "sume]"] {
            got.push_str(&f.feed(c));
        }
        got.push_str(&f.flush());
        assert_eq!(got, "안녕하세요\n\n");

        let mut f = MarkerFilter::default();
        let got = f.feed("see [link] here\n[x] y") + &f.flush();
        assert_eq!(got, "see [link] here\n[x] y");
    }

    // Go TestMarkerFilterInline
    #[test]
    fn marker_filter_hides_inline_marker() {
        let mut f = MarkerFilter::default();
        let mut raw = String::new();
        let mut got = String::new();
        for c in [
            "14개입니다. [ca",
            "t: 테스트 수",
            " 확인]",
            "\n다음 줄 [cat-switch: x]",
        ] {
            raw.push_str(c);
            got.push_str(&f.feed(c));
        }
        got.push_str(&f.flush());
        assert_eq!(got, "14개입니다.\n다음 줄");
        assert_eq!(parse_category(&raw), ("x".to_string(), true));

        let mut f = MarkerFilter::default();
        assert_eq!(f.feed("a [b] c [d") + &f.flush(), "a [b] c [d");
    }
}
