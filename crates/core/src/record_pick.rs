//! /resume record picker model (Go rpick), shared by the TUI (tui/records.rs keys and draw) and the GUI window.
//! The folders come from record_turn::pick (record_folder reads them); this holds the list, the cursor and the
//! row text. Reading a folder's blocks is record_folder::load_project.

use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::i18n::{Msg, tf};
use crate::localtime::local_civil;
use crate::record_folder::Project;
use crate::record_turn;

// The list date of a folder row: local yyyy-mm-dd of its newest write.
pub fn list_date(t: SystemTime) -> String {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let [y, mo, d, ..] = local_civil(secs);
    format!("{y:04}-{mo:02}-{d:02}")
}

// Go recordProjectLines row text: "<name> · 파일 <n>개 · <yyyy-mm-dd>".
pub fn row_text(p: &Project) -> String {
    tf(Msg::RecordRow, &[&p.name, &p.files, &list_date(p.last)])
}

// Go rpick and rpickSel: the folders the picker offers and the cursor row.
#[derive(Debug, Default, PartialEq)]
pub struct Picker {
    pub projects: Vec<Project>,
    pub sel: usize,
}

impl Picker {
    pub fn new(projects: Vec<Project>) -> Self {
        Picker { projects, sel: 0 }
    }

    // Go recordPick: the picker over the folders record_turn::pick offers. None when arg loads one folder directly
    // (a single hit or a number) or no folder exists; the caller then answers with record_turn::resume.
    pub fn open(root: &Path, arg: &str) -> io::Result<Option<Picker>> {
        let projects = record_turn::pick(root, arg)?;
        Ok(if projects.is_empty() {
            None
        } else {
            Some(Picker::new(projects))
        })
    }

    // Go wheel and Up/Down: moves the cursor d rows and wraps (no-op on an empty list).
    pub fn move_by(&mut self, d: isize) {
        let n = self.projects.len() as isize;
        if n > 0 {
            self.sel = (self.sel as isize + d).rem_euclid(n) as usize;
        }
    }

    // A digit or a click: moves the cursor to row i. Out-of-range rows are ignored.
    pub fn select(&mut self, i: usize) {
        if i < self.projects.len() {
            self.sel = i;
        }
    }

    // The folder under the cursor (enter loads it).
    pub fn chosen(&self) -> Option<&Project> {
        self.projects.get(self.sel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Block, Stamp, parse_entries, render};
    use crate::testutil::TempDir;
    use std::time::Duration;

    // One project folder with one record file: a user "q" and an assistant answer, written an hour ago.
    fn write_project(root: &Path, name: &str, answer: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let stamp = Stamp::new(2026, 10, 31, 10, 0, 0);
        let turn = [
            Block {
                kind: "user".into(),
                text: "q".into(),
            },
            Block {
                kind: "asst".into(),
                text: answer.into(),
            },
        ];
        let f = dir.join("261031-100000-0001.md");
        std::fs::write(&f, render(1, &stamp, name, &turn)).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&f)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(3600))
            .unwrap();
    }

    fn fixture(tmp: &TempDir) -> std::path::PathBuf {
        let root = tmp.path().join("record");
        write_project(&root, "alpha", "배포 방법");
        write_project(&root, "beta", "정규식 설명");
        write_project(&root, "gamma", "배포 체크리스트");
        root
    }

    // Go TestRecordPickCandidates: bare and multi-hit open the picker; a single hit, a number and no folder do not.
    #[test]
    fn open_follows_go_candidates() {
        let tmp = TempDir::new("pick-open");
        let root = fixture(&tmp);
        let len = |arg: &str| Picker::open(&root, arg).unwrap().map(|p| p.projects.len());
        assert_eq!(len(""), Some(3), "bare");
        assert_eq!(len("배포"), Some(2), "multi-hit");
        assert_eq!(len("정규식"), None, "single hit loads directly");
        assert_eq!(len("2"), None, "number loads directly");
        let empty = tmp.path().join("none");
        assert!(Picker::open(&empty, "").unwrap().is_none(), "no folder");
    }

    #[test]
    fn cursor_wraps_selects_in_range_and_chooses() {
        let tmp = TempDir::new("pick-cursor");
        let root = fixture(&tmp);
        let mut p = Picker::open(&root, "").unwrap().unwrap();
        p.move_by(-1);
        assert_eq!(p.sel, 2, "up wraps");
        p.move_by(1);
        assert_eq!(p.sel, 0, "down wraps");
        p.select(1);
        assert_eq!(p.sel, 1, "a row is selected");
        p.select(9);
        assert_eq!(p.sel, 1, "a row past the list is ignored");
        assert_eq!(p.chosen().map(|c| c.name.as_str()), Some("beta"));
        let loaded = crate::record_folder::load_project(&p.chosen().unwrap().dir).unwrap();
        assert_eq!(loaded.len(), 2, "every block of the chosen folder");
        let mut none = Picker::default();
        none.move_by(1);
        assert_eq!((none.sel, none.chosen()), (0, None), "empty list");
    }

    #[test]
    fn parse_entries_unescapes_body_header_lines() {
        let data = "<!-- rec #0002 asst 2026-10-31T10:00 cat=t -->\n## #0002 · 10:00 · t\n\n\\<!-- rec #0009 x -->\nplain\n\n";
        let e = parse_entries(data);
        assert_eq!(e.len(), 1);
        assert_eq!(
            (e[0].num, e[0].kind.as_str(), e[0].topic.as_str()),
            (2, "asst", "t")
        );
        assert_eq!(e[0].text, "<!-- rec #0009 x -->\nplain");
    }

    #[test]
    fn row_text_and_date_are_local_yyyy_mm_dd() {
        let tmp = TempDir::new("pick-row");
        let root = fixture(&tmp);
        let p = Picker::open(&root, "").unwrap().unwrap();
        let row = row_text(&p.projects[0]);
        assert!(
            row.starts_with(&format!("{} · ", p.projects[0].name)),
            "{row}"
        );
        let date = list_date(p.projects[0].last);
        assert_eq!(date.len(), 10, "{date}");
        assert!(row.ends_with(&date), "{row}");
    }
}
