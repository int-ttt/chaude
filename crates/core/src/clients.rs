//! Client registry (Go clients.go): one row per model client (child session) bound to a cwd, persisted as
//! ~/.chaude/clients.json. Each write re-reads the file and replaces only the caller's (cwd, model) row, so
//! rows written by other harness processes survive. The warm check, idle deadline and orphan reconcile live
//! here too. Liveness reuses handoff::pid_alive.

use crate::handoff::pid_alive;
use crate::osenv::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};

pub const LIVE: &str = "live";
pub const PARKED: &str = "parked";
pub const EXPIRED: &str = "expired";

/// A Go time.Time as clients.json holds it: the instant (Unix seconds and nanoseconds) plus the zone
/// offset in seconds that Go wrote. Go writes time.Now() in local time, so a row may carry "+09:00".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub secs: i64,
    pub nanos: u32,
    pub off: i32,
}

/// Unix seconds of Go's zero time.Time, 0001-01-01T00:00:00Z.
const GO_ZERO_SECS: i64 = -62_135_596_800;

impl Default for Stamp {
    // The zero value is Go's zero time.Time, so an absent last_api serializes as Go does.
    fn default() -> Self {
        Stamp {
            secs: GO_ZERO_SECS,
            nanos: 0,
            off: 0,
        }
    }
}

/// Current time as a Stamp (UTC).
pub fn now_stamp() -> Stamp {
    let d = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Stamp {
        secs: d.as_secs() as i64,
        nanos: d.subsec_nanos(),
        off: 0,
    }
}

/// ~/.chaude/clients.json (Go defaultClientsFile).
pub fn default_clients_file() -> PathBuf {
    home_dir().join(".chaude").join("clients.json")
}

/// One row of clients.json. Rows are keyed by (cwd, model).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ClientRec {
    pub cwd: String,
    pub model: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub session_id: String,
    pub state: String,
    #[serde(with = "rfc3339")]
    pub last_api: Stamp,
    #[serde(skip_serializing_if = "is_zero")]
    pub last_ctx: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ended_reason: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub design_path: String,
    /// Harness process that owns the row; 0 when none.
    #[serde(skip_serializing_if = "is_zero")]
    pub owner_pid: u32,
    /// claude -p process; 0 when none.
    #[serde(skip_serializing_if = "is_zero")]
    pub child_pid: u32,
}

fn is_zero<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// RFC3339Nano for ClientRec.last_api, as Go writes it.
mod rfc3339 {
    use super::{Stamp, format_stamp, parse_stamp};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Stamp, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format_stamp(*v))
    }

    // Go Unmarshal treats JSON null as a no-op, which leaves the zero time.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Stamp, D::Error> {
        match Option::<String>::deserialize(d)? {
            None => Ok(Stamp::default()),
            Some(s) => parse_stamp(&s).ok_or_else(|| serde::de::Error::custom("bad RFC3339 time")),
        }
    }
}

// Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

// RFC3339Nano as Go writes it: trailing zeros of the fraction are dropped, and the stamp's own zone
// offset is kept ("Z" when it is zero).
fn format_stamp(st: Stamp) -> String {
    let local = st.secs + i64::from(st.off);
    let (days, rem) = (local.div_euclid(86_400), local.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    let frac = if st.nanos == 0 {
        String::new()
    } else {
        format!(".{:09}", st.nanos)
            .trim_end_matches('0')
            .to_string()
    };
    let zone = if st.off == 0 {
        "Z".to_string()
    } else {
        let a = st.off.abs();
        format!(
            "{}{:02}:{:02}",
            if st.off < 0 { '-' } else { '+' },
            a / 3600,
            a % 3600 / 60
        )
    };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}{frac}{zone}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

// Parses an RFC3339 stamp and keeps its zone offset, so a Go-written local time round-trips.
fn parse_stamp(s: &str) -> Option<Stamp> {
    let (secs, nanos) = crate::import_claude::parse_rfc3339(s)?;
    let off = if s.ends_with('Z') {
        0
    } else {
        let t = s.get(s.len().checked_sub(6)?..)?;
        let hh: i32 = t.get(1..3)?.parse().ok()?;
        let mm: i32 = t.get(4..6)?.parse().ok()?;
        let v = hh * 3600 + mm * 60;
        if t.starts_with('-') { -v } else { v }
    };
    Some(Stamp { secs, nanos, off })
}

// Nanoseconds since the epoch, for signed differences (Go time.Sub).
fn nanos(s: Stamp) -> i128 {
    i128::from(s.secs) * 1_000_000_000 + i128::from(s.nanos)
}

// encoding/json (Marshal, not an Encoder with SetEscapeHTML(false)) writes <, > and & and U+2028 and
// U+2029 as escapes inside strings. serde_json does not. These bytes occur only inside string literals,
// so one byte pass gives the Go bytes.
fn go_escape(buf: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len());
    let mut i = 0;
    while i < buf.len() {
        match buf[i] {
            b'<' => out.extend_from_slice(b"\x5cu003c"),
            b'>' => out.extend_from_slice(b"\x5cu003e"),
            b'&' => out.extend_from_slice(b"\x5cu0026"),
            0xE2 if buf[i..].starts_with(&[0xE2, 0x80, 0xA8]) => {
                out.extend_from_slice(b"\x5cu2028");
                i += 2;
            }
            0xE2 if buf[i..].starts_with(&[0xE2, 0x80, 0xA9]) => {
                out.extend_from_slice(b"\x5cu2029");
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    out
}

/// Persisted client rows. An empty path is a no-op registry (Go nil or empty-path registry).
#[derive(Debug)]
pub struct ClientRegistry {
    path: PathBuf,
    mu: Mutex<()>,
}

impl ClientRegistry {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            mu: Mutex::new(()),
        }
    }

    fn enabled(&self) -> bool {
        !self.path.as_os_str().is_empty()
    }

    // Rows on disk; a missing or broken file reads as empty.
    fn read_locked(&self) -> Vec<ClientRec> {
        match fs::read(&self.path) {
            Ok(b) => serde_json::from_slice(&b).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    // Writes rows sorted by (cwd, model), through a tmp file and rename. Errors are ignored, as in Go.
    fn write_locked(&self, mut rows: Vec<ClientRec>) {
        rows.sort_by(|a, b| (&a.cwd, &a.model).cmp(&(&b.cwd, &b.model)));
        let mut buf = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(b" ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
        if serde::Serialize::serialize(&rows, &mut ser).is_err() {
            return;
        }
        let buf = go_escape(buf);
        let Some(dir) = self.path.parent() else {
            return;
        };
        if fs::create_dir_all(dir).is_err() {
            return;
        }
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        if write_private(&tmp, &buf).is_ok() {
            let _ = fs::rename(&tmp, &self.path);
        }
    }

    /// Row for (cwd, model) from disk.
    pub fn get(&self, cwd: &str, model: &str) -> Option<ClientRec> {
        if !self.enabled() {
            return None;
        }
        let _g = self.mu.lock().unwrap_or_else(|e| e.into_inner());
        self.read_locked()
            .into_iter()
            .find(|c| c.cwd == cwd && c.model == model)
    }

    /// Replaces the (rec.cwd, rec.model) row on disk and keeps every other row.
    pub fn upsert(&self, mut rec: ClientRec) {
        if !self.enabled() {
            return;
        }
        let _g = self.mu.lock().unwrap_or_else(|e| e.into_inner());
        let mut rows = self.read_locked();
        if let Some(i) = rows
            .iter()
            .position(|c| c.cwd == rec.cwd && c.model == rec.model)
        {
            if rec.design_path.is_empty() {
                // A state change does not forget the design file.
                rec.design_path = std::mem::take(&mut rows[i].design_path);
            }
            rows.remove(i);
        }
        rows.push(rec);
        self.write_locked(rows);
    }

    /// Sets the (cwd, model) row to expired with reason. A missing row is created with last_api = now.
    pub fn mark_ended(&self, cwd: &str, model: &str, reason: &str, now: Stamp) {
        if !self.enabled() {
            return;
        }
        let mut rec = self.get(cwd, model).unwrap_or_else(|| ClientRec {
            cwd: cwd.to_string(),
            model: model.to_string(),
            last_api: now,
            ..Default::default()
        });
        rec.state = EXPIRED.to_string();
        rec.ended_reason = reason.to_string();
        rec.child_pid = 0;
        self.upsert(rec);
    }

    /// Fixes live or parked rows whose owner harness died without ending them. Returns the orphans.
    ///   - child_pid alive: expired with "orphan-alive" (shown, never killed or resumed).
    ///   - live row: parked when it has a session_id and last called the API within deadline, else expired.
    ///   - parked row (child dead): stays parked.
    ///
    /// Rows of live owners are left alone.
    pub fn reconcile(&self, now: Stamp, deadline: Duration) -> Vec<ClientRec> {
        if !self.enabled() {
            return Vec::new();
        }
        let _g = self.mu.lock().unwrap_or_else(|e| e.into_inner());
        let mut rows = self.read_locked();
        let mut orphans = Vec::new();
        let mut changed = false;
        for c in rows.iter_mut() {
            let open = c.state == LIVE || c.state == PARKED;
            if !open || pid_alive(c.owner_pid) {
                continue;
            }
            if c.child_pid != 0 && pid_alive(c.child_pid) {
                c.state = EXPIRED.to_string();
                c.ended_reason = "orphan-alive".to_string();
                orphans.push(c.clone()); // copy taken before child_pid is cleared
            } else if c.state == LIVE {
                c.state = EXPIRED.to_string();
                if !c.session_id.is_empty() && recent(now, c.last_api, deadline) {
                    c.state = PARKED.to_string();
                }
                c.ended_reason = "harness-killed".to_string();
            } else {
                c.ended_reason = "harness-killed".to_string();
            }
            c.child_pid = 0;
            changed = true;
        }
        if changed {
            self.write_locked(rows);
        }
        orphans
    }
}

// now - then < deadline (Go now.Sub(t) < deadline).
fn recent(now: Stamp, then: Stamp, deadline: Duration) -> bool {
    nanos(now) - nanos(then) < deadline.as_nanos() as i128
}

// Owner-only file mode (0600 on unix), as Go writes the tmp file.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut o = OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)?.write_all(bytes)
}

/// A row can be resumed with its prompt cache still warm: live or parked, last API call within deadline,
/// and context below limit tokens.
pub fn is_warm(rec: &ClientRec, limit: i64, now: Stamp, deadline: Duration) -> bool {
    if rec.state != LIVE && rec.state != PARKED {
        return false;
    }
    recent(now, rec.last_api, deadline) && rec.last_ctx < limit
}

/// How long a client may sit idle before its prompt cache is assumed cold. ttl "1h" gives idle_rotate;
/// "5m" and an unobserved "auto" give 290s, capped by idle_rotate. idle_rotate zero disables it (returns zero).
pub fn idle_deadline(idle_rotate: Duration, ttl: &str) -> Duration {
    if idle_rotate.is_zero() {
        return Duration::ZERO;
    }
    if ttl == "1h" {
        return idle_rotate;
    }
    idle_rotate.min(Duration::from_secs(290))
}

/// Whether a deadline can expire at all. Zero means never. (Go also has negative durations, which
/// std::time::Duration cannot represent.)
pub fn expired_zero(deadline: Duration) -> bool {
    !deadline.is_zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    type Mutate = fn(&mut ClientRec);

    // Stamp `secs` seconds before now.
    fn ago(now: Stamp, secs: i64) -> Stamp {
        Stamp {
            secs: now.secs - secs,
            ..now
        }
    }

    // Go TestClientRegistryRoundTrip
    #[test]
    fn round_trip_upsert_get_mark_ended() {
        let dir = TempDir::new("clients-roundtrip");
        let path = dir.path().join("clients.json");
        let now = now_stamp();
        let me = std::process::id();
        ClientRegistry::new(&path).upsert(ClientRec {
            cwd: "/w/a".into(),
            model: "haiku".into(),
            session_id: "s1".into(),
            state: LIVE.into(),
            last_api: now,
            last_ctx: 42,
            owner_pid: me,
            child_pid: 7,
            ..Default::default()
        });

        let got = ClientRegistry::new(&path).get("/w/a", "haiku").unwrap();
        assert!(
            got.session_id == "s1"
                && got.state == LIVE
                && got.last_ctx == 42
                && got.child_pid == 7
                && got.owner_pid == me,
            "round trip mismatch: {got:?}"
        );
        assert!(ClientRegistry::new(&path).get("/w/a", "sonnet").is_none());

        let reg = ClientRegistry::new(&path);
        reg.mark_ended("/w/a", "haiku", "idle-rotate", now);
        let got = ClientRegistry::new(&path).get("/w/a", "haiku").unwrap();
        assert!(
            got.state == EXPIRED && got.ended_reason == "idle-rotate" && got.child_pid == 0,
            "mark_ended mismatch: {got:?}"
        );
    }

    // Go nil-registry half of TestClientRegistryRoundTrip and TestRegistryNilWhenNoFile: an empty path is a no-op.
    #[test]
    fn empty_path_is_noop_registry() {
        let reg = ClientRegistry::new("");
        reg.upsert(ClientRec {
            cwd: "/x".into(),
            model: "m".into(),
            ..Default::default()
        });
        assert!(reg.get("/x", "m").is_none());
        assert!(
            reg.reconcile(now_stamp(), Duration::from_secs(60))
                .is_empty()
        );
    }

    // Go TestClientRegistryBrokenJSON
    #[test]
    fn broken_json_reads_as_empty() {
        let dir = TempDir::new("clients-broken");
        let path = dir.path().join("clients.json");
        fs::write(&path, b"{not json").unwrap();
        let reg = ClientRegistry::new(&path);
        assert!(
            reg.get("/w/a", "haiku").is_none(),
            "broken file should read as empty"
        );
        reg.reconcile(now_stamp(), Duration::from_secs(60)); // must not panic
        reg.upsert(ClientRec {
            cwd: "/w/a".into(),
            model: "haiku".into(),
            state: LIVE.into(),
            ..Default::default()
        });
        assert!(
            ClientRegistry::new(&path).get("/w/a", "haiku").is_some(),
            "upsert after broken file should write a fresh row"
        );
    }

    // Go TestClientRegistryAtomicWrite
    #[test]
    fn atomic_write_leaves_no_tmp_file() {
        let dir = TempDir::new("clients-atomic");
        let path = dir.path().join("clients.json");
        let reg = ClientRegistry::new(&path);
        let row = |state: &str| ClientRec {
            cwd: "/w/a".into(),
            model: "haiku".into(),
            state: state.into(),
            ..Default::default()
        };
        reg.upsert(row(LIVE));
        reg.upsert(row(PARKED));
        let mut tmp = path.clone().into_os_string();
        tmp.push(".tmp");
        assert!(!Path::new(&tmp).exists(), "tmp file left behind");
        assert_eq!(reg.get("/w/a", "haiku").unwrap().state, PARKED);
    }

    // Go TestClientRegistryKeyedByCwdModel
    #[test]
    fn rows_keyed_by_cwd_and_model() {
        let dir = TempDir::new("clients-keyed");
        let path = dir.path().join("clients.json");
        let reg = ClientRegistry::new(&path);
        let row = |cwd: &str, model: &str, sid: &str, state: &str| ClientRec {
            cwd: cwd.into(),
            model: model.into(),
            session_id: sid.into(),
            state: state.into(),
            ..Default::default()
        };
        reg.upsert(row("/w/a", "haiku", "a-h", LIVE));
        reg.upsert(row("/w/a", "sonnet", "a-s", LIVE));
        reg.upsert(row("/w/b", "haiku", "b-h", LIVE));
        reg.upsert(row("/w/a", "haiku", "a-h2", PARKED));

        for (cwd, model, sid) in [
            ("/w/a", "haiku", "a-h2"),
            ("/w/a", "sonnet", "a-s"),
            ("/w/b", "haiku", "b-h"),
        ] {
            let got = ClientRegistry::new(&path).get(cwd, model);
            assert_eq!(
                got.as_ref().map(|c| c.session_id.as_str()),
                Some(sid),
                "{cwd}/{model}"
            );
        }
    }

    // Go TestIsWarm
    #[test]
    fn is_warm_checks_state_deadline_and_context() {
        const NOW: Stamp = Stamp {
            secs: 1_800_000_000,
            nanos: 0,
            off: 0,
        };
        let now = NOW;
        let (limit, deadline) = (100_000, Duration::from_secs(300));
        let base = ClientRec {
            cwd: "/w/a".into(),
            model: "haiku".into(),
            state: LIVE.into(),
            last_api: ago(NOW, 60),
            last_ctx: 1000,
            ..Default::default()
        };
        let cases: [(&str, Mutate, bool); 7] = [
            ("live recent small", |_| {}, true),
            ("parked recent small", |r| r.state = PARKED.into(), true),
            ("expired", |r| r.state = EXPIRED.into(), false),
            (
                "api older than deadline",
                |r| r.last_api = ago(NOW, 300),
                false,
            ),
            (
                "api just inside deadline",
                |r| r.last_api = ago(NOW, 299),
                true,
            ),
            ("context at limit", |r| r.last_ctx = 100_000, false),
            ("context below limit", |r| r.last_ctx = 99_999, true),
        ];
        for (name, mutate, want) in cases {
            let mut rec = base.clone();
            mutate(&mut rec);
            assert_eq!(is_warm(&rec, limit, now, deadline), want, "{name}: {rec:?}");
        }
    }

    // Go TestReconcileKilledOwner
    #[test]
    fn reconcile_marks_dead_owner_rows() {
        let dir = TempDir::new("clients-reconcile");
        let path = dir.path().join("clients.json");
        let now = now_stamp();
        let reg = ClientRegistry::new(&path);
        let row = |cwd: &str, sid: &str, secs: i64, owner: u32| ClientRec {
            cwd: cwd.into(),
            model: "haiku".into(),
            session_id: sid.into(),
            state: LIVE.into(),
            last_api: ago(now, secs),
            owner_pid: owner,
            ..Default::default()
        };
        // dead owner (pid 0), recent API, has session -> parked
        reg.upsert(row("/w/a", "s1", 60, 0));
        // dead owner, old API -> expired
        reg.upsert(row("/w/b", "s2", 3600, 0));
        // live owner (this process) -> untouched
        reg.upsert(row("/w/c", "s3", 60, std::process::id()));

        reg.reconcile(now, Duration::from_secs(600));

        let reg2 = ClientRegistry::new(&path);
        let a = reg2.get("/w/a", "haiku").unwrap();
        assert!(
            a.state == PARKED && a.ended_reason == "harness-killed",
            "recent dead-owner row: {a:?}"
        );
        let b = reg2.get("/w/b", "haiku").unwrap();
        assert!(
            b.state == EXPIRED && b.ended_reason == "harness-killed",
            "old dead-owner row: {b:?}"
        );
        let c = reg2.get("/w/c", "haiku").unwrap();
        assert!(
            c.state == LIVE && c.ended_reason.is_empty(),
            "live-owner row changed: {c:?}"
        );
    }

    // Go TestIdleDeadline
    #[test]
    fn idle_deadline_by_ttl_and_cap() {
        let h = Duration::from_secs(3600);
        let s290 = Duration::from_secs(290);
        let cases = [
            (h, "1h", h),
            (h, "5m", s290),
            (h, "auto", s290),
            (Duration::from_secs(100), "auto", Duration::from_secs(100)),
            (Duration::ZERO, "1h", Duration::ZERO),
            (Duration::ZERO, "5m", Duration::ZERO),
        ];
        for (rot, ttl, want) in cases {
            assert_eq!(
                idle_deadline(rot, ttl),
                want,
                "idle_deadline({rot:?}, {ttl:?})"
            );
        }
    }

    // Go TestExpiredZeroDeadline (the negative case is not representable in Duration)
    #[test]
    fn expired_zero_deadline() {
        assert!(!expired_zero(Duration::ZERO));
        assert!(expired_zero(Duration::from_secs(1)));
        assert!(expired_zero(Duration::from_secs(3599)));
    }

    #[test]
    fn stamp_round_trips_rfc3339_nano() {
        for s in [
            "2026-11-04T12:00:00Z",
            "1970-01-01T00:00:00Z",
            "0001-01-01T00:00:00Z",
            "2026-11-04T21:05:06.123+09:00",
            "2024-02-29T23:59:59.5-05:30",
        ] {
            assert_eq!(format_stamp(parse_stamp(s).unwrap()), s, "{s}");
        }
        let at = |secs, nanos| Stamp {
            secs,
            nanos,
            off: 0,
        };
        assert_eq!(format_stamp(at(0, 0)), "1970-01-01T00:00:00Z");
        assert_eq!(format_stamp(at(1, 500_000_000)), "1970-01-01T00:00:01.5Z");
        assert_eq!(format_stamp(Stamp::default()), "0001-01-01T00:00:00Z");
        // +00:00 is UTC, which Go writes as Z.
        assert_eq!(
            format_stamp(parse_stamp("2024-02-29T23:59:59.5+00:00").unwrap()),
            "2024-02-29T23:59:59.5Z"
        );
    }

    #[test]
    fn go_escape_matches_encoding_json() {
        assert_eq!(
            go_escape(b"\"a<b>&c\xE2\x80\xA8\xE2\x80\xA9\"".to_vec()),
            b"\"a\x5cu003cb\x5cu003e\x5cu0026c\x5cu2028\x5cu2029\"".to_vec()
        );
    }

    // A file as Go writes it: json.MarshalIndent of clientRec rows with TZ=Asia/Seoul (fixture
    // clients_go.json, generated by a standalone Go 1.23 program mirroring the struct). It covers the
    // Go zero time, HTML-escaped strings, a +09:00 local time, and omitted empty fields. Upserting each
    // row back rewrites the file byte for byte.
    #[test]
    fn go_written_file_round_trips_unchanged() {
        const GO_FILE: &[u8] = include_bytes!("../tests/fixtures/clients_go.json");
        let dir = TempDir::new("clients-go-fixture");
        let path = dir.path().join("clients.json");
        fs::write(&path, GO_FILE).unwrap();
        let rows: Vec<ClientRec> = serde_json::from_slice(GO_FILE).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].last_api.secs, 1_793_793_600); // 2026-11-04T12:00:00Z
        assert_eq!(rows[1].last_api, Stamp::default());
        assert_eq!(rows[1].cwd, "/w/b&c<d>");
        assert_eq!(rows[2].last_api.off, 9 * 3600);
        let reg = ClientRegistry::new(&path);
        for row in rows {
            reg.upsert(row);
        }
        assert_eq!(fs::read(&path).unwrap(), GO_FILE);
    }

    // Go zero time: a row without last_api writes 0001-01-01T00:00:00Z and omits empty fields (fixture
    // clients_go_zero.json). Go writes "null" for a nil slice and "[]" for an empty one; both read as no
    // rows, and this registry only writes after upserting at least one row.
    #[test]
    fn go_zero_time_and_empty_lists() {
        const GO_ZERO: &[u8] = include_bytes!("../tests/fixtures/clients_go_zero.json");
        let dir = TempDir::new("clients-go-zero");
        let path = dir.path().join("clients.json");
        let reg = ClientRegistry::new(&path);
        reg.upsert(ClientRec {
            cwd: "/w/z".into(),
            model: "haiku".into(),
            state: LIVE.into(),
            ..Default::default()
        });
        assert_eq!(fs::read(&path).unwrap(), GO_ZERO);
        for empty in ["null", "[]"] {
            fs::write(&path, empty).unwrap();
            assert!(
                ClientRegistry::new(&path).get("/w/z", "haiku").is_none(),
                "{empty}"
            );
        }
    }
}
