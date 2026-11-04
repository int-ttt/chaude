// Go /resume session picker (tui.go pick): the Claude Code session groups of this project that match the
// /resume terms. Enter resumes the newest session of the chosen group.

use std::path::Path;
use std::time::SystemTime;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use chaude_core::record;
use chaude_core::resume_group::{self, SessionGroup};

use super::bar::clip;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON};
use super::sessview::{home_dir, project_dir, stamp, wheel_sel};

const TITLE: &str = "Resume a session  (↑/↓ wheel or 1-9 select · enter resume · esc cancel)";
// Go /resume shows at most this many groups.
const MAX_ROWS: usize = 30;

// Go groupList: the groups of cwd's Claude Code project dir matching terms. Err is the status line.
pub fn groups(cwd: &str, terms: &[String]) -> Result<Vec<SessionGroup>, String> {
    let home = home_dir().ok_or_else(|| "$HOME is not defined".to_string())?;
    let claude = project_dir(&home, cwd);
    let marks = resume_group::clear_mark_path(&record::dir_for(Path::new(cwd)));
    resume_group::group_list(&claude, &marks, cwd, terms)
}

// Result of a key press on the session picker.
#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    // esc, q or ctrl+c: close the picker.
    Close,
    // enter: resume this session (the newest file of the chosen group).
    Pick(String),
}

// Go pick and pickSel: the open picker and its cursor.
#[derive(Debug, Default)]
pub struct GroupPicker {
    pub groups: Vec<SessionGroup>,
    pub sel: usize,
}

impl GroupPicker {
    pub fn new(mut groups: Vec<SessionGroup>) -> Self {
        groups.truncate(MAX_ROWS);
        GroupPicker { groups, sel: 0 }
    }

    // Go wheelSel on pickSel.
    pub fn wheel(&mut self, up: bool) {
        wheel_sel(&mut self.sel, self.groups.len(), up);
    }

    // Go pick key branch. Up/k and down/j wrap; 1-9 move the cursor to that row; enter picks; esc/q/ctrl+c close.
    pub fn key(&mut self, key: KeyEvent) -> Action {
        let n = self.groups.len();
        let plain = key.modifiers == KeyModifiers::NONE;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') if plain => wheel_sel(&mut self.sel, n, true),
            KeyCode::Down | KeyCode::Char('j') if plain => wheel_sel(&mut self.sel, n, false),
            KeyCode::Esc | KeyCode::Char('q') if plain => return Action::Close,
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Action::Close,
            KeyCode::Enter => {
                if let Some(g) = self.groups.get(self.sel) {
                    return Action::Pick(g.latest().id.clone());
                }
            }
            KeyCode::Char(c)
                if plain && ('1'..='9').contains(&c) && ((c as u8 - b'1') as usize) < n =>
            {
                self.sel = (c as u8 - b'1') as usize;
            }
            _ => {}
        }
        Action::None
    }

    // Go pick draw: a title row, then rows from a window around sel, exactly hh rows total.
    pub fn lines(&self, width: usize, hh: usize) -> Vec<String> {
        if hh == 0 {
            return Vec::new();
        }
        let n = self.groups.len() as isize;
        let from = (self.sel as isize - (hh / 2) as isize)
            .min(n - (hh as isize - 2))
            .max(0) as usize;
        let mut l = vec![format!("{ORANGE_ON}{TITLE}{FG_OFF}")];
        for (i, g) in self.groups.iter().enumerate().skip(from) {
            if l.len() >= hh - 1 {
                break;
            }
            let t = format!("{:>2}) {}", i + 1, g.label(&|t: SystemTime| stamp(t, true)));
            l.push(if i == self.sel {
                format!("{ORANGE_ON}{}{FG_OFF}", clip(&format!("> {t}"), width))
            } else {
                format!("{GREY_ON}{}{FG_OFF}", clip(&format!("  {t}"), width))
            });
        }
        l.resize(hh, String::new());
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chaude_core::resume_group::SessionInfo;
    use std::path::PathBuf;

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

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn enter_picks_the_selected_group_and_keys_move_the_cursor() {
        let mut p = GroupPicker::new(vec![group("a"), group("b"), group("c")]);
        assert_eq!(p.key(press(KeyCode::Char('2'))), Action::None);
        assert_eq!(p.sel, 1);
        assert_eq!(p.key(press(KeyCode::Enter)), Action::Pick("b".into()));
        assert_eq!(p.key(press(KeyCode::Down)), Action::None);
        assert_eq!(p.key(press(KeyCode::Down)), Action::None);
        assert_eq!(p.sel, 0, "down wraps to the first row");
        assert_eq!(p.key(press(KeyCode::Esc)), Action::Close);
    }

    #[test]
    fn lines_are_exactly_hh_rows_with_the_title_first() {
        let p = GroupPicker::new((0..10).map(|i| group(&i.to_string())).collect());
        let l = p.lines(120, 5);
        assert_eq!(l.len(), 5);
        assert!(l[0].contains("Resume a session"));
        assert!(l.iter().any(|r| r.contains("> ")));
        assert!(GroupPicker::new(Vec::new()).lines(80, 0).is_empty());
    }
}
