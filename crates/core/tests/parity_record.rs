// Parity with Go harness/record_test.go: TestRecordSplitByCaps, TestRecordNumberIgnoresMtime,
// TestRecordMidnightBoundary. Uses only the public record API (no crate-internal testutil).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chaude_core::record::{Block, Stamp, append_turn, list_files, next_number};

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let p =
            std::env::temp_dir().join(format!("chaude-parity-record-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Dir(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn turn(answer: &str) -> Vec<Block> {
    vec![
        Block {
            kind: "user".into(),
            text: "질문".into(),
        },
        Block {
            kind: "asst".into(),
            text: answer.into(),
        },
    ]
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = list_files(dir)
        .unwrap()
        .into_iter()
        .map(|f| f.name)
        .collect();
    v.sort();
    v
}

#[test]
fn split_by_caps_starts_new_file_at_101st_answer() {
    let d = Dir::new("caps");
    let at = Stamp::new(2026, 10, 31, 10, 0, 0);
    for _ in 0..101 {
        append_turn(d.path(), &at, "cap", &turn("x"), false).unwrap();
    }
    // The 101st turn (rec #0101) starts a new file named by its own rec number.
    assert_eq!(
        names(d.path()),
        vec!["261031-100000-0001.md", "261031-100000-0101.md"]
    );
}

#[test]
fn number_ignores_mtime() {
    let d = Dir::new("mtime");
    let at = Stamp::new(2026, 10, 31, 9, 0, 0);
    append_turn(d.path(), &at, "old", &turn("a"), false).unwrap();
    append_turn(d.path(), &at, "new", &turn("b"), true).unwrap();
    // Make the older file the newest by mtime; the number must still come from the newest rec.
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_893_456_000); // 2030-01-01
    fs::OpenOptions::new()
        .write(true)
        .open(d.path().join("261031-090000-0001.md"))
        .unwrap()
        .set_modified(old)
        .unwrap();
    assert_eq!(next_number(d.path()).unwrap(), 3);
}

#[test]
fn midnight_boundary_keeps_newest_file_and_stamps_new_day() {
    let d = Dir::new("midnight");
    append_turn(
        d.path(),
        &Stamp::new(2026, 10, 31, 23, 59, 50),
        "chaude metrics",
        &turn("a"),
        false,
    )
    .unwrap();
    // Same folder, next day, no split: still the newest file (a file starts only on /clear or a cap).
    append_turn(
        d.path(),
        &Stamp::new(2026, 11, 1, 0, 0, 10),
        "chaude metrics",
        &turn("b"),
        false,
    )
    .unwrap();
    assert_eq!(names(d.path()), vec!["261031-235950-0001.md"]);
    let data = fs::read_to_string(d.path().join("261031-235950-0001.md")).unwrap();
    assert!(
        data.contains("<!-- rec #0002 asst 2026-11-01T00:00 cat=chaude metrics -->"),
        "midnight turn header missing:\n{data}"
    );
}
