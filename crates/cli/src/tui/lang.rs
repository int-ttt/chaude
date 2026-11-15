//! /lang: the ko/en picker overlay. The picker model, the save and the live switch are core (settings::pickers) and
//! shared with the GUI; this file keeps the terminal keys and drawing. After a successful save the caller also sends
//! `/lang CODE` to the engine (is_code), which hands the main child over to a fresh one started with the new
//! language (proc.rs --settings), and the resident tui gets the language per prompt.

use chaude_core::i18n::{Msg, t};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::bar::clip;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON};

pub use chaude_core::settings::pickers::{
    LANG_CHOICES as CHOICES, LangPicker, lang_apply as apply, lang_current_note as current_note,
    lang_is_code as is_code,
};

// What one key does to the picker. Pick carries the settings value to apply.
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    None,
    Pick(&'static str),
    Cancel,
}

// Terminal keys and drawing for the shared LangPicker model.
pub trait LangView {
    fn wheel(&mut self, up: bool);
    fn key(&mut self, key: KeyEvent) -> Key;
    fn lines(&self, width: usize, h: usize) -> Vec<String>;
}

impl LangView for LangPicker {
    fn wheel(&mut self, up: bool) {
        super::sessview::wheel_sel(&mut self.sel, CHOICES.len(), up);
    }

    fn key(&mut self, key: KeyEvent) -> Key {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.step(true),
            KeyCode::Down | KeyCode::Char('j') => self.step(false),
            KeyCode::Char('1') => return Key::Pick(CHOICES[0].0),
            KeyCode::Char('2') => return Key::Pick(CHOICES[1].0),
            KeyCode::Enter => return Key::Pick(self.picked()),
            KeyCode::Esc | KeyCode::Char('q') => return Key::Cancel,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Key::Cancel;
            }
            _ => {}
        }
        Key::None
    }

    fn lines(&self, width: usize, h: usize) -> Vec<String> {
        let mut l = vec![format!(
            "{ORANGE_ON}{}{FG_OFF}",
            clip(t(Msg::LangPickTitle), width)
        )];
        for (i, (code, name)) in CHOICES.iter().enumerate() {
            let r = format!("{code}  {name}");
            let row = if i == self.sel {
                format!("{ORANGE_ON}{}{FG_OFF}", clip(&format!("> {r}"), width))
            } else {
                format!("{GREY_ON}{}{FG_OFF}", clip(&format!("  {r}"), width))
            };
            l.push(row);
        }
        l.push(format!(
            "{GREY_ON}{}{FG_OFF}",
            clip(t(Msg::LangHint), width)
        ));
        l.resize(h.max(l.len()), String::new());
        l.truncate(h);
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chaude_core::i18n;
    use chaude_core::settings::chaude_settings;
    use crossterm::event::KeyEventKind;
    use std::env;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn picker_keys_pick_cancel_and_move() {
        let mut p = LangPicker { sel: 0 };
        assert_eq!(p.key(press(KeyCode::Char('2'))), Key::Pick("en"));
        assert_eq!(p.key(press(KeyCode::Char('1'))), Key::Pick("ko"));
        assert_eq!(p.key(press(KeyCode::Down)), Key::None);
        assert_eq!(p.sel, 1);
        assert_eq!(p.key(press(KeyCode::Enter)), Key::Pick("en"));
        assert_eq!(p.key(press(KeyCode::Esc)), Key::Cancel);
        assert_eq!(p.key(press(KeyCode::Char('q'))), Key::Cancel);
    }

    #[test]
    fn apply_writes_setting_and_switches_language() {
        let _lang = crate::test_support::lang_lock();
        let dir = env::temp_dir().join(format!("chaude-lang-apply-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let note = apply(Some(&dir), "en");
        assert_eq!(i18n::current(), i18n::Lang::En);
        assert_eq!(chaude_settings::language(Some(&dir)).as_deref(), Some("en"));
        assert!(note.contains("en"));
        let note = apply(Some(&dir), "ko");
        assert_eq!(i18n::current(), i18n::Lang::Ko);
        assert!(note.contains("ko"));
        let fail = apply(Some(&dir), "fr");
        assert!(fail.contains("unknown language fr"), "{fail}");
        assert_eq!(i18n::current(), i18n::Lang::Ko);
        assert_eq!(chaude_settings::language(Some(&dir)).as_deref(), Some("ko"));
        let _ = std::fs::remove_dir_all(&dir);
        i18n::set_current(i18n::Lang::Ko);
    }
}
