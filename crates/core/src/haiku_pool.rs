//! Worker pool of Haiku sessions that already edited files (Go harness haikupool.go: workerPool).
//! In memory, per process. The caller holds it as `Option<&WorkerPool>` where Go allowed a nil pool.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use crate::import_view::trunc;

/// A Haiku session with edit history: a candidate for the next edit-type prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaikuWorker {
    pub id: String,
    /// First/last task it handled (terse, at most 80 chars).
    pub task: String,
    pub files: Vec<String>,
    pub updated: SystemTime,
}

#[derive(Debug, Default)]
pub struct WorkerPool {
    m: Mutex<HashMap<String, HaikuWorker>>,
}

impl WorkerPool {
    pub fn new() -> Self {
        Self::default()
    }

    fn map(&self) -> MutexGuard<'_, HashMap<String, HaikuWorker>> {
        self.m.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Records that Haiku session `id` edited `file` while handling `task`.
    /// An empty id or file is ignored. Files are deduplicated.
    pub fn note_edit(&self, id: &str, task: &str, file: &str) {
        if id.is_empty() || file.is_empty() {
            return;
        }
        let mut m = self.map();
        let w = m.entry(id.to_string()).or_insert_with(|| HaikuWorker {
            id: id.to_string(),
            task: String::new(),
            files: Vec::new(),
            updated: SystemTime::now(),
        });
        if !task.is_empty() {
            w.task = trunc(task, 80);
        }
        if !w.files.iter().any(|f| f == file) {
            w.files.push(file.to_string());
        }
        w.updated = SystemTime::now();
    }

    pub fn has(&self, id: &str) -> bool {
        self.map().contains_key(id)
    }

    /// Copy of the worker recorded under `id`.
    pub fn get(&self, id: &str) -> Option<HaikuWorker> {
        self.map().get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go TestWorkerPool
    #[test]
    fn worker_pool_notes_edits_and_dedups_files() {
        let p = WorkerPool::new();
        p.note_edit("", "t", "a.go");
        p.note_edit("s1", "task one", "a.go");
        p.note_edit("s1", "task one", "a.go");
        p.note_edit("s2", "task two", "b.go");
        assert!(p.has("s1") && p.has("s2"));
        assert!(!p.has("") && !p.has("nope"));
        assert_eq!(p.get("s1").unwrap().files.len(), 1, "s1 files dedup");
        assert_eq!(p.get("s1").unwrap().task, "task one");
    }

    #[test]
    fn worker_pool_truncates_task_to_80_runes() {
        let p = WorkerPool::new();
        let long: String = "가".repeat(100);
        p.note_edit("s1", &long, "a.go");
        assert_eq!(p.get("s1").unwrap().task.chars().count(), 80);
    }
}
