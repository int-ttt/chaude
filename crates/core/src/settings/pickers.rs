//! Shared models of the /lang and /settings pickers (TUI and GUI). Keys and drawing stay in each front end; the rows,
//! the cursor, the value cycling, the save and the language switch live here so both show the same thing.

use std::env;
use std::path::{Path, PathBuf};

use crate::i18n::{self, Msg, t, tf};
use crate::settings::chaude_settings::{
    self, MAX_EXTRA_CODERS, MODEL_CHOICES, RETAIN_CHOICES, Settings, file_for, load, save,
};

// Picker rows in display order: (settings value, native name).
pub const LANG_CHOICES: [(&str, &str); 2] = [("ko", "한국어"), ("en", "English")];

// The /lang cursor. It starts on the language in use, so enter on open keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangPicker {
    pub sel: usize,
}

impl LangPicker {
    pub fn open() -> Self {
        let sel = LANG_CHOICES
            .iter()
            .position(|(code, _)| *code == i18n::current().code())
            .unwrap_or(0);
        LangPicker { sel }
    }

    // Up (true) or down (false), wrapping over the choices.
    pub fn step(&mut self, up: bool) {
        let n = LANG_CHOICES.len();
        self.sel = if up {
            (self.sel + n - 1) % n
        } else {
            (self.sel + 1) % n
        };
    }

    // The settings value of the row under the cursor.
    pub fn picked(&self) -> &'static str {
        LANG_CHOICES[self.sel].0
    }
}

// The note for a bare /lang: the saved setting (auto when absent) and the language in use now.
pub fn lang_current_note(cwd: Option<&Path>) -> String {
    let setting = chaude_settings::language(cwd).unwrap_or_else(|| "auto".into());
    tf(Msg::LangCurrent, &[&setting, &i18n::current().code()])
}

// True for a code /lang accepts (ko | en | auto): only then does the caller send `/lang CODE` to the engine.
pub fn lang_is_code(code: &str) -> bool {
    matches!(code, "ko" | "en" | "auto")
}

// Saves `code` (ko | en | auto) to the project settings, then switches the live language. Returns the transcript note.
pub fn lang_apply(cwd: Option<&Path>, code: &str) -> String {
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

// Picker rows of /settings (the value rows; the header and hint text are the front end's).
pub const SETTINGS_ROWS: usize = 3;

// Go settingsPick: the open /settings picker over a working copy. `path` is None when the cwd is unknown (Go "").
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsPicker {
    pub s: Settings,
    pub sel: usize,
    pub path: Option<PathBuf>,
}

impl SettingsPicker {
    // Go openSettings: loads the file into the picker. Safe while busy: it only reads and edits a copy.
    pub fn open(cwd: Option<&Path>) -> Self {
        let path = file_for(cwd);
        SettingsPicker {
            s: load(path.as_deref()),
            sel: 0,
            path,
        }
    }

    // Up (true) or down (false), wrapping over the rows.
    pub fn step(&mut self, up: bool) {
        self.sel = if up {
            (self.sel + SETTINGS_ROWS - 1) % SETTINGS_ROWS
        } else {
            (self.sel + 1) % SETTINGS_ROWS
        };
    }

    // Go settingsPick.change: steps the selected row by dir (+1/-1), wrapping.
    pub fn change(&mut self, dir: i64) {
        match self.sel {
            0 => {
                let n = RETAIN_CHOICES.len() as i64;
                let i = RETAIN_CHOICES
                    .iter()
                    .position(|&v| v == self.s.session_retention_days)
                    .unwrap_or(0) as i64;
                self.s.session_retention_days = RETAIN_CHOICES[((i + dir).rem_euclid(n)) as usize];
            }
            1 => {
                let n = MAX_EXTRA_CODERS + 1;
                self.s.extra_coders = (self.s.extra_coders + dir).rem_euclid(n);
            }
            2 => {
                let n = MODEL_CHOICES.len() as i64;
                let i = MODEL_CHOICES
                    .iter()
                    .position(|&v| v == self.s.main_model)
                    .unwrap_or(0) as i64;
                self.s.main_model = MODEL_CHOICES[((i + dir).rem_euclid(n)) as usize].to_string();
            }
            _ => {}
        }
    }

    // The three value rows, label first, in the order of `sel`.
    pub fn rows(&self) -> [String; SETTINGS_ROWS] {
        [
            format!(
                "session_retention_days  {}",
                tf(Msg::RetainDays, &[&self.s.session_retention_days])
            ),
            format!(
                "extra_coders            {}",
                tf(
                    Msg::ExtraCoders,
                    &[&self.s.extra_coders, &crate::api::CODER_IDLE_TTL_SECS]
                )
            ),
            format!("main_model              {}", self.s.main_model),
        ]
    }

    // Go closeSettings: saves the working copy and returns the note that says what applies when.
    pub fn close(&mut self) -> String {
        match &self.path {
            None => t(Msg::SettingsNoPath).to_string(),
            Some(p) => match save(p, &self.s) {
                Ok(()) => tf(Msg::SettingsSaved, &[&p.display()]),
                Err(e) => tf(Msg::SettingsSaveFailed, &[&e]),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_step_wraps_and_picks_the_row() {
        let mut p = LangPicker { sel: 0 };
        assert_eq!(p.picked(), "ko");
        p.step(false);
        assert_eq!((p.sel, p.picked()), (1, "en"));
        p.step(false);
        assert_eq!(p.sel, 0, "down wraps to the first row");
        p.step(true);
        assert_eq!(p.sel, 1, "up wraps to the last row");
    }

    #[test]
    fn settings_step_and_change_wrap() {
        let mut p = SettingsPicker {
            s: Settings::default(),
            sel: 0,
            path: None,
        };
        p.step(true);
        assert_eq!(p.sel, 2, "up wraps to the last row");
        p.step(false);
        assert_eq!(p.sel, 0);
        p.change(-1);
        assert_eq!(p.s.session_retention_days, 7, "30 steps down to 7");
        p.s.session_retention_days = 0;
        p.change(-1);
        assert_eq!(
            p.s.session_retention_days, 365,
            "retention wraps at the start"
        );
        p.sel = 1;
        p.s.extra_coders = 0;
        p.change(-1);
        assert_eq!(p.s.extra_coders, 8, "extra coders wrap 0 -> 8");
        p.sel = 2;
        p.change(1);
        assert_eq!(p.s.main_model, "sonnet");
        assert_eq!(p.rows()[2], "main_model              sonnet");
    }

    #[test]
    fn settings_close_without_a_path_says_so() {
        let mut p = SettingsPicker {
            s: Settings::default(),
            sel: 0,
            path: None,
        };
        assert_eq!(p.close(), "설정 경로를 알 수 없음 (작업 폴더 없음)");
    }
}
