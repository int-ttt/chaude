//! The only module that knows OS differences. Everything else stays OS-neutral.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn is_windows() -> bool {
    cfg!(windows)
}

pub fn is_mac() -> bool {
    cfg!(target_os = "macos")
}

/// "windows", "mac" or "linux".
pub fn name() -> &'static str {
    if is_windows() {
        "windows"
    } else if is_mac() {
        "mac"
    } else {
        "linux"
    }
}

/// Directories listed in PATH.
pub fn path_dirs() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default()
}

/// Full path of `name` found in `dirs` (windows also tries PATHEXT suffixes); None when absent.
pub fn find_on_path(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .flat_map(|dir| candidates(dir, name))
        .find(|p| is_exec(p))
}

/// Full path of `name` on PATH (finds claude.cmd/.exe on Windows); falls back to the bare name.
pub fn resolve_bin(name: &str) -> String {
    match find_on_path(name, &path_dirs()) {
        Some(p) => p.to_string_lossy().into_owned(),
        None => name.to_string(),
    }
}

fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = vec![dir.join(name)];
    if is_windows() {
        let exts = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        out.extend(
            exts.split(';')
                .filter(|e| !e.is_empty())
                .map(|e| dir.join(format!("{name}{e}"))),
        );
    }
    out
}

#[cfg(unix)]
fn is_exec(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_exec(p: &Path) -> bool {
    p.is_file()
}

/// $HOME, then %USERPROFILE% (the old cli order on every OS); None when both are unset or empty.
pub fn try_home_dir() -> Option<PathBuf> {
    pick_home(env::var_os("HOME"), env::var_os("USERPROFILE"))
}

fn pick_home(home: Option<OsString>, profile: Option<OsString>) -> Option<PathBuf> {
    home.filter(|h| !h.is_empty())
        .or_else(|| profile.filter(|h| !h.is_empty()))
        .map(PathBuf::from)
}

/// $HOME, then %USERPROFILE%; "." when both are unset.
pub fn home_dir() -> PathBuf {
    try_home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// windows %LOCALAPPDATA%; linux XDG state; mac unchanged ~/.claude-harness (compat).
pub fn state_dir() -> PathBuf {
    let home = home_dir();
    if is_windows() {
        return match env::var_os("LOCALAPPDATA") {
            Some(d) if !d.is_empty() => PathBuf::from(d).join("claude-harness"),
            _ => home.join("claude-harness"),
        };
    }
    if is_mac() {
        return home.join(".claude-harness");
    }
    match env::var_os("XDG_STATE_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d).join("claude-harness"),
        _ => home.join(".local").join("state").join("claude-harness"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(s: &str) -> Option<OsString> {
        Some(OsString::from(s))
    }

    #[test]
    fn home_reads_home_first_then_userprofile() {
        assert_eq!(pick_home(os("/h"), os("C:\\u")), Some(PathBuf::from("/h")));
        assert_eq!(
            pick_home(Some(OsString::new()), os("C:\\u")),
            Some(PathBuf::from("C:\\u"))
        );
        assert_eq!(pick_home(None, os("C:\\u")), Some(PathBuf::from("C:\\u")));
        assert_eq!(pick_home(None, Some(OsString::new())), None);
        assert_eq!(pick_home(None, None), None);
    }
}
