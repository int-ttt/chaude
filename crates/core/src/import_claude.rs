//! /import --from-claude (Go importclaude.go, resume.go claudeProjectDir): reads a plain Claude Code transcript
//! and builds the pending handoff snapshot from it. Pure data: the caller installs the turns, files and snapshot
//! into the session and logs the record (see import_record).

use crate::error::Error;
use crate::import_view::{Turn, ViewBuilder, bget, list, parse_event, sget, trunc};
use crate::osenv::home_dir;
use crate::rotate::{PREAMBLE, extract_reco, with_reco};
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

// Marker Claude Code writes for the /clear command (user side).
const CLEAR_MARK: &str = "<command-name>/clear</command-name>";

// Snapshot defaults (Go repl.go flags: -tail-turns 4, -snapshot-chars 24000) and the per-side turn cap.
pub const DEFAULT_TAIL_TURNS: usize = 4;
pub const DEFAULT_SNAPSHOT_CHARS: usize = 24_000;
const TURN_SIDE_CHARS: usize = 1500;

// (unix seconds, nanoseconds) in UTC. Tuple order is time order.
pub type Stamp = (i64, u32);

// Result of reading one transcript. turns is the text-only snapshot input; view is the display transcript.
#[derive(Debug, Default)]
pub struct ClaudeImport {
    pub path: PathBuf,
    pub turns: Vec<Turn>,
    pub view: Vec<Turn>,
    pub subagents: usize,
    pub files: Vec<String>,
    pub reco: String,
    // First event timestamp (None when absent or unparsable).
    pub start: Option<Stamp>,
    pub cwd: String,
    // Summary title (for the /resume query).
    pub name: String,
    // cleared: a user /clear is in the file. clear_head: it came before any turn (new session start).
    pub cleared: bool,
    pub clear_head: bool,
    // /resume digest written by resume_note::write; its hint goes into the first input.
    pub note_path: Option<PathBuf>,
}

// Go claudeProjectDir: ~/.claude/projects/<project key of cwd>. An empty cwd means the current directory.
pub fn claude_project_dir(cwd: &str) -> PathBuf {
    let cwd = if cwd.is_empty() {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        cwd.to_string()
    };
    home_dir()
        .join(".claude")
        .join("projects")
        .join(project_key(&cwd))
}

// Go nonAlnum: every character other than ASCII letters and digits becomes one "-".
pub fn project_key(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

// Maps "latest", a session id or a path to a .jsonl file. A value with ".jsonl" or a separator is a path.
pub fn resolve_claude_transcript(arg: &str, cwd: &str) -> Result<PathBuf, Error> {
    if arg.ends_with(".jsonl") || arg.contains(MAIN_SEPARATOR) {
        return Ok(PathBuf::from(arg));
    }
    let dir = claude_project_dir(cwd);
    if arg != "latest" {
        return Ok(dir.join(format!("{arg}.jsonl")));
    }
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("jsonl") {
                continue;
            }
            let Ok(mt) = e.metadata().and_then(|m| m.modified()) else {
                continue;
            };
            if best.as_ref().is_none_or(|(_, bt)| mt > *bt) {
                best = Some((p, mt));
            }
        }
    }
    best.map(|(p, _)| p)
        .ok_or_else(|| Error::Msg(format!("no transcripts in {}", dir.display())))
}

// Resolves arg (one id/path/"latest", or a comma-separated list for a /resume group) and reads it.
// Several files are merged in time order; the returned path is the newest file's.
pub fn load_import_arg(arg: &str, cwd: &str) -> Result<(PathBuf, ClaudeImport), Error> {
    let mut imps = Vec::new();
    for a in arg.split(',') {
        let path = resolve_claude_transcript(a, cwd)?;
        imps.push(read_claude_transcript(&path)?);
    }
    if imps.len() == 1 {
        let imp = imps.remove(0);
        return Ok((imp.path.clone(), imp));
    }
    imps.sort_by_key(|a| a.start);
    let mut merged = ClaudeImport {
        start: imps[0].start,
        ..Default::default()
    };
    for imp in &imps {
        merged.turns.extend(imp.turns.iter().cloned());
        merged.view.extend(imp.view.iter().cloned());
        merged.subagents += imp.subagents;
        for f in &imp.files {
            if !merged.files.contains(f) {
                merged.files.push(f.clone());
            }
        }
        if !imp.reco.is_empty() {
            merged.reco = imp.reco.clone();
        }
        if !imp.cwd.is_empty() {
            merged.cwd = imp.cwd.clone();
        }
        if !imp.name.is_empty() {
            merged.name = imp.name.clone();
        }
    }
    let newest = imps.last().map(|i| i.path.clone()).unwrap_or_default();
    merged.path = newest.clone();
    Ok((newest, merged))
}

// Streams the file (lines can be over 100 KB) and extracts turns, edited files, the last recommendation,
// the start time, the cwd, the summary title, and the /clear markers.
pub fn read_claude_transcript(path: &Path) -> Result<ClaudeImport, Error> {
    let err = |e: std::io::Error| Error::Msg(format!("{}: {e}", path.display()));
    let f = File::open(path).map_err(err)?;
    let mut rd = BufReader::with_capacity(1 << 20, f);
    let mut imp = ClaudeImport {
        path: path.to_path_buf(),
        ..Default::default()
    };
    let mut vb = ViewBuilder::new(path);
    let mut cur_user = String::new();
    let mut asst: Vec<String> = Vec::new();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let n = rd.read_until(b'\n', &mut buf).map_err(err)?;
        if n == 0 {
            break;
        }
        let line = String::from_utf8_lossy(&buf).into_owned();
        if line.trim().is_empty() {
            continue;
        }
        let ev = parse_event(&line);
        vb.consume(&ev);
        let t = sget(&ev, "type");
        let side = bget(&ev, "isSidechain");
        let meta = bget(&ev, "isMeta");
        let msg = ev.get("message").unwrap_or(&Value::Null);
        if imp.cwd.is_empty() {
            imp.cwd = sget(&ev, "cwd").to_string();
        }
        if t == "summary" && imp.name.is_empty() {
            imp.name = sget(&ev, "summary").to_string();
        }
        if imp.start.is_none() {
            imp.start = parse_rfc3339(sget(&ev, "timestamp"));
        }
        if t == "user" && !side && line.contains(CLEAR_MARK) {
            imp.cleared = true;
            imp.clear_head = imp.clear_head || (imp.turns.is_empty() && cur_user.is_empty());
        }
        if side || meta || (t != "user" && t != "assistant") {
            continue;
        }
        if t == "user" {
            let content = msg.get("content");
            match content {
                Some(Value::String(s)) if !s.trim().is_empty() && !s.starts_with('<') => {
                    flush_turn(&mut imp.turns, &mut cur_user, &mut asst);
                    cur_user = s.clone();
                }
                _ => {
                    for b in list(content) {
                        let text = sget(b, "text");
                        if sget(b, "type") == "text"
                            && cur_user.is_empty()
                            && !text.starts_with('<')
                        {
                            cur_user = text.to_string();
                        }
                    }
                }
            }
        } else {
            for b in list(msg.get("content")) {
                match sget(b, "type") {
                    "text" => {
                        let tx = sget(b, "text").trim();
                        if !tx.is_empty() {
                            asst.push(tx.to_string());
                            let rc = extract_reco(tx);
                            if !rc.is_empty() {
                                imp.reco = rc;
                            }
                        }
                    }
                    "tool_use" => {
                        if matches!(sget(b, "name"), "Write" | "Edit" | "NotebookEdit") {
                            let input = b.get("input").unwrap_or(&Value::Null);
                            let fp = sget(input, "file_path");
                            if !fp.is_empty() && !imp.files.iter().any(|f| f == fp) {
                                imp.files.push(fp.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    flush_turn(&mut imp.turns, &mut cur_user, &mut asst);
    if imp.turns.is_empty() {
        return Err(Error::Msg(format!(
            "no user/assistant turns in {}",
            path.display()
        )));
    }
    imp.view = vb.finish();
    imp.subagents = vb.n_sub;
    Ok(imp)
}

fn flush_turn(turns: &mut Vec<Turn>, cur_user: &mut String, asst: &mut Vec<String>) {
    if !cur_user.is_empty() {
        turns.push((std::mem::take(cur_user), asst.join("\n")));
    }
    asst.clear();
}

// Go parseTime (RFC3339Nano, the form Claude Code writes): "YYYY-MM-DDTHH:MM:SS[.frac](Z|±HH:MM)".
pub fn parse_rfc3339(s: &str) -> Option<Stamp> {
    let b = s.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let t = s.get(r)?;
        if t.bytes().all(|c| c.is_ascii_digit()) {
            t.parse().ok()
        } else {
            None
        }
    };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 59 {
        return None;
    }
    let mut i = 19;
    let mut nanos: u32 = 0;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
        // Keep the first nine digits as nanoseconds.
        for (k, c) in b[start..i].iter().take(9).enumerate() {
            nanos += u32::from(c - b'0') * 10u32.pow(8 - k as u32);
        }
    }
    let offset = match &b[i..] {
        [b'Z'] => 0,
        [sign @ (b'+' | b'-'), rest @ ..] if rest.len() == 5 && rest[2] == b':' => {
            let oh = num_bytes(&rest[0..2])?;
            let om = num_bytes(&rest[3..5])?;
            if oh > 23 || om > 59 {
                return None;
            }
            let v = oh * 3600 + om * 60;
            if *sign == b'+' { v } else { -v }
        }
        _ => return None,
    };
    let days = days_from_civil(y, m, d);
    let secs = days * 86_400 + hh * 3600 + mm * 60 + ss - offset;
    Some((secs, nanos))
}

fn num_bytes(b: &[u8]) -> Option<i64> {
    if b.iter().all(|c| c.is_ascii_digit()) {
        std::str::from_utf8(b).ok()?.parse().ok()
    } else {
        None
    }
}

// (year, month, day) to days since 1970-01-01. Howard Hinnant's days_from_civil.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

// Go session note: the name the snapshot gives the imported session (filename without .jsonl).
pub fn session_stem(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.strip_suffix(".jsonl")
        .map(str::to_string)
        .unwrap_or(name)
}

// The handoff note that stands in for the missing handoff of a plain Claude Code session (Go applyImport).
pub fn import_note(stem: &str) -> String {
    format!(
        "(imported from a plain Claude Code session {stem}; no handoff note was written by it. Goal, Decisions, \
         In-progress work, Open questions, User preferences and exact values are in the last turns below; the \
         files it changed are listed. Re-read notes/ (FACTS.md, DECISIONS.md) for durable state.)"
    )
}

// Go buildSnapshot for an import (no background-agent lines: those come from the live tracker, which the caller
// adds). Parts join with a blank line, the last tail turns are kept, the whole is cut to max_chars, and the
// recommendation is placed by with_reco. tail == 0 keeps every turn.
pub fn import_snapshot(imp: &ClaudeImport, note: &str, tail: usize, max_chars: usize) -> String {
    let mut n = note.trim().to_string();
    if n.is_empty() {
        n = "(empty)".to_string();
    }
    let mut parts = vec![
        PREAMBLE.to_string(),
        "## Handoff note (written by previous session)".to_string(),
        n,
    ];
    if !imp.files.is_empty() {
        let l: Vec<String> = imp.files.iter().map(|f| format!("- {f}")).collect();
        parts.push("## Files changed".to_string());
        parts.push(l.join("\n"));
    }
    let turns = if tail > 0 && imp.turns.len() > tail {
        &imp.turns[imp.turns.len() - tail..]
    } else {
        &imp.turns[..]
    };
    if !turns.is_empty() {
        parts.push("## Last turns (verbatim, truncated)".to_string());
        for (u, a) in turns {
            parts.push(format!(
                "User: {}\nAssistant: {}",
                trunc(u, TURN_SIDE_CHARS),
                trunc(a, TURN_SIDE_CHARS)
            ));
        }
    }
    with_reco(&imp.reco, &trunc(&parts.join("\n\n"), max_chars))
}

// The metrics record for an import (Go applyImport Metrics.Log). The caller writes it with Metrics::log.
pub fn import_record(
    imp: &ClaudeImport,
    path: &Path,
    snap: &str,
    preserved: bool,
    dry_run: bool,
) -> Value {
    json!({
        "stage": "import",
        "source": path.to_string_lossy(),
        "turns": imp.turns.len(),
        "files": imp.files.len(),
        "snapshot_chars": snap.chars().count(),
        "reco_found": !imp.reco.is_empty(),
        "reco_preserved": preserved,
        "dry_run": dry_run,
    })
}

// The --import-dry text (Go importFromClaude). The caller exits 0 when preserved is true, 1 otherwise.
pub fn dry_run_report(imp: &ClaudeImport, path: &Path, snap: &str, preserved: bool) -> String {
    format!(
        "import dry-run: {}\nturns={} view_turns={} subagents={} files={} snapshot_chars={} snapshot_bytes={}\n\
         reco_found={} reco_preserved={} reco_bytes={}\n",
        path.display(),
        imp.turns.len(),
        imp.view.len(),
        imp.subagents,
        imp.files.len(),
        snap.chars().count(),
        snap.len(),
        !imp.reco.is_empty(),
        preserved,
        imp.reco.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rotate::reco_preserved;
    use crate::testutil::TempDir;

    fn write(dir: &Path, name: &str, lines: &[Value]) -> PathBuf {
        let p = dir.join(name);
        let text: String = lines.iter().map(|v| format!("{v}\n")).collect();
        fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn rfc3339_parses_utc_offset_and_fraction() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some((0, 0)));
        assert_eq!(
            parse_rfc3339("2000-02-29T01:01:01Z"),
            Some((951_782_400 + 3661, 0))
        );
        assert_eq!(
            parse_rfc3339("2000-02-29T02:01:01.5+01:00"),
            Some((951_782_400 + 3661, 500_000_000))
        );
        assert_eq!(
            parse_rfc3339("2000-02-29T00:01:01-00:00"),
            Some((951_782_400 + 61, 0))
        );
        assert_eq!(parse_rfc3339(""), None);
        assert_eq!(parse_rfc3339("2000-02-29T01:01:01"), None);
        assert_eq!(parse_rfc3339("2000-13-29T01:01:01Z"), None);
    }

    #[test]
    fn project_key_replaces_each_non_alnum_char() {
        assert_eq!(project_key("/Users/a/my-proj_1"), "-Users-a-my-proj-1");
        assert_eq!(project_key("/tmp/한글"), "-tmp---");
    }

    #[test]
    fn explicit_path_resolves_unchanged() {
        assert_eq!(
            resolve_claude_transcript("/x/y/s.jsonl", "").unwrap(),
            PathBuf::from("/x/y/s.jsonl")
        );
    }

    #[test]
    fn transcript_extracts_turns_files_name_and_clear_markers() {
        let tmp = TempDir::new("import-read");
        let reco_text = "done\n추천: 다음 단계는 테스트 실행";
        let p = write(
            tmp.path(),
            "s1.jsonl",
            &[
                json!({"type": "user", "cwd": "/w", "timestamp": "2026-01-02T03:04:05.123Z",
                       "message": {"content": "<command-name>/clear</command-name>"}}),
                json!({"type": "user", "isMeta": true, "message": {"content": "injected"}}),
                json!({"type": "summary", "summary": "Fix the parser"}),
                json!({"type": "user", "message": {"content": "first question"}}),
                json!({"type": "assistant", "message": {"content": [
                    {"type": "text", "text": "first answer"},
                    {"type": "tool_use", "id": "t1", "name": "Write", "input": {"file_path": "/w/a.rs"}},
                    {"type": "tool_use", "id": "t2", "name": "Edit", "input": {"file_path": "/w/a.rs"}},
                    {"type": "tool_use", "id": "t3", "name": "Bash", "input": {"command": "ls"}}
                ]}}),
                json!({"type": "user", "message": {"content": "second question"}}),
                json!({"type": "assistant", "message": {"content": [{"type": "text", "text": reco_text}]}}),
            ],
        );
        let imp = read_claude_transcript(&p).unwrap();
        assert_eq!(
            imp.turns,
            vec![
                ("first question".to_string(), "first answer".to_string()),
                ("second question".to_string(), reco_text.to_string()),
            ]
        );
        assert_eq!(imp.files, vec!["/w/a.rs".to_string()]);
        assert_eq!(imp.name, "Fix the parser");
        assert_eq!(imp.cwd, "/w");
        assert_eq!(imp.start, parse_rfc3339("2026-01-02T03:04:05.123Z"));
        assert!(imp.cleared);
        assert!(imp.clear_head, "the /clear came before any turn");
        assert_eq!(imp.reco, extract_reco(reco_text));
        assert!(!imp.reco.is_empty());
        // The view keeps the /clear note as its own "(session continued)" turn (Go viewBuilder.text).
        assert_eq!(imp.view.len(), 3);
        assert_eq!(imp.view[0].0, "(session continued)");
        assert!(imp.view[1].1.contains("▸ Write"));
    }

    #[test]
    fn transcript_without_turns_is_an_error() {
        let tmp = TempDir::new("import-empty");
        let p = write(
            tmp.path(),
            "e.jsonl",
            &[json!({"type": "summary", "summary": "x"})],
        );
        let err = read_claude_transcript(&p).unwrap_err().to_string();
        assert!(err.contains("no user/assistant turns"), "{err}");
    }

    #[test]
    fn snapshot_keeps_tail_turns_and_files() {
        let imp = ClaudeImport {
            turns: vec![
                ("first".into(), "a1".into()),
                ("second".into(), "a2".into()),
            ],
            files: vec!["/w/a.rs".into()],
            ..Default::default()
        };
        let snap = import_snapshot(&imp, "  ", 1, DEFAULT_SNAPSHOT_CHARS);
        assert!(snap.starts_with(PREAMBLE), "{snap}");
        assert!(
            snap.contains("## Handoff note (written by previous session)\n\n(empty)"),
            "{snap}"
        );
        assert!(snap.contains("## Files changed\n\n- /w/a.rs"), "{snap}");
        assert!(snap.contains("User: second\nAssistant: a2"), "{snap}");
        assert!(!snap.contains("first"), "{snap}");
    }

    #[test]
    fn snapshot_is_cut_to_max_chars() {
        let imp = ClaudeImport {
            turns: vec![("q".repeat(100), "a".repeat(100))],
            ..Default::default()
        };
        let snap = import_snapshot(&imp, "note", 0, 50);
        assert_eq!(snap.chars().count(), 50);
    }

    #[test]
    fn reco_survives_the_snapshot() {
        let imp = ClaudeImport {
            turns: vec![("q".into(), "a".into())],
            reco: "추천: 다음 작업".into(),
            ..Default::default()
        };
        let snap = import_snapshot(&imp, "note", DEFAULT_TAIL_TURNS, DEFAULT_SNAPSHOT_CHARS);
        assert!(reco_preserved(&imp.reco, &snap), "{snap}");
    }

    #[test]
    fn merge_orders_files_by_start_and_returns_newest_path() {
        let tmp = TempDir::new("import-merge");
        let late = write(
            tmp.path(),
            "late.jsonl",
            &[
                json!({"type": "user", "timestamp": "2026-01-02T00:00:00Z", "message": {"content": "later"}}),
                json!({"type": "assistant", "message": {"content": [{"type": "text", "text": "L"}]}}),
            ],
        );
        let early = write(
            tmp.path(),
            "early.jsonl",
            &[
                json!({"type": "user", "timestamp": "2026-01-01T00:00:00Z", "message": {"content": "earlier"}}),
                json!({"type": "assistant", "message": {"content": [{"type": "text", "text": "E"}]}}),
            ],
        );
        let arg = format!("{},{}", late.display(), early.display());
        let (path, merged) = load_import_arg(&arg, "").unwrap();
        assert_eq!(path, late);
        assert_eq!(merged.path, late);
        assert_eq!(merged.turns[0].0, "earlier");
        assert_eq!(merged.turns[1].0, "later");
        assert_eq!(merged.start, parse_rfc3339("2026-01-01T00:00:00Z"));
    }

    #[test]
    fn stem_note_and_record_follow_go() {
        assert_eq!(session_stem(Path::new("/a/b/abc-123.jsonl")), "abc-123");
        assert!(import_note("abc").starts_with("(imported from a plain Claude Code session abc; "));
        let imp = ClaudeImport {
            turns: vec![("q".into(), "a".into())],
            reco: "추천: x".into(),
            ..Default::default()
        };
        let rec = import_record(&imp, Path::new("/s.jsonl"), "snap", true, false);
        assert_eq!(rec["stage"], "import");
        assert_eq!(rec["turns"], 1);
        assert_eq!(rec["snapshot_chars"], 4);
        assert_eq!(rec["reco_found"], true);
        assert_eq!(rec["dry_run"], false);
    }

    #[test]
    fn dry_run_report_has_go_fields() {
        let imp = ClaudeImport {
            turns: vec![("q".into(), "a".into())],
            ..Default::default()
        };
        let text = dry_run_report(&imp, Path::new("/s.jsonl"), "snap", false);
        assert!(
            text.starts_with("import dry-run: /s.jsonl\nturns=1 view_turns=0 subagents=0 files=0 "),
            "{text}"
        );
        assert!(
            text.contains("reco_found=false reco_preserved=false reco_bytes=0\n"),
            "{text}"
        );
    }
}
