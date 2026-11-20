//! The GUI's recent folders (~/.chaude/recent.json) and the scratch dirs of chat mode (~/.chaude/chat/<id>).
//! Callers pass the config dir, so tests use temp dirs and never touch the real home.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// How many folders the start screen keeps.
pub const MAX_RECENT: usize = 10;

// One row of recent.json. `used` is the unix time (seconds) the folder was last opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentFolder {
    pub path: String,
    pub used: u64,
}

// The chaude config dir: ~/.chaude.
pub fn config_dir() -> PathBuf {
    crate::osenv::home_dir().join(".chaude")
}

// The recent folders file under the config dir.
pub fn recent_file(config: &Path) -> PathBuf {
    config.join("recent.json")
}

// Rows of the file, newest first. A missing or unreadable file gives none.
pub fn load(path: &Path) -> Vec<RecentFolder> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<RecentFolder>>(&text).ok())
        .map(|mut rows| {
            rows.sort_by(|a, b| b.used.cmp(&a.used).then_with(|| a.path.cmp(&b.path)));
            rows.truncate(MAX_RECENT);
            rows
        })
        .unwrap_or_default()
}

// Records that `folder` was opened at `now` (one row per path, the newest use wins) and saves the file. Returns the
// rows as saved. A failed write is an Err, and the caller keeps the rows it has.
pub fn touch(path: &Path, folder: &str, now: u64) -> io::Result<Vec<RecentFolder>> {
    let mut rows = load(path);
    rows.retain(|r| r.path != folder);
    rows.insert(
        0,
        RecentFolder {
            path: folder.to_string(),
            used: now,
        },
    );
    rows.truncate(MAX_RECENT);
    save(path, &rows)?;
    Ok(rows)
}

// Every row of the file as saved (no sort, no cap). A missing or unreadable file gives none.
pub fn all_rows(path: &Path) -> Vec<RecentFolder> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

// Drops the rows whose path is one of `folders` and saves the file when any went. Returns how many rows went. A
// missing file or no matching row changes nothing; other rows are kept as they are.
pub fn forget(path: &Path, folders: &[String]) -> io::Result<usize> {
    let mut rows = all_rows(path);
    let before = rows.len();
    rows.retain(|r| !folders.contains(&r.path));
    let gone = before - rows.len();
    if gone > 0 {
        save(path, &rows)?;
    }
    Ok(gone)
}

// Writes the rows through a tmp file and rename, so a crash never leaves half a file.
fn save(path: &Path, rows: &[RecentFolder]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(rows).map_err(io::Error::other)?;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

// The scratch dir of one chat: config/chat/<id>, created. Chat mode runs its engine here.
pub fn new_chat_dir(config: &Path, id: &str) -> io::Result<PathBuf> {
    let dir = config.join("chat").join(id);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

// The chat scratch dirs under config/chat, newest first (used = the dir's modified time in unix seconds), capped at
// MAX_RECENT. A missing or unreadable dir lists nothing. Only directories count; stray files are skipped.
pub fn chat_dirs(config: &Path) -> Vec<RecentFolder> {
    let Ok(entries) = fs::read_dir(config.join("chat")) else {
        return Vec::new();
    };
    let mut rows: Vec<RecentFolder> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| RecentFolder {
            path: e.path().display().to_string(),
            used: e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs()),
        })
        .collect();
    rows.sort_by(|a, b| b.used.cmp(&a.used).then_with(|| b.path.cmp(&a.path)));
    rows.truncate(MAX_RECENT);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    // A fresh temp dir per test; never the real home.
    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chaude-recent-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn touch_keeps_one_row_per_folder_newest_first() {
        let dir = temp("order");
        let file = recent_file(&dir);
        assert!(load(&file).is_empty());
        touch(&file, "/w/a", 10).unwrap();
        let rows = touch(&file, "/w/b", 20).unwrap();
        assert_eq!(
            rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            ["/w/b", "/w/a"]
        );
        let rows = touch(&file, "/w/a", 30).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            RecentFolder {
                path: "/w/a".into(),
                used: 30
            }
        );
        assert_eq!(load(&file), rows);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_is_capped_and_bad_file_reads_empty() {
        let dir = temp("cap");
        let file = recent_file(&dir);
        for i in 0..(MAX_RECENT as u64 + 3) {
            touch(&file, &format!("/w/{i}"), i).unwrap();
        }
        let rows = load(&file);
        assert_eq!(rows.len(), MAX_RECENT);
        assert_eq!(rows[0].path, format!("/w/{}", MAX_RECENT as u64 + 2));
        fs::write(&file, "{not json").unwrap();
        assert!(load(&file).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn chat_dir_is_created_under_config() {
        let dir = temp("chat");
        let chat = new_chat_dir(&dir, "c1").unwrap();
        assert_eq!(chat, dir.join("chat").join("c1"));
        assert!(chat.is_dir());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn chat_dirs_list_only_dirs_capped_and_missing_is_empty() {
        let dir = temp("chat-list");
        assert!(chat_dirs(&dir).is_empty());
        for i in 0..12 {
            new_chat_dir(&dir, &format!("c{i:02}")).unwrap();
        }
        fs::write(dir.join("chat").join("stray.txt"), "x").unwrap();
        let rows = chat_dirs(&dir);
        assert_eq!(rows.len(), MAX_RECENT);
        assert!(rows.iter().all(|r| Path::new(&r.path).is_dir()));
        assert!(!rows.iter().any(|r| r.path.ends_with("stray.txt")));
        let _ = fs::remove_dir_all(&dir);
    }
}
