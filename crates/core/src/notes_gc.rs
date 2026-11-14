//! Notes GC: bundles old <cwd>/notes/runs/ files into monthly archive files under <cwd>/notes/archive/.
//! Candidates are the direct regular files of notes/runs/ whose mtime is older than 14 days. Subfolders, notes/*.md,
//! notes/usage/, notes/research/, notes/tasks/ and every other path are never touched.
//! Each bundle entry is a "<!-- notes-gc: notes/runs/<name> (mtime YYYY-MM-DD) -->" line, then the file text. Only
//! text files (valid UTF-8, no NUL byte) are bundled; other files stay in place. A month's bundle is written to a temp
//! file and renamed into place; the originals are removed only after that rename succeeds. A rerun finds the entries
//! already bundled (new or legacy "=== notes/runs/<name> (mtime ...) ===" header) and only removes the leftover originals.
//! Session start runs it only for an interactive launch (stdin is a terminal, no positional word) and only when
//! notes/runs/ holds more than 200 files or notes/ is over 20 MB; /notes-gc runs it on demand (--dry-run reports only).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MAX_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);
pub const MAX_RUN_FILES: usize = 200;
pub const MAX_NOTES_BYTES: u64 = 20 * 1024 * 1024;
const USAGE: &str = "usage: /notes-gc [--dry-run]";
const SECS_PER_DAY: u64 = 86_400;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub dry_run: bool,
    // Old runs files removed from notes/runs/ (dry run: that would be removed).
    pub moved: usize,
    pub moved_bytes: u64,
    // Old runs files left in place: not UTF-8, unreadable, or not removable.
    pub skipped: usize,
    // Bundle paths relative to cwd, written (dry run: that would be written).
    pub bundles: Vec<String>,
}

struct Candidate {
    name: String,
    path: PathBuf,
    month: String,
    date: String,
    size: u64,
}

fn runs_dir(cwd: &Path) -> PathBuf {
    cwd.join("notes").join("runs")
}

fn archive_dir(cwd: &Path) -> PathBuf {
    cwd.join("notes").join("archive")
}

// Not a body-safe prefix: a text body may hold "=== ..." lines, so the header uses an HTML comment.
fn header_line(c: &Candidate) -> String {
    format!(
        "<!-- notes-gc: notes/runs/{} (mtime {}) -->",
        c.name, c.date
    )
}

// Header written by the first version; still recognised so a rerun does not bundle a second copy.
fn legacy_header_line(c: &Candidate) -> String {
    format!("=== notes/runs/{} (mtime {}) ===", c.name, c.date)
}

// Text only: valid UTF-8 with no NUL byte. Anything else stays in place.
fn read_text(path: &Path) -> Option<String> {
    let text = String::from_utf8(fs::read(path).ok()?).ok()?;
    (!text.contains('\0')).then_some(text)
}

// Days since the epoch to (year, month, day) in UTC (Howard Hinnant's civil_from_days).
fn civil(secs: u64) -> (i64, u32, u32) {
    let z = (secs / SECS_PER_DAY) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

fn secs_of(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn candidates(cwd: &Path, now: SystemTime) -> io::Result<Vec<Candidate>> {
    let rd = match fs::read_dir(runs_dir(cwd)) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for entry in rd {
        let entry = entry?;
        // entry.metadata() does not follow symlinks, so links and folders are skipped.
        let meta = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age <= MAX_AGE {
            continue;
        }
        let (y, m, d) = civil(secs_of(modified));
        out.push(Candidate {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path(),
            month: format!("{y:04}-{m:02}"),
            date: format!("{y:04}-{m:02}-{d:02}"),
            size: meta.len(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

// Temp file, then rename: the bundle is either the old text or the new text, never half of each.
fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("md.tmp");
    if let Err(e) = fs::write(&tmp, text) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

// Bundles every old runs file (see the module doc). Errors stop the run; files whose bundle write failed stay.
pub fn run(cwd: &Path, now: SystemTime, dry_run: bool) -> io::Result<Outcome> {
    let mut out = Outcome {
        dry_run,
        ..Outcome::default()
    };
    let mut months: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    for c in candidates(cwd, now)? {
        months.entry(c.month.clone()).or_default().push(c);
    }
    for (month, cands) in months {
        let bundle_name = format!("runs-{month}.md");
        let bundle = archive_dir(cwd).join(&bundle_name);
        let mut text = match fs::read_to_string(&bundle) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        };
        let mut added = false;
        let mut done: Vec<&Candidate> = Vec::new();
        for c in &cands {
            let header = header_line(c);
            let legacy = legacy_header_line(c);
            if !text.lines().any(|l| l == header || l == legacy) {
                let Some(body) = read_text(&c.path) else {
                    out.skipped += 1;
                    continue;
                };
                text.push_str(&header);
                text.push('\n');
                text.push_str(&body);
                if !body.ends_with('\n') {
                    text.push('\n');
                }
                text.push('\n');
                added = true;
            }
            done.push(c);
        }
        if done.is_empty() {
            continue;
        }
        if added {
            if !dry_run {
                write_atomic(&bundle, &text)?;
            }
            out.bundles.push(format!("notes/archive/{bundle_name}"));
        }
        for c in done {
            if dry_run || fs::remove_file(&c.path).is_ok() {
                out.moved += 1;
                out.moved_bytes += c.size;
            } else {
                out.skipped += 1;
            }
        }
    }
    Ok(out)
}

// One line for the transcript or the session log.
pub fn summary(o: &Outcome) -> String {
    let head = if o.dry_run {
        "notes gc (dry run): would move"
    } else {
        "notes gc: moved"
    };
    let mut s = format!(
        "{head} {} runs files ({} KB)",
        o.moved,
        o.moved_bytes / 1024
    );
    if !o.bundles.is_empty() {
        s.push_str(&format!(" into {}", o.bundles.join(", ")));
    }
    if o.skipped > 0 {
        s.push_str(&format!("; kept {} (not text or not removable)", o.skipped));
    }
    s
}

// Number of direct files in notes/runs/ (folders not counted).
fn run_file_count(cwd: &Path) -> usize {
    let Ok(rd) = fs::read_dir(runs_dir(cwd)) else {
        return 0;
    };
    rd.flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .count()
}

// Total bytes under a folder. Unreadable entries count as 0.
fn dir_bytes(dir: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(dir) else { return 0 };
    rd.flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_bytes(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

// Session start: nothing for a non-interactive launch (piped stdin, slash command, subcommand, prompt), nothing when
// both thresholds hold; otherwise a real run. Returns the line to log, if anything moved or failed. Never fails startup.
pub fn startup(cwd: &Path, now: SystemTime, interactive: bool) -> Option<String> {
    startup_with(cwd, now, interactive, MAX_RUN_FILES, MAX_NOTES_BYTES)
}

fn startup_with(
    cwd: &Path,
    now: SystemTime,
    interactive: bool,
    max_files: usize,
    max_bytes: u64,
) -> Option<String> {
    if !interactive {
        return None;
    }
    if run_file_count(cwd) <= max_files && dir_bytes(&cwd.join("notes")) <= max_bytes {
        return None;
    }
    match run(cwd, now, false) {
        Ok(o) if o.moved > 0 => Some(summary(&o)),
        Ok(_) => None,
        Err(e) => Some(format!("notes gc: {e}")),
    }
}

// /notes-gc [--dry-run], run in the current folder.
pub fn slash_text(cwd: &Path, args: &[String]) -> String {
    let dry_run = match args {
        [] => false,
        [a] if a == "--dry-run" => true,
        _ => return USAGE.to_string(),
    };
    match run(cwd, SystemTime::now(), dry_run) {
        Ok(o) => summary(&o),
        Err(e) => format!("notes gc: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    const DAY: u64 = 86_400;
    // 2027-01-15 08:00 UTC.
    const NOW: u64 = 1_800_000_000;

    fn now() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(NOW)
    }

    fn days_ago(d: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(NOW - d * DAY)
    }

    fn put(path: &Path, body: &[u8], mtime: SystemTime) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
    }

    fn month_of(t: SystemTime) -> String {
        let (y, m, _) = civil(secs_of(t));
        format!("{y:04}-{m:02}")
    }

    #[test]
    fn civil_known_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(11_017 * DAY), (2000, 3, 1));
    }

    #[test]
    fn old_runs_bundled_fresh_kept() {
        let t = TempDir::new("notes-gc-bundle");
        let cwd = t.path();
        let runs = cwd.join("notes/runs");
        put(&runs.join("old.md"), b"old body\n", days_ago(30));
        put(&runs.join("old.json"), b"{}", days_ago(31));
        put(&runs.join("fresh.md"), b"fresh\n", days_ago(1));

        let o = run(cwd, now(), false).unwrap();
        assert_eq!((o.moved, o.skipped), (2, 0));
        let bundle_rel = format!("notes/archive/runs-{}.md", month_of(days_ago(30)));
        assert_eq!(o.bundles, vec![bundle_rel.clone()]);
        let text = fs::read_to_string(cwd.join(&bundle_rel)).unwrap();
        assert!(text.contains("<!-- notes-gc: notes/runs/old.md (mtime "));
        assert!(text.contains("old body\n\n"));
        assert!(text.contains("<!-- notes-gc: notes/runs/old.json (mtime "));
        assert!(!runs.join("old.md").exists());
        assert!(!runs.join("old.json").exists());
        assert_eq!(
            fs::read_to_string(runs.join("fresh.md")).unwrap(),
            "fresh\n"
        );
    }

    #[test]
    fn rerun_is_idempotent_and_removes_leftover_originals() {
        let t = TempDir::new("notes-gc-idem");
        let cwd = t.path();
        let runs = cwd.join("notes/runs");
        put(&runs.join("a.md"), b"alpha\n", days_ago(20));
        run(cwd, now(), false).unwrap();
        let bundle = cwd.join(format!("notes/archive/runs-{}.md", month_of(days_ago(20))));
        let first = fs::read_to_string(&bundle).unwrap();

        // A leftover original (same name, same mtime) must not be appended twice.
        put(&runs.join("a.md"), b"alpha\n", days_ago(20));
        let o = run(cwd, now(), false).unwrap();
        assert_eq!(o.moved, 1);
        assert_eq!(fs::read_to_string(&bundle).unwrap(), first);
        assert!(!runs.join("a.md").exists());

        let o = run(cwd, now(), false).unwrap();
        assert_eq!((o.moved, o.bundles.len()), (0, 0));
        assert_eq!(fs::read_to_string(&bundle).unwrap(), first);
    }

    #[test]
    fn legacy_header_is_recognised_on_rerun() {
        let t = TempDir::new("notes-gc-legacy");
        let cwd = t.path();
        let runs = cwd.join("notes/runs");
        put(&runs.join("a.md"), b"alpha\n", days_ago(20));
        run(cwd, now(), false).unwrap();
        let bundle = cwd.join(format!("notes/archive/runs-{}.md", month_of(days_ago(20))));
        // Rewrite the bundle with the first version's header; a leftover original must not be bundled again.
        let legacy = fs::read_to_string(&bundle)
            .unwrap()
            .replace(
                "<!-- notes-gc: notes/runs/a.md (mtime ",
                "=== notes/runs/a.md (mtime ",
            )
            .replace(") -->", ") ===");
        fs::write(&bundle, &legacy).unwrap();
        put(&runs.join("a.md"), b"alpha\n", days_ago(20));
        let o = run(cwd, now(), false).unwrap();
        assert_eq!(o.moved, 1);
        assert_eq!(fs::read_to_string(&bundle).unwrap(), legacy);
    }

    #[test]
    fn text_with_nul_stays_and_is_counted() {
        let t = TempDir::new("notes-gc-nul");
        let cwd = t.path();
        put(&cwd.join("notes/runs/nul.log"), b"abc\0def\n", days_ago(30));
        put(&cwd.join("notes/runs/ok.md"), b"ok\n", days_ago(30));
        let o = run(cwd, now(), false).unwrap();
        assert_eq!((o.moved, o.skipped), (1, 1));
        assert!(cwd.join("notes/runs/nul.log").exists());
        assert!(summary(&o).ends_with("kept 1 (not text or not removable)"));
    }

    #[test]
    fn body_with_eq_lines_is_bundled_intact() {
        let t = TempDir::new("notes-gc-eq");
        let cwd = t.path();
        put(
            &cwd.join("notes/runs/test.md"),
            b"=== RUN TestFoo\n=== notes/runs/x.md (mtime 2026-01-01) ===\n",
            days_ago(30),
        );
        let o = run(cwd, now(), false).unwrap();
        assert_eq!(o.moved, 1);
        let bundle = cwd.join(format!("notes/archive/runs-{}.md", month_of(days_ago(30))));
        let text = fs::read_to_string(bundle).unwrap();
        assert!(text.contains("=== RUN TestFoo\n=== notes/runs/x.md (mtime 2026-01-01) ===\n"));
    }

    #[test]
    fn protected_paths_untouched() {
        let t = TempDir::new("notes-gc-protect");
        let cwd = t.path();
        let notes = cwd.join("notes");
        let kept = [
            "OPEN.md",
            "usage/0.12.7.md",
            "research/r.md",
            "tasks/t.md",
            "archive/existing.md",
            "runs/sub/inner.md",
        ];
        for k in kept {
            put(&notes.join(k), b"keep\n", days_ago(60));
        }
        let o = run(cwd, now(), false).unwrap();
        assert_eq!((o.moved, o.bundles.len()), (0, 0));
        for k in kept {
            assert_eq!(fs::read_to_string(notes.join(k)).unwrap(), "keep\n", "{k}");
        }
        assert_eq!(fs::read_dir(notes.join("archive")).unwrap().count(), 1);
    }

    #[test]
    fn bundle_write_failure_keeps_originals() {
        let t = TempDir::new("notes-gc-fail");
        let cwd = t.path();
        put(&cwd.join("notes/runs/old.md"), b"old\n", days_ago(30));
        // notes/archive as a file makes the bundle write fail.
        fs::write(cwd.join("notes/archive"), b"not a dir").unwrap();

        assert!(run(cwd, now(), false).is_err());
        assert_eq!(
            fs::read_to_string(cwd.join("notes/runs/old.md")).unwrap(),
            "old\n"
        );
        assert_eq!(
            fs::read_to_string(cwd.join("notes/archive")).unwrap(),
            "not a dir"
        );
    }

    #[test]
    fn dry_run_changes_nothing() {
        let t = TempDir::new("notes-gc-dry");
        let cwd = t.path();
        put(&cwd.join("notes/runs/old.md"), b"old\n", days_ago(30));
        let o = run(cwd, now(), true).unwrap();
        assert!(o.dry_run);
        assert_eq!((o.moved, o.bundles.len()), (1, 1));
        assert!(summary(&o).starts_with("notes gc (dry run): would move 1 runs files"));
        assert!(cwd.join("notes/runs/old.md").exists());
        assert!(!cwd.join("notes/archive").exists());
    }

    #[test]
    fn non_utf8_file_stays() {
        let t = TempDir::new("notes-gc-bin");
        let cwd = t.path();
        put(
            &cwd.join("notes/runs/bin.pyc"),
            &[0xff, 0xfe, 0x00],
            days_ago(30),
        );
        let o = run(cwd, now(), false).unwrap();
        assert_eq!((o.moved, o.skipped, o.bundles.len()), (0, 1, 0));
        assert!(cwd.join("notes/runs/bin.pyc").exists());
    }

    #[test]
    fn startup_only_above_thresholds() {
        let t = TempDir::new("notes-gc-startup");
        let cwd = t.path();
        put(&cwd.join("notes/runs/a.md"), b"a\n", days_ago(30));
        put(&cwd.join("notes/runs/b.md"), b"b\n", days_ago(30));

        assert_eq!(startup_with(cwd, now(), true, 2, u64::MAX), None);
        assert!(cwd.join("notes/runs/a.md").exists());

        let line = startup_with(cwd, now(), true, 1, u64::MAX).unwrap();
        assert!(line.starts_with("notes gc: moved 2 runs files"), "{line}");
        assert!(!cwd.join("notes/runs/a.md").exists());
    }

    #[test]
    fn startup_skipped_when_not_interactive() {
        let t = TempDir::new("notes-gc-piped");
        let cwd = t.path();
        put(&cwd.join("notes/runs/a.md"), b"a\n", days_ago(30));
        put(&cwd.join("notes/runs/b.md"), b"b\n", days_ago(30));

        // Over both thresholds, but piped or slash/prompt launch: no run, nothing moved.
        assert_eq!(startup_with(cwd, now(), false, 1, 1), None);
        assert!(cwd.join("notes/runs/a.md").exists());
        assert!(cwd.join("notes/runs/b.md").exists());
        assert!(!cwd.join("notes/archive").exists());
    }

    #[test]
    fn slash_usage_on_bad_args() {
        let t = TempDir::new("notes-gc-slash");
        assert_eq!(slash_text(t.path(), &["--bogus".to_string()]), USAGE);
    }
}
