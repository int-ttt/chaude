//! Startup cleanup (Go CleanupOld): removes the `.old` file a windows self-update leaves behind.
//! `~/.local/bin` is the install dir now, so startup no longer deletes files there (`migrate_legacy` is kept but not called).

use std::fs;
use std::path::{Path, PathBuf};

use crate::fsutil;
use crate::update::running_exe;

/// Go CleanupOld. Call it at startup. Errors are ignored.
pub fn cleanup_old() {
    let Ok(exe) = running_exe() else {
        return;
    };
    remove_old(&exe);
}

/// Go removeOld: deletes the `exe.old` a windows self-update left behind. Errors are ignored.
pub fn remove_old(exe: &Path) {
    let _ = fs::remove_file(fsutil::with_suffix(exe, ".old"));
}

/// Go os.UserHomeDir: $HOME on unix, USERPROFILE on windows.
pub fn home_dir() -> Option<PathBuf> {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Removes an install left by the old installers in `~/.local/bin`: `chaude`, `chaude.exe` and `chaude.<ver>.bak`.
/// On unix it also removes the exact PATH line the old installer appended to an rc file. Nothing is touched when there
/// is no legacy file, or when the running binary is itself a legacy file (it must keep working).
/// `exe` may be empty, which skips the same-file check.
pub fn migrate_legacy(home: &Path, goos: &str, exe: &Path) {
    let dir = home.join(".local").join("bin");
    let mut candidates = vec![dir.join("chaude"), dir.join("chaude.exe")];
    candidates.extend(legacy_baks(&dir));

    let mut found = Vec::new();
    for c in candidates {
        if fs::symlink_metadata(&c).is_err() {
            continue;
        }
        if !exe.as_os_str().is_empty() && same_file(&c, exe) {
            return;
        }
        found.push(c);
    }
    if found.is_empty() {
        return;
    }
    for c in &found {
        let _ = fs::remove_file(c);
    }
    if goos == "windows" {
        return;
    }
    let line = format!("export PATH=\"{}:$PATH\"", dir.display());
    for rc in [".zshrc", ".bash_profile", ".bashrc", ".profile"] {
        remove_line(&home.join(rc), line.as_bytes());
    }
}

/// Entries matching Go's glob `chaude.*.bak` in `dir`.
fn legacy_baks(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.len() >= 11 && n.starts_with("chaude.") && n.ends_with(".bak"))
        })
        .collect()
}

/// Go os.SameFile: same device and inode on unix. Elsewhere it compares canonical paths.
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(a), fs::metadata(b)) {
            (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        match (fs::canonicalize(a), fs::canonicalize(b)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        }
    }
}

/// Deletes every line that equals `line` exactly from `file`. A missing file or no match changes nothing.
fn remove_line(file: &Path, line: &[u8]) {
    let Ok(data) = fs::read(file) else {
        return;
    };
    let lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    let kept: Vec<&[u8]> = lines.iter().copied().filter(|l| *l != line).collect();
    if kept.len() == lines.len() {
        return;
    }
    let _ = fs::write(file, kept.join(&b'\n'));
}
