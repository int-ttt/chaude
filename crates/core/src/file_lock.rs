//! Cross-process lock for the ~/.chaude state files (agents.json, clients.json). Each writer holds an exclusive
//! flock on `<path>.lock` across its read-modify-write, so two chaude processes cannot lose each other's rows.
//! The lock file stays on disk, empty; the lock lives on the open descriptor and is released when it closes.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

/// Exclusive lock on `<path>.lock`, held until dropped. A lock file that cannot be opened gives a guard that
/// locks nothing: the state writers ignore IO errors, so a read-only home still behaves as before.
#[derive(Debug)]
pub struct FileLock {
    _file: Option<File>,
}

impl FileLock {
    /// Blocks until the exclusive lock for `path` is held. Creates the parent directory when missing.
    pub fn acquire(path: &Path) -> Self {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            let _ = fs::create_dir_all(dir);
        }
        let mut name = path.as_os_str().to_os_string();
        name.push(".lock");
        let mut opts = OpenOptions::new();
        opts.create(true).write(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let Ok(file) = opts.open(PathBuf::from(name)) else {
            return FileLock { _file: None };
        };
        lock_exclusive(&file);
        FileLock { _file: Some(file) }
    }
}

/// Blocks on flock(LOCK_EX), retrying when a signal interrupts the wait. Any other error leaves the guard
/// unlocked (best effort, as the callers ignore IO errors).
#[cfg(unix)]
fn lock_exclusive(file: &File) {
    use std::os::unix::io::AsRawFd;
    loop {
        // SAFETY: the descriptor belongs to `file`, which stays open for the duration of the call.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
            return;
        }
        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return;
        }
    }
}

/// Blocks on LockFileEx(LOCKFILE_EXCLUSIVE_LOCK) over the first byte. The lock belongs to the handle and is
/// released when the file closes, the same as flock. Failures are ignored, as on unix.
#[cfg(windows)]
fn lock_exclusive(file: &File) {
    lock_range(file, 0);
}

/// Non-blocking variant for tests: true when the lock was taken.
#[cfg(all(test, unix))]
fn try_lock_exclusive(file: &File) -> bool {
    use std::os::unix::io::AsRawFd;
    // SAFETY: the descriptor belongs to `file`, which stays open for the duration of the call.
    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) == 0 }
}

#[cfg(all(test, windows))]
fn try_lock_exclusive(file: &File) -> bool {
    lock_range(
        file,
        windows_sys::Win32::Storage::FileSystem::LOCKFILE_FAIL_IMMEDIATELY,
    )
}

#[cfg(windows)]
fn lock_range(file: &File, extra_flags: u32) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{LOCKFILE_EXCLUSIVE_LOCK, LockFileEx};
    use windows_sys::Win32::System::IO::OVERLAPPED;
    // SAFETY: OVERLAPPED is plain data, and an all-zero value is a valid offset of 0 for a synchronous call.
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    // SAFETY: the handle belongs to `file`, which stays open for the duration of the call, and `overlapped`
    // outlives it.
    unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK | extra_flags,
            0,
            1,
            0,
            &mut overlapped,
        ) != 0
    }
}

// Targets with neither flock nor LockFileEx have no cross-process guard; the in-process mutexes remain.
#[cfg(not(any(unix, windows)))]
fn lock_exclusive(_file: &File) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    // The lock file sits beside the state file, and a second acquire waits until the first guard drops.
    #[test]
    fn lock_file_sits_beside_state_and_serializes() {
        let dir = TempDir::new("file-lock");
        let path = dir.path().join("state.json");
        let first = FileLock::acquire(&path);
        assert!(dir.path().join("state.json.lock").exists());
        let (tx, rx) = std::sync::mpsc::channel();
        let p2 = path.clone();
        let h = std::thread::spawn(move || {
            let _second = FileLock::acquire(&p2);
            tx.send(()).expect("send");
        });
        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(200))
                .is_err(),
            "second lock must wait while the first is held"
        );
        drop(first);
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .expect("second lock acquired after release");
        h.join().expect("thread");
    }

    // A second handle on the same lock file cannot take the lock while the guard is held, and can after it drops.
    #[test]
    fn second_handle_try_lock_fails_while_held() {
        let dir = TempDir::new("file-lock-try");
        let path = dir.path().join("state.json");
        let first = FileLock::acquire(&path);
        let lock_path = dir.path().join("state.json.lock");
        let probe = OpenOptions::new()
            .write(true)
            .open(&lock_path)
            .expect("open second handle");
        assert!(
            !try_lock_exclusive(&probe),
            "second handle must not lock while held"
        );
        drop(first);
        assert!(
            try_lock_exclusive(&probe),
            "second handle locks after release"
        );
    }
}
