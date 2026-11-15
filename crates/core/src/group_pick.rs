//! /resume session group picker model (Go pick), shared by the TUI (tui/resume_pick.rs keys and draw) and the GUI
//! window. The groups come from resume_group (the Claude Code session files of this project); this holds the list,
//! the cursor, the import key of a pick, and the group of a killed run. Reading the picked transcripts is
//! import_claude::import_for_resume.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::handoff;
use crate::import_claude::project_key;
use crate::localtime::local_civil;
use crate::record;
use crate::resume_group::{self, SessionGroup};
use crate::tmp_layout;

// Go groupList: the groups of cwd's Claude Code project dir under home matching terms. Err is the status line; a
// None home (no $HOME) is that line too. Callers pass the user's home, tests a temp dir.
pub fn groups(
    home: Option<&Path>,
    cwd: &str,
    terms: &[String],
) -> Result<Vec<SessionGroup>, String> {
    let home = home.ok_or_else(|| "$HOME is not defined".to_string())?;
    let claude = home.join(".claude").join("projects").join(project_key(cwd));
    let marks = resume_group::clear_mark_path(&record::dir_for(Path::new(cwd)));
    resume_group::group_list(&claude, &marks, cwd, terms)
}

// Go killed-run bare /resume: the main session the previous run did not finish, when that run was killed.
// Read before this run writes its own live marker.
pub fn killed_session(cwd: &str) -> Option<String> {
    let base = handoff::handoff_file(&tmp_layout::long_dir(Path::new(cwd)));
    handoff::killed_main_session(&handoff::read_path(&base, None))
}

// The import key of the group that holds sid (as /resume claude does). When no group lists it, the bare id is the
// key and the import reports the error.
pub fn killed_key(home: Option<&Path>, cwd: &str, sid: &str) -> String {
    groups(home, cwd, &[])
        .ok()
        .and_then(|gs| gs.into_iter().find(|g| g.files.iter().any(|f| f.id == sid)))
        .map_or_else(|| sid.to_string(), |g| g.key())
}

// Local "yyyy-mm-dd hh:mm" of a session write time (the TUI stamp with the year).
pub fn stamp(t: SystemTime) -> String {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let [y, mo, d, h, mi, _] = local_civil(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}")
}

// Go pick and pickSel: the open picker and its cursor. Every matching group stays in the list (no row cap).
#[derive(Debug, Default)]
pub struct GroupPicker {
    pub groups: Vec<SessionGroup>,
    pub sel: usize,
}

impl GroupPicker {
    pub fn new(groups: Vec<SessionGroup>) -> Self {
        GroupPicker { groups, sel: 0 }
    }

    // Go wheel and Up/Down: moves the cursor d rows and wraps (no-op on an empty list).
    pub fn move_by(&mut self, d: isize) {
        let n = self.groups.len() as isize;
        if n > 0 {
            self.sel = (self.sel as isize + d).rem_euclid(n) as usize;
        }
    }

    // A digit or a click: moves the cursor to row i. Out-of-range rows are ignored.
    pub fn select(&mut self, i: usize) {
        if i < self.groups.len() {
            self.sel = i;
        }
    }

    // Enter: the import key of the cursor group (SessionGroup::key, the ids oldest first).
    pub fn chosen_key(&self) -> Option<String> {
        self.groups.get(self.sel).map(|g| g.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resume_group::SessionInfo;
    use std::path::PathBuf;
    use std::time::Duration;

    // No $HOME is the status line; a temp home without the project dir is the list's own error. Neither reads the
    // real $HOME.
    #[test]
    fn groups_take_an_injected_home() {
        let tmp = crate::testutil::TempDir::new("group-home");
        let cwd = tmp.path().join("proj").display().to_string();
        assert_eq!(groups(None, &cwd, &[]).unwrap_err(), "$HOME is not defined");
        let err = groups(Some(tmp.path()), &cwd, &[]).unwrap_err();
        assert!(err.starts_with("no Claude Code sessions in "), "{err}");
        assert_eq!(
            killed_key(None, &cwd, "sid-1"),
            "sid-1",
            "no group: the bare id"
        );
    }

    fn group(id: &str) -> SessionGroup {
        SessionGroup {
            files: vec![SessionInfo {
                id: id.into(),
                path: PathBuf::new(),
                modified: SystemTime::UNIX_EPOCH,
                turns: 2,
                first: "first prompt".into(),
                last: "last prompt".into(),
                start: None,
                cwd: String::new(),
                name: String::new(),
                cleared: false,
                clear_head: false,
            }],
        }
    }

    #[test]
    fn move_by_wraps_and_select_ignores_out_of_range() {
        let mut p = GroupPicker::new(vec![group("a"), group("b"), group("c")]);
        p.move_by(-1);
        assert_eq!(p.sel, 2, "up from the first row wraps to the last");
        p.move_by(1);
        assert_eq!(p.sel, 0, "down wraps to the first row");
        p.select(1);
        assert_eq!(p.chosen_key().as_deref(), Some("b"));
        p.select(9);
        assert_eq!(p.sel, 1, "out-of-range select is ignored");
        assert_eq!(GroupPicker::default().chosen_key(), None);
    }

    #[test]
    fn stamp_has_year_minute_precision() {
        let s = stamp(SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000));
        assert_eq!(s.len(), 16, "yyyy-mm-dd hh:mm: {s}");
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[13..14], ":");
    }
}
