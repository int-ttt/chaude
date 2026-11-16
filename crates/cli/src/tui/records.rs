//! /resume record picker key and draw (Go tui.go rpick keys and draw). The model (project folders, cursor, row
//! text, the folder a pick loads) is core `chaude_core::record_pick`; this file only reads keys and draws rows.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use chaude_core::record_folder::Project;
use chaude_core::record_pick::{Picker, row_text};

use super::bar::clip;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON};
use super::sessview::wheel_sel;

pub use chaude_core::record_folder::Project as RecordProject;
pub use chaude_core::record_pick::list_date;

// Go rpick title (tui.go draw).
pub const TITLE: &str = "Resume a project  (↑/↓ wheel or 1-9 select · enter load · esc cancel)";

// Result of a key press on the record picker.
#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    // esc, q or ctrl+c: close the picker.
    Close,
    // enter: close the picker and load this folder (Go loadRecordPick).
    Load(Project),
}

// Go wheelSel on rpickSel (shared with the /view picker).
pub fn wheel(rp: &mut Picker, up: bool) {
    wheel_sel(&mut rp.sel, rp.projects.len(), up);
}

// Go rpick key branch. Up/k and down/j wrap; 1-9 move the cursor to that row; enter loads; esc/q/ctrl+c close.
pub fn key(rp: &mut Picker, key: KeyEvent) -> Action {
    let n = rp.projects.len();
    let plain = key.modifiers == KeyModifiers::NONE;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if plain => rp.move_by(-1),
        KeyCode::Down | KeyCode::Char('j') if plain => rp.move_by(1),
        KeyCode::Esc | KeyCode::Char('q') if plain => return Action::Close,
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Action::Close,
        KeyCode::Enter => {
            if let Some(p) = rp.chosen() {
                return Action::Load(p.clone());
            }
        }
        KeyCode::Char(c)
            if plain && ('1'..='9').contains(&c) && ((c as u8 - b'1') as usize) < n =>
        {
            rp.select((c as u8 - b'1') as usize);
        }
        _ => {}
    }
    Action::None
}

// Go rpick draw: a title row, then rows from a window around sel, exactly hh rows total.
pub fn lines(rp: &Picker, width: usize, hh: usize) -> Vec<String> {
    if hh == 0 {
        return Vec::new();
    }
    let n = rp.projects.len() as isize;
    let from = (rp.sel as isize - (hh / 2) as isize)
        .min(n - (hh as isize - 2))
        .max(0) as usize;
    let mut l = vec![format!("{ORANGE_ON}{TITLE}{FG_OFF}")];
    for (i, p) in rp.projects.iter().enumerate().skip(from) {
        if l.len() >= hh - 1 {
            break;
        }
        let t = format!("{:>2}) {}", i + 1, row_text(p));
        l.push(if i == rp.sel {
            format!("{ORANGE_ON}{}{FG_OFF}", clip(&format!("> {t}"), width))
        } else {
            format!("{GREY_ON}{}{FG_OFF}", clip(&format!("  {t}"), width))
        });
    }
    l.resize(hh, String::new());
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    const LIST_TITLE: &str = "record projects (/resume <번호|이름|검색어> opens one):";

    // The core resume with the picker's date format.
    fn resume(
        root: &Path,
        arg: &str,
    ) -> Result<(Vec<String>, Vec<chaude_core::record::Block>), String> {
        chaude_core::record_turn::resume(root, arg, &list_date).map_err(|e| e.to_string())
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // Writes one project folder with one record file holding a user "q" and an assistant answer.
    fn write_project(root: &Path, name: &str, answer: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let body = format!(
            "<!-- rec #0001 user 2026-10-31T10:00 cat={name} -->\n## #0001 · 10:00 · {name}\n\nq\n\n\
             <!-- rec #0001 asst 2026-10-31T10:00 cat={name} -->\n## #0001 · 10:00 · {name}\n\n{answer}\n\n"
        );
        let f = dir.join("261031-100000-0001.md");
        std::fs::write(&f, body).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&f)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(3600))
            .unwrap();
    }

    fn fixture(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("chaude-records-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        write_project(&root, "alpha", "배포 방법");
        write_project(&root, "beta", "정규식 설명");
        write_project(&root, "gamma", "배포 체크리스트");
        root
    }

    fn open_all(root: &Path) -> Picker {
        Picker::open(root, "").unwrap().unwrap()
    }

    // Go TestRecordPickerKeysWheelEnter: arrows and wheel move the cursor, enter loads that folder and closes,
    // esc cancels.
    #[test]
    fn picker_keys_wheel_enter_follow_go() {
        let root = fixture("keys");
        let mut p = open_all(&root);
        let ps = p.projects.clone();
        assert_eq!(key(&mut p, press(KeyCode::Down)), Action::None);
        assert_eq!(p.sel, 1, "down");
        key(&mut p, press(KeyCode::Down));
        assert_eq!(p.sel, 2);
        key(&mut p, press(KeyCode::Down));
        assert_eq!(p.sel, 0, "down wraps");
        wheel(&mut p, false);
        assert_eq!(p.sel, 1, "wheel down");
        wheel(&mut p, true);
        assert_eq!(p.sel, 0, "wheel up");
        key(&mut p, press(KeyCode::Char('2')));
        assert_eq!(p.sel, 1, "digit selects row");
        key(&mut p, press(KeyCode::Char('9')));
        assert_eq!(p.sel, 1, "digit past the list is ignored");
        key(&mut p, press(KeyCode::Up));
        let Action::Load(chosen) = key(&mut p, press(KeyCode::Enter)) else {
            panic!("enter must load the selected folder");
        };
        assert_eq!(chosen, ps[0]);
        let loaded = chaude_core::record_folder::load_project(&chosen.dir).unwrap();
        assert_eq!(
            loaded.len(),
            2 * chosen.files,
            "every block of the chosen folder"
        );
        assert_eq!(
            key(&mut open_all(&root), press(KeyCode::Esc)),
            Action::Close
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn picker_draws_window_with_title_and_padding() {
        let root = fixture("draw");
        let p = open_all(&root);
        let rows = lines(&p, 80, 5);
        assert_eq!(rows.len(), 5);
        assert!(rows[0].contains("Resume a project"));
        assert!(rows[1].contains(" 1) alpha") || rows[1].contains("> ") || rows[1].contains("1) "));
        assert_eq!(rows[4], "", "padded to the body height");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_lists_then_rejects_bad_numbers() {
        let _lang = crate::test_support::lang_lock();
        let root = fixture("resume");
        let (lines, blocks) = resume(&root, "").unwrap();
        assert_eq!(lines[0], LIST_TITLE);
        assert_eq!(lines.len(), 4, "title plus three folders");
        assert!(blocks.is_empty());
        assert_eq!(
            resume(&root, "9").err().as_deref(),
            Some("no project 9 (1-3)")
        );
        assert_eq!(
            resume(&root, "nope").err().as_deref(),
            Some("no project matches \"nope\"")
        );
        let (_, one) = resume(&root, "정규식").unwrap();
        assert_eq!(one.len(), 2, "user and asst blocks");
        assert!(one.iter().any(|e| e.text == "정규식 설명"), "{one:?}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
