//! Offline tests for the update flow. Every HTTP call goes to a fake transport; archives are built in memory.

use std::cell::RefCell;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::cleanup::migrate_legacy;
use crate::fsutil::TempDir;
use crate::http::{Request, Response};
use crate::update::{Options, asset_name, check_latest_with, run};
use crate::{Error, Version};

const API: &str = "https://api.test";

/// Fake GitHub. `files` maps asset urls to bodies. `bad_token` makes any tokened request return 401.
/// `log` records (url, token) for each call.
struct Fake {
    release: Value,
    files: Vec<(String, Vec<u8>)>,
    bad_token: bool,
    status: u16,
    log: RefCell<Vec<(String, String)>>,
}

impl Fake {
    fn new(release: Value, files: Vec<(String, Vec<u8>)>) -> Fake {
        Fake {
            release,
            files,
            bad_token: false,
            status: 200,
            log: RefCell::new(Vec::new()),
        }
    }

    fn call(&self, req: &Request) -> Result<Response, Error> {
        self.log
            .borrow_mut()
            .push((req.url.clone(), req.token.clone()));
        if self.bad_token && !req.token.is_empty() {
            return Ok(Response {
                status: 401,
                body: Vec::new(),
            });
        }
        if req.url.ends_with("/releases/latest") {
            return Ok(Response {
                status: self.status,
                body: serde_json::to_vec(&self.release).unwrap(),
            });
        }
        match self.files.iter().find(|(u, _)| *u == req.url) {
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

fn release(tag: &str, assets: &[(&str, &str)]) -> Value {
    let list: Vec<Value> = assets
        .iter()
        .map(|(n, u)| json!({ "name": n, "url": u }))
        .collect();
    json!({ "tag_name": tag, "assets": list })
}

fn sha_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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

fn sums_line(name: &str, data: &[u8]) -> String {
    format!("{}  {}\n", sha_hex(data), name)
}

fn opts(exe: &Path, goos: &str, goarch: &str) -> Options {
    Options {
        api_base: API.to_string(),
        exe: exe.to_path_buf(),
        goos: goos.to_string(),
        goarch: goarch.to_string(),
        ..Options::default()
    }
}

fn write(p: &Path, data: &str) {
    fs::write(p, data).unwrap();
}

#[test]
fn version_parse_and_order() {
    assert_eq!(Version::parse("v0.9.13"), Version::parse("0.9.13"));
    assert!(Version::parse("0.9.13") < Version::parse("0.10.0"));
    assert_eq!(
        Version::parse("v1.2.3-rc1").map(|v| v.to_string()),
        Some("1.2.3".to_string())
    );
    assert_eq!(Version::parse("dev"), None);
    assert_eq!(Version::parse("1.2"), None);
    assert_eq!(Version::parse("1.2.3.4"), None);
}

#[test]
fn asset_names_match_installer_rules() {
    assert_eq!(
        asset_name("1.0.0", "darwin", "arm64").unwrap(),
        "chaude_1.0.0_darwin_arm64.tar.gz"
    );
    assert_eq!(
        asset_name("1.0.0", "windows", "arm64").unwrap(),
        "chaude_1.0.0_windows_amd64.zip"
    );
    assert!(matches!(
        asset_name("1.0.0", "darwin", "amd64"),
        Err(Error::Failed(m)) if m.contains("Intel Mac")
    ));
}

#[test]
fn check_reports_newer_only() {
    let fake = Fake::new(release("v0.11.0", &[]), Vec::new());
    let f = |r: &Request| fake.call(r);
    let got = check_latest_with(
        "0.10.0",
        opts(Path::new("/nonexistent/chaude"), "linux", "amd64"),
        &f,
    )
    .unwrap();
    assert_eq!(got.map(|v| v.to_string()), Some("0.11.0".to_string()));

    let got = check_latest_with(
        "0.11.0",
        opts(Path::new("/nonexistent/chaude"), "linux", "amd64"),
        &f,
    )
    .unwrap();
    assert_eq!(got, None);

    let err = check_latest_with(
        "dev",
        opts(Path::new("/nonexistent/chaude"), "linux", "amd64"),
        &f,
    )
    .unwrap_err();
    assert!(matches!(err, Error::Failed(m) if m.contains("not a release version")));
}

#[test]
fn check_maps_transport_and_status_errors() {
    let fail = |_: &Request| Err(Error::Offline("cannot reach GitHub: dns".to_string()));
    let err = check_latest_with("0.10.0", Options::default(), &fail).unwrap_err();
    assert!(matches!(err, Error::Offline(m) if m.contains("offline?")));

    let mut fake = Fake::new(release("v0.11.0", &[]), Vec::new());
    fake.status = 403;
    let f = |r: &Request| fake.call(r);
    let err = check_latest_with("0.10.0", Options::default(), &f).unwrap_err();
    assert!(matches!(err, Error::Failed(m) if m.contains("rate limited")));
}

#[test]
fn rejected_token_is_dropped_and_retried_anonymously() {
    let mut fake = Fake::new(release("v0.11.0", &[]), Vec::new());
    fake.bad_token = true;
    let f = |r: &Request| fake.call(r);
    let o = Options {
        token: "bad".to_string(),
        ..Options::default()
    };
    let got = check_latest_with("0.10.0", o, &f).unwrap();
    assert!(got.is_some());
    let log = fake.log.borrow();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].1, "bad");
    assert_eq!(log[1].1, "");
}

#[test]
fn up_to_date_installs_nothing() {
    let dir = TempDir::new("chaude-update-test-").unwrap();
    let exe = dir.path().join("chaude");
    write(&exe, "OLD");
    let fake = Fake::new(release("v0.10.0", &[]), Vec::new());
    let f = |r: &Request| fake.call(r);
    let msg = run("0.10.0", opts(&exe, "linux", "amd64"), &f).unwrap();
    assert_eq!(msg, "chaude 0.10.0 is up to date (latest 0.10.0).");
    assert_eq!(fs::read_to_string(&exe).unwrap(), "OLD");
}

#[test]
fn installs_tar_gz_release_over_running_binary() {
    let dir = TempDir::new("chaude-update-test-").unwrap();
    let exe = dir.path().join("chaude");
    write(&exe, "OLD");
    let name = "chaude_0.11.0_linux_amd64.tar.gz";
    let archive = tar_gz("chaude", b"NEW");
    let sums = sums_line(name, &archive);
    let fake = Fake::new(
        release(
            "v0.11.0",
            &[
                (name, "https://api.test/a1"),
                ("checksums.txt", "https://api.test/s1"),
            ],
        ),
        vec![
            ("https://api.test/a1".to_string(), archive),
            ("https://api.test/s1".to_string(), sums.into_bytes()),
        ],
    );
    let f = |r: &Request| fake.call(r);
    let msg = run("0.10.0", opts(&exe, "linux", "amd64"), &f).unwrap();
    assert!(msg.starts_with("chaude updated: 0.10.0 -> 0.11.0"), "{msg}");
    assert_eq!(fs::read_to_string(&exe).unwrap(), "NEW");
}

#[test]
fn windows_zip_moves_running_exe_aside() {
    let dir = TempDir::new("chaude-update-test-").unwrap();
    let exe = dir.path().join("chaude.exe");
    write(&exe, "OLD");
    let name = "chaude_0.11.0_windows_amd64.zip";
    let archive = zip_bytes("chaude.exe", b"NEW");
    let sums = sums_line(name, &archive);
    let fake = Fake::new(
        release(
            "v0.11.0",
            &[
                (name, "https://api.test/a1"),
                ("checksums.txt", "https://api.test/s1"),
            ],
        ),
        vec![
            ("https://api.test/a1".to_string(), archive),
            ("https://api.test/s1".to_string(), sums.into_bytes()),
        ],
    );
    let f = |r: &Request| fake.call(r);
    run("0.10.0", opts(&exe, "windows", "amd64"), &f).unwrap();
    assert_eq!(fs::read_to_string(&exe).unwrap(), "NEW");
    let old = PathBuf::from(format!("{}.old", exe.display()));
    assert_eq!(fs::read_to_string(old).unwrap(), "OLD");
}

#[test]
fn checksum_mismatch_installs_nothing() {
    let dir = TempDir::new("chaude-update-test-").unwrap();
    let exe = dir.path().join("chaude");
    write(&exe, "OLD");
    let name = "chaude_0.11.0_linux_amd64.tar.gz";
    let archive = tar_gz("chaude", b"NEW");
    let sums = sums_line(name, b"something else");
    let fake = Fake::new(
        release(
            "v0.11.0",
            &[
                (name, "https://api.test/a1"),
                ("checksums.txt", "https://api.test/s1"),
            ],
        ),
        vec![
            ("https://api.test/a1".to_string(), archive),
            ("https://api.test/s1".to_string(), sums.into_bytes()),
        ],
    );
    let f = |r: &Request| fake.call(r);
    let err = run("0.10.0", opts(&exe, "linux", "amd64"), &f).unwrap_err();
    assert!(
        matches!(&err, Error::Failed(m) if m.contains("checksum mismatch")),
        "{err}"
    );
    assert_eq!(fs::read_to_string(&exe).unwrap(), "OLD");
}

#[test]
fn missing_asset_is_reported_before_download() {
    let dir = TempDir::new("chaude-update-test-").unwrap();
    let exe = dir.path().join("chaude");
    write(&exe, "OLD");
    let fake = Fake::new(
        release("v0.11.0", &[("checksums.txt", "https://api.test/s1")]),
        Vec::new(),
    );
    let f = |r: &Request| fake.call(r);
    let err = run("0.10.0", opts(&exe, "linux", "amd64"), &f).unwrap_err();
    assert!(
        matches!(&err, Error::Failed(m) if m.contains("not found in release v0.11.0")),
        "{err}"
    );
    assert!(fake.log.borrow().iter().all(|(u, _)| !u.contains("/a1")));
    assert_eq!(fs::read_to_string(&exe).unwrap(), "OLD");
}

#[test]
fn legacy_install_and_path_line_are_removed() {
    let home = TempDir::new("chaude-update-test-").unwrap();
    let bin = home.path().join(".local").join("bin");
    fs::create_dir_all(&bin).unwrap();
    let running = home.path().join("elsewhere").join("chaude");
    fs::create_dir_all(running.parent().unwrap()).unwrap();
    write(&running, "NEW");
    write(&bin.join("chaude"), "LEGACY");
    write(&bin.join("chaude.0.9.0.bak"), "LEGACY");
    let line = format!("export PATH=\"{}:$PATH\"", bin.display());
    write(&home.path().join(".zshrc"), &format!("a\n{line}\nb\n"));
    write(&home.path().join(".profile"), "keep\n");

    migrate_legacy(home.path(), "darwin", &running);

    assert!(!bin.join("chaude").exists());
    assert!(!bin.join("chaude.0.9.0.bak").exists());
    assert_eq!(
        fs::read_to_string(home.path().join(".zshrc")).unwrap(),
        "a\nb\n"
    );
    assert_eq!(
        fs::read_to_string(home.path().join(".profile")).unwrap(),
        "keep\n"
    );
    assert!(running.exists());
}

#[test]
fn running_legacy_file_is_left_alone() {
    let home = TempDir::new("chaude-update-test-").unwrap();
    let bin = home.path().join(".local").join("bin");
    fs::create_dir_all(&bin).unwrap();
    let legacy = bin.join("chaude");
    write(&legacy, "LEGACY");
    write(&bin.join("chaude.0.9.0.bak"), "LEGACY");

    migrate_legacy(home.path(), "darwin", &legacy);

    assert_eq!(fs::read_to_string(&legacy).unwrap(), "LEGACY");
    assert!(bin.join("chaude.0.9.0.bak").exists());
}
