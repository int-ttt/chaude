//! Parity with Go update tests (Go tree (v0.11.2) internal/update/update_test.go and install_path_test.go).
//! Offline only: every HTTP call goes through an injected `Fetch`, and archives are built in memory.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use chaude_update::Error;
use chaude_update::cleanup::{migrate_legacy, remove_old};
use chaude_update::fsutil::TempDir;
use chaude_update::http::{Request, Response};
use chaude_update::update::{Options, Version, run, run_cli};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const API: &str = "https://api.test";
const RELEASE_URL: &str = "https://api.test/repos/int-ttt/chaude/releases/latest";

/// Fake GitHub. Serves the latest-release JSON and the asset bodies. `reject_tokens` answers any tokened release read
/// with 401 (Go fakeGitHub.token). `offline` fails every call like a transport error. `log` records (url, token).
struct Fake {
    tag: String,
    assets: Vec<(String, Vec<u8>)>,
    reject_tokens: bool,
    offline: bool,
    log: RefCell<Vec<(String, String)>>,
}

impl Fake {
    fn new(tag: &str) -> Fake {
        Fake {
            tag: tag.to_string(),
            assets: Vec::new(),
            reject_tokens: false,
            offline: false,
            log: RefCell::new(Vec::new()),
        }
    }

    fn asset(mut self, name: &str, data: Vec<u8>) -> Fake {
        self.assets.push((name.to_string(), data));
        self
    }

    /// Asset urls requested so far (the release read is excluded).
    fn downloads(&self) -> Vec<String> {
        self.log
            .borrow()
            .iter()
            .map(|(u, _)| u.clone())
            .filter(|u| u != RELEASE_URL)
            .collect()
    }

    fn call(&self, req: &Request) -> Result<Response, Error> {
        self.log
            .borrow_mut()
            .push((req.url.clone(), req.token.clone()));
        if self.offline {
            return Err(Error::Offline("cannot reach GitHub: dns".to_string()));
        }
        if req.url == RELEASE_URL {
            if self.reject_tokens && !req.token.is_empty() {
                return Ok(Response {
                    status: 401,
                    body: Vec::new(),
                });
            }
            let list: Vec<Value> = self
                .assets
                .iter()
                .map(|(n, _)| json!({ "name": n, "url": format!("{API}/assets/{n}") }))
                .collect();
            let body = json!({ "tag_name": self.tag, "assets": list });
            return Ok(Response {
                status: 200,
                body: serde_json::to_vec(&body).unwrap(),
            });
        }
        match self
            .assets
            .iter()
            .find(|(n, _)| req.url == format!("{API}/assets/{n}"))
        {
            Some((_, data)) => match &req.output {
                Some(p) => {
                    fs::write(p, data).unwrap();
                    Ok(Response {
                        status: 200,
                        body: Vec::new(),
                    })
                }
                None => Ok(Response {
                    status: 200,
                    body: data.clone(),
                }),
            },
            None => Ok(Response {
                status: 404,
                body: Vec::new(),
            }),
        }
    }
}

fn opts(exe: &Path, goos: &str, goarch: &str) -> Options {
    Options {
        repo: "int-ttt/chaude".to_string(),
        api_base: API.to_string(),
        goos: goos.to_string(),
        goarch: goarch.to_string(),
        exe: exe.to_path_buf(),
        ..Options::default()
    }
}

fn sha_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn sums_line(data: &[u8], name: &str) -> String {
    format!("{}  {}\n", sha_hex(data), name)
}

fn tar_gz(name: &str, data: &[u8]) -> Vec<u8> {
    let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut b = tar::Builder::new(gz);
    let mut h = tar::Header::new_gnu();
    h.set_size(data.len() as u64);
    h.set_mode(0o755);
    h.set_entry_type(tar::EntryType::Regular);
    b.append_data(&mut h, name, data).unwrap();
    b.into_inner().unwrap().finish().unwrap()
}

fn zip_bytes(name: &str, data: &[u8]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(name, zip::write::SimpleFileOptions::default())
        .unwrap();
    w.write_all(data).unwrap();
    w.finish().unwrap().into_inner()
}

/// Writes the "running" binary with old contents and returns its path.
fn exe_in(dir: &TempDir, name: &str) -> PathBuf {
    let p = dir.path().join(name);
    fs::write(&p, "OLD").unwrap();
    p
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn putfile(p: &Path, content: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

// Go TestCompareVersion. Go compareVersion returns -1/0/1; Rust orders Version with Ord.
#[test]
fn compare_version() {
    let cases = [
        ("0.9.13", "v0.9.14", -1),
        ("v0.9.14", "0.9.13", 1),
        ("0.9.13", "0.9.13", 0),
        ("0.10.0", "0.9.99", 1),
        ("1.0.0", "0.99.99", 1),
        ("0.9.13-rc1", "v0.9.13", 0),
    ];
    for (a, b, want) in cases {
        let va = Version::parse(a).unwrap_or_else(|| panic!("parse {a}"));
        let vb = Version::parse(b).unwrap_or_else(|| panic!("parse {b}"));
        let got = match va.cmp(&vb) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };
        assert_eq!(got, want, "compare({a}, {b})");
    }
    assert_eq!(Version::parse("unknown"), None);
}

// Go TestUpToDateDownloadsNothing.
#[test]
fn up_to_date_downloads_nothing() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let fake = Fake::new("v0.9.13")
        .asset("chaude_0.9.13_linux_amd64.tar.gz", b"x".to_vec())
        .asset("checksums.txt", b"x".to_vec());
    let f = |r: &Request| fake.call(r);
    let msg = run("0.9.13", opts(&exe, "linux", "amd64"), &f).unwrap();
    assert!(
        msg.contains("up to date") && msg.contains("0.9.13"),
        "{msg}"
    );
    assert!(fake.downloads().is_empty(), "{:?}", fake.downloads());
    assert_eq!(fs::read_to_string(&exe).unwrap(), "OLD");
}

// Go TestUpdateUnixReplacesBinaryAtomically.
#[test]
fn update_unix_replaces_binary_atomically() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let name = "chaude_0.9.14_linux_amd64.tar.gz";
    let archive = tar_gz("chaude", b"NEW");
    let sums = sums_line(&archive, name);
    let fake = Fake::new("v0.9.14")
        .asset(name, archive)
        .asset("checksums.txt", sums.into_bytes());
    let f = |r: &Request| fake.call(r);
    let msg = run("0.9.13", opts(&exe, "linux", "amd64"), &f).unwrap();
    assert!(
        msg.contains("0.9.13 -> 0.9.14") && msg.contains("Restart chaude to use it"),
        "{msg}"
    );
    assert_eq!(fs::read_to_string(&exe).unwrap(), "NEW");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&exe).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
    }
    assert_eq!(names(dir.path()), vec!["chaude".to_string()]);
}

// Go TestUpdateWindowsRenamesOldAndCleanupRemovesIt. Runs with goos=windows on any host (the rename path is pure fs).
#[test]
fn update_windows_renames_old_and_cleanup_removes_it() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude.exe");
    let name = "chaude_0.9.14_windows_amd64.zip";
    let archive = zip_bytes("chaude.exe", b"NEW");
    let sums = sums_line(&archive, name);
    let fake = Fake::new("v0.9.14")
        .asset(name, archive)
        .asset("checksums.txt", sums.into_bytes());
    let f = |r: &Request| fake.call(r);
    run("v0.9.13", opts(&exe, "windows", "amd64"), &f).unwrap();
    assert_eq!(fs::read_to_string(&exe).unwrap(), "NEW");
    let old = PathBuf::from(format!("{}.old", exe.display()));
    assert_eq!(fs::read_to_string(&old).unwrap(), "OLD");
    remove_old(&exe);
    assert!(
        !old.exists(),
        "{} still present after cleanup",
        old.display()
    );
}

// Go TestChecksumMismatchInstallsNothing.
#[test]
fn checksum_mismatch_installs_nothing() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let name = "chaude_0.9.14_linux_amd64.tar.gz";
    let archive = tar_gz("chaude", b"NEW");
    let sums = sums_line(b"tampered", name);
    let fake = Fake::new("v0.9.14")
        .asset(name, archive)
        .asset("checksums.txt", sums.into_bytes());
    let f = |r: &Request| fake.call(r);
    let err = run("0.9.13", opts(&exe, "linux", "amd64"), &f).unwrap_err();
    assert!(
        matches!(&err, Error::Failed(m) if m.contains("checksum mismatch")),
        "{err}"
    );
    assert_eq!(fs::read_to_string(&exe).unwrap(), "OLD");
    assert_eq!(names(dir.path()), vec!["chaude".to_string()]);
}

// Go TestChecksumsRequired.
#[test]
fn checksums_required() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let name = "chaude_0.9.14_linux_amd64.tar.gz";
    let fake = Fake::new("v0.9.14").asset(name, tar_gz("chaude", b"NEW"));
    let f = |r: &Request| fake.call(r);
    let err = run("0.9.13", opts(&exe, "linux", "amd64"), &f).unwrap_err();
    assert!(
        matches!(&err, Error::Failed(m) if m.contains("checksums.txt not found")),
        "{err}"
    );
    assert_eq!(fs::read_to_string(&exe).unwrap(), "OLD");
}

// Go TestNoAssetForPlatform.
#[test]
fn no_asset_for_platform() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let fake = Fake::new("v0.9.14").asset("checksums.txt", b"x".to_vec());
    let f = |r: &Request| fake.call(r);
    let err = run("0.9.13", opts(&exe, "linux", "riscv64"), &f).unwrap_err();
    assert!(
        matches!(&err, Error::Failed(m) if m.contains("no prebuilt binary")),
        "{err}"
    );
}

// Go TestOfflineIsClearError. Go closes the test server; here the injected transport reports Offline.
#[test]
fn offline_is_clear_error() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let mut fake = Fake::new("v0.9.14");
    fake.offline = true;
    let f = |r: &Request| fake.call(r);
    let err = run("0.9.13", opts(&exe, "linux", "amd64"), &f).unwrap_err();
    assert!(
        matches!(&err, Error::Offline(m) if m.contains("offline")),
        "{err}"
    );
}

// Go TestRejectedTokenIsDroppedAndRetried. The stale token is dropped and the release is read again anonymously.
#[test]
fn rejected_token_is_dropped_and_retried() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let mut fake = Fake::new("v0.9.13").asset("checksums.txt", b"x".to_vec());
    fake.reject_tokens = true;
    let f = |r: &Request| fake.call(r);
    let o = Options {
        token: "stale".to_string(),
        ..opts(&exe, "linux", "amd64")
    };
    let msg = run("0.9.13", o, &f).unwrap();
    assert!(msg.contains("up to date"), "{msg}");
    let log = fake.log.borrow();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].1, "stale");
    assert_eq!(log[1].1, "");
}

// Go TestUnknownCurrentVersionIsRefused. Go returns ErrUnknownVersion; Rust has no sentinel variant and returns
// Error::Failed with the same refusal text. Behaviour (refused before any download) matches.
#[test]
fn unknown_current_version_is_refused() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let fake = Fake::new("v0.9.14").asset("checksums.txt", b"x".to_vec());
    let f = |r: &Request| fake.call(r);
    let err = run("unknown", opts(&exe, "linux", "amd64"), &f).unwrap_err();
    assert!(
        matches!(&err, Error::Failed(m) if m.contains("not a release version")),
        "{err}"
    );
    assert!(fake.downloads().is_empty());
    assert_eq!(fs::read_to_string(&exe).unwrap(), "OLD");
}

// Go TestMainUsage.
#[test]
fn main_usage() {
    assert_eq!(run_cli("0.9.13", &["extra".to_string()]), 2);
}

// Go TestAssetNamePlatforms. asset_name is crate-private, so the expected name is read from the error of `run`
// when the release has no assets (the name is checked before any download).
#[test]
fn asset_name_platforms() {
    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let cases = [
        ("windows", "amd64", "chaude_0.9.14_windows_amd64.zip"),
        ("windows", "arm64", "chaude_0.9.14_windows_amd64.zip"),
        ("darwin", "arm64", "chaude_0.9.14_darwin_arm64.tar.gz"),
        ("linux", "arm64", "chaude_0.9.14_linux_arm64.tar.gz"),
    ];
    for (goos, goarch, want) in cases {
        let fake = Fake::new("v0.9.14");
        let f = |r: &Request| fake.call(r);
        let err = run("0.9.13", opts(&exe, goos, goarch), &f).unwrap_err();
        assert!(
            matches!(&err, Error::Failed(m) if m.contains(want)),
            "{goos}/{goarch}: {err}"
        );
    }
    let fake = Fake::new("v0.9.14");
    let f = |r: &Request| fake.call(r);
    let err = run("0.9.13", opts(&exe, "darwin", "amd64"), &f).unwrap_err();
    assert!(
        matches!(&err, Error::Failed(m) if m.contains("Intel Mac")),
        "darwin/amd64: {err}"
    );
}

// Go TestNotWritableChangesNothing. Skipped when the probe write succeeds (root or a permissive ACL).
#[cfg(unix)]
#[test]
fn not_writable_changes_nothing() {
    use std::os::unix::fs::PermissionsExt;

    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
        }
    }

    let dir = TempDir::new("chaude-parity-").unwrap();
    let exe = exe_in(&dir, "chaude");
    let parent = dir.path().to_path_buf();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).unwrap();
    let _restore = Restore(parent.clone());
    if fs::write(parent.join(".probe"), b"").is_ok() {
        eprintln!("skip: install dir is writable (root or ACL); nothing to check");
        return;
    }
    let fake = Fake::new("v0.9.14")
        .asset("chaude_0.9.14_linux_amd64.tar.gz", tar_gz("chaude", b"NEW"))
        .asset("checksums.txt", b"x".to_vec());
    let f = |r: &Request| fake.call(r);
    let err = run("0.9.13", opts(&exe, "linux", "amd64"), &f).unwrap_err();
    assert!(
        err.to_string().contains("reinstall chaude to ~/.local/bin"),
        "{err}"
    );
    assert_eq!(fs::read_to_string(&exe).unwrap(), "OLD");
    assert!(fake.downloads().is_empty(), "{:?}", fake.downloads());
}

// Go TestMigrateLegacyRemovesOldInstallAndPathLine.
#[test]
fn migrate_legacy_removes_old_install_and_path_line() {
    let home = TempDir::new("chaude-parity-").unwrap();
    let legacy = home.path().join(".local").join("bin");
    for n in [
        "chaude",
        "chaude.exe",
        "chaude.0.9.12.bak",
        "chaude.0.9.11.bak",
    ] {
        putfile(&legacy.join(n), "OLD");
    }
    putfile(&legacy.join("other"), "KEEP");
    let line = format!("export PATH=\"{}:$PATH\"", legacy.display());
    putfile(
        &home.path().join(".zshrc"),
        &format!("export FOO=1\n# chaude\n{line}\nalias x=y\n"),
    );
    let running = home.path().join("elsewhere").join("chaude");
    putfile(&running, "OLD");

    migrate_legacy(home.path(), "darwin", &running);

    for n in [
        "chaude",
        "chaude.exe",
        "chaude.0.9.12.bak",
        "chaude.0.9.11.bak",
    ] {
        assert!(!legacy.join(n).exists(), "{n} still present");
    }
    assert_eq!(fs::read_to_string(legacy.join("other")).unwrap(), "KEEP");
    assert_eq!(
        fs::read_to_string(home.path().join(".zshrc")).unwrap(),
        "export FOO=1\n# chaude\nalias x=y\n"
    );
}

// Go TestMigrateLegacySkipsWhenRunningLegacyBinary.
#[test]
fn migrate_legacy_skips_when_running_legacy_binary() {
    let home = TempDir::new("chaude-parity-").unwrap();
    let legacy = home.path().join(".local").join("bin");
    let exe = legacy.join("chaude");
    putfile(&exe, "RUNNING");
    let line = format!("export PATH=\"{}:$PATH\"", legacy.display());
    putfile(&home.path().join(".profile"), &format!("{line}\n"));

    migrate_legacy(home.path(), "linux", &exe);

    assert_eq!(fs::read_to_string(&exe).unwrap(), "RUNNING");
    assert_eq!(
        fs::read_to_string(home.path().join(".profile")).unwrap(),
        format!("{line}\n")
    );
}

// Go TestMigrateLegacyNoopWithoutLegacyFiles.
#[test]
fn migrate_legacy_noop_without_legacy_files() {
    let home = TempDir::new("chaude-parity-").unwrap();
    let line = format!(
        "export PATH=\"{}:$PATH\"",
        home.path().join(".local/bin").display()
    );
    putfile(&home.path().join(".bashrc"), &format!("{line}\n"));
    let exe = home.path().join("elsewhere").join("chaude");
    putfile(&exe, "OLD");

    migrate_legacy(home.path(), "linux", &exe);

    assert_eq!(
        fs::read_to_string(home.path().join(".bashrc")).unwrap(),
        format!("{line}\n")
    );
}
