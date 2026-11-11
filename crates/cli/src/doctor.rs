//! `chaude doctor` and `/doctor`: install and setup checks. Each check reports OK, WARN or FAIL with a fix hint.
//! Other chaude binaries are only read as bytes, never executed.

use chaude_core::i18n::{self, Msg};
use chaude_core::osenv;
use chaude_core::settings::project;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Installs older than this are reported as old.
const CLEAN_FROM: Version = (0, 12, 0);
/// Marker a chaude build carries in its bytes. `main.rs` embeds it; builds without it report "unknown".
pub const MARKER_PREFIX: &str = "chaude-version:";
const GIB: u64 = 1024 * 1024 * 1024;
const TARGET_WARN_BYTES: u64 = 10 * GIB;
const AUTH_TIMEOUT: Duration = Duration::from_secs(3);

type Version = (u64, u64, u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub title: &'static str,
    pub status: Status,
    pub detail: String,
    pub fix: String,
}

fn check(title: &'static str, status: Status, detail: String, fix: String) -> Check {
    Check {
        title,
        status,
        detail,
        fix,
    }
}

fn ok(title: &'static str, detail: String) -> Check {
    check(
        title,
        Status::Ok,
        detail,
        i18n::t(Msg::DoctorNoFix).to_string(),
    )
}

fn show(p: &Path) -> String {
    p.display().to_string()
}

fn version_str(v: Version) -> String {
    format!("{}.{}.{}", v.0, v.1, v.2)
}

fn parse_version(s: &str) -> Option<Version> {
    let mut it = s.split('.');
    let v = (
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    );
    it.next().is_none().then_some(v)
}

/// Version from the first marker in `bytes` that is followed by a version; None when the build has no marker.
pub fn version_in_bytes(bytes: &[u8]) -> Option<Version> {
    let pat = MARKER_PREFIX.as_bytes();
    bytes
        .windows(pat.len())
        .enumerate()
        .filter(|(_, w)| *w == pat)
        .find_map(|(at, _)| {
            let tail: Vec<u8> = bytes[at + pat.len()..]
                .iter()
                .take_while(|b| b.is_ascii_digit() || **b == b'.')
                .copied()
                .collect();
            parse_version(std::str::from_utf8(&tail).ok()?)
        })
}

fn exe_name() -> &'static str {
    if osenv::is_windows() {
        "chaude.exe"
    } else {
        "chaude"
    }
}

fn canon(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn same(a: &Path, b: &Path) -> bool {
    canon(a) == canon(b)
}

/// Directories to search for other installs: every PATH dir plus the usual fixed install dirs.
pub fn candidate_dirs(home: &Path, path: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = path.to_vec();
    dirs.push(PathBuf::from("/usr/local/bin"));
    dirs.push(home.join(".local").join("bin"));
    dirs.push(home.join("go").join("bin"));
    dirs.push(home.join(".cargo").join("bin"));
    let mut seen: Vec<PathBuf> = Vec::new();
    dirs.retain(|d| {
        if seen.contains(d) {
            false
        } else {
            seen.push(d.clone());
            true
        }
    });
    dirs
}

/// Other chaude binaries found in `dirs`, excluding `exe` itself. The version is None when the bytes carry no marker.
pub fn other_installs(exe: &Path, dirs: &[PathBuf]) -> Vec<(PathBuf, Option<Version>)> {
    let mut seen = vec![canon(exe)];
    let mut out = Vec::new();
    for dir in dirs {
        let Some(p) = osenv::find_on_path(exe_name(), std::slice::from_ref(dir)) else {
            continue;
        };
        let c = canon(&p);
        if seen.contains(&c) {
            continue;
        }
        seen.push(c);
        let version = fs::read(&p).ok().and_then(|b| version_in_bytes(&b));
        out.push((p, version));
    }
    out
}

/// One WARN per older or unknown-version install; OK when there are none.
pub fn old_checks(installs: &[(PathBuf, Option<Version>)]) -> Vec<Check> {
    let title = i18n::t(Msg::DoctorCheckOld);
    let mut out = Vec::new();
    for (path, version) in installs {
        let shown = show(path);
        match version {
            Some(v) if *v >= CLEAN_FROM => {}
            Some(v) => out.push(check(
                title,
                Status::Warn,
                i18n::tf(Msg::DoctorOldFound, &[&shown, &version_str(*v)]),
                i18n::tf(Msg::DoctorOldFix, &[&shown]),
            )),
            None => out.push(check(
                title,
                Status::Warn,
                i18n::tf(Msg::DoctorOldUnknown, &[&shown]),
                i18n::tf(Msg::DoctorOldFix, &[&shown]),
            )),
        }
    }
    if out.is_empty() {
        out.push(ok(title, i18n::t(Msg::DoctorOldNone).to_string()));
    }
    out
}

/// Every PATH dir that holds a chaude, in PATH order.
pub fn path_hits(path: &[PathBuf]) -> Vec<PathBuf> {
    path.iter()
        .filter_map(|d| osenv::find_on_path(exe_name(), std::slice::from_ref(d)))
        .collect()
}

/// The first PATH hit should be the running install. FAIL when chaude is not on PATH at all.
pub fn path_check(exe: &Path, hits: &[PathBuf]) -> Check {
    let title = i18n::t(Msg::DoctorCheckPath);
    match hits.first() {
        None => check(
            title,
            Status::Fail,
            i18n::t(Msg::DoctorPathNone).to_string(),
            i18n::t(Msg::DoctorPathNoneFix).to_string(),
        ),
        Some(first) if same(first, exe) => ok(title, i18n::tf(Msg::DoctorPathOk, &[&show(first)])),
        Some(first) => check(
            title,
            Status::Warn,
            i18n::tf(Msg::DoctorPathShadow, &[&show(first), &show(exe)]),
            i18n::tf(Msg::DoctorPathShadowFix, &[&show(first)]),
        ),
    }
}

/// True when the `stat -f %Sf` flag list (comma separated, or "-") contains uchg.
pub fn has_uchg(flags: &str) -> bool {
    flags.trim().split(',').any(|f| f.trim() == "uchg")
}

/// macOS only: an install with the uchg flag cannot be replaced by update. Other OSes are skipped as OK.
pub fn uchg_check(exe: &Path) -> Check {
    let title = i18n::t(Msg::DoctorCheckUchg);
    if !osenv::is_mac() {
        return ok(title, i18n::t(Msg::DoctorUchgNa).to_string());
    }
    let shown = show(exe);
    match Command::new("stat").args(["-f", "%Sf"]).arg(exe).output() {
        Ok(out) if out.status.success() => {
            if has_uchg(&String::from_utf8_lossy(&out.stdout)) {
                check(
                    title,
                    Status::Warn,
                    i18n::tf(Msg::DoctorUchgLocked, &[&shown]),
                    i18n::tf(Msg::DoctorUchgFix, &[&shown]),
                )
            } else {
                ok(title, i18n::tf(Msg::DoctorUchgClear, &[&shown]))
            }
        }
        _ => check(
            title,
            Status::Warn,
            i18n::tf(Msg::DoctorUchgStatFailed, &[&shown]),
            i18n::tf(Msg::DoctorUchgFix, &[&shown]),
        ),
    }
}

fn collect_commands(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (key, val) in map {
                match (key.as_str(), val.as_str()) {
                    ("command", Some(s)) => out.push(s.to_string()),
                    _ => collect_commands(val, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|it| collect_commands(it, out)),
        _ => {}
    }
}

/// Every hook command in the generated set must appear in `<cwd>/.claude/settings.json`.
pub fn hooks_check(cwd: &Path, generated: &str) -> Check {
    let title = i18n::t(Msg::DoctorCheckHooks);
    let path = cwd.join(".claude").join("settings.json");
    let shown = show(&path);
    let fix = i18n::t(Msg::DoctorHooksFix).to_string();
    let Ok(text) = fs::read_to_string(&path) else {
        return check(
            title,
            Status::Warn,
            i18n::tf(Msg::DoctorHooksNoFile, &[&shown]),
            fix,
        );
    };
    let (Ok(have), Ok(want)) = (
        serde_json::from_str::<Value>(&text),
        serde_json::from_str::<Value>(generated),
    ) else {
        return check(
            title,
            Status::Warn,
            i18n::tf(Msg::DoctorHooksBad, &[&shown]),
            fix,
        );
    };
    let (mut have_cmds, mut want_cmds) = (Vec::new(), Vec::new());
    collect_commands(&have, &mut have_cmds);
    collect_commands(&want, &mut want_cmds);
    let total = want_cmds.len();
    let missing = want_cmds.iter().filter(|c| !have_cmds.contains(c)).count();
    if missing == 0 {
        ok(title, i18n::tf(Msg::DoctorHooksOk, &[&total, &shown]))
    } else {
        check(
            title,
            Status::Warn,
            i18n::tf(Msg::DoctorHooksMissing, &[&missing, &total, &shown]),
            fix,
        )
    }
}

/// `claude auth status` exit result, waited at most AUTH_TIMEOUT. None when claude cannot run in time.
pub fn claude_auth_ok() -> Option<bool> {
    let mut child = Command::new(osenv::resolve_bin("claude"))
        .args(["auth", "status"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + AUTH_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.success()),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// `auth`: the `claude auth status` result when it ran. Without it, the credentials file decides.
pub fn login_check(home: &Path, auth: Option<bool>) -> Check {
    let title = i18n::t(Msg::DoctorCheckLogin);
    let creds = home.join(".claude").join(".credentials.json");
    match auth {
        Some(true) => ok(
            title,
            i18n::tf(Msg::DoctorLoginOk, &[&"claude auth status"]),
        ),
        Some(false) => check(
            title,
            Status::Warn,
            i18n::t(Msg::DoctorLoginNo).to_string(),
            i18n::t(Msg::DoctorLoginFix).to_string(),
        ),
        None if creds.is_file() => ok(title, i18n::tf(Msg::DoctorLoginOk, &[&"credentials file"])),
        None => check(
            title,
            Status::Warn,
            i18n::t(Msg::DoctorLoginUnknown).to_string(),
            i18n::t(Msg::DoctorLoginFix).to_string(),
        ),
    }
}

/// Total bytes under `path`; symlinks are not followed.
pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    fs::read_dir(path)
        .map(|rd| rd.flatten().map(|e| dir_size(&e.path())).sum())
        .unwrap_or(0)
}

/// Bytes as GB with one decimal, e.g. "3.5".
pub fn gb(bytes: u64) -> String {
    format!("{}.{}", bytes / GIB, bytes % GIB * 10 / GIB)
}

/// WARN only above 10 GB.
pub fn target_status(bytes: u64) -> Status {
    if bytes > TARGET_WARN_BYTES {
        Status::Warn
    } else {
        Status::Ok
    }
}

/// Size of `<repo>/target`. OK when there is no target dir.
pub fn target_check(repo: &Path) -> Check {
    let title = i18n::t(Msg::DoctorCheckTarget);
    let dir = repo.join("target");
    if !dir.is_dir() {
        return ok(title, i18n::tf(Msg::DoctorTargetNone, &[&show(repo)]));
    }
    let bytes = dir_size(&dir);
    let size = gb(bytes);
    match target_status(bytes) {
        Status::Warn => check(
            title,
            Status::Warn,
            i18n::tf(Msg::DoctorTargetBig, &[&size]),
            i18n::t(Msg::DoctorTargetFix).to_string(),
        ),
        _ => ok(title, i18n::tf(Msg::DoctorTargetOk, &[&size])),
    }
}

/// Every check, in display order. `home` and `cwd` are passed in so tests can use temp dirs.
pub fn run_checks(cwd: &Path, home: &Path, exe: &Path) -> Vec<Check> {
    let path = osenv::path_dirs();
    let mut checks = old_checks(&other_installs(exe, &candidate_dirs(home, &path)));
    checks.push(path_check(exe, &path_hits(&path)));
    checks.push(uchg_check(exe));
    checks.push(match project::guard_settings_text() {
        Ok(generated) => hooks_check(cwd, &generated),
        Err(e) => check(
            i18n::t(Msg::DoctorCheckHooks),
            Status::Warn,
            e.to_string(),
            i18n::t(Msg::DoctorHooksFix).to_string(),
        ),
    });
    checks.push(login_check(home, claude_auth_ok()));
    checks.push(target_check(cwd));
    checks
}

/// Checks for the running process: current dir, home, and current exe.
pub fn checks_now() -> Vec<Check> {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from(exe_name()));
    run_checks(&cwd, &osenv::home_dir(), &exe)
}

/// Printable lines: one status line and one fix line per check, then a summary.
pub fn render(checks: &[Check]) -> Vec<String> {
    let mut lines = vec![i18n::t(Msg::DoctorTitle).to_string()];
    for c in checks {
        let label = match c.status {
            Status::Ok => i18n::t(Msg::DoctorOk),
            Status::Warn => i18n::t(Msg::DoctorWarn),
            Status::Fail => i18n::t(Msg::DoctorFail),
        };
        lines.push(format!("[{label}] {}: {}", c.title, c.detail));
        lines.push(format!("     {}: {}", i18n::t(Msg::DoctorFixLabel), c.fix));
    }
    let count = |s: Status| checks.iter().filter(|c| c.status == s).count();
    let (ok_n, warn_n, fail_n) = (count(Status::Ok), count(Status::Warn), count(Status::Fail));
    lines.push(i18n::tf(Msg::DoctorSummary, &[&ok_n, &warn_n, &fail_n]));
    lines
}

/// `chaude doctor`: print the checks; exit 1 when any check fails.
pub fn run() -> i32 {
    let checks = checks_now();
    for line in render(&checks) {
        println!("{line}");
    }
    if checks.iter().any(|c| c.status == Status::Fail) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = env::temp_dir().join(format!(
            "chaude-doctor-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn put_bin(dir: &Path, bytes: &[u8]) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let p = dir.join(exe_name());
        fs::write(&p, bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        }
        p
    }

    #[test]
    fn version_marker_is_read_from_bytes() {
        assert_eq!(
            version_in_bytes(b"\x7fELF..chaude-version:0.11.3\0rest"),
            Some((0, 11, 3))
        );
        assert_eq!(
            version_in_bytes(b"chaude-version:\0 chaude-version:0.12.2\n"),
            Some((0, 12, 2))
        );
        assert_eq!(version_in_bytes(b"no marker here 0.11.3"), None);
    }

    #[test]
    fn old_installs_are_found_and_unknown_versions_are_warned() {
        let root = temp("old");
        let exe = put_bin(&root.join("install"), b"self");
        let old = put_bin(&root.join("old"), b"xx chaude-version:0.11.3\n");
        let unknown = put_bin(&root.join("unknown"), b"no marker");
        let new = put_bin(&root.join("new"), b"chaude-version:0.12.2\n");
        let dirs = vec![
            root.join("missing"),
            root.join("old"),
            root.join("unknown"),
            root.join("new"),
            root.join("install"),
        ];
        let found = other_installs(&exe, &dirs);
        assert_eq!(
            found,
            vec![
                (old.clone(), Some((0, 11, 3))),
                (unknown.clone(), None),
                (new.clone(), Some((0, 12, 2)))
            ]
        );
        let checks = old_checks(&found);
        assert_eq!(checks.len(), 2);
        assert!(checks.iter().all(|c| c.status == Status::Warn));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_other_install_is_ok_and_newer_is_not_reported() {
        let none = old_checks(&[]);
        assert_eq!(none.len(), 1);
        assert_eq!(none[0].status, Status::Ok);
        let newer = old_checks(&[(PathBuf::from("/x/chaude"), Some((0, 12, 2)))]);
        assert_eq!(newer[0].status, Status::Ok);
    }

    #[test]
    fn path_order_fails_when_missing_and_warns_when_shadowed() {
        let exe = PathBuf::from("/nonexistent/install/chaude");
        assert_eq!(path_check(&exe, &[]).status, Status::Fail);
        assert_eq!(
            path_check(&exe, std::slice::from_ref(&exe)).status,
            Status::Ok
        );
        let other = PathBuf::from("/nonexistent/other/chaude");
        assert_eq!(path_check(&exe, &[other, exe.clone()]).status, Status::Warn);
    }

    #[test]
    fn uchg_flag_is_detected_in_stat_output() {
        assert!(has_uchg("uchg\n"));
        assert!(has_uchg("hidden,uchg"));
        assert!(!has_uchg("-\n"));
        assert!(!has_uchg("hidden"));
    }

    #[test]
    fn hooks_missing_file_or_commands_warn() {
        let cwd = temp("hooks");
        let generated = project::guard_settings_text().unwrap();
        assert_eq!(hooks_check(&cwd, &generated).status, Status::Warn);
        fs::create_dir_all(cwd.join(".claude")).unwrap();
        fs::write(cwd.join(".claude").join("settings.json"), r#"{"hooks":{}}"#).unwrap();
        assert_eq!(hooks_check(&cwd, &generated).status, Status::Warn);
        fs::write(cwd.join(".claude").join("settings.json"), &generated).unwrap();
        assert_eq!(hooks_check(&cwd, &generated).status, Status::Ok);
        let _ = fs::remove_dir_all(&cwd);
    }

    #[test]
    fn login_uses_auth_status_then_credentials_file() {
        let home = temp("login");
        assert_eq!(login_check(&home, Some(true)).status, Status::Ok);
        assert_eq!(login_check(&home, Some(false)).status, Status::Warn);
        assert_eq!(login_check(&home, None).status, Status::Warn);
        fs::create_dir_all(home.join(".claude")).unwrap();
        fs::write(home.join(".claude").join(".credentials.json"), "{}").unwrap();
        assert_eq!(login_check(&home, None).status, Status::Ok);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn target_warns_only_above_ten_gb() {
        assert_eq!(target_status(TARGET_WARN_BYTES), Status::Ok);
        assert_eq!(target_status(TARGET_WARN_BYTES + 1), Status::Warn);
        assert_eq!(gb(3 * GIB + GIB / 2), "3.5");
        let repo = temp("target");
        assert_eq!(target_check(&repo).status, Status::Ok);
        let sub = repo.join("target").join("debug");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("a"), [0u8; 3]).unwrap();
        fs::write(sub.join("b"), [0u8; 5]).unwrap();
        assert_eq!(dir_size(&repo.join("target")), 8);
        assert_eq!(target_check(&repo).status, Status::Ok);
        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn render_has_status_fix_and_summary_lines() {
        let checks = vec![
            ok("a", "fine".to_string()),
            check("b", Status::Warn, "warn".to_string(), "fix b".to_string()),
        ];
        let lines = render(&checks);
        assert_eq!(lines.len(), 1 + 4 + 1);
        assert!(lines.last().unwrap().contains('1'));
    }
}
