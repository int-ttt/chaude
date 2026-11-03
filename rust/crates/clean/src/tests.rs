use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chaude_core::settings::project::{embedded_hook, ensure_project_settings};

use super::*;

/// 2026-11-01 09:30:00 UTC.
fn fixed_now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_793_525_400)
}

fn no_pkg_hook(_: &str) -> Option<&'static [u8]> {
    None
}

fn write_test(p: &Path, body: &str) {
    write_file(p, body.as_bytes()).unwrap();
}

fn exists(p: &Path) -> bool {
    fs::metadata(p).is_ok()
}

/// Lists every file under dir with its contents, for before/after comparison.
fn snapshot(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        let Ok(rd) = fs::read_dir(dir) else {
            return;
        };
        for e in rd {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                let b = fs::read(&p).unwrap();
                out.push(format!(
                    "{}\x00{}",
                    p.display(),
                    String::from_utf8_lossy(&b)
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

/// Writes a project as the old install step left it: package files, CLAUDE.md policy block above user content,
/// settings.json with user keys and a user hook next to the package hook, plus notes/ and record/.
fn install_fixture(dir: &Path) -> PathBuf {
    for &rel in PKG_FILES {
        let body = match rel.strip_prefix(HOOKS_DIR) {
            Some(name) => String::from_utf8(
                embedded_hook(name)
                    .unwrap_or_else(|| panic!("no embedded copy of {rel}"))
                    .to_vec(),
            )
            .unwrap(),
            None => format!("pkg {rel}"),
        };
        write_test(&dir.join(rel), &body);
    }
    write_test(
        &dir.join("CLAUDE.md"),
        &format!("{MARK_START}\npolicy\n{MARK_END}\n\n# my project\nkeep me\n"),
    );
    write_test(
        &dir.join(SETTINGS_REL),
        r#"{
  "model": "opus",
  "env": {"X": "1"},
  "hooks": {
    "PreToolUse": [
      {"matcher": "Bash", "hooks": [
        {"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/main-thread-guard.py"},
        {"type": "command", "command": "/usr/local/bin/my-own-hook.sh"}
      ]}
    ],
    "Stop": [
      {"hooks": [{"type": "command", "command": "python3 .claude/hooks/ledger.py"}]}
    ]
  }
}"#,
    );
    write_test(&dir.join("notes/keep.md"), "notes");
    write_test(&dir.join("record/keep.jsonl"), "record");
    write_test(&dir.join(".claude/user-file.txt"), "user");
    dir.to_path_buf()
}

fn tmp() -> tempdir::Dir {
    tempdir::Dir::new()
}

/// Minimal temp dir (no dev-dependency): unique path under the system temp dir, removed on drop.
mod tempdir {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    pub struct Dir(PathBuf);

    impl Dir {
        pub fn new() -> Self {
            let n = N.fetch_add(1, Ordering::SeqCst);
            let p =
                std::env::temp_dir().join(format!("chaude-clean-test-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Dir(p)
        }
        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

/// A hook file that differs from the package copy is kept; identical ones are removed.
#[test]
fn apply_keeps_user_modified_hook() {
    let t = tmp();
    let dir = install_fixture(t.path());
    let edited = ".claude/hooks/main-thread-guard.py";
    write_test(&dir.join(edited), "my own guard edit");
    let p = make_plan(&dir).unwrap();
    assert!(
        p.kept().iter().any(|k| k == edited),
        "modified hook not kept: {:?}",
        p.kept()
    );
    p.apply(fixed_now()).unwrap();
    assert_eq!(
        fs::read_to_string(dir.join(edited)).unwrap(),
        "my own guard edit"
    );
    assert!(
        !exists(&dir.join(".claude/hooks/ledger.py")),
        "unmodified package hook was not removed"
    );
    let s = fs::read_to_string(dir.join(SETTINGS_REL)).unwrap();
    assert!(
        s.contains("main-thread-guard.py"),
        "settings entry of kept hook removed:\n{s}"
    );
    assert!(
        !s.contains("ledger.py"),
        "settings entry of removed hook stayed:\n{s}"
    );
}

#[test]
fn apply_keeps_hooks_without_package_copy() {
    let t = tmp();
    let dir = install_fixture(t.path());
    let p = plan_with(&dir, no_pkg_hook).unwrap();
    let hooks = PKG_FILES
        .iter()
        .filter(|r| r.starts_with(HOOKS_DIR))
        .count();
    assert!(hooks > 0);
    assert_eq!(p.kept().len(), hooks, "kept should be all hook files");
    p.apply(fixed_now()).unwrap();
    for rel in p.kept() {
        assert!(exists(&dir.join(rel)), "kept hook removed: {rel}");
    }
    for &rel in PKG_FILES {
        if !rel.starts_with(HOOKS_DIR) {
            assert!(
                !exists(&dir.join(rel)),
                "non-hook manifest file still present: {rel}"
            );
        }
    }
}

#[test]
fn dry_run_changes_nothing() {
    let t = tmp();
    let dir = install_fixture(t.path());
    let before = snapshot(&dir);
    let p = make_plan(&dir).unwrap();
    assert!(
        !p.lines().is_empty(),
        "expected targets in a fixture with package files"
    );
    assert_eq!(before, snapshot(&dir), "dry-run changed the tree");
}

#[test]
fn apply_removes_manifest_and_backs_up() {
    let t = tmp();
    let dir = install_fixture(t.path());
    let p = make_plan(&dir).unwrap();
    let bdir = p.apply(fixed_now()).unwrap();
    assert_eq!(bdir.file_name().unwrap(), ".chaude-backup-20261101-093000");
    for &rel in PKG_FILES {
        assert!(
            !exists(&dir.join(rel)),
            "manifest file still present: {rel}"
        );
        assert!(
            exists(&bdir.join(rel)),
            "manifest file not backed up: {rel}"
        );
    }
    for rel in ["CLAUDE.md", SETTINGS_REL] {
        assert!(exists(&bdir.join(rel)), "{rel} not in backup");
    }
    assert!(
        exists(&dir.join(".claude/user-file.txt")),
        "file outside manifest was removed"
    );
    assert!(
        exists(&dir.join(".claude")),
        ".claude removed although a user file is in it"
    );
}

#[test]
fn apply_keeps_notes_record_and_user_settings() {
    let t = tmp();
    let dir = install_fixture(t.path());
    make_plan(&dir).unwrap().apply(SystemTime::now()).unwrap();
    for rel in ["notes/keep.md", "record/keep.jsonl"] {
        assert!(exists(&dir.join(rel)), "{rel} was removed");
    }
    assert_eq!(
        fs::read_to_string(dir.join(CLAUDE_MD)).unwrap(),
        "# my project\nkeep me\n"
    );
    let s = fs::read_to_string(dir.join(SETTINGS_REL)).unwrap();
    for want in [r#""model": "opus""#, r#""X": "1""#, "my-own-hook.sh"] {
        assert!(s.contains(want), "settings lost {want:?}:\n{s}");
    }
    for gone in ["main-thread-guard.py", "ledger.py"] {
        assert!(!s.contains(gone), "package hook {gone} still in settings");
    }
}

#[test]
fn claude_md_without_block_untouched() {
    let t = tmp();
    write_test(&t.path().join(CLAUDE_MD), "plain user file\n");
    let p = make_plan(t.path()).unwrap();
    assert!(
        p.lines().is_empty(),
        "expected no targets, got {:?}",
        p.lines()
    );
}

#[test]
fn claude_md_only_block_is_deleted() {
    let t = tmp();
    let dir = t.path();
    write_test(
        &dir.join(CLAUDE_MD),
        &format!("\u{feff}{MARK_START}\r\npolicy\r\n{MARK_END}\r\n"),
    );
    let p = make_plan(dir).unwrap();
    assert!(
        p.md_delete,
        "expected CLAUDE.md to be deleted when only the block remains"
    );
    p.apply(SystemTime::now()).unwrap();
    assert!(!exists(&dir.join(CLAUDE_MD)), "CLAUDE.md still exists");
}

#[test]
fn broken_markers_fail_before_any_change() {
    let t = tmp();
    write_test(
        &t.path().join(CLAUDE_MD),
        &format!("{MARK_START}\nno end\n"),
    );
    let err = make_plan(t.path()).unwrap_err().to_string();
    assert!(err.contains("start marker without end marker"), "{err}");
}

/// Keeps PKG_FILES equal to the tracked .claude/{agents,hooks,skills} tree of this repo.
#[test]
fn manifest_matches_repo() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut got: Vec<String> = Vec::new();
    for sub in ["agents", "hooks", "skills"] {
        let base = root.join(".claude").join(sub);
        walk_repo(&root, &base, &mut got);
    }
    got.sort();
    let mut want: Vec<String> = PKG_FILES.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(got.join("\n"), want.join("\n"), "manifest drift");
}

fn walk_repo(root: &Path, dir: &Path, out: &mut Vec<String>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            if !p.to_string_lossy().contains("__pycache__") {
                walk_repo(root, &p, out);
            }
        } else {
            let rel = p.strip_prefix(root).unwrap();
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// What ensure_project_settings creates from the embedded copies, clean --yes removes (backup kept).
#[test]
fn guard_settings_removed_by_clean() {
    let t = tmp();
    let cwd = t.path().join("project");
    fs::create_dir_all(&cwd).unwrap();
    let home = t.path().join("home");
    fs::create_dir_all(&home).unwrap();
    assert!(ensure_project_settings(&cwd, Some(&home)).unwrap());
    let p = make_plan(&cwd).unwrap();
    assert!(
        p.kept().is_empty(),
        "embedded copies should match, kept: {:?}",
        p.kept()
    );
    p.apply(SystemTime::now()).unwrap();
    for rel in [
        SETTINGS_REL,
        ".claude/hooks/main-thread-guard.py",
        ".claude/hooks/_lean.py",
        ".claude",
    ] {
        assert!(!exists(&cwd.join(rel)), "still present after clean: {rel}");
    }
}

#[test]
fn stamp_is_utc_civil_time() {
    assert_eq!(stamp(fixed_now()), "20261101-093000");
    assert_eq!(stamp(UNIX_EPOCH), "19700101-000000");
}
