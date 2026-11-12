//! Project scratch layout under <cwd>/notes/tmp/ (gitignored). Nothing chaude writes goes to the system temp dir.
//! long/: exit and resume data (the /resume digest <session>.md, killed-session markers), kept 30 days.
//! test/: short-lived scratch (the MCP config and socket dirs, the /usage run dir), kept 24 hours.
//! Startup moves the old notes/tmp/*.md digests into long/ and removes expired entries. Errors never fail startup.
//! Startup never removes an entry of a live peer: one touched in the last 15 minutes, a "<pid>-..." or
//! "usage-<pid>" entry whose pid runs, or the live marker whose owner pid runs.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::handoff::{
    EXIT_HANDOFF_FILE, LIVE_HANDOFF_FILE, SESSIONS_FILE, pid_alive, read_exit_handoff,
};
use crate::record;

pub const LONG_DIR: &str = "long";
pub const TEST_DIR: &str = "test";
pub const LONG_MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
pub const TEST_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
// An entry touched this recently may still belong to a peer that is starting or idle.
const PEER_GRACE: Duration = Duration::from_secs(15 * 60);
// Scratch dir name of the /usage run: "usage-<pid>".
const USAGE_PREFIX: &str = "usage-";

// <cwd>/notes/tmp
pub fn tmp_root(cwd: &Path) -> PathBuf {
    cwd.join("notes").join("tmp")
}

// <cwd>/notes/tmp/long: exit and resume data.
pub fn long_dir(cwd: &Path) -> PathBuf {
    tmp_root(cwd).join(LONG_DIR)
}

// <cwd>/notes/tmp/test: short-lived scratch.
pub fn test_dir(cwd: &Path) -> PathBuf {
    tmp_root(cwd).join(TEST_DIR)
}

// Chaude startup: migrate, then clean both dirs as of now. Returns how many entries were removed.
pub fn startup(cwd: &Path) -> usize {
    startup_at(cwd, SystemTime::now())
}

// Same as startup with the clock passed in, so the age rules are testable.
pub fn startup_at(cwd: &Path, now: SystemTime) -> usize {
    startup_in(cwd, &record::dir_for(cwd), now)
}

// startup_at with the old record folder passed in, so tests never touch the real one.
fn startup_in(cwd: &Path, old_record: &Path, now: SystemTime) -> usize {
    migrate_md(cwd);
    migrate_handoff(old_record, cwd);
    clean_dir(&test_dir(cwd), TEST_MAX_AGE, now) + clean_dir(&long_dir(cwd), LONG_MAX_AGE, now)
}

// The exit handoff, live marker and session log used to sit in the record folder. Each one is copied into long/
// and removed from the old folder, once: when long/ has no copy yet. Later starts read long/ only.
fn migrate_handoff(old_dir: &Path, cwd: &Path) {
    let long = long_dir(cwd);
    for name in [EXIT_HANDOFF_FILE, LIVE_HANDOFF_FILE, SESSIONS_FILE] {
        let from = old_dir.join(name);
        let to = long.join(name);
        if !from.is_file() || to.exists() {
            continue;
        }
        if fs::create_dir_all(&long).is_err() {
            return;
        }
        if fs::copy(&from, &to).is_ok() {
            let _ = fs::remove_file(&from);
        }
    }
}

// Moves the top-level .md files of notes/tmp (the old digests) into long/. A file already in long/ stays put.
fn migrate_md(cwd: &Path) {
    let Ok(ents) = fs::read_dir(tmp_root(cwd)) else {
        return;
    };
    let long = long_dir(cwd);
    for ent in ents.flatten() {
        let path = ent.path();
        let is_md_file = path.extension().is_some_and(|e| e == "md")
            && ent.file_type().is_ok_and(|t| t.is_file());
        let Some(name) = path.file_name() else {
            continue;
        };
        if !is_md_file || long.join(name).exists() {
            continue;
        }
        if fs::create_dir_all(&long).is_err() {
            return;
        }
        let _ = fs::rename(&path, long.join(name));
    }
}

// Removes the entries of dir whose mtime is older than max_age. Live peers' entries are kept (see live_peer) and
// so is anything touched within PEER_GRACE. Returns the count removed. Errors are skipped.
fn clean_dir(dir: &Path, max_age: Duration, now: SystemTime) -> usize {
    let Ok(ents) = fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for ent in ents.flatten() {
        let path = ent.path();
        if live_peer(&path, &ent.file_name()) {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        let Ok(mtime) = meta.modified() else {
            continue;
        };
        let age = now.duration_since(mtime).unwrap_or_default();
        if age <= max_age || age < PEER_GRACE {
            continue;
        }
        let res = if meta.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        if res.is_ok() {
            removed += 1;
        }
    }
    removed
}

// True when the entry belongs to a running chaude: a "<pid>-<seq>" or "usage-<pid>" scratch entry whose pid is
// alive, or the live marker whose owner pid is alive. A file that does not parse is not a live marker.
fn live_peer(path: &Path, name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    if name == LIVE_HANDOFF_FILE {
        return read_exit_handoff(path).is_some_and(|h| h.owner_pid != 0 && pid_alive(h.owner_pid));
    }
    let owner = name.strip_prefix(USAGE_PREFIX).unwrap_or(name);
    owner
        .split('-')
        .next()
        .and_then(|p| p.parse::<u32>().ok())
        .is_some_and(pid_alive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    const DAY: u64 = 24 * 60 * 60;

    fn at(base: SystemTime, days: u64) -> SystemTime {
        base + Duration::from_secs(days * DAY)
    }

    // Startup with an empty old record folder inside the test cwd.
    fn startup_t(cwd: &Path, now: SystemTime) -> usize {
        startup_in(cwd, &cwd.join("record"), now)
    }

    #[test]
    fn handoff_files_move_from_record_once_and_keep_the_long_copy() {
        let tmp = TempDir::new("tmp-layout-handoff");
        let cwd = tmp.path();
        let old = cwd.join("record");
        fs::create_dir_all(&old).unwrap();
        for name in [EXIT_HANDOFF_FILE, LIVE_HANDOFF_FILE, SESSIONS_FILE] {
            fs::write(old.join(name), format!("old {name}")).unwrap();
        }
        fs::write(old.join("other.md"), "x").unwrap();

        migrate_handoff(&old, cwd);

        for name in [EXIT_HANDOFF_FILE, LIVE_HANDOFF_FILE, SESSIONS_FILE] {
            assert_eq!(
                fs::read_to_string(long_dir(cwd).join(name)).unwrap(),
                format!("old {name}")
            );
            assert!(!old.join(name).exists());
        }
        assert!(old.join("other.md").exists());
        // A second start with a newer copy in record/ does not overwrite long/.
        fs::write(old.join(EXIT_HANDOFF_FILE), "newer").unwrap();
        migrate_handoff(&old, cwd);
        assert_eq!(
            fs::read_to_string(long_dir(cwd).join(EXIT_HANDOFF_FILE)).unwrap(),
            format!("old {EXIT_HANDOFF_FILE}")
        );
    }

    #[test]
    fn paths_sit_under_notes_tmp() {
        let cwd = Path::new("/proj");
        assert_eq!(tmp_root(cwd), PathBuf::from("/proj/notes/tmp"));
        assert_eq!(long_dir(cwd), PathBuf::from("/proj/notes/tmp/long"));
        assert_eq!(test_dir(cwd), PathBuf::from("/proj/notes/tmp/test"));
    }

    #[test]
    fn test_dir_expires_after_a_day_and_long_after_thirty() {
        let tmp = TempDir::new("tmp-layout-ages");
        let cwd = tmp.path();
        fs::create_dir_all(test_dir(cwd).join("scratch")).unwrap();
        fs::write(test_dir(cwd).join("file.txt"), "x").unwrap();
        fs::create_dir_all(long_dir(cwd)).unwrap();
        fs::write(long_dir(cwd).join("s1.md"), "digest").unwrap();
        let base = SystemTime::now();

        // 23h: nothing is old enough.
        assert_eq!(startup_t(cwd, base + Duration::from_secs(23 * 3600)), 0);
        // 25h: test/ entries go, long/ stays.
        assert_eq!(startup_t(cwd, at(base, 1) + Duration::from_secs(3600)), 2);
        assert!(!test_dir(cwd).join("scratch").exists());
        assert!(!test_dir(cwd).join("file.txt").exists());
        assert!(long_dir(cwd).join("s1.md").exists());
        // 29 days: long/ still keeps it.
        assert_eq!(startup_t(cwd, at(base, 29)), 0);
        assert!(long_dir(cwd).join("s1.md").exists());
        // 31 days: long/ entry goes.
        assert_eq!(startup_t(cwd, at(base, 31)), 1);
        assert!(!long_dir(cwd).join("s1.md").exists());
    }

    #[test]
    fn migrates_top_level_md_into_long_without_overwriting() {
        let tmp = TempDir::new("tmp-layout-migrate");
        let cwd = tmp.path();
        let root = tmp_root(cwd);
        fs::create_dir_all(root.join("keep-dir")).unwrap();
        fs::write(root.join("old.md"), "old").unwrap();
        fs::write(root.join("dup.md"), "new").unwrap();
        fs::create_dir_all(long_dir(cwd)).unwrap();
        fs::write(long_dir(cwd).join("dup.md"), "kept").unwrap();
        fs::write(root.join("note.txt"), "x").unwrap();

        startup_t(cwd, SystemTime::now());

        assert_eq!(
            fs::read_to_string(long_dir(cwd).join("old.md")).unwrap(),
            "old"
        );
        assert!(!root.join("old.md").exists());
        assert_eq!(
            fs::read_to_string(long_dir(cwd).join("dup.md")).unwrap(),
            "kept"
        );
        assert!(root.join("dup.md").exists());
        assert!(root.join("note.txt").exists());
        assert!(root.join("keep-dir").is_dir());
    }

    #[test]
    fn live_owner_entries_are_kept_and_dead_ones_removed() {
        let tmp = TempDir::new("tmp-layout-owner");
        let cwd = tmp.path();
        let dir = test_dir(cwd);
        let own = format!("{}-7", std::process::id());
        fs::create_dir_all(dir.join(&own)).unwrap();
        fs::create_dir_all(dir.join("4294967295-1")).unwrap();

        let removed = startup_t(cwd, at(SystemTime::now(), 2));

        assert_eq!(removed, 1);
        assert!(
            dir.join(&own).exists(),
            "a live owner keeps its scratch dir"
        );
        assert!(!dir.join("4294967295-1").exists());
    }

    #[test]
    fn clean_errors_never_panic_or_count() {
        let tmp = TempDir::new("tmp-layout-errors");
        let cwd = tmp.path();
        // notes/tmp is a file: read_dir fails on both sides.
        fs::create_dir_all(cwd.join("notes")).unwrap();
        fs::write(tmp_root(cwd), "not a dir").unwrap();
        assert_eq!(startup_t(cwd, SystemTime::now()), 0);
        // A cwd that does not exist.
        assert_eq!(startup_t(&cwd.join("missing"), SystemTime::now()), 0);
    }

    #[test]
    fn live_peer_scratch_and_marker_survive_expiry() {
        let tmp = TempDir::new("tmp-layout-peers");
        let cwd = tmp.path();
        let (test, long) = (test_dir(cwd), long_dir(cwd));
        fs::create_dir_all(&test).unwrap();
        fs::create_dir_all(&long).unwrap();
        let me = std::process::id();
        fs::create_dir_all(test.join(format!("usage-{me}"))).unwrap();
        fs::create_dir_all(test.join("usage-4294967295")).unwrap();
        let marker = long.join(LIVE_HANDOFF_FILE);
        fs::write(&marker, format!("{{\"owner_pid\":{me}}}")).unwrap();
        let two_days = at(SystemTime::now(), 2);

        // Own pid is alive: its usage dir and marker stay. The dead pid's usage dir goes.
        assert_eq!(startup_t(cwd, two_days), 1);
        assert!(test.join(format!("usage-{me}")).is_dir());
        assert!(!test.join("usage-4294967295").exists());
        assert!(marker.exists());

        // Marker owner gone: the stale marker is removed too.
        fs::write(&marker, "{\"owner_pid\":4294967295}").unwrap();
        // long/ keeps entries 30 days, so the marker must be older than that.
        assert_eq!(startup_t(cwd, at(SystemTime::now(), 31)), 1);
        assert!(!marker.exists());
    }
}
