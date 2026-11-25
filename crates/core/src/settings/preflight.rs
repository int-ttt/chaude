//! Startup checks for the setup that lets a project's hooks and agents load on a machine (a Mac with another home,
//! trust state or python than the machine the project was made on). Each check returns one short line; the caller
//! prints them. The python probe and the managed-settings path are parameters so tests never spawn or read them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::i18n::{self, Lang};

/// macOS managed (enterprise) settings file.
pub const MANAGED_SETTINGS: &str = "/Library/Application Support/ClaudeCode/managed-settings.json";

/// The checks with the real python3 probe, the macOS managed-settings path and the UI language.
pub fn check(cwd: &Path, home: &Path, env: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    check_with(
        cwd,
        home,
        env,
        Path::new(MANAGED_SETTINGS),
        i18n::current(),
        &python3_ok,
    )
}

/// `check` with every outside input passed in: `managed` is the managed-settings file, `python_ok` runs the probe.
pub fn check_with(
    cwd: &Path,
    home: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    managed: &Path,
    lang: Lang,
    python_ok: &dyn Fn() -> bool,
) -> Vec<String> {
    let mut out = Vec::new();
    if !has_project(cwd, home) {
        out.push(tr(
            lang,
            "프로젝트 폴더 안에서 chaude를 실행하세요.".into(),
            "run chaude inside the project folder".into(),
        ));
    }
    let config = match env("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir).join(".claude.json"),
        None => home.join(".claude.json"),
    };
    if !trusted(&config, cwd) {
        out.push(tr(
            lang,
            "이 폴더에서 `claude`를 한 번 실행해 신뢰 확인 창을 수락하세요.".into(),
            "run `claude` here once and accept the trust prompt".into(),
        ));
    }
    if !cfg!(windows) && !python_ok() {
        out.push(tr(
            lang,
            "python3가 없습니다. xcode-select --install 또는 python.org로 설치하세요. 훅이 꺼져 있습니다.".into(),
            "python3 missing; install via xcode-select --install or python.org; hooks are off".into(),
        ));
    }
    if let Some(dir) = env("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
        out.push(format!("CLAUDE_CONFIG_DIR={dir}"));
    }
    let files = [
        managed.to_path_buf(),
        home.join(".claude").join("settings.json"),
        cwd.join(".claude").join("settings.local.json"),
    ];
    for file in files {
        if hooks_disabled(&file) {
            out.push(tr(
                lang,
                format!(
                    "{} 때문에 훅이 꺼져 있습니다 (disableAllHooks/allowManagedHooksOnly).",
                    file.display()
                ),
                format!(
                    "hooks are off: {} sets disableAllHooks or allowManagedHooksOnly",
                    file.display()
                ),
            ));
        }
    }
    out
}

/// True when cwd is a project folder: not the home folder itself, and it has `.claude/agents`.
fn has_project(cwd: &Path, home: &Path) -> bool {
    let same = cwd == home
        || matches!(
            (fs::canonicalize(cwd), fs::canonicalize(home)),
            (Ok(a), Ok(b)) if a == b
        );
    !same && cwd.join(".claude").join("agents").is_dir()
}

/// True when cwd or one of its ancestors is marked `hasTrustDialogAccepted` in the `projects` map of `config`.
/// A missing or unreadable file is not trusted.
fn trusted(config: &Path, cwd: &Path) -> bool {
    let Ok(text) = fs::read(config) else {
        return false;
    };
    let Ok(v) = serde_json::from_slice::<Value>(&text) else {
        return false;
    };
    let Some(projects) = v.get("projects").and_then(Value::as_object) else {
        return false;
    };
    let mut dirs = vec![cwd.to_path_buf()];
    if let Some(real) = fs::canonicalize(cwd).ok().filter(|real| real != cwd) {
        dirs.push(real);
    }
    dirs.iter().any(|d| {
        d.ancestors().any(|a| {
            let key = a.to_string_lossy();
            projects
                .get(key.as_ref())
                .and_then(|p| p.get("hasTrustDialogAccepted"))
                .and_then(Value::as_bool)
                == Some(true)
        })
    })
}

/// True when the settings file turns hooks off (`disableAllHooks` or `allowManagedHooksOnly` set to true).
fn hooks_disabled(file: &Path) -> bool {
    let Ok(text) = fs::read(file) else {
        return false;
    };
    let Ok(v) = serde_json::from_slice::<Value>(&text) else {
        return false;
    };
    ["disableAllHooks", "allowManagedHooksOnly"]
        .iter()
        .any(|k| v.get(*k).and_then(Value::as_bool) == Some(true))
}

/// `python3 -c "import sys"` must exit 0 within 3 seconds. A missing binary, a failed run or a timeout is false.
pub fn python3_ok() -> bool {
    let Ok(mut child) = Command::new("python3")
        .args(["-c", "import sys"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

fn tr(lang: Lang, ko: String, en: String) -> String {
    match lang {
        Lang::Ko => ko,
        Lang::En => en,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    // A project folder under `root` with .claude/agents, and a home folder that is not the project.
    fn project(root: &Path) -> (PathBuf, PathBuf) {
        let cwd = root.join("proj");
        let home = root.join("home");
        fs::create_dir_all(cwd.join(".claude").join("agents")).expect("mkdir agents");
        fs::create_dir_all(&home).expect("mkdir home");
        (cwd, home)
    }

    fn run(
        cwd: &Path,
        home: &Path,
        env: &dyn Fn(&str) -> Option<String>,
        managed: &Path,
        py: bool,
    ) -> Vec<String> {
        check_with(cwd, home, env, managed, Lang::En, &|| py)
    }

    fn trust_file(home: &Path, body: &str) {
        fs::write(home.join(".claude.json"), body).expect("write config");
    }

    #[test]
    fn clean_trusted_project_has_no_warnings() {
        let root = TempDir::new("pre-clean");
        let (cwd, home) = project(root.path());
        let body = serde_json::json!({
            "projects": {cwd.to_string_lossy(): {"hasTrustDialogAccepted": true}}
        });
        trust_file(&home, &body.to_string());
        let out = run(&cwd, &home, &no_env, &root.path().join("none.json"), true);
        assert!(out.is_empty(), "unexpected: {out:?}");
    }

    #[test]
    fn home_or_no_agents_dir_warns() {
        let root = TempDir::new("pre-project");
        let (cwd, home) = project(root.path());
        trust_file(&home, "{}");
        let managed = root.path().join("none.json");
        let out = run(&home, &home, &no_env, &managed, true);
        assert!(
            out.iter().any(|w| w.contains("inside the project folder")),
            "home: {out:?}"
        );
        let bare = root.path().join("bare");
        fs::create_dir_all(&bare).expect("mkdir bare");
        let out = run(&bare, &home, &no_env, &managed, true);
        assert!(
            out.iter().any(|w| w.contains("inside the project folder")),
            "bare: {out:?}"
        );
        let out = run(&cwd, &home, &no_env, &managed, true);
        assert!(
            !out.iter().any(|w| w.contains("inside the project folder")),
            "ok: {out:?}"
        );
    }

    #[test]
    fn trust_inherits_from_ancestor_and_missing_file_warns() {
        let root = TempDir::new("pre-trust");
        let (cwd, home) = project(root.path());
        let managed = root.path().join("none.json");
        let missing = run(&cwd, &home, &no_env, &managed, true);
        assert!(
            missing
                .iter()
                .any(|w| w.contains("accept the trust prompt")),
            "missing: {missing:?}"
        );

        let parent = root.path().to_string_lossy().to_string();
        trust_file(
            &home,
            &serde_json::json!({"projects": {parent: {"hasTrustDialogAccepted": true}}})
                .to_string(),
        );
        let inherited = run(&cwd, &home, &no_env, &managed, true);
        assert!(
            !inherited.iter().any(|w| w.contains("trust")),
            "ancestor: {inherited:?}"
        );

        trust_file(&home, &serde_json::json!({"projects": {cwd.to_string_lossy(): {"hasTrustDialogAccepted": false}}}).to_string());
        let untrusted = run(&cwd, &home, &no_env, &managed, true);
        assert!(
            untrusted
                .iter()
                .any(|w| w.contains("accept the trust prompt")),
            "false: {untrusted:?}"
        );
    }

    #[test]
    fn config_dir_env_selects_trust_file_and_is_reported() {
        let root = TempDir::new("pre-config-dir");
        let (cwd, home) = project(root.path());
        let cfg_dir = root.path().join("alt");
        fs::create_dir_all(&cfg_dir).expect("mkdir alt");
        let body = serde_json::json!({"projects": {cwd.to_string_lossy(): {"hasTrustDialogAccepted": true}}});
        fs::write(cfg_dir.join(".claude.json"), body.to_string()).expect("write alt config");
        trust_file(&home, "{}");
        let dir = cfg_dir.to_string_lossy().to_string();
        let env = move |k: &str| (k == "CLAUDE_CONFIG_DIR").then(|| dir.clone());
        let out = run(&cwd, &home, &env, &root.path().join("none.json"), true);
        assert_eq!(out.len(), 1, "expected only the info line: {out:?}");
        assert!(out[0].starts_with("CLAUDE_CONFIG_DIR="), "info: {out:?}");
        assert!(out[0].ends_with("alt"), "info value: {out:?}");
    }

    #[test]
    fn python_missing_warns_only_when_probe_fails() {
        let root = TempDir::new("pre-python");
        let (cwd, home) = project(root.path());
        trust_file(&home, &serde_json::json!({"projects": {cwd.to_string_lossy(): {"hasTrustDialogAccepted": true}}}).to_string());
        let managed = root.path().join("none.json");
        let fail = run(&cwd, &home, &no_env, &managed, false);
        if cfg!(windows) {
            assert!(fail.is_empty(), "windows skips probe: {fail:?}");
        } else {
            assert_eq!(fail.len(), 1, "python: {fail:?}");
            assert!(fail[0].contains("python3 missing"), "text: {fail:?}");
        }
        let ok = run(&cwd, &home, &no_env, &managed, true);
        assert!(ok.is_empty(), "probe ok: {ok:?}");
    }

    #[test]
    fn hooks_off_names_the_file_for_each_source() {
        let root = TempDir::new("pre-hooks-off");
        let (cwd, home) = project(root.path());
        trust_file(&home, &serde_json::json!({"projects": {cwd.to_string_lossy(): {"hasTrustDialogAccepted": true}}}).to_string());
        let managed = root.path().join("managed.json");
        fs::write(&managed, r#"{"allowManagedHooksOnly": true}"#).expect("write managed");
        fs::create_dir_all(home.join(".claude")).expect("mkdir home claude");
        fs::write(
            home.join(".claude").join("settings.json"),
            r#"{"disableAllHooks": true}"#,
        )
        .expect("write home");
        fs::write(
            cwd.join(".claude").join("settings.local.json"),
            r#"{"disableAllHooks": false}"#,
        )
        .expect("write local");
        let out = run(&cwd, &home, &no_env, &managed, true);
        assert_eq!(out.len(), 2, "managed and home only: {out:?}");
        assert!(
            out[0].contains(&managed.display().to_string()),
            "managed: {out:?}"
        );
        assert!(out[1].contains("settings.json"), "home: {out:?}");
        assert!(
            !out.iter().any(|w| w.contains("settings.local.json")),
            "local false: {out:?}"
        );
    }

    #[test]
    fn korean_lang_uses_korean_text() {
        let root = TempDir::new("pre-ko");
        let (_cwd, home) = project(root.path());
        let out = check_with(
            &home,
            &home,
            &no_env,
            &root.path().join("none.json"),
            Lang::Ko,
            &|| true,
        );
        assert!(
            out.iter().any(|w| w.contains("프로젝트 폴더")),
            "ko: {out:?}"
        );
    }
}
