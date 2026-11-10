//! `chaude update` (Go update.go): read the latest GitHub release, download and verify its archive, and replace the
//! running binary. Same API endpoint, asset names and checksum rule as install.sh / install.ps1.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::Error;
use crate::archive;
use crate::fsutil::{self, TempDir};
use crate::http::{self, Request, Response};

const DEFAULT_REPO: &str = "int-ttt/chaude";
const DEFAULT_API: &str = "https://api.github.com";
const USAGE: &str =
    "usage: chaude update   check GitHub for a newer release and replace this binary\n";
/// Whole-request budget (Go: 10 minute client timeout and context).
const TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// Go getJSON reads at most 4 MiB of the release JSON.
const JSON_LIMIT: usize = 4 << 20;

/// HTTP GET used by every request. `default_fetch` goes through curl; tests pass a fake.
pub type Fetch<'a> = &'a dyn Fn(&Request) -> Result<Response, Error>;

/// A release version. Ordering is by (major, minor, patch), which is Go's compareVersion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// Go parseVersion: "v0.9.13", "0.9.13" or "0.9.13-rc1" (a pre-release or build suffix is ignored for ordering).
    pub fn parse(s: &str) -> Option<Version> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let s = match s.find(['-', '+']) {
            Some(i) => &s[..i],
            None => s,
        };
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let mut n = [0u64; 3];
        for (slot, p) in n.iter_mut().zip(parts) {
            *slot = p.parse().ok()?;
        }
        Some(Version {
            major: n[0],
            minor: n[1],
            patch: n[2],
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Update settings. Empty fields take the Go defaults (see `resolve`); `from_env` reads the environment.
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// owner/name; default int-ttt/chaude
    pub repo: String,
    /// default https://api.github.com
    pub api_base: String,
    /// Dropped (and the request retried anonymously) when GitHub rejects it.
    pub token: String,
    /// Asset platform; default the host OS in Go spelling (darwin, linux, windows).
    pub goos: String,
    /// Asset platform; default the host arch in Go spelling (amd64, arm64).
    pub goarch: String,
    /// Binary to replace; default the running executable with symlinks resolved.
    pub exe: PathBuf,
}

impl Options {
    /// Go DefaultOptions: CHAUDE_REPO; GH_TOKEN, else GITHUB_TOKEN, else `gh auth token` (3 second limit).
    pub fn from_env() -> Options {
        let mut o = Options {
            repo: non_empty_env("CHAUDE_REPO").unwrap_or_else(|| DEFAULT_REPO.to_string()),
            ..Options::default()
        };
        o.token = non_empty_env("GH_TOKEN")
            .or_else(|| non_empty_env("GITHUB_TOKEN"))
            .unwrap_or_else(gh_token);
        o
    }

    /// Go withDefaults, without the executable lookup (that is done by `run`).
    fn resolve(mut self) -> Options {
        if self.repo.is_empty() {
            self.repo = DEFAULT_REPO.to_string();
        }
        if self.api_base.is_empty() {
            self.api_base = DEFAULT_API.to_string();
        }
        if self.goos.is_empty() {
            self.goos = host_goos().to_string();
        }
        if self.goarch.is_empty() {
            self.goarch = host_goarch().to_string();
        }
        self
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// `gh auth token` with a 3 second limit. Any failure yields an empty token.
fn gh_token() -> String {
    let Ok(mut child) = Command::new("gh")
        .args(["auth", "token"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return String::new();
    };
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return String::new();
                }
                let mut out = String::new();
                if let Some(mut so) = child.stdout.take() {
                    let _ = so.read_to_string(&mut out);
                }
                return out.trim().to_string();
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return String::new();
            }
        }
    }
}

/// The real transport: curl, with the 10 minute budget counted from this call.
pub fn default_fetch(req: &Request) -> Result<Response, Error> {
    http::fetch(req, Instant::now() + TIMEOUT)
}

/// Checks GitHub for a release newer than `current` with the environment settings and curl.
/// `Ok(None)` means up to date; `Ok(Some(v))` means `v` is available.
pub fn check_latest(current: &str) -> Result<Option<Version>, Error> {
    check_latest_with(current, Options::from_env(), &default_fetch)
}

/// `check_latest` with explicit options and transport.
pub fn check_latest_with(
    current: &str,
    opts: Options,
    fetch: Fetch,
) -> Result<Option<Version>, Error> {
    let mut o = opts.resolve();
    let rel = fetch_release(&mut o, fetch).map_err(|e| release_error(e, &o.repo))?;
    newer_than(current, &rel)
}

/// Go Text: runs `run` for the REPL. The result is the message to show; failures start with "update failed:".
pub fn text(current: &str) -> String {
    match run(current, Options::from_env(), &default_fetch) {
        Ok(msg) => msg,
        Err(e) => format!("update failed: {e}"),
    }
}

/// Go Main: `chaude update`. Exit 0 when up to date or updated, 1 on failure, 2 on bad arguments.
pub fn run_cli(current: &str, args: &[String]) -> i32 {
    if let Some(first) = args.first() {
        if first == "-h" || first == "--help" {
            print!("{USAGE}");
            return 0;
        }
        eprint!("{USAGE}");
        return 2;
    }
    match run(current, Options::from_env(), &default_fetch) {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("update failed: {e}");
            1
        }
    }
}

/// Go Run: checks the latest release and, when it is newer than `current`, installs it over the running binary.
/// Returns the message to show. An `Err` means nothing was installed.
pub fn run(current: &str, opts: Options, fetch: Fetch) -> Result<String, Error> {
    let mut o = opts.resolve();
    if o.exe.as_os_str().is_empty() {
        o.exe = running_exe()?;
    }

    let rel = fetch_release(&mut o, fetch).map_err(|e| release_error(e, &o.repo))?;
    let new_v = trim_v(&rel.tag).to_string();
    let old_v = trim_v(current).to_string();
    if newer_than(current, &rel)?.is_none() {
        return Ok(format!("chaude {old_v} is up to date (latest {new_v})."));
    }

    // Probe the install dir before any download: an unwritable dir changes nothing.
    check_writable(exe_dir(&o.exe), &o.goos)?;

    let name = asset_name(&new_v, &o.goos, &o.goarch)?;
    let asset = find_asset(&rel, &name).ok_or_else(|| {
        Error::Failed(format!(
            "{name} not found in release {} (no prebuilt binary for {}/{}); nothing installed",
            rel.tag, o.goos, o.goarch
        ))
    })?;
    let sums = find_asset(&rel, "checksums.txt").ok_or_else(|| {
        Error::Failed(format!(
            "checksums.txt not found in release {}; refusing to install",
            rel.tag
        ))
    })?;

    let tmp = TempDir::new("chaude-update-")
        .map_err(|e| Error::Failed(format!("cannot create a temp dir: {e}")))?;
    let archive_path = tmp.path().join(&name);
    download(fetch, &o.token, &asset.url, &archive_path)
        .map_err(|e| wrap(e, &format!("download failed: {name}: ")))?;
    let sums_path = tmp.path().join("checksums.txt");
    download(fetch, &o.token, &sums.url, &sums_path)
        .map_err(|e| wrap(e, "download failed: checksums.txt: "))?;
    archive::verify(&archive_path, &name, &sums_path)
        .map_err(|e| append(e, "; nothing installed"))?;

    let bin = bin_name(&o.goos);
    let new_bin = tmp.path().join(bin);
    archive::extract(&archive_path, bin, &new_bin)
        .map_err(|e| wrap(e, &format!("cannot extract {name}: ")))?;
    replace(&o, &new_bin)?;
    Ok(format!(
        "chaude updated: {old_v} -> {new_v} ({}). Restart chaude to use it.",
        o.exe.display()
    ))
}

/// Go compareVersion + the tag/current checks of Run. `Ok(None)` when `current` is not older than the tag.
fn newer_than(current: &str, rel: &Release) -> Result<Option<Version>, Error> {
    let latest = Version::parse(&rel.tag).ok_or_else(|| {
        Error::Failed(format!(
            "release tag {:?} is not a version (vX.Y.Z expected)",
            rel.tag
        ))
    })?;
    let cur = Version::parse(current).ok_or_else(|| {
        Error::Failed(format!(
            "current version is not a release version: {current:?}. Reinstall with install.sh to get updates (latest is {})",
            trim_v(&rel.tag)
        ))
    })?;
    Ok((cur < latest).then_some(latest))
}

fn trim_v(s: &str) -> &str {
    s.strip_prefix('v').unwrap_or(s)
}

/// Go Run's wrapping of a release-fetch failure. Offline keeps its variant and gets a hint.
fn release_error(e: Error, repo: &str) -> Error {
    match e {
        Error::Offline(m) => {
            Error::Offline(format!("{m} (offline? try again when the network is up)"))
        }
        Error::Failed(m) => Error::Failed(format!("cannot read the latest release of {repo}: {m}")),
    }
}

fn wrap(e: Error, prefix: &str) -> Error {
    match e {
        Error::Offline(m) => Error::Offline(format!("{prefix}{m}")),
        Error::Failed(m) => Error::Failed(format!("{prefix}{m}")),
    }
}

fn append(e: Error, suffix: &str) -> Error {
    match e {
        Error::Offline(m) => Error::Offline(format!("{m}{suffix}")),
        Error::Failed(m) => Error::Failed(format!("{m}{suffix}")),
    }
}

/// The running binary with symlinks resolved (Go runningExe).
pub(crate) fn running_exe() -> Result<PathBuf, Error> {
    let exe = std::env::current_exe()
        .map_err(|e| Error::Failed(format!("cannot find the running binary: {e}")))?;
    Ok(fs::canonicalize(&exe).unwrap_or(exe))
}

fn exe_dir(exe: &Path) -> &Path {
    exe.parent().unwrap_or(Path::new("."))
}

/// Go host GOOS: Rust calls macOS "macos".
pub(crate) fn host_goos() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

/// Go host GOARCH.
pub(crate) fn host_goarch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "386",
        other => other,
    }
}

/// Creates and removes a probe file in `dir`. On unix a permission error means the install dir is not writable
/// (e.g. an old `/usr/local/bin` install); the user is told to reinstall to `~/.local/bin`, which needs no sudo.
pub(crate) fn check_writable(dir: &Path, goos: &str) -> Result<(), Error> {
    let probe = dir.join(format!(".chaude-probe-{}", fsutil::unique_suffix()));
    match OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            Ok(())
        }
        Err(e) => {
            if goos != "windows" && e.kind() == io::ErrorKind::PermissionDenied {
                return Err(Error::Failed(format!(
                    "{} is not writable; nothing changed. reinstall chaude to ~/.local/bin (no sudo): curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash",
                    dir.display()
                )));
            }
            Err(Error::Failed(format!(
                "cannot write to {}: {e}; nothing changed",
                dir.display()
            )))
        }
    }
}

/// The release archive install.sh / install.ps1 would download. windows/arm64 gets the windows/amd64 build.
/// darwin/amd64 has no build.
pub(crate) fn asset_name(ver: &str, goos: &str, goarch: &str) -> Result<String, Error> {
    if goos == "darwin" && goarch == "amd64" {
        return Err(Error::Failed(
            "unsupported: Intel Mac (darwin/amd64). chaude ships Apple Silicon (darwin/arm64) builds only; nothing installed".to_string(),
        ));
    }
    if goos == "windows" {
        let arch = if goarch == "arm64" { "amd64" } else { goarch };
        return Ok(format!("chaude_{ver}_windows_{arch}.zip"));
    }
    Ok(format!("chaude_{ver}_{goos}_{goarch}.tar.gz"))
}

pub(crate) fn bin_name(goos: &str) -> &'static str {
    if goos == "windows" {
        "chaude.exe"
    } else {
        "chaude"
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Release {
    #[serde(default, rename = "tag_name")]
    pub tag: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Asset {
    #[serde(default)]
    pub name: String,
    /// API url; downloaded with Accept: application/octet-stream.
    #[serde(default)]
    pub url: String,
}

fn find_asset<'a>(rel: &'a Release, name: &str) -> Option<&'a Asset> {
    rel.assets.iter().find(|a| a.name == name)
}

/// Reads the latest release JSON. When the request fails with a token, the token is dropped (for the rest of the run)
/// and the request is retried anonymously, as Go does.
fn fetch_release(o: &mut Options, fetch: Fetch) -> Result<Release, Error> {
    let url = format!("{}/repos/{}/releases/latest", o.api_base, o.repo);
    let mut body = get_json(fetch, &o.token, &url);
    if body.is_err() && !o.token.is_empty() {
        o.token.clear();
        body = get_json(fetch, "", &url);
    }
    let body = body?;
    let rel: Release = serde_json::from_slice(&body)
        .map_err(|e| Error::Failed(format!("unreadable release JSON: {e}")))?;
    if rel.tag.is_empty() {
        return Err(Error::Failed(
            "could not resolve the release tag".to_string(),
        ));
    }
    Ok(rel)
}

fn get_json(fetch: Fetch, token: &str, url: &str) -> Result<Vec<u8>, Error> {
    let res = fetch(&Request {
        url: url.to_string(),
        accept: "application/vnd.github+json".to_string(),
        token: token.to_string(),
        output: None,
        max_bytes: Some(JSON_LIMIT),
    })?;
    if res.status != 200 {
        return Err(status_error(res.status));
    }
    Ok(res.body)
}

fn status_error(code: u16) -> Error {
    if code == 403 {
        return Error::Failed(
            "GitHub returned HTTP 403 (rate limited?); set GH_TOKEN and try again".to_string(),
        );
    }
    Error::Failed(format!("GitHub returned HTTP {code}"))
}

/// Downloads `url` into `dst`. Asset API urls need Accept: application/octet-stream and redirect to storage.
fn download(fetch: Fetch, token: &str, url: &str, dst: &Path) -> Result<(), Error> {
    let res = fetch(&Request {
        url: url.to_string(),
        accept: "application/octet-stream".to_string(),
        token: token.to_string(),
        output: Some(dst.to_path_buf()),
        max_bytes: None,
    })?;
    if res.status != 200 {
        return Err(status_error(res.status));
    }
    Ok(())
}

/// Copies `src` into a new file in `dir` (same filesystem as the target, so the later rename is atomic), synced and
/// marked 0755.
fn stage_copy(dir: &Path, src: &Path) -> io::Result<PathBuf> {
    let mut input = fs::File::open(src)?;
    let name = dir.join(format!(".chaude-update-{}", fsutil::unique_suffix()));
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&name)?;
    let res = io::copy(&mut input, &mut out)
        .and_then(|_| out.sync_all())
        .and_then(|_| set_executable(&name));
    drop(out);
    if let Err(e) = res {
        let _ = fs::remove_file(&name);
        return Err(e);
    }
    Ok(name)
}

#[cfg(unix)]
fn set_executable(p: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(p, fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn set_executable(_p: &Path) -> io::Result<()> {
    Ok(())
}

/// Puts `new_bin` in place of `o.exe`.
/// Unix: stage in the same directory, then rename over the target (atomic).
/// Windows: a running exe cannot be overwritten, so it is renamed to `exe.old`, the staged file is renamed into place,
/// and `cleanup_old` removes `exe.old` on the next start. If the second rename fails, the old file is renamed back.
fn replace(o: &Options, new_bin: &Path) -> Result<(), Error> {
    let dir = exe_dir(&o.exe);
    let stage = stage_copy(dir, new_bin).map_err(|e| {
        Error::Failed(format!(
            "cannot write to {}: {e} (check permissions, or reinstall with install.sh / install.ps1)",
            dir.display()
        ))
    })?;
    if o.goos != "windows" {
        return fs::rename(&stage, &o.exe).map_err(|e| {
            let _ = fs::remove_file(&stage);
            Error::Failed(format!("cannot replace {}: {e}", o.exe.display()))
        });
    }
    let old = fsutil::with_suffix(&o.exe, ".old");
    let _ = fs::remove_file(&old);
    if let Err(e) = fs::rename(&o.exe, &old) {
        let _ = fs::remove_file(&stage);
        return Err(Error::Failed(format!(
            "cannot move the running {} aside: {e}",
            o.exe.display()
        )));
    }
    if let Err(e) = fs::rename(&stage, &o.exe) {
        let _ = fs::rename(&old, &o.exe);
        let _ = fs::remove_file(&stage);
        return Err(Error::Failed(format!(
            "cannot put the new binary in place: {e}"
        )));
    }
    Ok(())
}
