//! Small filesystem helpers: unique names and a self-removing temp dir (Go os.MkdirTemp / os.CreateTemp).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A process-unique suffix such as "1234-1700000000000000000-7".
pub fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{}", std::process::id(), nanos, n)
}

/// Appends `suffix` to the whole path (Go `exe + ".old"`), not to the extension.
pub fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// A directory under the system temp dir, removed (recursively) on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates `<temp>/<prefix><unique>`.
    pub fn new(prefix: &str) -> io::Result<TempDir> {
        let base = std::env::temp_dir();
        loop {
            let path = base.join(format!("{prefix}{}", unique_suffix()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(TempDir { path }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
