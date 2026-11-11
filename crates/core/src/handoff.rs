//! Exit handoff (Go exit_handoff.go): the session ids of one chaude run, read by the next start in the same cwd
//! so /view and /resume can find them again.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// File name in <cwd>/notes/tmp/long (tmp_layout). Not .md, so the record listing ignores it.
pub const EXIT_HANDOFF_FILE: &str = ".exit-handoff.json";
// Written while a run is live and removed on clean exit. Still present at the next start = the run was killed.
pub const LIVE_HANDOFF_FILE: &str = ".live-handoff.json";
// ctx_tokens key of the main child; residents use their role name.
pub const MAIN_ROLE: &str = "main";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExitHandoff {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub cwd: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub main_session: String,
    // role -> session id (Go resident_sessions)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resident_sessions: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owned_sessions: Vec<String>,
    // role -> context tokens at the session's last result (MAIN_ROLE for the main child). Chaude only.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ctx_tokens: BTreeMap<String, u64>,
    // Unix seconds. Go writes an RFC 3339 "saved_at" string, which is not read here.
    #[serde(default)]
    pub saved_at_unix: u64,
    // Harness pid that wrote the file (Go owner_pid). Only the live marker needs it; 0 when unknown.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub owner_pid: u32,
    // role -> claude child pid while the child runs (Go child_pid). Only the live marker needs it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub child_pids: BTreeMap<String, u32>,
    // role -> model alias of that child. Only the orphan notice shows it; empty for files from older runs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub child_models: BTreeMap<String, String>,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

// What a session load does with a previous session (see resume_mode).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Load {
    // Start the session with `--resume <id>`.
    Resume(String),
    // Start a fresh session (the handoff path carries the context).
    Fresh,
}

impl ExitHandoff {
    // Go loadPrevOwned: the owned sessions of the previous run plus its main session. /view lists these.
    pub fn prev_owned(&self) -> BTreeSet<String> {
        let mut ids: BTreeSet<String> = self.owned_sessions.iter().cloned().collect();
        if !self.main_session.is_empty() {
            ids.insert(self.main_session.clone());
        }
        ids
    }
}

// Go reconcileStart: children a dead harness left running. Empty while the owner is alive. Otherwise each
// (role, pid) whose child is alive, in role order. `alive` is pid_alive in production.
pub fn orphans(prev: &ExitHandoff, alive: impl Fn(u32) -> bool) -> Vec<(String, u32)> {
    if prev.owner_pid != 0 && alive(prev.owner_pid) {
        return Vec::new();
    }
    prev.child_pids
        .iter()
        .filter(|(_, pid)| **pid != 0 && alive(**pid))
        .map(|(role, pid)| (role.clone(), *pid))
        .collect()
}

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

// errno ESRCH (no such process) is 3 on Linux and macOS.
#[cfg(unix)]
const ESRCH: i32 = 3;

// Alive when kill succeeded, or failed with anything but ESRCH. EPERM means the pid exists under another user.
#[cfg(unix)]
fn alive_after_kill(ok: bool, errno: Option<i32>) -> bool {
    ok || errno != Some(ESRCH)
}

// Windows: OpenProcess with query-only access, then GetExitCodeProcess reports STILL_ACTIVE while the process runs.
// A failed open reads as dead unless it is ERROR_ACCESS_DENIED (the pid exists under another user).
#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const ERROR_ACCESS_DENIED: u32 = 5;
    const STILL_ACTIVE: u32 = 259;

    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn GetLastError() -> u32;
    }

    pub fn pid_alive(pid: u32) -> bool {
        // SAFETY: OpenProcess takes no pointers; a non-null handle is closed below.
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if h.is_null() {
            return unsafe { GetLastError() } == ERROR_ACCESS_DENIED;
        }
        let mut code = 0u32;
        // SAFETY: h is a live handle and code is a valid out pointer.
        let ok = unsafe { GetExitCodeProcess(h, &mut code) } != 0;
        // SAFETY: h came from OpenProcess and is closed exactly once.
        unsafe { CloseHandle(h) };
        // When the query fails, keep the pid as alive: only a confirmed exit reads as dead.
        !ok || code == STILL_ACTIVE
    }
}

// Go pidAlive. kill(pid, 0) sends no signal: only ESRCH reads as dead, EPERM reads as alive (not ours).
pub fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // pid_t is i32. A larger pid would wrap negative, and kill(-n) signals a process group. pid 0 is the caller's group.
        let Ok(pid) = i32::try_from(pid) else {
            return false;
        };
        if pid == 0 {
            return false;
        }
        // SAFETY: kill with signal 0 only checks the pid and touches no memory.
        let ok = unsafe { kill(pid, 0) } == 0;
        alive_after_kill(ok, io::Error::last_os_error().raw_os_error())
    }
    #[cfg(windows)]
    {
        win::pid_alive(pid)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

// <cwd>/notes/tmp/long/.exit-handoff.json
pub fn handoff_file(project_dir: &Path) -> PathBuf {
    project_dir.join(EXIT_HANDOFF_FILE)
}

// The live marker next to an exit handoff file (same folder, LIVE_HANDOFF_FILE).
pub fn live_handoff_file(exit_path: &Path) -> PathBuf {
    exit_path.with_file_name(LIVE_HANDOFF_FILE)
}

// The record of the previous run and whether it was killed. A live marker means the run never reached its
// clean exit, so it is newer than the exit handoff and wins. None when neither file parses.
pub fn previous_run(exit_path: &Path) -> Option<(ExitHandoff, bool)> {
    if let Some(live) = read_exit_handoff(&live_handoff_file(exit_path)) {
        return Some((live, true));
    }
    read_exit_handoff(exit_path).map(|h| (h, false))
}

// The main session of the previous run when that run was killed (no clean exit), else None. Bare /resume uses
// it to import that session's digest. Read before the new run writes its own live marker.
pub fn killed_main_session(exit_path: &Path) -> Option<String> {
    match previous_run(exit_path) {
        Some((h, true)) if !h.main_session.is_empty() => Some(h.main_session),
        _ => None,
    }
}

// Decides how a session of `role` (MAIN_ROLE or a resident role) starts, from the previous run's record.
// Resume when the run was killed (no clean exit handoff), or when the session's context is below the
// rotation threshold. Fresh when there is no session id, or a clean exit above the threshold (the handoff
// path carries it). threshold 0 = rotation off, so every session resumes. A missing ctx counts as 0.
pub fn resume_mode(prev: &ExitHandoff, role: &str, killed: bool, threshold: u64) -> Load {
    let sid = if role == MAIN_ROLE {
        prev.main_session.as_str()
    } else {
        prev.resident_sessions.get(role).map_or("", String::as_str)
    };
    if sid.is_empty() {
        return Load::Fresh;
    }
    let ctx = prev.ctx_tokens.get(role).copied().unwrap_or(0);
    if killed || threshold == 0 || ctx < threshold {
        Load::Resume(sid.to_string())
    } else {
        Load::Fresh
    }
}

// Clean exit: the live marker is no longer needed. A missing file is fine.
pub fn remove_live_handoff(exit_path: &Path) {
    let _ = fs::remove_file(live_handoff_file(exit_path));
}

// Go writeExitHandoff: version 1 and saved_at now, written to a temp file then renamed.
pub fn write_exit_handoff(path: &Path, h: &ExitHandoff) -> io::Result<()> {
    let mut out = h.clone();
    out.version = 1;
    out.saved_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let text = serde_json::to_string_pretty(&out).map_err(io::Error::other)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, format!("{text}\n"))?;
    fs::rename(&tmp, path)
}

// Go readExitHandoff: None when the file is missing or does not parse.
pub fn read_exit_handoff(path: &Path) -> Option<ExitHandoff> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

// Append-only log of the Claude session ids a record folder has run (one JSON object per line). /resume lists
// it. Not .md, so the record listing ignores it.
pub const SESSIONS_FILE: &str = ".sessions.jsonl";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRef {
    pub session_id: String,
    // MAIN_ROLE or a resident role.
    pub role: String,
    // Unix seconds when the id was first seen by this run.
    pub saved_at_unix: u64,
}

// The session log next to an exit handoff file (same folder, SESSIONS_FILE).
pub fn sessions_file(exit_path: &Path) -> PathBuf {
    exit_path.with_file_name(SESSIONS_FILE)
}

// Appends one session id. Empty ids are ignored.
pub fn append_session(exit_path: &Path, role: &str, session_id: &str) -> io::Result<()> {
    use std::io::Write;
    if session_id.is_empty() {
        return Ok(());
    }
    let row = SessionRef {
        session_id: session_id.to_string(),
        role: role.to_string(),
        saved_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    let line = serde_json::to_string(&row).map_err(io::Error::other)?;
    if let Some(dir) = exit_path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(sessions_file(exit_path))?;
    writeln!(f, "{line}")
}

// The session log, oldest first, each session id once (its first row). Unparsable lines are skipped.
pub fn read_sessions(exit_path: &Path) -> Vec<SessionRef> {
    let Ok(text) = fs::read_to_string(sessions_file(exit_path)) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    text.lines()
        .filter_map(|l| serde_json::from_str::<SessionRef>(l).ok())
        .filter(|r| seen.insert(r.session_id.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn session_log_appends_and_reads_each_id_once() {
        let dir = TempDir::new("handoff-sessions");
        let path = handoff_file(dir.path());
        assert!(read_sessions(&path).is_empty());
        append_session(&path, MAIN_ROLE, "s1").unwrap();
        append_session(&path, "coder", "c1").unwrap();
        append_session(&path, MAIN_ROLE, "s1").unwrap();
        append_session(&path, MAIN_ROLE, "").unwrap();
        let got = read_sessions(&path);
        let ids: Vec<(&str, &str)> = got
            .iter()
            .map(|r| (r.session_id.as_str(), r.role.as_str()))
            .collect();
        assert_eq!(ids, vec![("s1", MAIN_ROLE), ("c1", "coder")]);
    }

    fn sample() -> ExitHandoff {
        let mut h = ExitHandoff {
            cwd: "/work/app".to_string(),
            main_session: "m1".to_string(),
            owned_sessions: vec!["m0".to_string(), "m1".to_string()],
            ..Default::default()
        };
        h.resident_sessions
            .insert("coder".to_string(), "c1".to_string());
        h
    }

    #[test]
    fn write_then_read_keeps_ids_and_sets_version() {
        let dir = TempDir::new("handoff-rt");
        let path = handoff_file(dir.path());
        write_exit_handoff(&path, &sample()).expect("write");
        let got = read_exit_handoff(&path).expect("read");
        assert_eq!(got.version, 1);
        assert_eq!(got.main_session, "m1");
        assert_eq!(
            got.resident_sessions.get("coder").map(String::as_str),
            Some("c1")
        );
        assert_eq!(got.owned_sessions, vec!["m0", "m1"]);
        assert!(got.saved_at_unix > 0);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn missing_or_bad_file_reads_as_none() {
        let dir = TempDir::new("handoff-bad");
        assert!(read_exit_handoff(&handoff_file(dir.path())).is_none());
        let path = handoff_file(dir.path());
        fs::write(&path, "{not json").unwrap();
        assert!(read_exit_handoff(&path).is_none());
    }

    #[test]
    fn reads_go_written_file_with_string_saved_at() {
        let dir = TempDir::new("handoff-go");
        let path = handoff_file(dir.path());
        let go = r#"{"version":1,"cwd":"/w","main_session":"g1",
            "resident_sessions":{"tui":"t1"},"owned_sessions":["g0","g1"],
            "saved_at":"2026-11-04T10:00:00+09:00"}"#;
        fs::write(&path, go).unwrap();
        let got = read_exit_handoff(&path).expect("go file parses");
        assert_eq!(got.main_session, "g1");
        assert_eq!(
            got.resident_sessions.get("tui").map(String::as_str),
            Some("t1")
        );
        assert_eq!(got.saved_at_unix, 0);
    }

    #[test]
    fn empty_main_session_is_omitted() {
        let dir = TempDir::new("handoff-omit");
        let path = handoff_file(dir.path());
        let h = ExitHandoff {
            cwd: "/w".to_string(),
            ..Default::default()
        };
        write_exit_handoff(&path, &h).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("main_session"), "{text}");
        assert!(!text.contains("resident_sessions"), "{text}");
    }

    #[test]
    fn prev_owned_is_owned_plus_main_without_duplicates() {
        let ids = sample().prev_owned();
        let want: BTreeSet<String> = ["m0", "m1"].iter().map(|s| s.to_string()).collect();
        assert_eq!(ids, want);
    }

    fn with_ctx(main: u64, coder: u64) -> ExitHandoff {
        let mut h = sample();
        h.ctx_tokens.insert(MAIN_ROLE.to_string(), main);
        h.ctx_tokens.insert("coder".to_string(), coder);
        h
    }

    #[test]
    fn killed_run_resumes_even_above_threshold() {
        let h = with_ctx(150_000, 0);
        assert_eq!(
            resume_mode(&h, MAIN_ROLE, true, 80_000),
            Load::Resume("m1".to_string())
        );
    }

    #[test]
    fn clean_exit_below_threshold_resumes() {
        let h = with_ctx(79_999, 0);
        assert_eq!(
            resume_mode(&h, MAIN_ROLE, false, 80_000),
            Load::Resume("m1".to_string())
        );
    }

    #[test]
    fn clean_exit_at_or_above_threshold_is_fresh() {
        assert_eq!(
            resume_mode(&with_ctx(80_000, 0), MAIN_ROLE, false, 80_000),
            Load::Fresh
        );
        assert_eq!(
            resume_mode(&with_ctx(120_000, 0), MAIN_ROLE, false, 80_000),
            Load::Fresh
        );
    }

    #[test]
    fn resident_role_uses_its_own_session_and_ctx() {
        let h = with_ctx(150_000, 10_000);
        assert_eq!(
            resume_mode(&h, "coder", false, 80_000),
            Load::Resume("c1".to_string())
        );
        // tui has no recorded session id, so it starts fresh whatever its ctx.
        assert_eq!(
            resume_mode(&h, "tui", false, 80_000),
            Load::Fresh,
            "tui has no session id"
        );
    }

    #[test]
    fn no_session_id_is_fresh_even_when_killed() {
        let h = ExitHandoff::default();
        assert_eq!(resume_mode(&h, MAIN_ROLE, true, 80_000), Load::Fresh);
    }

    #[test]
    fn zero_threshold_resumes_every_session() {
        assert_eq!(
            resume_mode(&with_ctx(500_000, 0), MAIN_ROLE, false, 0),
            Load::Resume("m1".to_string())
        );
    }

    #[test]
    fn live_marker_wins_over_exit_handoff_and_is_removed_on_clean_exit() {
        let dir = TempDir::new("handoff-live");
        let exit = handoff_file(dir.path());
        let old = ExitHandoff {
            main_session: "old".to_string(),
            ..Default::default()
        };
        write_exit_handoff(&exit, &old).unwrap();
        assert_eq!(
            previous_run(&exit).map(|(h, k)| (h.main_session, k)),
            Some(("old".to_string(), false))
        );

        write_exit_handoff(&live_handoff_file(&exit), &sample()).unwrap();
        let (h, killed) = previous_run(&exit).expect("live record");
        assert!(killed);
        assert_eq!(h.main_session, "m1");

        remove_live_handoff(&exit);
        assert!(!live_handoff_file(&exit).exists());
        assert_eq!(
            previous_run(&exit).map(|(h, k)| (h.main_session, k)),
            Some(("old".to_string(), false))
        );
    }

    #[test]
    fn killed_main_session_only_for_unclean_exit() {
        let dir = TempDir::new("handoff-killed");
        let exit = handoff_file(dir.path());
        // No files: nothing to report.
        assert_eq!(killed_main_session(&exit), None);
        // Clean exit: the exit handoff alone is not a kill.
        write_exit_handoff(&exit, &sample()).unwrap();
        assert_eq!(killed_main_session(&exit), None);
        // Live marker left behind (no clean exit): its main session is the killed one.
        write_exit_handoff(&live_handoff_file(&exit), &sample()).unwrap();
        assert_eq!(killed_main_session(&exit), Some("m1".to_string()));
        // Clean exit again removes the marker.
        remove_live_handoff(&exit);
        assert_eq!(killed_main_session(&exit), None);
        // Killed run with no main session has no digest target.
        let no_main = ExitHandoff {
            main_session: String::new(),
            ..Default::default()
        };
        write_exit_handoff(&live_handoff_file(&exit), &no_main).unwrap();
        assert_eq!(killed_main_session(&exit), None);
    }

    #[test]
    fn ctx_tokens_round_trip_and_empty_map_is_omitted() {
        let dir = TempDir::new("handoff-ctx");
        let path = handoff_file(dir.path());
        write_exit_handoff(&path, &with_ctx(42, 7)).unwrap();
        let got = read_exit_handoff(&path).unwrap();
        assert_eq!(got.ctx_tokens.get(MAIN_ROLE), Some(&42));
        assert_eq!(got.ctx_tokens.get("coder"), Some(&7));
        write_exit_handoff(&path, &sample()).unwrap();
        assert!(!fs::read_to_string(&path).unwrap().contains("ctx_tokens"));
    }

    #[test]
    fn orphans_lists_alive_children_only_when_owner_is_dead() {
        let mut h = sample();
        h.owner_pid = 100;
        h.child_pids = BTreeMap::from([
            ("main".to_string(), 201),
            ("coder".to_string(), 202),
            ("tui".to_string(), 0),
        ]);
        // Owner dead: the alive children are orphans; a zero pid is never one.
        let alive = |pid: u32| pid == 201 || pid == 202;
        assert_eq!(
            orphans(&h, alive),
            vec![("coder".to_string(), 202), ("main".to_string(), 201)]
        );
        // Owner alive (another run in the same folder): nothing to report.
        assert!(orphans(&h, |pid| pid == 100 || pid == 201).is_empty());
        // Children all gone: nothing to report.
        assert!(orphans(&h, |_| false).is_empty());
    }

    #[test]
    fn owner_and_child_pids_round_trip_and_stay_out_of_clean_files() {
        let dir = TempDir::new("handoff-pids");
        let path = handoff_file(dir.path());
        let mut h = sample();
        h.owner_pid = 7;
        h.child_pids = BTreeMap::from([("main".to_string(), 9)]);
        write_exit_handoff(&path, &h).unwrap();
        let got = read_exit_handoff(&path).expect("readable");
        assert_eq!(got.owner_pid, 7);
        assert_eq!(got.child_pids.get("main"), Some(&9));
        write_exit_handoff(&path, &sample()).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("owner_pid") && !text.contains("child_pids"),
            "{text}"
        );
    }

    #[test]
    fn child_models_round_trip_and_stay_out_of_clean_files() {
        let dir = TempDir::new("handoff-models");
        let path = handoff_file(dir.path());
        let mut h = sample();
        h.child_models = BTreeMap::from([
            ("main".to_string(), "opus".to_string()),
            ("coder".to_string(), "haiku".to_string()),
        ]);
        write_exit_handoff(&path, &h).unwrap();
        let got = read_exit_handoff(&path).expect("readable");
        assert_eq!(
            got.child_models.get("coder").map(String::as_str),
            Some("haiku")
        );
        assert_eq!(
            got.child_models.get("main").map(String::as_str),
            Some("opus")
        );
        write_exit_handoff(&path, &sample()).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("child_models"), "{text}");
    }

    #[cfg(unix)]
    #[test]
    fn kill_errno_only_esrch_means_dead() {
        assert!(alive_after_kill(true, None));
        assert!(
            alive_after_kill(false, Some(1)),
            "EPERM: exists, owned by another user"
        );
        assert!(!alive_after_kill(false, Some(ESRCH)));
    }

    #[test]
    fn pid_alive_own_pid_portable() {
        assert!(pid_alive(std::process::id()));
        assert!(!pid_alive(0));
        assert!(!pid_alive(u32::MAX));
    }

    #[cfg(unix)]
    #[test]
    fn pid_alive_on_real_pids() {
        assert!(pid_alive(std::process::id()));
        assert!(pid_alive(1), "pid 1 is alive even when not ours (EPERM)");
        assert!(!pid_alive(0));
        assert!(!pid_alive(u32::MAX));
    }
}
