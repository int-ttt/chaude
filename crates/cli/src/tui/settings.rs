//! /settings cursor picker over <project>/.chaude/settings.json (Go settings_picker.go; chaude_settings.go).
//! The picker edits a working copy. Esc, ctrl+c or q saves it and closes; the caller appends the returned note.
//! Changes to extra_coders and main_model apply at the next session start; session_retention_days too.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;

use super::bar::clip;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON};

// Go chaude_settings.go constants.
pub const MAX_EXTRA_CODERS: i64 = 8;
pub const DEFAULT_RETAIN: i64 = 30;
pub const DEFAULT_EXTRA_CODERS: i64 = 4;
pub const DEFAULT_MODEL: &str = "opus";

// Go settingsRetainChoices / settingsModelChoices / settingsRows.
pub const RETAIN_CHOICES: [i64; 5] = [0, 7, 30, 90, 365];
pub const MODEL_CHOICES: [&str; 3] = ["opus", "sonnet", "fable"];
pub const ROWS: usize = 3;

// Go chaudeSettings: the three keys of .chaude/settings.json.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub session_retention_days: i64, // 0 = keep forever
    pub extra_coders: i64,           // extra haiku coders (coder2...) on a 270s idle TTL, 0-8
    pub main_model: String,          // opus | sonnet | fable
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            session_retention_days: DEFAULT_RETAIN,
            extra_coders: DEFAULT_EXTRA_CODERS,
            main_model: DEFAULT_MODEL.to_string(),
        }
    }
}

impl Settings {
    // Go clamp: retention >= 0, extra coders 0-8, unknown main model -> opus.
    pub fn clamp(mut self) -> Self {
        self.session_retention_days = self.session_retention_days.max(0);
        self.extra_coders = self.extra_coders.clamp(0, MAX_EXTRA_CODERS);
        if model_id(&self.main_model).is_none() {
            self.main_model = DEFAULT_MODEL.to_string();
        }
        self
    }

    // Go modelID: the claude model id for MainModel (after clamp).
    pub fn model_id(&self) -> &'static str {
        model_id(&self.clone().clamp().main_model).unwrap_or("claude-opus-5-5")
    }
}

// Go mainModelIDs.
fn model_id(alias: &str) -> Option<&'static str> {
    match alias {
        "opus" => Some("claude-opus-5-5"),
        "sonnet" => Some("claude-sonnet-5-5"),
        "fable" => Some("claude-fable-5-1"),
        _ => None,
    }
}

// Go settingsFileFor: <cwd>/.chaude/settings.json. None when the cwd is unknown (an empty cwd means the
// process working directory, as in Go).
pub fn file_for(cwd: Option<&Path>) -> Option<PathBuf> {
    let base = match cwd {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => std::env::current_dir().ok()?,
    };
    Some(base.join(".chaude").join("settings.json"))
}

// Go loadChaudeSettings: a missing or unreadable file yields the defaults; absent fields keep their
// defaults; the result is clamped. The legacy key coder_slots is read when extra_coders is absent.
pub fn load(path: Option<&Path>) -> Settings {
    let mut s = Settings::default();
    let Some(path) = path else {
        return s;
    };
    let Ok(text) = fs::read_to_string(path) else {
        return s;
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return s.clamp();
    };
    if let Some(n) = v.get("session_retention_days").and_then(Value::as_i64) {
        s.session_retention_days = n;
    }
    if let Some(n) = v.get("extra_coders").and_then(Value::as_i64) {
        s.extra_coders = n;
    } else if let Some(n) = v.get("coder_slots").and_then(Value::as_i64) {
        s.extra_coders = n;
    }
    if let Some(m) = v.get("main_model").and_then(Value::as_str) {
        s.main_model = m.to_string();
    }
    s.clamp()
}

// Go saveChaudeSettings: writes the clamped settings through a tmp file and a rename, keys in Go's order.
pub fn save(path: &Path, s: &Settings) -> io::Result<()> {
    let s = s.clone().clamp();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let model = serde_json::to_string(&s.main_model).map_err(io::Error::other)?;
    let body = format!(
        "{{\n  \"session_retention_days\": {},\n  \"extra_coders\": {},\n  \"main_model\": {}\n}}\n",
        s.session_retention_days, s.extra_coders, model
    );
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    {
        let mut f = open_private(&tmp)?;
        f.write_all(body.as_bytes())?;
    }
    fs::rename(&tmp, path)
}

#[cfg(unix)]
fn open_private(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn open_private(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

// Go settingsPick: the open picker. `path` is None when the cwd is unknown (Go "").
pub struct Picker {
    pub s: Settings,
    pub sel: usize,
    pub path: Option<PathBuf>,
}

impl Picker {
    // Go openSettings: loads the file into the picker. Safe while busy: it only reads and edits a copy.
    pub fn open(cwd: Option<&Path>) -> Self {
        let path = file_for(cwd);
        Picker {
            s: load(path.as_deref()),
            sel: 0,
            path,
        }
    }

    // Go tui.go wheel branch on the settings picker: wheelSel over the three rows.
    pub fn wheel(&mut self, up: bool) {
        super::sessview::wheel_sel(&mut self.sel, ROWS, up);
    }

    // Go settingsKey. Returns the note to append when the key closes the picker (after saving), else None.
    pub fn key(&mut self, key: KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.sel = (self.sel + ROWS - 1) % ROWS,
            KeyCode::Down | KeyCode::Char('j') => self.sel = (self.sel + 1) % ROWS,
            KeyCode::Left | KeyCode::Char('h') => self.change(-1),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => self.change(1),
            KeyCode::Esc => return Some(self.close()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Some(self.close());
            }
            KeyCode::Char('q') => return Some(self.close()),
            _ => {}
        }
        None
    }

    // Go closeSettings: saves the working copy and returns the note that says what applies when.
    pub fn close(&mut self) -> String {
        match &self.path {
            None => "설정 경로를 알 수 없음 (작업 폴더 없음)".to_string(),
            Some(p) => match save(p, &self.s) {
                Ok(()) => format!(
                    "설정 저장: {} · 보존 기간·추가 coder 수·메인 모델은 다음 세션 시작 시 적용",
                    p.display()
                ),
                Err(e) => format!("설정 저장 실패: {e}"),
            },
        }
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

    // Go settingsLines: header, three rows (selected orange with "> ", others dim), a dim note; padded or
    // cut to `h` lines. Rows are clipped to `width` columns.
    pub fn lines(&self, width: usize, h: usize) -> Vec<String> {
        let mut l = vec![format!(
            "{ORANGE_ON}{}{FG_OFF}",
            clip(
                "Settings  (↑/↓ wheel move · ←/→ or enter change · esc save & close)",
                width
            )
        )];
        let rows = [
            format!(
                "session_retention_days  {}일 (0 = 영구 보존)",
                self.s.session_retention_days
            ),
            format!(
                "extra_coders            {} (추가 coder (270초) 수, 0-8)",
                self.s.extra_coders
            ),
            format!("main_model              {}", self.s.main_model),
        ];
        for (i, r) in rows.iter().enumerate() {
            let row = if i == self.sel {
                format!("{ORANGE_ON}{}{FG_OFF}", clip(&format!("> {r}"), width))
            } else {
                format!("{GREY_ON}{}{FG_OFF}", clip(&format!("  {r}"), width))
            };
            l.push(row);
        }
        l.push(format!(
            "{GREY_ON}{}{FG_OFF}",
            clip("  변경은 다음 세션 시작 시 적용됩니다.", width)
        ));
        l.resize(h.max(l.len()), String::new());
        l.truncate(h);
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    // A fresh directory under the system temp dir, unique per call.
    fn tmp_dir(tag: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = std::env::temp_dir().join(format!("chaude-settings-{tag}-{n}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // Go TestSettingsDefaultsWhenMissing.
    #[test]
    fn defaults_when_missing() {
        let d = tmp_dir("missing");
        let want = Settings {
            session_retention_days: 30,
            extra_coders: 4,
            main_model: "opus".into(),
        };
        assert_eq!(load(Some(&d.join("none.json"))), want);
        assert_eq!(load(None), want);
    }

    // Go TestSettingsClamp.
    #[test]
    fn clamp_fixes_out_of_range() {
        let s = Settings {
            session_retention_days: -5,
            extra_coders: 99,
            main_model: "gpt".into(),
        }
        .clamp();
        assert_eq!(
            (
                s.session_retention_days,
                s.extra_coders,
                s.main_model.as_str()
            ),
            (0, 8, "opus")
        );
        let low = Settings {
            session_retention_days: 30,
            extra_coders: -1,
            main_model: "sonnet".into(),
        }
        .clamp();
        assert_eq!((low.extra_coders, low.main_model.as_str()), (0, "sonnet"));
    }

    // Go TestSettingsSaveLoadRoundTrip.
    #[test]
    fn save_load_round_trip_leaves_no_tmp() {
        let path = tmp_dir("rt").join(".chaude").join("settings.json");
        let input = Settings {
            session_retention_days: 0,
            extra_coders: 2,
            main_model: "fable".into(),
        };
        save(&path, &input).unwrap();
        assert_eq!(load(Some(&path)), input);
        assert!(!PathBuf::from(format!("{}.tmp", path.display())).exists());
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("{\n  \"session_retention_days\": 0,\n  \"extra_coders\": 2,"));
        assert!(text.ends_with("\"main_model\": \"fable\"\n}\n"));
    }

    // Go TestSettingsPartialFileKeepsDefaults.
    #[test]
    fn partial_file_keeps_defaults_and_reads_legacy_key() {
        let path = tmp_dir("partial").join("s.json");
        fs::write(&path, r#"{"coder_slots": 6}"#).unwrap();
        let s = load(Some(&path));
        assert_eq!(
            (
                s.extra_coders,
                s.session_retention_days,
                s.main_model.as_str()
            ),
            (6, 30, "opus")
        );
    }

    // Go TestLegacyCoderSlotsKey: extra_coders wins when both are present.
    #[test]
    fn extra_coders_wins_over_legacy_key() {
        let dir = tmp_dir("legacy");
        let legacy = dir.join("old.json");
        fs::write(&legacy, r#"{"coder_slots": 2}"#).unwrap();
        assert_eq!(load(Some(&legacy)).extra_coders, 2);
        let both = dir.join("both.json");
        fs::write(&both, r#"{"coder_slots": 2, "extra_coders": 5}"#).unwrap();
        assert_eq!(load(Some(&both)).extra_coders, 5);
    }

    // Go TestSettingsModelID.
    #[test]
    fn model_ids_follow_alias() {
        let id = |m: &str| {
            Settings {
                main_model: m.into(),
                ..Settings::default()
            }
            .model_id()
        };
        assert_eq!(id("opus"), "claude-opus-5-5");
        assert_eq!(id("sonnet"), "claude-sonnet-5-5");
        assert_eq!(id("fable"), "claude-fable-5-1");
        assert_eq!(id("gpt"), "claude-opus-5-5");
    }

    // Go TestSettingsFileIsPerProject and TestSettingsFileEmptyCwdUsesWorkingDir.
    #[test]
    fn settings_file_is_per_project() {
        let (a, b) = (tmp_dir("proj-a"), tmp_dir("proj-b"));
        let path_a = file_for(Some(&a)).unwrap();
        assert_eq!(path_a, a.join(".chaude").join("settings.json"));
        let in_a = Settings {
            session_retention_days: 7,
            extra_coders: 2,
            main_model: "sonnet".into(),
        };
        save(&path_a, &in_a).unwrap();
        assert_eq!(load(file_for(Some(&b)).as_deref()), Settings::default());
        assert_eq!(load(Some(&path_a)), in_a);
        let wd = std::env::current_dir().unwrap();
        assert_eq!(
            file_for(None).unwrap(),
            wd.join(".chaude").join("settings.json")
        );
    }

    // Go settingsKey: right/enter steps the value up, left steps it down, wrapping at the ends.
    #[test]
    fn keys_step_and_wrap_values() {
        let mut p = Picker {
            s: Settings::default(),
            sel: 0,
            path: None,
        };
        p.key(press(KeyCode::Right));
        assert_eq!(p.s.session_retention_days, 90);
        p.key(press(KeyCode::Left));
        p.key(press(KeyCode::Left));
        assert_eq!(p.s.session_retention_days, 7);
        p.key(press(KeyCode::Left));
        assert_eq!(p.s.session_retention_days, 0);
        p.key(press(KeyCode::Left));
        assert_eq!(p.s.session_retention_days, 365, "wraps at the start");

        p.key(press(KeyCode::Down));
        assert_eq!(p.sel, 1);
        p.key(press(KeyCode::Char('l')));
        assert_eq!(p.s.extra_coders, 5);
        p.sel = 1;
        p.s.extra_coders = 0;
        p.key(press(KeyCode::Char('h')));
        assert_eq!(p.s.extra_coders, 8, "extra coders wrap 0 -> 8");

        p.key(press(KeyCode::Down));
        p.key(press(KeyCode::Enter));
        assert_eq!(p.s.main_model, "sonnet");
        p.key(press(KeyCode::Up));
        assert_eq!(p.sel, 1);
        p.key(press(KeyCode::Up));
        p.key(press(KeyCode::Up));
        assert_eq!(p.sel, 2, "up wraps to the last row");
    }

    // Go settingsKey close paths: esc, q and ctrl+c save and return the note; the unknown-path note is used
    // when there is no folder; a failed save reports the error.
    #[test]
    fn close_saves_and_reports() {
        let dir = tmp_dir("close");
        let mut p = Picker::open(Some(&dir));
        assert_eq!(
            p.path.as_deref(),
            Some(dir.join(".chaude").join("settings.json").as_path())
        );
        p.change(1);
        let note = p.key(press(KeyCode::Esc)).unwrap();
        assert!(note.starts_with("설정 저장: "));
        assert!(note.ends_with(" · 보존 기간·추가 coder 수·메인 모델은 다음 세션 시작 시 적용"));
        assert_eq!(load(p.path.as_deref()).session_retention_days, 90);
        assert_eq!(p.s.session_retention_days, 90);

        let mut none = Picker {
            s: Settings::default(),
            sel: 0,
            path: None,
        };
        assert_eq!(
            none.key(press(KeyCode::Char('q'))).unwrap(),
            "설정 경로를 알 수 없음 (작업 폴더 없음)"
        );

        let blocker = dir.join("file");
        fs::write(&blocker, "x").unwrap();
        let mut bad = Picker {
            s: Settings::default(),
            sel: 0,
            path: Some(blocker.join("settings.json")),
        };
        let err = bad.close();
        assert!(err.starts_with("설정 저장 실패: "), "{err}");
    }

    #[test]
    fn lines_fill_and_mark_the_selected_row() {
        let p = Picker {
            s: Settings::default(),
            sel: 2,
            path: None,
        };
        let rows = p.lines(120, 6);
        assert_eq!(rows.len(), 6);
        assert!(rows[0].contains("Settings  (↑/↓ wheel move"));
        assert!(rows[3].starts_with(ORANGE_ON));
        assert!(rows[3].contains("> main_model"));
        assert!(rows[1].contains("session_retention_days  30일 (0 = 영구 보존)"));
        assert_eq!(rows[5], "");
        assert_eq!(p.lines(120, 2).len(), 2);
    }
}
