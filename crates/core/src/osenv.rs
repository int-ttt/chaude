//! The only module that knows OS differences. Everything else stays OS-neutral.

use std::env;
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

/// Go's os.UserHomeDir: $HOME, or %USERPROFILE% on windows; "." when unset.
pub fn home_dir() -> PathBuf {
    let var = if is_windows() { "USERPROFILE" } else { "HOME" };
    match env::var_os(var) {
        Some(h) if !h.is_empty() => PathBuf::from(h),
        _ => PathBuf::from("."),
    }
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
