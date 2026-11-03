//! Guard hook for a project that has no .claude/settings.json (Go: internal/settings/project.go).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::windows;
use crate::osenv;

/// Project settings written when a folder has no .claude/settings.json: the main-thread guard only
/// (same matcher and command form as the repo's .claude/settings.json; posix form, windows converted by `windows`).
const GUARD_HOOKS: &str = r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Bash|PowerShell|Grep|Glob|WebSearch|WebFetch",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\"",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
"#;

/// Hook scripts the guard needs (main-thread-guard.py imports _lean.py).
const GUARD_FILES: &[&str] = &["main-thread-guard.py", "_lean.py"];

/// Package copies of the repo's .claude/hooks scripts. The binary carries them, so an installed chaude needs no
/// files next to it. embedded_hooks_match_repo keeps them equal to the repo copies.
pub const HOOKS: &[(&str, &[u8])] = &[
    ("_lean.py", include_bytes!("hooks/_lean.py")),
    ("agent-model-pin.py", include_bytes!("hooks/agent-model-pin.py")),
    ("caveman-format.py", include_bytes!("hooks/caveman-format.py")),
    ("context-alert.py", include_bytes!("hooks/context-alert.py")),
    ("correction-guard.py", include_bytes!("hooks/correction-guard.py")),
    ("haiku-context-guard.py", include_bytes!("hooks/haiku-context-guard.py")),
    ("history-export.py", include_bytes!("hooks/history-export.py")),
    ("ledger.py", include_bytes!("hooks/ledger.py")),
    ("main-thread-guard.py", include_bytes!("hooks/main-thread-guard.py")),
    ("session-context.py", include_bytes!("hooks/session-context.py")),
    ("style-adapter.py", include_bytes!("hooks/style-adapter.py")),
];

/// Package copy of a hook script (file name only, e.g. "ledger.py").
pub fn embedded_hook(name: &str) -> Option<&'static [u8]> {
    HOOKS.iter().find(|(n, _)| *n == name).map(|(_, body)| *body)
}

/// Makes a project runnable outside this repo: when `<cwd>/.claude/settings.json` does not exist, creates
/// `<cwd>/.claude/hooks`, writes the guard hook scripts from the embedded copies (existing files are kept) and writes
/// settings.json with the main-thread guard registered. An existing settings.json is never touched, and cwd == home
/// is skipped (that is the user-global .claude). Returns true when settings.json was created.
pub fn ensure_project_settings(cwd: &Path, home: Option<&Path>) -> io::Result<bool> {
    if cwd.as_os_str().is_empty() {
        return Ok(false);
    }
    if home.is_some_and(|h| same_dir(cwd, h)) {
        return Ok(false);
    }
    let dst = cwd.join(".claude").join("settings.json");
    match fs::symlink_metadata(&dst) {
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let hooks_dir = cwd.join(".claude").join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    for name in GUARD_FILES {
        let body = embedded_hook(name)
            .ok_or_else(|| io::Error::other(format!("chaude: embedded hook missing: {name}")))?;
        copy_new(body, &hooks_dir.join(name))?;
    }
    let text = if osenv::is_windows() {
        windows(GUARD_HOOKS.as_bytes(), hook_python()).map_err(io::Error::other)?
    } else {
        GUARD_HOOKS.to_string()
    };
    write_new(&dst, text.as_bytes())?;
    Ok(true)
}

/// Hook interpreter for the windows guard command: the first of python3, python, py found on PATH, else "python".
pub fn hook_python() -> &'static str {
    hook_python_in(&osenv::path_dirs())
}

/// `hook_python` over the given PATH directories.
pub fn hook_python_in(dirs: &[PathBuf]) -> &'static str {
    for (exe, cmd) in [("python3", "python3"), ("python", "python"), ("py", "py -3")] {
        if osenv::find_on_path(exe, dirs).is_some() {
            return cmd;
        }
    }
    "python"
}

/// True when both paths name the same existing directory.
fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Writes `body` to `dst` unless `dst` already exists.
fn copy_new(body: &[u8], dst: &Path) -> io::Result<()> {
    if fs::symlink_metadata(dst).is_ok() {
        return Ok(());
    }
    write_new(dst, body)
}

/// Creates `dst` (and its folder) without overwriting an existing file.
fn write_new(dst: &Path, body: &[u8]) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(dst)?;
    file.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn creates_guard() {
        let root = TempDir::new("guard-create");
        let cwd = root.path().join("cwd");
        let home = root.path().join("home");
        fs::create_dir_all(&cwd).expect("mkdir cwd");
        fs::create_dir_all(&home).expect("mkdir home");
        assert!(ensure_project_settings(&cwd, Some(&home)).expect("ensure"));

        let settings = fs::read_to_string(cwd.join(".claude").join("settings.json")).expect("read settings");
        assert!(settings.contains("\"PreToolUse\""), "guard hook missing:\n{settings}");
        if !osenv::is_windows() {
            assert!(
                settings.contains(r#"python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\""#),
                "guard command missing:\n{settings}"
            );
        }
        for name in GUARD_FILES {
            let got = fs::read(cwd.join(".claude").join("hooks").join(name)).expect("hook written");
            assert_eq!(got.as_slice(), embedded_hook(name).expect("embedded"), "hook {name} differs");
        }
    }

    #[test]
    fn creates_claude_folders_when_missing() {
        let root = TempDir::new("guard-folders");
        let cwd = root.path().join("fresh");
        fs::create_dir_all(&cwd).expect("mkdir cwd");
        assert!(ensure_project_settings(&cwd, None).expect("ensure"));
        for d in [cwd.join(".claude"), cwd.join(".claude").join("hooks")] {
            assert!(d.is_dir(), "folder not created: {}", d.display());
        }
    }

    #[test]
    fn embedded_hooks_match_repo() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.claude/hooks");
        if !repo.is_dir() {
            eprintln!("skip: repo .claude/hooks not reachable from here");
            return;
        }
        for (name, body) in HOOKS {
            let repo_body = fs::read(repo.join(name)).unwrap_or_else(|e| panic!("{name}: repo copy missing: {e}"));
            assert_eq!(*body, repo_body.as_slice(), "{name}: embedded copy differs from .claude/hooks");
        }
    }

    #[test]
    fn keeps_existing_settings() {
        let root = TempDir::new("guard-keep");
        let cwd = root.path().join("cwd");
        let home = root.path().join("home");
        let existing = cwd.join(".claude").join("settings.json");
        fs::create_dir_all(existing.parent().expect("parent")).expect("mkdir");
        fs::create_dir_all(&home).expect("mkdir home");
        const USER: &str = r#"{"model":"sonnet"}"#;
        fs::write(&existing, USER).expect("write user settings");

        assert!(!ensure_project_settings(&cwd, Some(&home)).expect("ensure"));
        assert_eq!(fs::read_to_string(&existing).expect("read"), USER);
        assert!(!cwd.join(".claude").join("hooks").join("main-thread-guard.py").exists());
    }

    #[test]
    fn skips_home() {
        let root = TempDir::new("guard-home");
        let home = root.path().join("home");
        fs::create_dir_all(&home).expect("mkdir home");
        assert!(!ensure_project_settings(&home, Some(&home)).expect("ensure"));
        assert!(!home.join(".claude").exists(), "home was written");
    }

    #[test]
    fn hook_python_order() {
        let cases: [(&[&str], &str); 4] = [
            (&["python3", "python", "py"], "python3"),
            (&["python", "py"], "python"),
            (&["py"], "py -3"),
            (&[], "python"),
        ];
        for (have, want) in cases {
            let root = TempDir::new("hook-python");
            for name in have {
                let p = root.path().join(name);
                fs::write(&p, "#!/bin/sh\n").expect("write fake interpreter");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod");
                }
            }
            let dirs = [root.path().to_path_buf()];
            assert_eq!(hook_python_in(&dirs), want, "have {have:?}");
        }
    }
}
