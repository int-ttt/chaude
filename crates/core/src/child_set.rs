//! Parked-live child set (Go harness children.go: childKey, childSet) and the worker-cap admit step
//! (Go dispatch.go: Repl.admit). The child session type is generic: anything that can be closed.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime};

/// Default cap on concurrent live worker sessions (Go DefaultMaxWorkers).
pub const DEFAULT_MAX_WORKERS: usize = 3;

/// Identifies a live child by working directory, model and role (Go childKey).
/// An empty cwd means the process working directory. Symlinks are resolved when the path exists,
/// so /var and /private/var map to one key. An empty role gives the role-less key; a non-empty
/// role appends one more NUL-separated field.
pub fn child_key(cwd: &str, model: &str, role: &str) -> String {
    let cwd = if cwd.is_empty() { "." } else { cwd };
    let p = Path::new(cwd);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
    };
    let real = std::fs::canonicalize(&abs).unwrap_or(abs);
    let real = real.to_string_lossy();
    if role.is_empty() {
        format!("{real}\0{model}")
    } else {
        format!("{real}\0{model}\0{role}")
    }
}

/// Ends a child session. Go's `closer`. Must be Send so CloseAll can close children in parallel.
pub trait Close: Send {
    fn close(&mut self);
}

/// A child kept alive outside the active slot (Go liveChild, the fields childSet and admit need).
pub struct Parked<S: Close> {
    pub sess: S,
    pub model: String,
    pub last_api: SystemTime,
    pub last_ctx: u64,
    /// Delegated agents were running when the child was parked (Go Tracker.running() > 0).
    /// Busy children are not expired by the idle sweep (Go sweepIdle `!busy`).
    pub busy: bool,
}

/// Parked-live children of one process, keyed by `child_key`.
pub struct ChildSet<S: Close> {
    m: Mutex<HashMap<String, Parked<S>>>,
}

impl<S: Close> Default for ChildSet<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Close> ChildSet<S> {
    pub fn new() -> Self {
        ChildSet {
            m: Mutex::new(HashMap::new()),
        }
    }

    fn map(&self) -> MutexGuard<'_, HashMap<String, Parked<S>>> {
        self.m.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Stores `c` under `key`. A child already stored there is replaced and closed.
    pub fn park(&self, key: &str, c: Parked<S>) {
        let old = self.map().insert(key.to_string(), c);
        if let Some(mut old) = old {
            old.sess.close();
        }
    }

    pub fn len(&self) -> usize {
        self.map().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Removes and returns the parked child with the oldest `last_api` (ties: smaller key).
    /// The caller retires it.
    pub fn evict_lru(&self) -> Option<(String, Parked<S>)> {
        let mut m = self.map();
        let key = m
            .iter()
            .min_by(|(ka, a), (kb, b)| a.last_api.cmp(&b.last_api).then_with(|| ka.cmp(kb)))
            .map(|(k, _)| k.clone())?;
        let c = m.remove(&key)?;
        Some((key, c))
    }

    /// Removes and returns the child stored under `key`.
    pub fn take(&self, key: &str) -> Option<Parked<S>> {
        self.map().remove(key)
    }

    /// Removes and returns every stored child, busy ones included (Go endAllParked). The caller ends them.
    pub fn take_all(&self) -> Vec<(String, Parked<S>)> {
        self.map().drain().collect()
    }

    /// Last-API time and context size of the child under `key`, without removing it.
    pub fn warm_info(&self, key: &str) -> Option<(SystemTime, u64)> {
        self.map().get(key).map(|c| (c.last_api, c.last_ctx))
    }

    /// Sorted keys of idle children whose last API call is at least `deadline` old. Busy children
    /// are skipped (Go sweepIdle). Does not remove them.
    pub fn expired(&self, now: SystemTime, deadline: Duration) -> Vec<String> {
        let mut keys: Vec<String> = self
            .map()
            .iter()
            .filter(|(_, c)| !c.busy)
            .filter(|(_, c)| match now.duration_since(c.last_api) {
                Ok(age) => age >= deadline,
                Err(_) => false,
            })
            .map(|(k, _)| k.clone())
            .collect();
        keys.sort();
        keys
    }

    /// Ends every stored child in parallel and empties the set. Each close waits for its process,
    /// so serial closing would add up the waits.
    pub fn close_all(&self) {
        let list: Vec<Parked<S>> = self.map().drain().map(|(_, c)| c).collect();
        std::thread::scope(|scope| {
            for mut c in list {
                scope.spawn(move || c.sess.close());
            }
        });
    }
}

/// Makes room for one more live worker before a child is spawned (Go Repl.admit).
/// Counts the parked children plus the active one (`active`, it is parked by the caller next) and
/// evicts the oldest parked child, passing it to `retire`, while the count is at or above `limit`.
/// `limit` 0 means DEFAULT_MAX_WORKERS. Fails when the cap is reached and nothing can be evicted.
pub fn admit<S: Close>(
    set: &ChildSet<S>,
    active: bool,
    limit: usize,
    mut retire: impl FnMut(Parked<S>),
) -> Result<(), String> {
    let limit = if limit == 0 {
        DEFAULT_MAX_WORKERS
    } else {
        limit
    };
    let mut count = set.len() + usize::from(active);
    while count >= limit {
        let Some((_, c)) = set.evict_lru() else {
            return Err(worker_cap_err(limit));
        };
        retire(c);
        count -= 1;
    }
    Ok(())
}

/// Go workerCapErr: user-facing refusal when the worker cap is reached.
pub fn worker_cap_err(max: usize) -> String {
    format!(
        "워커 세션 상한({max}개)에 도달해 새 세션을 시작하지 않습니다. 기존 세션을 정리하거나 -max-workers 값을 올려 주세요."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeClose {
        delay: Duration,
        closed: Arc<AtomicUsize>,
    }

    impl Close for FakeClose {
        fn close(&mut self) {
            std::thread::sleep(self.delay);
            self.closed.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn fake(
        closed: &Arc<AtomicUsize>,
        delay: Duration,
        model: &str,
        last_api: SystemTime,
    ) -> Parked<FakeClose> {
        Parked {
            sess: FakeClose {
                delay,
                closed: Arc::clone(closed),
            },
            model: model.to_string(),
            last_api,
            last_ctx: 0,
            busy: false,
        }
    }

    // Go TestChildKey
    #[test]
    fn child_key_matches_empty_dot_and_working_dir() {
        let wd = std::env::current_dir().unwrap();
        let wd = wd.to_string_lossy().to_string();
        let k = child_key("", "sonnet", "");
        assert_eq!(k, child_key(".", "sonnet", ""));
        assert_eq!(k, child_key(&wd, "sonnet", ""));
        assert_ne!(child_key(&wd, "sonnet", ""), child_key(&wd, "haiku", ""));
        assert_ne!(
            child_key(&wd, "sonnet", ""),
            child_key(&wd, "sonnet", "exec")
        );
    }

    // Go sweepIdle: a parked child with agents still running is not expired, even when cold.
    #[test]
    fn expired_skips_busy_parked_children() {
        let n = Arc::new(AtomicUsize::new(0));
        let s = ChildSet::new();
        let now = SystemTime::now();
        let old = now - Duration::from_secs(7200);
        s.park("idle", fake(&n, Duration::ZERO, "", old));
        let mut busy = fake(&n, Duration::ZERO, "", old);
        busy.busy = true;
        s.park("busy", busy);
        assert_eq!(
            s.expired(now, Duration::from_secs(3600)),
            vec!["idle".to_string()],
            "busy child must stay parked"
        );
    }

    // Go endAllParked: every parked child is taken, busy ones included.
    #[test]
    fn take_all_returns_busy_and_idle_children() {
        let n = Arc::new(AtomicUsize::new(0));
        let s = ChildSet::new();
        let now = SystemTime::now();
        s.park("idle", fake(&n, Duration::ZERO, "", now));
        let mut busy = fake(&n, Duration::ZERO, "", now);
        busy.busy = true;
        s.park("busy", busy);
        let mut keys: Vec<String> = s.take_all().into_iter().map(|(k, _)| k).collect();
        keys.sort();
        assert_eq!(keys, vec!["busy".to_string(), "idle".to_string()]);
        assert!(s.is_empty(), "take_all empties the set");
    }

    // Go TestChildSetParkTake
    #[test]
    fn park_take_replaces_closes_once_and_expires() {
        let n = Arc::new(AtomicUsize::new(0));
        let zero = Duration::ZERO;
        let s = ChildSet::new();
        let now = SystemTime::now();
        s.park("k1", fake(&n, zero, "", now));
        s.park("k1", fake(&n, zero, "", now));
        assert_eq!(
            n.load(Ordering::SeqCst),
            1,
            "replacing a parked child closes the old one once"
        );
        assert_eq!(s.len(), 1, "same key keeps one entry");
        assert!(s.take("k1").is_some(), "take finds the parked child");
        assert!(s.take("k1").is_none(), "take removes the child");

        s.park("old", fake(&n, zero, "", now - Duration::from_secs(7200)));
        s.park("new", fake(&n, zero, "", now - Duration::from_secs(60)));
        assert_eq!(
            s.expired(now, Duration::from_secs(3600)),
            vec!["old".to_string()]
        );
        s.close_all();
        assert_eq!(s.len(), 0, "close_all empties the set");
    }

    // Go TestCloseAllParallel
    #[test]
    fn close_all_closes_in_parallel() {
        let n = Arc::new(AtomicUsize::new(0));
        let s = ChildSet::new();
        let now = SystemTime::now();
        for k in ["a", "b", "c"] {
            s.park(k, fake(&n, Duration::from_millis(100), "", now));
        }
        let start = std::time::Instant::now();
        s.close_all();
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(250),
            "close_all took {elapsed:?}, want < 250ms (parallel)"
        );
        assert_eq!(n.load(Ordering::SeqCst), 3);
    }

    // Go TestAdmitEvictsOldestParked
    #[test]
    fn admit_evicts_oldest_parked() {
        let old_n = Arc::new(AtomicUsize::new(0));
        let new_n = Arc::new(AtomicUsize::new(0));
        let s = ChildSet::new();
        let now = SystemTime::now();
        s.park(
            "haiku",
            fake(
                &old_n,
                Duration::ZERO,
                "haiku",
                now - Duration::from_secs(7200),
            ),
        );
        s.park("sonnet", fake(&new_n, Duration::ZERO, "sonnet", now));
        let mut retired = Vec::new();
        admit(&s, false, 2, |c| retired.push(c)).expect("admit");
        assert_eq!(retired.len(), 1);
        assert_eq!(retired[0].model, "haiku", "oldest parked child is evicted");
        for c in retired.iter_mut() {
            c.sess.close();
        }
        assert_eq!(old_n.load(Ordering::SeqCst), 1);
        assert_eq!(new_n.load(Ordering::SeqCst), 0);
        assert_eq!(s.len(), 1, "parked after admit");
    }

    // Go TestAdmitRefusesWhenNothingEvictable
    #[test]
    fn admit_refuses_when_nothing_evictable() {
        let s: ChildSet<FakeClose> = ChildSet::new();
        assert!(
            admit(&s, true, 1, |_| {}).is_err(),
            "cap 1 with an active child and no parked child: refusal"
        );
    }
}
