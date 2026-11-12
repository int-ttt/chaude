//! Reads the main model from <project>/.chaude/settings.json (Go: chaude_settings.go, modelID).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::Value;

use crate::i18n;
use crate::record::dir_for;
use crate::record_folder::prune_record_files;

// Go settingsDefaultModel "opus": the id used when the file, the field, or the value is missing or unknown.
const DEFAULT_ID: &str = "claude-opus-5-5";

// Go mainModelIDs: settings alias -> claude model id.
fn model_id(alias: &str) -> Option<&'static str> {
    match alias {
        "opus" => Some("claude-opus-5-5"),
        "sonnet" => Some("claude-sonnet-5-5"),
        "fable" => Some("claude-fable-5-1"),
        _ => None,
    }
}

// Pure: the main model id for settings JSON text. Invalid JSON, a missing field, or an unknown alias
// all fall back to the default alias (Go loadChaudeSettings + clamp).
pub fn main_model_from_json(text: &str) -> &'static str {
    let alias = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| {
            v.get("main_model")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    model_id(&alias).unwrap_or(DEFAULT_ID)
}

// The main model id for a project folder. None means the process working directory (Go settingsFileFor).
// A missing or unreadable file yields the default.
pub fn main_model(cwd: Option<&Path>) -> &'static str {
    let dir = match cwd {
        Some(d) => d.to_path_buf(),
        None => match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => return main_model_from_json(""),
        },
    };
    match fs::read_to_string(dir.join(".chaude").join("settings.json")) {
        Ok(text) => main_model_from_json(&text),
        Err(_) => main_model_from_json(""),
    }
}

// The raw `language` value (auto|ko|en) from the project settings text, or None when absent or not a string.
pub fn language_from_json(text: &str) -> Option<String> {
    serde_json::from_str::<Value>(text)
        .ok()?
        .get("language")?
        .as_str()
        .map(str::to_owned)
}

// The raw `language` value for a project folder (same file and cwd rule as main_model).
pub fn language(cwd: Option<&Path>) -> Option<String> {
    let dir = match cwd {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir().ok()?,
    };
    fs::read_to_string(dir.join(".chaude").join("settings.json"))
        .ok()
        .and_then(|text| language_from_json(&text))
}

// Writes `language` (auto|ko|en or a locale that maps to one) into the project settings, keeping every other key.
// An invalid value or an unreadable/non-object file is rejected before anything is written.
pub fn save_language(cwd: &Path, raw: &str) -> io::Result<()> {
    let code = i18n::pref_code(raw).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown language {raw}"),
        )
    })?;
    let dir = cwd.join(".chaude");
    let path = dir.join("settings.json");
    let mut obj = match fs::read_to_string(&path) {
        Ok(text) if text.trim().is_empty() => serde_json::Map::new(),
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(m)) => m,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "settings.json is not a JSON object",
                ));
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => serde_json::Map::new(),
        Err(e) => return Err(e),
    };
    obj.insert("language".into(), Value::String(code.into()));
    let mut text = serde_json::to_string_pretty(&Value::Object(obj)).map_err(io::Error::other)?;
    text.push('\n');
    fs::create_dir_all(&dir)?;
    let tmp = dir.join("settings.json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &path)
}

// The `agent_wait` flag: a main reply waits for its running Agent/Task calls. Absent or not a bool means true.
pub fn agent_wait_from_json(text: &str) -> bool {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.get("agent_wait").and_then(Value::as_bool))
        .unwrap_or(true)
}

// The agent_wait flag for a project folder (same file and cwd rule as main_model).
pub fn agent_wait(cwd: Option<&Path>) -> bool {
    let dir = match cwd {
        Some(d) => d.to_path_buf(),
        None => match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => return true,
        },
    };
    fs::read_to_string(dir.join(".chaude").join("settings.json"))
        .map_or(true, |text| agent_wait_from_json(&text))
}

// Go chaude_settings.go constants.
pub const MAX_EXTRA_CODERS: i64 = 8;
pub const DEFAULT_RETAIN: i64 = 30;
pub const DEFAULT_EXTRA_CODERS: i64 = 4;
pub const DEFAULT_MODEL: &str = "opus";

// Go settingsRetainChoices / settingsModelChoices / settingsRows.
pub const RETAIN_CHOICES: [i64; 5] = [0, 7, 30, 90, 365];
pub const MODEL_CHOICES: [&str; 3] = ["opus", "sonnet", "fable"];

// Go chaudeSettings: the three keys of .chaude/settings.json.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub session_retention_days: i64, // 0 = keep forever
    pub extra_coders: i64,           // extra haiku coders (coder2...) on a 3570s idle TTL, 0-8
    pub main_model: String,          // opus | sonnet | fable
    pub language: String,            // auto | ko | en (file-only key; no picker row, kept on save)
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            session_retention_days: DEFAULT_RETAIN,
            extra_coders: DEFAULT_EXTRA_CODERS,
            main_model: DEFAULT_MODEL.to_string(),
            language: "auto".to_string(),
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
        if !matches!(self.language.as_str(), "auto" | "ko" | "en") {
            self.language = "auto".to_string();
        }
        self
    }

    // Go modelID: the claude model id for MainModel (after clamp).
    pub fn model_id(&self) -> &'static str {
        model_id(&self.clone().clamp().main_model).unwrap_or("claude-opus-5-5")
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
    if let Some(l) = v.get("language").and_then(Value::as_str) {
        s.language = l.to_string();
    }
    s.clamp()
}

// Go repl.go:1357 pruneRecordFiles(recordDir(cwd), SessionRetentionDays, time.Now()): the startup prune of
// old records. Errors are ignored, as in Go (`_, _ =`). Returns the removed count.
pub fn prune_records_at_startup(cwd: &Path, s: &Settings, now: SystemTime) -> usize {
    prune_record_files(&dir_for(cwd), s.session_retention_days, now).unwrap_or(0)
}

// Go saveChaudeSettings: writes the clamped settings through a tmp file and a rename, keys in Go's order.
pub fn save(path: &Path, s: &Settings) -> io::Result<()> {
    let s = s.clone().clamp();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let model = serde_json::to_string(&s.main_model).map_err(io::Error::other)?;
    let language = serde_json::to_string(&s.language).map_err(io::Error::other)?;
    let body = format!(
        "{{\n  \"session_retention_days\": {},\n  \"extra_coders\": {},\n  \"main_model\": {},\n  \"language\": {}\n}}\n",
        s.session_retention_days, s.extra_coders, model, language
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_wait_defaults_true_and_reads_bool() {
        assert!(agent_wait_from_json("{}"));
        assert!(agent_wait_from_json(r#"{"agent_wait":"no"}"#));
        assert!(!agent_wait_from_json(r#"{"agent_wait":false}"#));
        assert!(agent_wait_from_json("not json"));
    }

    #[test]
    fn language_field_is_optional_raw_string() {
        assert_eq!(
            language_from_json(r#"{"language":"en"}"#).as_deref(),
            Some("en")
        );
        assert_eq!(language_from_json(r#"{"main_model":"opus"}"#), None);
        assert_eq!(language_from_json(r#"{"language":3}"#), None);
        assert_eq!(language_from_json("not json"), None);
    }

    #[test]
    fn save_language_keeps_other_keys_and_rejects_invalid() {
        let dir = std::env::temp_dir().join(format!("chaude-lang-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        save_language(&dir, "en").unwrap();
        assert_eq!(language(Some(&dir)).as_deref(), Some("en"));
        fs::write(
            dir.join(".chaude").join("settings.json"),
            r#"{"main_model":"sonnet","agent_wait":false}"#,
        )
        .unwrap();
        save_language(&dir, "ko").unwrap();
        let text = fs::read_to_string(dir.join(".chaude").join("settings.json")).unwrap();
        assert_eq!(language_from_json(&text).as_deref(), Some("ko"));
        assert_eq!(main_model_from_json(&text), "claude-sonnet-5-5");
        assert!(!agent_wait_from_json(&text));
        assert!(save_language(&dir, "fr").is_err());
        assert_eq!(
            fs::read_to_string(dir.join(".chaude").join("settings.json")).unwrap(),
            text,
            "invalid value must not touch the file"
        );
        fs::write(dir.join(".chaude").join("settings.json"), "not json").unwrap();
        assert!(save_language(&dir, "en").is_err());
        assert_eq!(
            fs::read_to_string(dir.join(".chaude").join("settings.json")).unwrap(),
            "not json",
            "corrupt file must not be clobbered"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn known_aliases_map_to_ids() {
        assert_eq!(
            main_model_from_json(r#"{"main_model":"opus"}"#),
            "claude-opus-5-5"
        );
        assert_eq!(
            main_model_from_json(r#"{"main_model":"sonnet"}"#),
            "claude-sonnet-5-5"
        );
        assert_eq!(
            main_model_from_json(r#"{"main_model":"fable"}"#),
            "claude-fable-5-1"
        );
    }

    #[test]
    fn missing_or_unknown_falls_back_to_opus() {
        assert_eq!(
            main_model_from_json(r#"{"extra_coders":2}"#),
            "claude-opus-5-5"
        );
        assert_eq!(
            main_model_from_json(r#"{"main_model":"gpt"}"#),
            "claude-opus-5-5"
        );
        assert_eq!(main_model_from_json("not json"), "claude-opus-5-5");
        assert_eq!(main_model_from_json(""), "claude-opus-5-5");
    }

    #[test]
    fn reads_project_file_and_defaults_when_absent() {
        let dir = std::env::temp_dir().join(format!("chaude-settings-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(main_model(Some(&dir)), "claude-opus-5-5");
        fs::create_dir_all(dir.join(".chaude")).unwrap();
        fs::write(
            dir.join(".chaude").join("settings.json"),
            r#"{"main_model":"sonnet"}"#,
        )
        .unwrap();
        assert_eq!(main_model(Some(&dir)), "claude-sonnet-5-5");
        let _ = fs::remove_dir_all(&dir);
    }
}
