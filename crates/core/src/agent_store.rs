//! Persistent store of sub-agent records (Go harness agentstore.go: agentStore, savedAgent).
//! One JSON file, rewritten atomically on every change, shaped like the Go file so both binaries
//! can read it. Times are RFC3339 with fraction. Callers hold `Option<&AgentStore>` where Go allowed
//! a nil store. The tracker that produces the records is not ported yet, so the tracker-coupled
//! parts of Go restoreAgents and markInterrupted are not here.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::handoff::pid_alive;
use crate::import_claude::{Stamp, parse_rfc3339};
use crate::metrics::rfc3339;

pub const STO_RUNNING: &str = "running";
pub const STO_DONE: &str = "done";
pub const STO_FAILED: &str = "failed";
pub const STO_INTERRUPTED: &str = "interrupted";

const MAX_STORED_AGENTS: usize = 200;
const STORE_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);
/// Startup restore window for interrupted agents.
const RESTORE_MAX_AGE: Duration = Duration::from_secs(24 * 3600);

/// On-disk form of one tracked sub-agent (Go savedAgent). Zero-valued optional fields are omitted.
/// A zero `updated` time (UNIX_EPOCH here) means "not set yet" and is stamped on first put.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedAgent {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prompt: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    /// Parent Claude Code session that started the agent.
    #[serde(
        default,
        rename = "parent_session",
        skip_serializing_if = "String::is_empty"
    )]
    pub parent: String,
    /// Parent transcript, which holds the sub-agent's events.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub transcript: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub out_path: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub background: bool,
    /// Harness process that tracked the agent. 0 = none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub owner_pid: u32,
    /// Launch handle, for SendMessage resume.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent_id: String,
    #[serde(default = "epoch", with = "rfc3339_time")]
    pub started: SystemTime,
    #[serde(default = "epoch", with = "rfc3339_time")]
    pub updated: SystemTime,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

fn epoch() -> SystemTime {
    UNIX_EPOCH
}

impl SavedAgent {
    /// Record with only the identity, status and start time set.
    pub fn new(id: &str, status: &str, started: SystemTime) -> Self {
        SavedAgent {
            id: id.to_string(),
            model: String::new(),
            name: String::new(),
            task: String::new(),
            prompt: String::new(),
            status: status.to_string(),
            summary: String::new(),
            parent: String::new(),
            transcript: String::new(),
            out_path: String::new(),
            background: false,
            owner_pid: 0,
            agent_id: String::new(),
            started,
            updated: UNIX_EPOCH,
        }
    }
}

/// RFC3339 with fraction for `SystemTime`, the form Go time.MarshalJSON writes (trailing zeros cut).
mod rfc3339_time {
    use super::*;
    use serde::de::Error;

    pub fn serialize<S: serde::Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format_rfc3339(*t))
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let s = String::deserialize(d)?;
        parse_rfc3339(&s)
            .map(stamp_to_time)
            .ok_or_else(|| D::Error::custom("bad RFC3339 time"))
    }
}

fn time_to_stamp(t: SystemTime) -> Stamp {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i64, d.subsec_nanos()),
        Err(e) => (-(e.duration().as_secs() as i64), 0),
    }
}

fn stamp_to_time((secs, nanos): Stamp) -> SystemTime {
    let d = Duration::new(secs.unsigned_abs(), nanos);
    if secs >= 0 {
        UNIX_EPOCH + d
    } else {
        UNIX_EPOCH.checked_sub(d).unwrap_or(UNIX_EPOCH)
    }
}

fn format_rfc3339(t: SystemTime) -> String {
    let (secs, nanos) = time_to_stamp(t);
    let base = rfc3339(secs);
    if nanos == 0 {
        return base;
    }
    let frac = format!("{nanos:09}");
    format!(
        "{}.{}Z",
        &base[..base.len() - 1],
        frac.trim_end_matches('0')
    )
}

#[derive(Debug, Default)]
struct Inner {
    m: HashMap<String, SavedAgent>,
    /// Last text written, so an unchanged store is not rewritten.
    last: String,
}

/// Persists `SavedAgent`s to one JSON file. An empty path makes every method a no-op.
#[derive(Debug)]
pub struct AgentStore {
    path: PathBuf,
    inner: Mutex<Inner>,
}

impl AgentStore {
    /// Loads the file at `path` if it exists and parses. Records without an id are skipped.
    pub fn new(path: &Path) -> Self {
        let mut m = HashMap::new();
        if !path.as_os_str().is_empty() {
            if let Ok(b) = fs::read(path) {
                if let Ok(list) = serde_json::from_slice::<Vec<SavedAgent>>(&b) {
                    for a in list.into_iter().filter(|a| !a.id.is_empty()) {
                        m.insert(a.id.clone(), a);
                    }
                }
            }
        }
        AgentStore {
            path: path.to_path_buf(),
            inner: Mutex::new(Inner {
                m,
                last: String::new(),
            }),
        }
    }

    fn off(&self) -> bool {
        self.path.as_os_str().is_empty()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Merges `recs` (newest wins per id), prunes and writes the file if anything changed.
    /// Each record is stamped with this process as owner.
    pub fn put(&self, recs: &[SavedAgent]) {
        if self.off() || recs.is_empty() {
            return;
        }
        let mut g = self.lock();
        merge_disk(&self.path, &mut g.m);
        let pid = std::process::id();
        let now = SystemTime::now();
        for r in recs {
            let mut a = r.clone();
            a.owner_pid = pid;
            if let Some(old) = g.m.get(&a.id) {
                a.updated = old.updated;
                if *old == a {
                    continue;
                }
                a.updated = now;
            } else if a.updated == UNIX_EPOCH {
                a.updated = now;
            }
            g.m.insert(a.id.clone(), a);
        }
        self.flush(&mut g);
    }

    /// Marks stored running agents interrupted: those of this process (`own`, shutdown) or of dead
    /// processes (crash recovery, `!own`). Agents of other live harnesses are left alone.
    pub fn interrupt_running(&self, own: bool) {
        if self.off() {
            return;
        }
        let mut g = self.lock();
        merge_disk(&self.path, &mut g.m);
        let pid = std::process::id();
        let now = SystemTime::now();
        for a in g.m.values_mut() {
            let mine = own && a.owner_pid == pid;
            let dead = !own && !pid_alive(a.owner_pid);
            if a.status == STO_RUNNING && (mine || dead) {
                a.status = STO_INTERRUPTED.to_string();
                a.updated = now;
            }
        }
        self.flush(&mut g);
    }

    /// Stored agents matching `parent` (any when empty), oldest first. Without a parent only
    /// interrupted agents updated within 24 hours are returned.
    pub fn load(&self, parent: &str) -> Vec<SavedAgent> {
        let g = self.lock();
        let now = SystemTime::now();
        let mut l: Vec<SavedAgent> =
            g.m.values()
                .filter(|a| {
                    if !parent.is_empty() {
                        a.parent == parent
                    } else {
                        a.status == STO_INTERRUPTED
                            && now
                                .duration_since(a.updated)
                                .map_or(true, |age| age <= RESTORE_MAX_AGE)
                    }
                })
                .cloned()
                .collect();
        l.sort_by(|a, b| a.started.cmp(&b.started).then_with(|| a.id.cmp(&b.id)));
        l
    }

    /// Drops records older than 7 days, keeps the newest 200, and writes the file if its text changed.
    fn flush(&self, g: &mut Inner) {
        let cut = SystemTime::now()
            .checked_sub(STORE_MAX_AGE)
            .unwrap_or(UNIX_EPOCH);
        g.m.retain(|_, a| a.updated >= cut);
        let mut l: Vec<SavedAgent> = g.m.values().cloned().collect();
        l.sort_by(|a, b| b.updated.cmp(&a.updated).then_with(|| a.id.cmp(&b.id)));
        if l.len() > MAX_STORED_AGENTS {
            for a in &l[MAX_STORED_AGENTS..] {
                g.m.remove(&a.id);
            }
            l.truncate(MAX_STORED_AGENTS);
        }
        let mut buf = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(b" ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
        if l.serialize(&mut ser).is_err() {
            return;
        }
        let Ok(text) = String::from_utf8(buf) else {
            return;
        };
        if text == g.last {
            return;
        }
        if let Some(dir) = self.path.parent().filter(|d| !d.as_os_str().is_empty()) {
            if fs::create_dir_all(dir).is_err() {
                return;
            }
        }
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        if write_private(&tmp, text.as_bytes()).is_ok() && fs::rename(&tmp, &self.path).is_ok() {
            g.last = text;
        }
    }
}

/// Adds records other harness processes wrote since we last looked. Ours win by id, except entries
/// another process updated later.
fn merge_disk(path: &Path, m: &mut HashMap<String, SavedAgent>) {
    let Ok(b) = fs::read(path) else {
        return;
    };
    let Ok(list) = serde_json::from_slice::<Vec<SavedAgent>>(&b) else {
        return;
    };
    let pid = std::process::id();
    for a in list {
        let take = match m.get(&a.id) {
            None => true,
            Some(cur) => a.updated > cur.updated && a.owner_pid != pid,
        };
        if take {
            m.insert(a.id.clone(), a);
        }
    }
}

/// Writes `data` with owner-only permissions on unix, as Go's 0600 file mode.
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    // Go TestAgentStorePersistsEveryChange (store part; the tracker that feeds it is not ported)
    #[test]
    fn agent_store_persists_every_change() {
        let dir = TempDir::new("agent-store-persist");
        let path = dir.path().join("sub").join("agents.json");
        let st = AgentStore::new(&path);
        let mut a = SavedAgent::new("a", STO_RUNNING, SystemTime::now());
        a.parent = "sess1".into();
        a.model = "haiku".into();
        a.name = "w".into();
        a.transcript = "/t/sess1.jsonl".into();
        st.put(std::slice::from_ref(&a));
        let saved = fs::read_to_string(&path).expect("saved on start");
        assert!(
            saved.contains("\"status\": \"running\"")
                && saved.contains("\"parent_session\": \"sess1\""),
            "{saved}"
        );
        a.status = STO_DONE.into();
        a.summary = "result line".into();
        st.put(std::slice::from_ref(&a));
        let got = AgentStore::new(&path).load("sess1");
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(
            got[0].status == STO_DONE
                && got[0].summary == "result line"
                && got[0].model == "haiku"
                && got[0].name == "w"
                && got[0].transcript == "/t/sess1.jsonl",
            "{got:?}"
        );
    }

    // Go TestAgentRestoreMarksInterrupted (store part: shutdown marks, restart loads, no-parent restore)
    #[test]
    fn agent_restore_marks_interrupted() {
        let dir = TempDir::new("agent-store-restore");
        let path = dir.path().join("agents.json");
        let st = AgentStore::new(&path);
        let started = SystemTime::now();
        let mut bg = SavedAgent::new("a", STO_RUNNING, started);
        bg.parent = "s1".into();
        bg.background = true;
        let mut fg = SavedAgent::new("b", STO_DONE, started);
        fg.parent = "s1".into();
        st.put(&[bg, fg]);
        st.interrupt_running(true);

        let st2 = AgentStore::new(&path);
        let by_id: HashMap<String, SavedAgent> = st2
            .load("s1")
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect();
        assert_eq!(by_id["a"].status, STO_INTERRUPTED);
        assert_eq!(by_id["b"].status, STO_DONE);

        let restore: Vec<String> = st2.load("").into_iter().map(|a| a.id).collect();
        assert_eq!(
            restore,
            vec!["a".to_string()],
            "startup restore: only interrupted"
        );
    }

    // Go TestAgentStoreCrashRecoveryAndOwners
    #[test]
    fn agent_store_crash_recovery_and_owners() {
        let dir = TempDir::new("agent-store-owners");
        let path = dir.path().join("agents.json");
        let st = AgentStore::new(&path);
        st.put(&[SavedAgent::new("x", STO_RUNNING, SystemTime::now())]);
        st.interrupt_running(false);
        assert!(
            st.load("").is_empty(),
            "live owner's agent must stay running"
        );
        st.lock().m.get_mut("x").expect("x stored").owner_pid = 1 << 30;
        st.interrupt_running(false);
        let l = st.load("");
        assert_eq!(l.len(), 1, "{l:?}");
        assert_eq!(l[0].status, STO_INTERRUPTED, "dead owner's agent");
    }

    // Go TestAgentStoreOffAndNil (empty path part; a nil store is Option::None at the call site)
    #[test]
    fn agent_store_empty_path_is_off() {
        let st = AgentStore::new(Path::new(""));
        st.put(&[SavedAgent::new("a", STO_RUNNING, SystemTime::now())]);
        st.interrupt_running(true);
        assert!(st.load("").is_empty());
        assert!(st.lock().m.is_empty(), "empty path must not store");
    }

    #[test]
    fn saved_agent_json_uses_go_shape() {
        let started = UNIX_EPOCH + Duration::new(1_700_000_000, 123_000_000);
        let a = SavedAgent::new("a", STO_DONE, started);
        let json = serde_json::to_string(&a).expect("serialize");
        assert!(
            json.contains("\"started\":\"2023-11-14T22:13:20.123Z\""),
            "{json}"
        );
        assert!(!json.contains("owner_pid"), "zero owner is omitted: {json}");
        let back: SavedAgent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, a);
    }
}
