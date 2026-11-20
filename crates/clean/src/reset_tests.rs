use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

const BLOCK: &str = "<!-- claude-lean:start -->\npolicy\n<!-- claude-lean:end -->";

/// Minimal temp dir (no dev-dependency): unique path under the system temp dir, removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("chaude-reset-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Dir(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(p: &Path, body: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

fn exists(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok()
}

/// A folder as chaude leaves it: .claude/, the shipped script, notes/, record/, logs/, CLAUDE.md with the block.
fn installed(root: &Path) {
    write(&root.join(".claude/agents/haiku-coder.md"), b"agent");
    write(&root.join(".claude/hooks/_lean.py"), b"hook");
    write(&root.join(".claude/settings.json"), b"{}");
    write(&root.join(".claude/.chaude-version"), b"0.12.11");
    write(&root.join("scripts").join(SCRIPTS[0].0), SCRIPTS[0].1);
    write(&root.join("notes/a.md"), b"note");
    write(&root.join("record/r.jsonl"), b"{}");
    write(&root.join("logs/l.txt"), b"log");
    write(
        &root.join(CLAUDE_MD),
        format!("{BLOCK}\n\nmy project rules\n").as_bytes(),
    );
}

#[test]
fn dry_run_lists_and_changes_nothing() {
    let t = Dir::new();
    let (proj, cfg) = (t.path().join("proj"), t.path().join("cfg"));
    installed(&proj);
    let plan = make_plan(&proj, &cfg, &t.path().join("state"), false).unwrap();
    let lines = plan.lines();
    assert!(lines.contains(&"remove  .claude/settings.json".to_string()));
    assert!(lines.contains(&"remove  scripts/squeeze-run.py".to_string()));
    assert!(lines.iter().any(|l| l.starts_with("policy  CLAUDE.md")));
    assert!(plan.keep().iter().any(|k| k.starts_with("notes/ (kept")));
    assert!(exists(&proj.join(".claude/settings.json")));
    assert!(exists(&proj.join("scripts/squeeze-run.py")));
    assert!(
        fs::read_to_string(proj.join(CLAUDE_MD))
            .unwrap()
            .contains("claude-lean:start")
    );
}

#[test]
fn apply_removes_claude_and_shipped_script_keeps_notes() {
    let t = Dir::new();
    let (proj, cfg) = (t.path().join("proj"), t.path().join("cfg"));
    installed(&proj);
    let plan = make_plan(&proj, &cfg, &t.path().join("state"), false).unwrap();
    plan.apply().unwrap();
    assert!(!exists(&proj.join(".claude")));
    assert!(!exists(&proj.join("scripts")), "empty scripts/ is removed");
    assert!(exists(&proj.join("notes/a.md")));
    assert!(exists(&proj.join("record/r.jsonl")));
    assert!(exists(&proj.join("logs/l.txt")));
    let md = fs::read_to_string(proj.join(CLAUDE_MD)).unwrap();
    assert!(!md.contains("claude-lean"), "block removed");
    assert!(md.contains("my project rules"), "project content kept");
}

#[test]
fn with_notes_removes_notes_record_and_logs() {
    let t = Dir::new();
    let (proj, cfg) = (t.path().join("proj"), t.path().join("cfg"));
    installed(&proj);
    make_plan(&proj, &cfg, &t.path().join("state"), true)
        .unwrap()
        .apply()
        .unwrap();
    assert!(!exists(&proj.join("notes")));
    assert!(!exists(&proj.join("record")));
    assert!(!exists(&proj.join("logs")));
    assert!(!exists(&proj.join(".claude")));
}

#[test]
fn modified_script_and_its_folder_are_kept() {
    let t = Dir::new();
    let (proj, cfg) = (t.path().join("proj"), t.path().join("cfg"));
    write(&proj.join("scripts/squeeze-run.py"), b"my own edit");
    let plan = make_plan(&proj, &cfg, &t.path().join("state"), false).unwrap();
    assert!(
        plan.keep()
            .iter()
            .any(|k| k.starts_with("scripts/squeeze-run.py"))
    );
    plan.apply().unwrap();
    assert!(exists(&proj.join("scripts/squeeze-run.py")));
}

#[test]
fn claude_md_only_block_is_deleted_and_plain_one_untouched() {
    let t = Dir::new();
    let (proj, cfg) = (t.path().join("proj"), t.path().join("cfg"));
    write(&proj.join(CLAUDE_MD), format!("{BLOCK}\n").as_bytes());
    make_plan(&proj, &cfg, &t.path().join("state"), false)
        .unwrap()
        .apply()
        .unwrap();
    assert!(!exists(&proj.join(CLAUDE_MD)));

    let other = t.path().join("other");
    write(&other.join(CLAUDE_MD), b"no block here\n");
    make_plan(&other, &cfg, &t.path().join("state"), false)
        .unwrap()
        .apply()
        .unwrap();
    assert_eq!(fs::read(other.join(CLAUDE_MD)).unwrap(), b"no block here\n");
}

#[test]
fn broken_markers_fail_before_any_change() {
    let t = Dir::new();
    let (proj, cfg) = (t.path().join("proj"), t.path().join("cfg"));
    installed(&proj);
    write(
        &proj.join(CLAUDE_MD),
        b"<!-- claude-lean:start -->\nno end\n",
    );
    assert!(make_plan(&proj, &cfg, &t.path().join("state"), false).is_err());
    assert!(exists(&proj.join(".claude/settings.json")));
}

#[test]
fn refuses_home_and_root() {
    let t = Dir::new();
    let home = t.path().join("home");
    fs::create_dir_all(&home).unwrap();
    assert!(refusal(&home, Some(&home)).is_some());
    assert!(refusal(Path::new("/"), None).is_some());
    let proj = t.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    assert!(refusal(&proj, Some(&home)).is_none());
}

#[test]
fn answer_needs_yes() {
    assert!(answer_is_yes("y\n"));
    assert!(answer_is_yes("YES"));
    assert!(!answer_is_yes(""));
    assert!(!answer_is_yes("n"));
}

#[cfg(unix)]
#[test]
fn links_are_removed_not_followed() {
    let t = Dir::new();
    let (proj, cfg) = (t.path().join("proj"), t.path().join("cfg"));
    let outside = t.path().join("outside");
    write(&outside.join("keep.txt"), b"outside");
    write(&proj.join("notes/x.md"), b"x");
    // A link inside .claude pointing out, and .claude/ itself reached through a link elsewhere.
    fs::create_dir_all(proj.join(".claude")).unwrap();
    std::os::unix::fs::symlink(&outside, proj.join(".claude/out")).unwrap();
    make_plan(&proj, &cfg, &t.path().join("state"), false)
        .unwrap()
        .apply()
        .unwrap();
    assert!(!exists(&proj.join(".claude")));
    assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"outside");

    let proj2 = t.path().join("proj2");
    fs::create_dir_all(&proj2).unwrap();
    let ext = t.path().join("ext");
    write(&ext.join("keep.txt"), b"ext");
    std::os::unix::fs::symlink(&ext, proj2.join(".claude")).unwrap();
    make_plan(&proj2, &cfg, &t.path().join("state"), false)
        .unwrap()
        .apply()
        .unwrap();
    assert!(!exists(&proj2.join(".claude")));
    assert_eq!(fs::read(ext.join("keep.txt")).unwrap(), b"ext");
}

#[test]
fn forgets_only_this_folder_in_recent() {
    let t = Dir::new();
    let (proj, cfg) = (t.path().join("proj"), t.path().join("cfg"));
    fs::create_dir_all(&proj).unwrap();
    let mine = proj.display().to_string();
    let rows = format!(r#"[{{"path":"{mine}","used":2}},{{"path":"/elsewhere/other","used":1}}]"#);
    write(&cfg.join("recent.json"), rows.as_bytes());
    let plan = make_plan(&proj, &cfg, &t.path().join("state"), false).unwrap();
    assert!(plan.lines().iter().any(|l| l.starts_with("forget  ")));
    plan.apply().unwrap();
    let left = recent_folders::all_rows(&cfg.join("recent.json"));
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].path, "/elsewhere/other");
}

#[test]
fn nothing_installed_is_an_empty_plan() {
    let t = Dir::new();
    let proj = t.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    let plan = make_plan(&proj, &t.path().join("cfg"), &t.path().join("state"), false).unwrap();
    assert!(plan.is_empty());
}

#[test]
fn local_chaude_dir_is_removed() {
    let t = Dir::new();
    let (proj, cfg, state) = (
        t.path().join("proj"),
        t.path().join("cfg"),
        t.path().join("state"),
    );
    write(&proj.join(".chaude/history.jsonl"), b"{}");
    let plan = make_plan(&proj, &cfg, &state, false).unwrap();
    assert!(
        plan.lines()
            .iter()
            .any(|l| l == "remove  .chaude/ (whole dir)")
    );
    plan.apply().unwrap();
    assert!(!exists(&proj.join(".chaude")));
}

#[cfg(unix)]
#[test]
fn local_chaude_link_is_removed_not_followed() {
    let t = Dir::new();
    let (proj, cfg, state) = (
        t.path().join("proj"),
        t.path().join("cfg"),
        t.path().join("state"),
    );
    let outside = t.path().join("outside");
    write(&outside.join("keep.txt"), b"keep");
    fs::create_dir_all(&proj).unwrap();
    std::os::unix::fs::symlink(&outside, proj.join(".chaude")).unwrap();
    make_plan(&proj, &cfg, &state, false)
        .unwrap()
        .apply()
        .unwrap();
    assert!(!exists(&proj.join(".chaude")));
    assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"keep");
}

#[test]
fn clients_rows_of_this_folder_removed_others_byte_identical() {
    let t = Dir::new();
    let (proj, cfg, state) = (
        t.path().join("proj"),
        t.path().join("cfg"),
        t.path().join("state"),
    );
    fs::create_dir_all(&proj).unwrap();
    let other = "{\n  \"cwd\": \"/somewhere/else\",\n  \"model\": \"b\"\n }";
    let text = format!(
        "[\n {{\n  \"cwd\": \"{}\",\n  \"model\": \"a\"\n }},\n {other}\n]",
        proj.display()
    );
    write(&state.join("clients.json"), text.as_bytes());
    write(&state.join("agents.json"), b"[]");
    let plan = make_plan(&proj, &cfg, &state, false).unwrap();
    assert!(
        plan.lines()
            .iter()
            .any(|l| l.starts_with("entry in ") && l.ends_with("1 row(s) for this folder"))
    );
    assert!(plan.keep().iter().any(|k| k.contains("agents.json")));
    plan.apply().unwrap();
    assert_eq!(
        fs::read_to_string(state.join("clients.json")).unwrap(),
        format!("[\n {other}\n]")
    );
    assert_eq!(fs::read(state.join("agents.json")).unwrap(), b"[]");
}

#[test]
fn broken_clients_json_fails_before_any_change() {
    let t = Dir::new();
    let (proj, cfg, state) = (
        t.path().join("proj"),
        t.path().join("cfg"),
        t.path().join("state"),
    );
    write(&proj.join(".chaude/x"), b"x");
    write(&state.join("clients.json"), b"{not json");
    assert!(make_plan(&proj, &cfg, &state, false).is_err());
    assert!(exists(&proj.join(".chaude/x")));
    assert_eq!(fs::read(state.join("clients.json")).unwrap(), b"{not json");
}
