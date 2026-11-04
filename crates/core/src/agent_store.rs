//! Persistent store of sub-agent records (Go harness agentstore.go: agentStore, savedAgent).
//! One JSON file, rewritten atomically on every change, shaped like the Go file so both binaries
//! can read it. Times are RFC3339 with fraction. Callers hold `Option<&AgentStore>` where Go allowed
//! a nil store. `AgentTracker` (Go agentTracker, drain.go and agentview.go) keeps the live records
//! of one session and persists each change. Not ported: result files (writeResults, ResultPath),
//! SendMessage resume (resumeLocked), and the completion callback (lines are returned instead).

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::handoff::pid_alive;
use crate::import_claude::{Stamp, parse_rfc3339};
use crate::import_view::{bget, json_str, list, result_text, sget, trunc};
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
        if !path.as_os_str().is_empty()
            && let Ok(b) = fs::read(path)
            && let Ok(list) = serde_json::from_slice::<Vec<SavedAgent>>(&b)
        {
            for a in list.into_iter().filter(|a| !a.id.is_empty()) {
                m.insert(a.id.clone(), a);
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
        if let Some(dir) = self.path.parent().filter(|d| !d.as_os_str().is_empty())
            && fs::create_dir_all(dir).is_err()
        {
            return;
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

/// Live log lines kept per agent (Go maxAgentLog).
const MAX_AGENT_LOG: usize = 400;

/// One delegated sub-agent of a session (Go agentRec).
#[derive(Debug, Clone)]
pub struct AgentRec {
    pub id: String,
    pub task: String,
    /// First line of the prompt handed to the agent.
    pub prompt: String,
    /// `model` input of the Agent call (empty: inherits the session model).
    pub model: String,
    /// `name` input, else subagent_type: the name shown in the panel.
    pub name: String,
    pub failed: bool,
    /// Live transcript lines of the sub-agent (text, tool calls, results).
    pub log: Vec<String>,
    pub background: bool,
    /// Launch handle from "agentId: <id>", kept for SendMessage resume.
    pub agent_id: String,
    pub started: SystemTime,
    pub done: bool,
    /// Child gone before the agent finished (restored from disk, or harness exit).
    pub interrupted: bool,
    /// Loaded from the agent store, not started in this process.
    pub restored: bool,
    /// Restored agent hidden from the panel (still in the full list).
    pub dismissed: bool,
    /// Last text line of the agent.
    pub summary: String,
}

impl AgentRec {
    fn new(id: &str, started: SystemTime) -> Self {
        AgentRec {
            id: id.to_string(),
            task: String::new(),
            prompt: String::new(),
            model: String::new(),
            name: String::new(),
            failed: false,
            log: Vec::new(),
            background: false,
            agent_id: String::new(),
            started,
            done: false,
            interrupted: false,
            restored: false,
            dismissed: false,
            summary: String::new(),
        }
    }

    /// Go agentRec.saved: the stored form, with the status taken from the flags.
    fn saved(&self, parent: &str, transcript: &str) -> SavedAgent {
        let status = if self.interrupted {
            STO_INTERRUPTED
        } else if self.failed {
            STO_FAILED
        } else if self.done {
            STO_DONE
        } else {
            STO_RUNNING
        };
        let mut s = SavedAgent::new(&self.id, status, self.started);
        s.model = self.model.clone();
        s.name = self.name.clone();
        s.task = self.task.clone();
        s.prompt = self.prompt.clone();
        s.summary = self.summary.clone();
        s.parent = parent.to_string();
        s.transcript = transcript.to_string();
        s.background = self.background;
        s.agent_id = self.agent_id.clone();
        s
    }
}

/// Go markDoneLocked: marks done, and returns the completion line only on the first call.
fn mark_done(a: &mut AgentRec) -> Option<String> {
    if a.done {
        return None;
    }
    a.done = true;
    Some(done_line(a))
}

/// Go doneLine: the one-line completion notice.
fn done_line(a: &AgentRec) -> String {
    let kind = if a.failed {
        "[agent 실패]"
    } else {
        "[agent 완료]"
    };
    let model = if a.model.is_empty() {
        "상속"
    } else {
        a.model.as_str()
    };
    let mut what = a.summary.clone();
    if what.is_empty() {
        what = last_summary(&a.log);
    }
    if what.is_empty() {
        what = a.task.clone();
    }
    let line = format!("{kind} {model} {} · {what}", a.name);
    trunc(&line.split_whitespace().collect::<Vec<_>>().join(" "), 120)
}

/// Go lastSummary: the last plain text line of a log (not a tool call or a result line).
pub fn last_summary(log: &[String]) -> String {
    log.iter()
        .rev()
        .find(|l| !l.is_empty() && !l.starts_with(' ') && !l.starts_with('●'))
        .map(|l| trunc(l, 200))
        .unwrap_or_default()
}

/// Go toolSummary: the first informative input field, else the compact JSON of the input.
pub fn tool_summary(input: &Value) -> String {
    for k in [
        "command",
        "file_path",
        "pattern",
        "path",
        "url",
        "query",
        "description",
        "prompt",
    ] {
        let v = sget(input, k);
        if !v.is_empty() {
            return trunc(&v.replace('\n', " "), 100);
        }
    }
    match input.as_object() {
        Some(m) if !m.is_empty() => trunc(&json_str(input), 100),
        _ => String::new(),
    }
}

/// Go agentState, with the labels shown in the panel.
pub fn agent_state(a: &AgentRec) -> &'static str {
    if a.failed {
        "실패"
    } else if a.interrupted {
        "중단"
    } else if a.done {
        "완료"
    } else if a.started == UNIX_EPOCH {
        "대기"
    } else {
        "실행 중"
    }
}

fn first_line(s: &str) -> String {
    s.split('\n')
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Agent id from an "Async agent launched" result ("agentId: <id>").
fn launch_id(text: &str) -> Option<String> {
    let rest = text.split("agentId:").nth(1)?.trim_start();
    let id: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    (!id.is_empty()).then_some(id)
}

/// Ids named by `<tool-use-id>` tags of a task notification (Go notifIDRe).
fn tool_use_ids(text: &str) -> Vec<&str> {
    const OPEN: &str = "<tool-use-id>";
    const CLOSE: &str = "</tool-use-id>";
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(OPEN) {
        let after = &rest[i + OPEN.len()..];
        let Some(j) = after.find(CLOSE) else {
            break;
        };
        let id = &after[..j];
        if !id.is_empty() && !id.contains('<') {
            out.push(id);
        }
        rest = &after[j + CLOSE.len()..];
    }
    out
}

/// The `message.content` blocks of a stream event.
fn content_blocks(ev: &Value) -> &[Value] {
    list(ev.get("message").and_then(|m| m.get("content")))
}

/// Go agentTracker: the sub-agents of one session. Each change is written to `store` (when set),
/// owned by `parent` with `transcript`. Restored agents are never written back.
#[derive(Debug)]
pub struct AgentTracker<'a> {
    agents: Mutex<HashMap<String, AgentRec>>,
    store: Option<&'a AgentStore>,
    parent: String,
    transcript: String,
}

impl<'a> AgentTracker<'a> {
    pub fn new(store: Option<&'a AgentStore>, parent: &str, transcript: &str) -> Self {
        AgentTracker {
            agents: Mutex::new(HashMap::new()),
            store,
            parent: parent.to_string(),
            transcript: transcript.to_string(),
        }
    }

    /// Re-points persistence (Go assigning tracker.store and tracker.parent).
    pub fn set_persist(&mut self, store: Option<&'a AgentStore>, parent: &str, transcript: &str) {
        self.store = store;
        self.parent = parent.to_string();
        self.transcript = transcript.to_string();
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, AgentRec>> {
        self.agents.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Go persistLocked: writes every tracked agent except restored ones.
    fn persist_locked(&self, m: &mut HashMap<String, AgentRec>) {
        let Some(store) = self.store else {
            return;
        };
        let mut recs = Vec::with_capacity(m.len());
        for a in m.values_mut() {
            if a.restored {
                continue;
            }
            if a.done && a.summary.is_empty() {
                a.summary = last_summary(&a.log);
            }
            recs.push(a.saved(&self.parent, &self.transcript));
        }
        store.put(&recs);
    }

    /// Go onToolUse: an Agent or Task call starts a running record. Other tools are ignored.
    pub fn on_tool_use(&self, id: &str, name: &str, input: &Value, now: SystemTime) {
        if name != "Agent" && name != "Task" {
            return;
        }
        let mut m = self.lock();
        if m.contains_key(id) {
            return;
        }
        let mut a = AgentRec::new(id, now);
        a.task = sget(input, "description").to_string();
        a.prompt = first_line(sget(input, "prompt"));
        a.background = bget(input, "run_in_background");
        let name = sget(input, "name");
        a.name = if name.is_empty() {
            sget(input, "subagent_type")
        } else {
            name
        }
        .to_string();
        a.model = sget(input, "model").to_string();
        m.insert(id.to_string(), a);
        self.persist_locked(&mut m);
    }

    /// Go onToolResult: a foreground agent finishes with its tool result. Returns completion lines.
    pub fn on_tool_result(&self, id: &str) -> Vec<String> {
        self.on_tool_result_text(id, "")
    }

    /// Go onToolResultText: an "Async agent launched" result keeps the agent running in the
    /// background and records its agentId. Any other result finishes a foreground agent.
    pub fn on_tool_result_text(&self, id: &str, text: &str) -> Vec<String> {
        let mut m = self.lock();
        let mut lines = Vec::new();
        let Some(a) = m.get_mut(id) else {
            return lines;
        };
        if text.contains("Async agent launched") {
            a.background = true;
            if let Some(aid) = launch_id(text) {
                a.agent_id = aid;
            }
        } else if !a.background {
            lines.extend(mark_done(a));
        }
        self.persist_locked(&mut m);
        lines
    }

    /// Go onSubEvent: appends a sub-agent's text, tool calls and tool results to its log.
    pub fn on_sub_event(&self, ev: &Value) {
        let p = sget(ev, "parent_tool_use_id");
        if p.is_empty() {
            return;
        }
        let mut add: Vec<String> = Vec::new();
        match sget(ev, "type") {
            "assistant" => {
                for b in content_blocks(ev) {
                    match sget(b, "type") {
                        "text" => {
                            let s = sget(b, "text").trim();
                            if !s.is_empty() {
                                add.extend(s.split('\n').map(str::to_string));
                            }
                        }
                        "tool_use" => {
                            let n = sget(b, "name");
                            let a = tool_summary(b.get("input").unwrap_or(&Value::Null));
                            add.push(if a.is_empty() {
                                format!("● {n}")
                            } else {
                                format!("● {n}({a})")
                            });
                        }
                        _ => {}
                    }
                }
            }
            "user" => {
                for b in content_blocks(ev) {
                    if sget(b, "type") == "tool_result" {
                        let s = result_text(b.get("content"));
                        let s = s.trim();
                        let l: Vec<&str> = s.split('\n').collect();
                        let mut x = format!("  ⎿  {}", trunc(l[0], 160));
                        if l.len() > 1 {
                            x.push_str(&format!(" … (+{} lines)", l.len() - 1));
                        }
                        add.push(x);
                    }
                }
            }
            _ => {}
        }
        if add.is_empty() {
            return;
        }
        let mut m = self.lock();
        if let Some(a) = m.get_mut(p) {
            a.log.extend(add);
            let n = a.log.len();
            if n > MAX_AGENT_LOG {
                a.log.drain(..n - MAX_AGENT_LOG);
            }
        }
    }

    /// Go onNotification: a <task-notification> finishes the agents it names. Returns whether any
    /// named agent is tracked, and the completion lines.
    pub fn on_notification(&self, text: &str) -> (bool, Vec<String>) {
        if !text.contains("<task-notification>") {
            return (false, Vec::new());
        }
        let mut m = self.lock();
        let mut found = false;
        let mut lines = Vec::new();
        for id in tool_use_ids(text) {
            if let Some(a) = m.get_mut(id) {
                lines.extend(mark_done(a));
                found = true;
            }
        }
        if found {
            self.persist_locked(&mut m);
        }
        (found, lines)
    }

    /// Go interruptRunning: unfinished agents become done and interrupted (harness shutdown).
    pub fn interrupt_running(&self) {
        let mut m = self.lock();
        for a in m.values_mut() {
            if !a.done {
                a.done = true;
                a.interrupted = true;
            }
        }
        self.persist_locked(&mut m);
    }

    /// Go restore: adds a stored agent as finished. Unfinished ones show as interrupted, since
    /// their child process is gone.
    pub fn restore(&self, s: &SavedAgent) {
        let mut m = self.lock();
        if m.contains_key(&s.id) {
            return;
        }
        let mut a = AgentRec::new(&s.id, s.started);
        a.task = s.task.clone();
        a.prompt = s.prompt.clone();
        a.model = s.model.clone();
        a.name = s.name.clone();
        a.background = s.background;
        a.agent_id = s.agent_id.clone();
        a.summary = s.summary.clone();
        a.restored = true;
        a.done = true;
        a.failed = s.status == STO_FAILED;
        a.interrupted = s.status == STO_INTERRUPTED || s.status == STO_RUNNING;
        if a.interrupted && !s.summary.is_empty() {
            a.log = vec![format!("(restored) {}", s.summary)];
        }
        m.insert(s.id.clone(), a);
    }

    /// Go restoreAgents: seeds the tracker from `store` (empty `parent`: recent interrupted ones).
    pub fn restore_agents(&self, store: &AgentStore, parent: &str) {
        for s in store.load(parent) {
            self.restore(&s);
        }
    }

    /// Go running: agents not finished, oldest first.
    pub fn running(&self) -> Vec<AgentRec> {
        self.sorted_by(|a| !a.done)
    }

    /// Go sortedAll: every tracked agent, oldest first.
    pub fn sorted_all(&self) -> Vec<AgentRec> {
        self.sorted_by(|_| true)
    }

    /// Go sortedRestored: restored agents still shown in the panel, oldest first.
    pub fn sorted_restored(&self) -> Vec<AgentRec> {
        self.sorted_by(|a| a.restored && !a.dismissed)
    }

    fn sorted_by(&self, keep: impl Fn(&AgentRec) -> bool) -> Vec<AgentRec> {
        let m = self.lock();
        let mut l: Vec<AgentRec> = m.values().filter(|a| keep(a)).cloned().collect();
        l.sort_by(|a, b| a.started.cmp(&b.started).then_with(|| a.id.cmp(&b.id)));
        l
    }

    /// Go dismissRestored: hides restored agents from the panel (they stay in the full list).
    pub fn dismiss_restored(&self) {
        for a in self.lock().values_mut() {
            if a.restored {
                a.dismissed = true;
            }
        }
    }

    /// Go snapshot: a copy of one agent, None if unknown.
    pub fn snapshot(&self, id: &str) -> Option<AgentRec> {
        self.lock().get(id).cloned()
    }
}

/// Go markInterrupted: every unfinished agent of `trackers` becomes interrupted, then this
/// process's running agents are marked interrupted in the store.
pub fn mark_interrupted(store: &AgentStore, trackers: &[&AgentTracker<'_>]) {
    for t in trackers {
        t.interrupt_running();
    }
    store.interrupt_running(true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use serde_json::json;

    // Go TestAgentStorePersistsEveryChange
    #[test]
    fn agent_store_persists_every_change() {
        let dir = TempDir::new("agent-store-persist");
        let path = dir.path().join("sub").join("agents.json");
        let st = AgentStore::new(&path);
        let tk = AgentTracker::new(Some(&st), "sess1", "/t/sess1.jsonl");
        tk.on_tool_use(
            "a",
            "Agent",
            &json!({"description": "d", "name": "w", "model": "haiku", "run_in_background": true}),
            SystemTime::now(),
        );
        let saved = fs::read_to_string(&path).expect("saved on start");
        assert!(
            saved.contains("\"status\": \"running\"")
                && saved.contains("\"parent_session\": \"sess1\""),
            "{saved}"
        );
        tk.on_sub_event(&json!({
            "type": "assistant",
            "parent_tool_use_id": "a",
            "message": {"content": [{"type": "text", "text": "result line"}]}
        }));
        let (found, lines) = tk
            .on_notification("<task-notification><tool-use-id>a</tool-use-id></task-notification>");
        assert!(found && lines.len() == 1, "{found} {lines:?}");
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

    // Go TestAgentRestoreMarksInterrupted
    #[test]
    fn agent_restore_marks_interrupted() {
        let dir = TempDir::new("agent-store-restore");
        let path = dir.path().join("agents.json");
        let st = AgentStore::new(&path);
        let tk = AgentTracker::new(Some(&st), "s1", "/t/s1.jsonl");
        let now = SystemTime::now();
        tk.on_tool_use(
            "a",
            "Agent",
            &json!({"description": "bg", "run_in_background": true}),
            now,
        );
        tk.on_tool_use("b", "Agent", &json!({"description": "fg"}), now);
        tk.on_tool_result("b");
        // shutdown: unfinished agents become interrupted, finished ones stay done
        tk.interrupt_running();
        st.interrupt_running(true);

        let st2 = AgentStore::new(&path); // restart
        let by_id: HashMap<String, SavedAgent> = st2
            .load("s1")
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect();
        assert_eq!(by_id["a"].status, STO_INTERRUPTED);
        assert_eq!(by_id["b"].status, STO_DONE);

        // startup restore (no parent): only interrupted ones
        let r = AgentTracker::new(None, "", "");
        r.restore_agents(&st2, "");
        let all = r.sorted_all();
        assert!(
            all.len() == 1 && all[0].id == "a" && all[0].interrupted && all[0].done,
            "restored: {all:?}"
        );
        assert!(
            r.running().is_empty(),
            "restored agent must not count as running"
        );
        let rs = r.sorted_restored();
        assert!(
            rs.len() == 1 && agent_state(&rs[0]) == "중단",
            "panel rows: {rs:?}"
        );
        r.dismiss_restored();
        assert!(
            r.sorted_restored().is_empty() && r.sorted_all().len() == 1,
            "dismissed agent leaves the panel but stays in the agent view"
        );

        // /resume restores everything of that parent session
        let mut r2 = AgentTracker::new(None, "", "");
        r2.restore_agents(&st2, "s1");
        assert_eq!(r2.sorted_all().len(), 2, "resume restore");
        // restored agents are not rewritten under another parent
        r2.set_persist(Some(&st2), "other", "");
        r2.on_tool_use("c", "Agent", &json!({"description": "n"}), now);
        assert_eq!(
            AgentStore::new(&path).load("other").len(),
            1,
            "restored agents re-saved under a new parent"
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
