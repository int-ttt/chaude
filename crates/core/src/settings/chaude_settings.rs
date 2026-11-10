//! Reads the main model from <project>/.chaude/settings.json (Go: chaude_settings.go, modelID).

use std::fs;
use std::path::Path;

use serde_json::Value;

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

#[cfg(test)]
mod tests {
    use super::*;

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
