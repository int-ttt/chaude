// Session groups for /resume (Go resume_group.go, resume.go): the Claude Code transcripts of one project
// joined into conversations, the harness /clear marks that split them, and the term matching of /resume.
// Times are unix nanoseconds (i128) so marks and transcript times compare directly.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::import_claude::{Stamp, read_claude_transcript};
use crate::import_view::trunc;

const NS: i128 = 1_000_000_000;
// Consecutive files further apart than this start a new group.
pub const GROUP_MAX_GAP_NS: i128 = 48 * 3600 * NS;
// Jaccard word overlap of the first prompts needed to join two files.
pub const GROUP_MIN_OVERLAP: f64 = 0.2;
// In the record dir: one "<unixnano>\t<cwd>" line per harness /clear.
pub const CLEAR_MARK_FILE: &str = ".clear-marks";

// Unix nanoseconds of a time.
pub fn unix_ns(t: SystemTime) -> i128 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos() as i128,
        Err(e) => -(e.duration().as_nanos() as i128),
    }
}

fn stamp_ns((secs, nanos): Stamp) -> i128 {
    secs as i128 * NS + nanos as i128
}

// One Claude Code transcript summarized for the /resume list (Go sessionInfo).
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub path: PathBuf,
    pub modified: SystemTime,
    pub turns: usize,
    pub first: String,
    pub last: String,
    // First event time (None: the modification time is used).
    pub start: Option<i128>,
    pub cwd: String,
    pub name: String,
    // cleared: a user /clear is in the file, so the next file starts a new group.
    // clear_head: the /clear is before the first turn, so this file starts a new group.
    pub cleared: bool,
    pub clear_head: bool,
}

impl SessionInfo {
    // Go sessionStart: the first event time, or the modification time when the file has none.
    fn start_ns(&self) -> i128 {
        self.start.unwrap_or_else(|| unix_ns(self.modified))
    }

    fn mod_ns(&self) -> i128 {
        unix_ns(self.modified)
    }
}

// One conversation made of consecutive session files, newest first. Picking it resumes the newest file;
// the older ones are merged in as history.
#[derive(Debug, Clone)]
pub struct SessionGroup {
    pub files: Vec<SessionInfo>,
}

impl SessionGroup {
    // The id list /import loads: all files of the group, oldest first.
    pub fn key(&self) -> String {
        self.files
            .iter()
            .rev()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    // The newest file: its id is the session the group resumes.
    pub fn latest(&self) -> &SessionInfo {
        &self.files[0]
    }

    pub fn turns(&self) -> usize {
        self.files.iter().map(|s| s.turns).sum()
    }

    // date formats a modification time as local "yyyy-mm-dd hh:mm".
    pub fn label(&self, date: &dyn Fn(SystemTime) -> String) -> String {
        let newest = self.latest();
        let oldest = &self.files[self.files.len() - 1];
        let mut s = format!(
            "{}  {:>3} turns  {} … {}",
            date(newest.modified),
            self.turns(),
            trunc(&oldest.first, 40),
            trunc(&newest.last, 40)
        );
        if self.files.len() > 1 {
            s.push_str(&format!("  ({} files)", self.files.len()));
        }
        s
    }

    // A term (lower case) is a substring of a member's id, name, first/last prompt or cwd (or of the
    // project cwd). Terms of 4+ letters also match a word one edit away (typos).
    fn matches_term(&self, term: &str, cwd: &str) -> bool {
        self.files.iter().any(|s| {
            [
                s.id.as_str(),
                s.name.as_str(),
                s.first.as_str(),
                s.last.as_str(),
                s.cwd.as_str(),
                cwd,
            ]
            .iter()
            .any(|f| {
                f.to_lowercase().contains(term)
                    || (term.chars().count() >= 4
                        && prompt_words(f).iter().any(|w| levenshtein(term, w) <= 1))
            })
        })
    }
}

// Go matchGroups: keeps the groups matching every term (case-insensitive, whitespace-split). No terms keeps all.
pub fn match_groups(gs: &[SessionGroup], cwd: &str, terms: &[String]) -> Vec<SessionGroup> {
    let ts: Vec<String> = terms
        .iter()
        .flat_map(|t| {
            t.to_lowercase()
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect();
    if ts.is_empty() {
        return gs.to_vec();
    }
    let cwd = cwd.to_lowercase();
    gs.iter()
        .filter(|g| ts.iter().all(|t| g.matches_term(t, &cwd)))
        .cloned()
        .collect()
}

// Go levenshtein: edit distance between a and b (chars).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let ra: Vec<char> = a.chars().collect();
    let rb: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=rb.len()).collect();
    for i in 1..=ra.len() {
        let mut cur = vec![i; rb.len() + 1];
        for j in 1..=rb.len() {
            let cost = usize::from(ra[i - 1] != rb[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[rb.len()]
}

// Go promptWords: the lower-case word set of a prompt (letters and digits, any script).
fn prompt_words(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

// Go wordJaccard: |A∩B| / |A∪B| over the word sets (0 when both are empty).
pub fn word_jaccard(a: &str, b: &str) -> f64 {
    let (wa, wb) = (prompt_words(a), prompt_words(b));
    let inter = wa.intersection(&wb).count();
    let union = wa.len() + wb.len() - inter;
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

// Go markBetween: whether a /clear mark lies in [from, to].
fn mark_between(marks: &[i128], from: i128, to: i128) -> bool {
    marks.iter().any(|&t| from <= t && t <= to)
}

// Go continues: whether cur belongs to the group that prev (its predecessor) ends. A /clear always ends a
// group: prev.cleared or cur.clear_head, or a harness /clear mark between the two files.
fn continues(prev: &SessionInfo, cur: &SessionInfo, marks: &[i128]) -> bool {
    if prev.cleared || cur.clear_head {
        return false;
    }
    if mark_between(marks, prev.mod_ns(), cur.start_ns()) {
        return false;
    }
    if prev.cwd != cur.cwd {
        return false;
    }
    if cur.start_ns() - prev.mod_ns() > GROUP_MAX_GAP_NS {
        return false;
    }
    word_jaccard(&prev.first, &cur.first) >= GROUP_MIN_OVERLAP
}

// Go groupSessions: joins consecutive session files into groups, newest group first. marks are the harness
// /clear times of this project (see load_clear_marks).
pub fn group_sessions(l: &[SessionInfo], marks: &[i128]) -> Vec<SessionGroup> {
    let mut asc: Vec<&SessionInfo> = l.iter().collect();
    asc.sort_by_key(|s| s.start_ns());
    let mut groups: Vec<Vec<SessionInfo>> = Vec::new();
    for s in asc {
        match groups.last_mut() {
            Some(g) if continues(g.last().expect("group is not empty"), s, marks) => {
                g.push(s.clone())
            }
            _ => groups.push(vec![s.clone()]),
        }
    }
    let mut out: Vec<SessionGroup> = groups
        .into_iter()
        .map(|mut g| {
            g.sort_by_key(|s| std::cmp::Reverse(s.modified));
            SessionGroup { files: g }
        })
        .collect();
    out.sort_by_key(|g| std::cmp::Reverse(g.latest().modified));
    out
}

// Go listClaudeSessions: the transcripts in dir with at least one turn, newest first.
pub fn list_sessions(dir: &Path) -> Vec<SessionInfo> {
    let Ok(ents) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ent in ents.flatten() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(modified) = ent.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        let Ok(imp) = read_claude_transcript(&path) else {
            continue;
        };
        if imp.turns.is_empty() {
            continue;
        }
        let last = &imp.turns[imp.turns.len() - 1].0;
        out.push(SessionInfo {
            id: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string(),
            path,
            modified,
            turns: imp.turns.len(),
            first: one_line(&imp.turns[0].0),
            last: one_line(last),
            start: imp.start.map(stamp_ns),
            cwd: imp.cwd,
            name: imp.name,
            cleared: imp.cleared,
            clear_head: imp.clear_head,
        });
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.modified));
    out
}

// Go oneLine: the text with runs of whitespace collapsed to one space.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// Go groupList: the /resume session answer for cwd, from its Claude Code project dir. Errors are the
// messages the plain loop prints.
pub fn group_list(
    claude_dir: &Path,
    marks_path: &Path,
    cwd: &str,
    terms: &[String],
) -> Result<Vec<SessionGroup>, String> {
    let l = list_sessions(claude_dir);
    if l.is_empty() {
        return Err(format!(
            "no Claude Code sessions in {}",
            claude_dir.display()
        ));
    }
    let marks = load_clear_marks(marks_path, cwd);
    let gs = match_groups(&group_sessions(&l, &marks), cwd, terms);
    if gs.is_empty() {
        return Err(format!("no session matches {}", terms.join(" ")));
    }
    Ok(gs)
}

// Go clearMarkPath: the /clear mark file of a record dir.
pub fn clear_mark_path(record_dir: &Path) -> PathBuf {
    record_dir.join(CLEAR_MARK_FILE)
}

// Go recordClearMark: appends one "<unixnano>\t<cwd>" line to path at time at (unix nanoseconds).
pub fn record_clear_mark(path: &Path, cwd: &str, at_ns: i128) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{at_ns}\t{cwd}")
}

// Go clearMark: records a harness /clear in the project at cwd, now. A /resume split happens there.
pub fn clear_mark_now(record_dir: &Path, cwd: &str) -> io::Result<()> {
    let now = unix_ns(SystemTime::now());
    record_clear_mark(&clear_mark_path(record_dir), cwd, now)
}

// Go loadClearMarks: the marks of cwd in path (empty when the file is missing or has none).
pub fn load_clear_marks(path: &Path, cwd: &str) -> Vec<i128> {
    let Ok(data) = fs::read_to_string(path) else {
        return Vec::new();
    };
    data.split('\n')
        .filter_map(|line| {
            let (ts, c) = line.split_once('\t')?;
            if c != cwd {
                return None;
            }
            ts.parse::<i64>().ok().map(i128::from)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_claude::parse_rfc3339;
    use crate::testutil::TempDir;
    use serde_json::json;
    use std::time::Duration;

    // (a prompts, b prompts, gap secs, clear marks, want groups)
    type Case = (
        &'static [&'static str],
        &'static [&'static str],
        i64,
        Vec<i128>,
        usize,
    );

    // Go grpT0: the base time of the test sessions (2026-10-30 09:00 UTC), in unix seconds.
    fn t0() -> i64 {
        parse_rfc3339("2026-10-30T09:00:00Z").expect("base time").0
    }

    // RFC3339 UTC text of unix seconds (civil date from days since 1970).
    fn iso(secs: i64) -> String {
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400);
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
            rem / 3600,
            rem % 3600 / 60,
            rem % 60
        )
    }

    fn at(secs: i64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs as u64)
    }

    // Go grpSess: one session file. Each prompt is a user turn (2 min apart from start) with an assistant
    // reply. A prompt of "/clear" is the Claude Code /clear command instead.
    fn write_session(dir: &Path, id: &str, start: i64, modified: i64, prompts: &[&str]) {
        let mut body = String::new();
        for (i, p) in prompts.iter().enumerate() {
            let ts = iso(start + 120 * i as i64);
            if *p == "/clear" {
                let line = json!({"type": "user", "timestamp": ts,
                    "message": {"role": "user", "content": "<command-name>/clear</command-name>"}});
                body.push_str(&format!("{line}\n"));
                continue;
            }
            let user = json!({"type": "user", "timestamp": ts,
                "message": {"role": "user", "content": p}});
            let asst = json!({"type": "assistant", "timestamp": ts,
                "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}]}});
            body.push_str(&format!("{user}\n{asst}\n"));
        }
        let path = dir.join(format!("{id}.jsonl"));
        fs::write(&path, body).expect("write session");
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("open session")
            .set_modified(at(modified))
            .expect("set mtime");
    }

    fn info(id: &str, first: &str, last: &str) -> SessionInfo {
        SessionInfo {
            id: id.to_string(),
            path: PathBuf::new(),
            modified: UNIX_EPOCH,
            turns: 1,
            first: first.to_string(),
            last: last.to_string(),
            start: None,
            cwd: String::new(),
            name: String::new(),
            cleared: false,
            clear_head: false,
        }
    }

    fn group(files: Vec<SessionInfo>) -> SessionGroup {
        SessionGroup { files }
    }

    #[test]
    fn group_sessions_joins_or_splits_by_clear_gap_and_overlap() {
        let t0 = t0();
        let mark = i128::from(t0 + 300) * NS;
        let cases: [Case; 7] = [
            (&["parser plan"], &["parser plan step"], 600, vec![], 1),
            (
                &["parser plan", "/clear"],
                &["parser plan step"],
                600,
                vec![],
                2,
            ),
            (
                &["parser plan"],
                &["/clear", "parser plan step"],
                600,
                vec![],
                2,
            ),
            (
                &["parser plan"],
                &["parser plan step"],
                26 * 3600,
                vec![],
                1,
            ),
            (
                &["parser plan"],
                &["parser plan step"],
                50 * 3600,
                vec![],
                2,
            ),
            (&["parser plan"], &["weather tomorrow"], 600, vec![], 2),
            (&["parser plan"], &["parser plan step"], 600, vec![mark], 2),
        ];
        for (i, (a, b, gap, marks, want)) in cases.into_iter().enumerate() {
            let d = TempDir::new(&format!("rg-group-{i}"));
            write_session(d.path(), "a", t0, t0 + 300, a);
            let bs = t0 + gap;
            write_session(d.path(), "b", bs, bs + 300, b);
            let gs = group_sessions(&list_sessions(d.path()), &marks);
            assert_eq!(gs.len(), want, "case {i}: {a:?} then {b:?}, gap {gap}s");
        }
    }

    #[test]
    fn group_order_and_key() {
        let t0 = t0();
        let d = TempDir::new("rg-order");
        write_session(d.path(), "a", t0, t0 + 300, &["parser plan"]);
        write_session(d.path(), "b", t0 + 600, t0 + 900, &["parser plan step"]);
        let gs = group_sessions(&list_sessions(d.path()), &[]);
        assert_eq!(gs.len(), 1);
        assert_eq!(gs[0].key(), "a,b", "key is time order, oldest first");
        assert_eq!(gs[0].latest().id, "b");
        assert_eq!(gs[0].turns(), 2);
    }

    #[test]
    fn match_groups_terms_typos_and_cwd() {
        let gs = vec![
            group(vec![info("s1", "claude harness refactor", "fix resume")]),
            group(vec![info("s2", "weather notes", "tomorrow")]),
        ];
        let cases: [(&[&str], usize); 8] = [
            (&["claude"], 1),
            (&["clade"], 1),             // typo: one edit from "claude"
            (&["Weather"], 1),           // case-insensitive
            (&["s2"], 1),                // session id
            (&["proj"], 2),              // cwd is part of the haystack
            (&["claude", "weather"], 0), // all terms must match
            (&["claude", "resume"], 1),
            (&["nothing"], 0),
        ];
        for (terms, want) in cases {
            let terms: Vec<String> = terms.iter().map(|t| t.to_string()).collect();
            let got = match_groups(&gs, "/w/proj", &terms);
            assert_eq!(got.len(), want, "terms {terms:?}");
        }
    }

    #[test]
    fn clear_marks_are_per_cwd_and_missing_file_is_empty() {
        let d = TempDir::new("rg-marks");
        let p = d.path().join("marks.txt");
        let at = 1_700_000_000_000_000_005i128;
        record_clear_mark(&p, "/w/a", at).unwrap();
        record_clear_mark(&p, "/w/b", at + NS).unwrap();
        assert_eq!(load_clear_marks(&p, "/w/a"), vec![at]);
        assert!(load_clear_marks(&p, "/w/c").is_empty());
        assert!(load_clear_marks(&d.path().join("none"), "/w/a").is_empty());
    }

    #[test]
    fn group_list_reports_missing_sessions_and_no_match() {
        let t0 = t0();
        let d = TempDir::new("rg-list");
        let claude = d.path().join("claude");
        fs::create_dir_all(&claude).unwrap();
        let marks = d.path().join("marks.txt");
        let none: Vec<String> = Vec::new();
        assert_eq!(
            group_list(&claude, &marks, "/w/proj", &none).unwrap_err(),
            format!("no Claude Code sessions in {}", claude.display())
        );
        write_session(&claude, "a", t0, t0 + 300, &["parser plan"]);
        let zzz = vec!["zzz".to_string()];
        assert_eq!(
            group_list(&claude, &marks, "/w/proj", &zzz).unwrap_err(),
            "no session matches zzz"
        );
        assert_eq!(
            group_list(&claude, &marks, "/w/proj", &none).unwrap().len(),
            1
        );
    }

    #[test]
    fn label_and_word_helpers() {
        let g = group(vec![info("b", "two", "last b"), info("a", "one", "last a")]);
        let date = |_: SystemTime| "2026-10-30 09:00".to_string();
        assert_eq!(
            g.label(&date),
            "2026-10-30 09:00    2 turns  one … last b  (2 files)"
        );
        assert_eq!(levenshtein("clade", "claude"), 1);
        assert_eq!(word_jaccard("parser plan", "parser plan step"), 2.0 / 3.0);
        assert_eq!(word_jaccard("", ""), 0.0);
    }
}
