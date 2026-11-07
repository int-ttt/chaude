//! Derives the OS-specific .claude/settings.json from the posix source (Go: internal/settings).
//! mac/linux output is the source unchanged. windows hooks use exec form (command=<python>, args=[..., script])
//! so no shell is needed.

pub mod chaude_settings;
pub mod project;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::lean::{self, ParseError};
use crate::osenv;

const GEN_USAGE: &str =
    "usage: chaude gen-settings [windows|mac|linux] [--out FILE] [--src FILE] [--python CMD]
  .claude/settings.json(posix 원본)으로부터 OS별 설정을 생성한다.
  OS 생략 시 현재 실행 OS (windows|mac|linux). --out 생략 시 표준출력.
  --out FILE       결과를 이 파일에 쓴다
  --src FILE       원본 설정 (기본: CLAUDE_PROJECT_DIR/.claude/settings.json 또는 bin/ 상위 폴더)
  --python CMD     windows 훅 인터프리터 (기본: py -3 가 있으면 py -3, 아니면 python)";

const VALUE_FLAGS: &[&str] = &["out", "python", "src"];
const POSIX_PREFIX: &str = "python3 \"$CLAUDE_PROJECT_DIR/";
const WIN_PREFIX: &str = "${CLAUDE_PROJECT_DIR}/";

/// Source used when --src is omitted: CLAUDE_PROJECT_DIR, else <exe>/../.claude (bin/ layout) when it exists,
/// else <cwd>/.claude when it exists, else the exe form so the read error names a concrete path.
pub fn default_src(proj_dir: Option<&Path>, cwd: Option<&Path>, exe: &Path) -> PathBuf {
    if let Some(p) = proj_dir {
        return p.join(".claude").join("settings.json");
    }
    let exe_src = exe
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new(".."))
        .join(".claude")
        .join("settings.json");
    if is_file(&exe_src) {
        return exe_src;
    }
    if let Some(c) = cwd {
        let p = c.join(".claude").join("settings.json");
        if is_file(&p) {
            return p;
        }
    }
    exe_src
}

fn is_file(p: &Path) -> bool {
    fs::metadata(p).is_ok_and(|m| !m.is_dir())
}

/// `py -3` when the launcher is on PATH (only checked on windows), else `python`.
pub fn windows_python() -> String {
    if osenv::is_windows() && osenv::find_on_path("py", &osenv::path_dirs()).is_some() {
        "py -3".to_string()
    } else {
        "python".to_string()
    }
}

/// Script name from the posix hook command `python3 "$CLAUDE_PROJECT_DIR/<x>.py"`.
fn posix_script(cmd: &str) -> Option<&str> {
    script_name(cmd.strip_prefix(POSIX_PREFIX)?.strip_suffix('"')?)
}

/// Script name from the windows hook argument `${CLAUDE_PROJECT_DIR}/<x>.py` (re-run on an installed windows file).
fn win_script(arg: &str) -> Option<&str> {
    script_name(arg.strip_prefix(WIN_PREFIX)?)
}

/// Matches `[^"]+\.py`: at least one character, no quote, ending in .py.
fn script_name(s: &str) -> Option<&str> {
    (s.len() > 3 && s.ends_with(".py") && !s.contains('"')).then_some(s)
}

/// Sets `key` like Go's insertAfter: replaces it in place when present, else places it right after `after`
/// (appended when `after` is absent).
fn insert_after(obj: &mut Map<String, Value>, after: &str, key: &str, val: Value) {
    if obj.contains_key(key) || !obj.contains_key(after) {
        obj.insert(key.to_string(), val);
        return;
    }
    let mut out = Map::new();
    let mut val = Some(val);
    for (k, v) in std::mem::take(obj) {
        let anchor = k == after;
        out.insert(k, v);
        if anchor && let Some(v) = val.take() {
            out.insert(key.to_string(), v);
        }
    }
    *obj = out;
}

/// Converts the posix settings text to the windows form. `py` is the interpreter command line (e.g. "py -3",
/// "python", or a full path with spaces); it is split on whitespace unless it names an existing file.
pub fn windows(src: &[u8], py: &str) -> Result<String, String> {
    let mut cfg: Value = serde_json::from_slice(src).map_err(|e| e.to_string())?;
    let mut py_parts: Vec<String> = if is_file(Path::new(py)) {
        vec![py.to_string()]
    } else {
        py.split_whitespace().map(str::to_string).collect()
    };
    if py_parts.is_empty() {
        py_parts.push("python".to_string());
    }

    if let Some(hooks) = cfg.get_mut("hooks").and_then(Value::as_object_mut) {
        for groups in hooks.values_mut().filter_map(Value::as_array_mut) {
            for group in groups.iter_mut() {
                let Some(hs) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
                    continue;
                };
                for h in hs.iter_mut() {
                    let Some(h) = h.as_object_mut() else {
                        continue;
                    };
                    let script = h
                        .get("command")
                        .and_then(Value::as_str)
                        .and_then(posix_script)
                        .or_else(|| {
                            h.get("args")
                                .and_then(Value::as_array)
                                .and_then(|a| a.last())
                                .and_then(Value::as_str)
                                .and_then(win_script)
                        })
                        .map(str::to_string);
                    let Some(script) = script else {
                        continue;
                    };
                    let mut args: Vec<Value> =
                        py_parts[1..].iter().cloned().map(Value::String).collect();
                    args.push(Value::String(format!("${{CLAUDE_PROJECT_DIR}}/{script}")));
                    insert_after(h, "", "command", Value::String(py_parts[0].clone()));
                    insert_after(h, "command", "args", Value::Array(args));
                }
            }
        }
    }

    if let Some(allow) = cfg
        .pointer_mut("/permissions/allow")
        .and_then(Value::as_array_mut)
    {
        for item in allow.clone() {
            let Some(rest) = item
                .as_str()
                .and_then(|s| s.strip_prefix("Bash(bin/chaude "))
            else {
                continue;
            };
            let ps = format!("PowerShell(bin/chaude.exe {rest}");
            if !allow.iter().any(|x| x.as_str() == Some(ps.as_str())) {
                allow.push(Value::String(ps));
            }
        }
    }

    let mut text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    text.push('\n');
    Ok(text)
}

/// Creates the folder that will hold `out`; when that folder is .claude, its hooks/ folder too.
/// Existing folders and files are left as they are.
fn ensure_settings_dir(out: &Path) -> io::Result<()> {
    let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) else {
        return Ok(());
    };
    fs::create_dir_all(dir)?;
    if dir.file_name() == Some(OsStr::new(".claude")) {
        fs::create_dir_all(dir.join("hooks"))?;
    }
    Ok(())
}

/// `chaude gen-settings [windows|mac|linux] [--out FILE] [--src FILE] [--python CMD]`. Returns the exit code.
pub fn gen_settings_main(args: &[String]) -> i32 {
    let exe = env::current_exe().unwrap_or_default();
    let cwd = env::current_dir().ok();
    let proj = env::var_os("CLAUDE_PROJECT_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    let parsed = match lean::parse_interspersed(args, VALUE_FLAGS) {
        Ok(p) => p,
        Err(ParseError::Help) => {
            eprintln!("{GEN_USAGE}");
            return 0;
        }
        Err(ParseError::Msg(msg)) => {
            eprintln!("{msg}\n{GEN_USAGE}");
            return 2;
        }
    };
    let values = &parsed.values;
    let pos = &parsed.positionals;
    if pos.len() > 1
        || pos
            .first()
            .is_some_and(|p| !matches!(p.as_str(), "windows" | "mac" | "linux"))
    {
        eprintln!("{GEN_USAGE}");
        return 2;
    }
    let os_name = pos
        .first()
        .cloned()
        .unwrap_or_else(|| osenv::name().to_string());
    let src = match values.get("src") {
        Some(s) => PathBuf::from(s),
        None => default_src(proj.as_deref(), cwd.as_deref(), &exe),
    };
    let text = match fs::read(&src) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "gen-settings: 원본 설정을 읽을 수 없습니다 (--src 로 지정): {}: {e}",
                src.display()
            );
            return 1;
        }
    };
    let out_bytes = if os_name == "windows" {
        let py = values
            .get("python")
            .filter(|p| !p.is_empty())
            .cloned()
            .unwrap_or_else(windows_python);
        match windows(&text, &py) {
            Ok(t) => t.into_bytes(),
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        }
    } else {
        text
    };
    if serde_json::from_slice::<Value>(&out_bytes).is_err() {
        eprintln!("generated settings is not valid JSON");
        return 1;
    }
    match values.get("out").filter(|o| !o.is_empty()) {
        None => {
            let _ = io::stdout().write_all(&out_bytes);
            0
        }
        Some(out) => {
            let out = Path::new(out);
            if let Err(e) = ensure_settings_dir(out).and_then(|()| fs::write(out, &out_bytes)) {
                eprintln!("{}: {e}", out.display());
                return 1;
            }
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn write_settings(dir: &Path) -> PathBuf {
        let p = dir.join(".claude").join("settings.json");
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(&p, "{}").expect("write");
        p
    }

    #[test]
    fn default_src_precedence() {
        let root = TempDir::new("default-src");
        let proj = root.path().join("proj");
        let cwd = root.path().join("elsewhere");
        let exe = proj.join("bin").join("chaude");
        let proj_src = write_settings(&proj);
        let cwd_src = write_settings(&cwd);

        // 1. CLAUDE_PROJECT_DIR wins over everything.
        assert_eq!(
            default_src(Some(&proj), Some(&cwd), &exe),
            proj.join(".claude").join("settings.json")
        );
        // 2. no env: <project>/bin/chaude -> <project>/.claude when it exists.
        assert_eq!(default_src(None, Some(&cwd), &exe), proj_src);
        // 3. binary elsewhere (no bin/ layout): fall back to cwd/.claude.
        let foreign = root
            .path()
            .join("tmp")
            .join("go-build")
            .join("exe")
            .join("chaude");
        assert_eq!(default_src(None, Some(&cwd), &foreign), cwd_src);
        // 4. nothing exists: exe form returned so the read error names a concrete path.
        let none = root.path().join("none");
        assert_eq!(
            default_src(None, Some(&none), &foreign),
            root.path()
                .join("tmp")
                .join("go-build")
                .join(".claude")
                .join("settings.json")
        );
    }

    #[test]
    fn gen_settings_creates_settings_folder() {
        let root = TempDir::new("gen-folder");
        let src = root.path().join("src.json");
        fs::write(&src, "{}").expect("write src");
        let fresh = root.path().join("fresh").join(".claude");
        let out = fresh.join("settings.json");
        let args: Vec<String> = [
            "mac",
            "--src",
            src.to_str().expect("utf8"),
            "--out",
            out.to_str().expect("utf8"),
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(gen_settings_main(&args), 0);
        for d in [fresh.clone(), fresh.join("hooks")] {
            assert!(d.is_dir(), "folder not created: {}", d.display());
        }
        assert!(out.is_file(), "output not written");
    }

    #[test]
    fn windows_converts_posix_hooks_and_permissions() {
        let src = br#"{"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/ledger.py\"","timeout":5}]}]},"permissions":{"allow":["Bash(bin/chaude metrics)"]}}"#;
        let out = windows(src, "py -3").expect("convert");
        let v: Value = serde_json::from_str(&out).expect("valid json");
        let h = &v["hooks"]["PreToolUse"][0]["hooks"][0];
        assert_eq!(h["command"], "py");
        assert_eq!(
            h["args"],
            serde_json::json!(["-3", "${CLAUDE_PROJECT_DIR}/.claude/hooks/ledger.py"])
        );
        assert_eq!(h["timeout"], 5);
        let keys: Vec<&str> = h
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["type", "command", "args", "timeout"]);
        let allow = v["permissions"]["allow"].as_array().expect("allow");
        assert!(allow.contains(&Value::String(
            "PowerShell(bin/chaude.exe metrics)".to_string()
        )));
    }

    #[test]
    fn windows_repo_settings_have_no_python3() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.claude/settings.json");
        let Ok(src) = fs::read(&repo) else {
            eprintln!("skip: repo .claude/settings.json not reachable from here");
            return;
        };
        for py in ["py -3", "python"] {
            let out = windows(&src, py).expect("convert");
            assert!(!out.contains("python3"), "python3 left ({py}):\n{out}");
        }
    }
}
