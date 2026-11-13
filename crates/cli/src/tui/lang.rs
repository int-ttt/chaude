//! /lang: the ko/en picker overlay and the apply step shared by the TUI. Saving goes through
//! chaude_settings::save_language; the live switch sets the process language and CHAUDE_LANG. After a successful
//! save the caller also sends `/lang CODE` to the engine (is_code), which hands the main child over to a fresh one
//! started with the new language (proc.rs --settings), and the resident tui gets the language per prompt.

use std::env;
use std::path::Path;

use chaude_core::i18n::{self, Msg, t, tf};
use chaude_core::settings::chaude_settings;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::bar::clip;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON};

// Picker rows in display order: (settings value, native name).
pub const CHOICES: [(&str, &str); 2] = [("ko", "한국어"), ("en", "English")];

// What one key does to the picker. Pick carries the settings value to apply.
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    None,
    Pick(&'static str),
    Cancel,
}

pub struct LangPicker {
    pub sel: usize,
}

impl LangPicker {
    // The cursor starts on the language in use, so enter on open keeps it.
    pub fn open() -> Self {
        let sel = CHOICES
            .iter()
            .position(|(code, _)| *code == i18n::current().code())
            .unwrap_or(0);
        LangPicker { sel }
    }

    pub fn wheel(&mut self, up: bool) {
        super::sessview::wheel_sel(&mut self.sel, CHOICES.len(), up);
    }

    pub fn key(&mut self, key: KeyEvent) -> Key {
        let n = CHOICES.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.sel = (self.sel + n - 1) % n,
            KeyCode::Down | KeyCode::Char('j') => self.sel = (self.sel + 1) % n,
            KeyCode::Char('1') => return Key::Pick(CHOICES[0].0),
            KeyCode::Char('2') => return Key::Pick(CHOICES[1].0),
            KeyCode::Enter => return Key::Pick(CHOICES[self.sel].0),
            KeyCode::Esc | KeyCode::Char('q') => return Key::Cancel,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Key::Cancel;
            }
            _ => {}
        }
        Key::None
    }

    pub fn lines(&self, width: usize, h: usize) -> Vec<String> {
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

// The note for a bare /lang: the saved setting (auto when absent) and the language in use now.
pub fn current_note(cwd: Option<&Path>) -> String {
    let setting = chaude_settings::language(cwd).unwrap_or_else(|| "auto".into());
    tf(Msg::LangCurrent, &[&setting, &i18n::current().code()])
}

// True for a code /lang accepts (ko | en | auto): only then does the caller send `/lang CODE` to the engine.
pub fn is_code(code: &str) -> bool {
    matches!(code, "ko" | "en" | "auto")
}

// Saves `code` (ko | en | auto) to the project settings, then switches the live language. Returns the transcript note.
pub fn apply(cwd: Option<&Path>, code: &str) -> String {
    let base = match cwd {
        Some(p) => p.to_path_buf(),
        None => match env::current_dir() {
            Ok(p) => p,
            Err(e) => return tf(Msg::LangSaveFailed, &[&e]),
        },
    };
    if let Err(e) = chaude_settings::save_language(&base, code) {
        return tf(Msg::LangSaveFailed, &[&e]);
    }
    // The picked value wins over CHAUDE_LANG here, the same order the hooks use (settings file first, then CHAUDE_LANG, then OS locale).
    let lang = i18n::resolve(None, Some(code), i18n::os_locale().as_deref());
    i18n::set_current(lang);
    // SAFETY: the TUI and plain loops run on the main thread; no other thread reads CHAUDE_LANG while it is set here.
    // This follows the startup write in run_tui, which runs before any thread is spawned.
    unsafe { env::set_var(i18n::ENV_KEY, lang.code()) };
    tf(Msg::LangSet, &[&code])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

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
