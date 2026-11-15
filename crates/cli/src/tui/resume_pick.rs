// Go /resume session picker (tui.go pick): the Claude Code session groups of this project that match the
// /resume terms. Enter imports the chosen group. The model (groups, cursor, import key) is core group_pick.

use std::time::SystemTime;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub use chaude_core::group_pick::GroupPicker;

use super::bar::clip;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON};
use super::sessview::{stamp, wheel_sel};

const TITLE: &str = "Resume a session  (↑/↓ wheel or 1-9 select · enter resume · esc cancel)";

// Result of a key press on the session picker.
#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    // esc, q or ctrl+c: close the picker.
    Close,
    // enter: import this group (Go sendImport; the key is the group's id list, see SessionGroup::key).
    Pick(String),
}

// Go wheelSel on pickSel.
pub fn wheel(p: &mut GroupPicker, up: bool) {
    let n = p.groups.len();
    wheel_sel(&mut p.sel, n, up);
}

// Go pick key branch. Up/k and down/j wrap; 1-9 move the cursor to that row; enter picks; esc/q/ctrl+c close.
pub fn key(p: &mut GroupPicker, key: KeyEvent) -> Action {
    let n = p.groups.len();
    let plain = key.modifiers == KeyModifiers::NONE;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if plain => p.move_by(-1),
        KeyCode::Down | KeyCode::Char('j') if plain => p.move_by(1),
        KeyCode::Esc | KeyCode::Char('q') if plain => return Action::Close,
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Action::Close,
        KeyCode::Enter => {
            if let Some(k) = p.chosen_key() {
                return Action::Pick(k);
            }
        }
        KeyCode::Char(c)
            if plain && ('1'..='9').contains(&c) && ((c as u8 - b'1') as usize) < n =>
        {
            p.select((c as u8 - b'1') as usize);
        }
        _ => {}
    }
    Action::None
}

// Go pick draw: a title row, then rows from a window around sel, exactly hh rows total.
// All matching groups stay: the window is drawn around sel, so older groups are reachable by wheel.
pub fn lines(p: &GroupPicker, width: usize, hh: usize) -> Vec<String> {
    if hh == 0 {
        return Vec::new();
    }
    let n = p.groups.len() as isize;
    let from = (p.sel as isize - (hh / 2) as isize)
        .min(n - (hh as isize - 2))
        .max(0) as usize;
    let mut l = vec![format!("{ORANGE_ON}{TITLE}{FG_OFF}")];
    for (i, g) in p.groups.iter().enumerate().skip(from) {
        if l.len() >= hh - 1 {
            break;
        }
        let t = format!("{:>2}) {}", i + 1, g.label(&|t: SystemTime| stamp(t, true)));
        l.push(if i == p.sel {
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
    use chaude_core::resume_group::{SessionGroup, SessionInfo};
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
        assert_eq!(key(&mut p, press(KeyCode::Char('2'))), Action::None);
        assert_eq!(p.sel, 1);
        assert_eq!(key(&mut p, press(KeyCode::Enter)), Action::Pick("b".into()));
        assert_eq!(key(&mut p, press(KeyCode::Down)), Action::None);
        assert_eq!(key(&mut p, press(KeyCode::Down)), Action::None);
        assert_eq!(p.sel, 0, "down wraps to the first row");
        assert_eq!(key(&mut p, press(KeyCode::Esc)), Action::Close);
    }

    #[test]
    fn enter_picks_the_whole_group_oldest_first() {
        let mut g = group("new");
        g.files.push(group("old").files.remove(0));
        let mut p = GroupPicker::new(vec![g]);
        assert_eq!(
            key(&mut p, press(KeyCode::Enter)),
            Action::Pick("old,new".into())
        );
    }

    #[test]
    fn groups_beyond_thirty_stay_reachable() {
        let mut p = GroupPicker::new((0..45).map(|i| group(&i.to_string())).collect());
        assert_eq!(p.groups.len(), 45, "no row cap");
        wheel(&mut p, true);
        assert_eq!(
            p.sel, 44,
            "wheel up from the first row reaches the oldest group"
        );
        assert_eq!(
            key(&mut p, press(KeyCode::Enter)),
            Action::Pick("44".into())
        );
        let l = lines(&p, 120, 8);
        assert!(
            l.iter().any(|r| r.contains("> 45)")),
            "selected old row is drawn"
        );
    }

    #[test]
    fn lines_are_exactly_hh_rows_with_the_title_first() {
        let p = GroupPicker::new((0..10).map(|i| group(&i.to_string())).collect());
        let l = lines(&p, 120, 5);
        assert_eq!(l.len(), 5);
        assert!(l[0].contains("Resume a session"));
        assert!(l.iter().any(|r| r.contains("> ")));
        assert!(lines(&GroupPicker::new(Vec::new()), 80, 0).is_empty());
    }
}
