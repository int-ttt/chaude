//! Guard hook for a project that has no .claude/settings.json (Go: internal/settings/project.go).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::windows;
use crate::osenv;
use serde_json::{Map, Value};

/// Project settings written when a folder has no .claude/settings.json: the main-thread guard only
/// (same matcher and command form as the repo's .claude/settings.json; posix form, windows converted by `windows`).
const GUARD_HOOKS: &str = r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Bash|PowerShell|Grep|Glob|WebSearch|WebFetch",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\"",
            "timeout": 5
          }
        ]
      },
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/bash-py-guard.py\"",
            "timeout": 5
          }
        ]
      },
      {
        "matcher": "Agent|Task",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\"",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
"#;

/// Hook scripts the guard needs (main-thread-guard.py imports _lean.py; bash-py-guard.py is standalone).
const GUARD_FILES: &[&str] = &["main-thread-guard.py", "bash-py-guard.py", "_lean.py"];

/// Scripts the merge looks for to tell that a PreToolUse group already runs a guard.
const GUARD_SCRIPTS: &[&str] = &["main-thread-guard.py", "bash-py-guard.py"];

/// Package copies of the repo's .claude/agents definitions (haiku-*.md and opus-architect.md). A first-opened
/// folder gets them, so the main model can delegate to named agents. embedded_agents_match_repo keeps them equal.
pub const AGENTS: &[(&str, &[u8])] = &[
    (
        "haiku-batch-worker.md",
        include_bytes!("agents/haiku-batch-worker.md"),
    ),
    (
        "haiku-checker.md",
        include_bytes!("agents/haiku-checker.md"),
    ),
    ("haiku-coder.md", include_bytes!("agents/haiku-coder.md")),
    (
        "haiku-drafter.md",
        include_bytes!("agents/haiku-drafter.md"),
    ),
    (
        "haiku-extractor.md",
        include_bytes!("agents/haiku-extractor.md"),
    ),
    (
        "haiku-prompter.md",
        include_bytes!("agents/haiku-prompter.md"),
    ),
    (
        "haiku-researcher.md",
        include_bytes!("agents/haiku-researcher.md"),
    ),
    ("haiku-runner.md", include_bytes!("agents/haiku-runner.md")),
    ("haiku-scout.md", include_bytes!("agents/haiku-scout.md")),
    (
        "haiku-test-lead.md",
        include_bytes!("agents/haiku-test-lead.md"),
    ),
    ("haiku-tui.md", include_bytes!("agents/haiku-tui.md")),
    (
        "opus-architect.md",
        include_bytes!("agents/opus-architect.md"),
    ),
    (
        "sonnet-writer.md",
        include_bytes!("agents/sonnet-writer.md"),
    ),
];

/// Package copies of the repo's .claude/hooks scripts. The binary carries them, so an installed chaude needs no
/// files next to it. embedded_hooks_match_repo keeps them equal to the repo copies.
pub const HOOKS: &[(&str, &[u8])] = &[
    ("_lean.py", include_bytes!("hooks/_lean.py")),
    (
        "agent-model-pin.py",
        include_bytes!("hooks/agent-model-pin.py"),
    ),
    (
        "caveman-format.py",
        include_bytes!("hooks/caveman-format.py"),
    ),
    ("bash-py-guard.py", include_bytes!("hooks/bash-py-guard.py")),
    ("context-alert.py", include_bytes!("hooks/context-alert.py")),
    (
        "correction-guard.py",
        include_bytes!("hooks/correction-guard.py"),
    ),
    (
        "haiku-context-guard.py",
        include_bytes!("hooks/haiku-context-guard.py"),
    ),
    (
        "history-export.py",
        include_bytes!("hooks/history-export.py"),
    ),
    ("ledger.py", include_bytes!("hooks/ledger.py")),
    (
        "main-thread-guard.py",
        include_bytes!("hooks/main-thread-guard.py"),
    ),
    (
        "session-context.py",
        include_bytes!("hooks/session-context.py"),
    ),
    ("style-adapter.py", include_bytes!("hooks/style-adapter.py")),
];

/// Package copy of a hook script (file name only, e.g. "ledger.py").
pub fn embedded_hook(name: &str) -> Option<&'static [u8]> {
    HOOKS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, body)| *body)
}

/// Makes a project runnable outside this repo: writes the agent definitions into `<cwd>/.claude/agents` and the guard
/// hook scripts into `<cwd>/.claude/hooks` (existing files are kept, never overwritten), then creates
/// `<cwd>/.claude/settings.json` or merges the guard into an existing one (user keys and hooks kept, guard groups
/// appended only when missing, so a second run changes nothing). cwd == home is skipped (that is the user-global
/// .claude). Returns true when settings.json was created.
pub fn ensure_project_settings(cwd: &Path, home: Option<&Path>) -> io::Result<bool> {
    if cwd.as_os_str().is_empty() {
        return Ok(false);
    }
    if home.is_some_and(|h| same_dir(cwd, h)) {
        return Ok(false);
    }
    let claude = cwd.join(".claude");
    for (name, body) in AGENTS {
        copy_new(body, &claude.join("agents").join(name))?;
    }
    for name in GUARD_FILES {
        let body = embedded_hook(name)
            .ok_or_else(|| io::Error::other(format!("chaude: embedded hook missing: {name}")))?;
        copy_new(body, &claude.join("hooks").join(name))?;
    }
    let text = guard_settings_text()?;
    let dst = claude.join("settings.json");
    match fs::read(&dst) {
        Ok(old) => {
            if let Some(merged) = merge_guard(&old, &text).map_err(io::Error::other)? {
                fs::write(&dst, merged)?;
            }
            Ok(false)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            write_new(&dst, text.as_bytes())?;
            Ok(true)
        }
        Err(e) => Err(e),
    }
}

/// Name of the version record in chaude's state dir: the version the last startup ran with.
const VERSION_FILE: &str = "version";

/// The guard settings text for this platform: the posix form, or the windows form with `hook_python`.
pub fn guard_settings_text() -> io::Result<String> {
    if osenv::is_windows() {
        windows(GUARD_HOOKS.as_bytes(), hook_python()).map_err(io::Error::other)
    } else {
        Ok(GUARD_HOOKS.to_string())
    }
}

/// Startup version check. When the record in `state` is not `current` (or there is none), the chaude guard groups in
/// `<cwd>/.claude/settings.json` are replaced with `guard` (see `reset_guard_settings`), the shipped agent and hook
/// files are overwritten with their embedded copies where they differ (see `overwrite_shipped`), and `current` is
/// recorded. The record is written only after a successful reset, so a failed reset runs again on the next start.
/// cwd == home is not reset (that is the user-global .claude), but the version is still recorded. Returns true when a
/// reset ran. Update state: none is persisted (the update check always fetches live), so there is nothing to clear.
pub fn refresh_on_version_change(
    state: &Path,
    cwd: &Path,
    home: Option<&Path>,
    current: &str,
    guard: &str,
) -> io::Result<bool> {
    let record = state.join(VERSION_FILE);
    if fs::read_to_string(&record).is_ok_and(|v| v.trim() == current) {
        return Ok(false);
    }
    let project = !cwd.as_os_str().is_empty() && !home.is_some_and(|h| same_dir(cwd, h));
    if project {
        let claude = cwd.join(".claude");
        reset_guard_settings(&claude.join("settings.json"), guard)?;
        overwrite_shipped(&claude)?;
    }
    fs::create_dir_all(state)?;
    fs::write(&record, format!("{current}\n"))?;
    Ok(project)
}

/// Overwrites each shipped file under `claude` (the AGENTS and GUARD_FILES set that `ensure_project_settings` copies)
/// whose content differs from the embedded copy. Files outside that set are never touched.
fn overwrite_shipped(claude: &Path) -> io::Result<()> {
    for (name, body) in AGENTS {
        overwrite_if_differs(body, &claude.join("agents").join(name))?;
    }
    for name in GUARD_FILES {
        let body = embedded_hook(name)
            .ok_or_else(|| io::Error::other(format!("chaude: embedded hook missing: {name}")))?;
        overwrite_if_differs(body, &claude.join("hooks").join(name))?;
    }
    Ok(())
}

/// Writes `body` to `dst` (creating its folder) unless `dst` already holds exactly `body`.
fn overwrite_if_differs(body: &[u8], dst: &Path) -> io::Result<()> {
    if fs::read(dst).is_ok_and(|old| old == body) {
        return Ok(());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dst, body)
}

/// Replaces the chaude guard groups in the settings file `dst` with those of `guard`. A group is chaude's when a hook
/// in it runs a GUARD_SCRIPTS script (any matcher). The old ones are removed and the fresh ones go where the first
/// old one was (or at the end). Every other key and group is kept. A missing file is written as `guard`. Returns true
/// when the file changed.
fn reset_guard_settings(dst: &Path, guard: &str) -> io::Result<bool> {
    let old = match fs::read(dst) {
        Ok(old) => old,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(dst, guard)?;
            return Ok(true);
        }
        Err(e) => return Err(e),
    };
    let mut cfg: Value = serde_json::from_slice(&old).map_err(io::Error::other)?;
    let guard_cfg: Value = serde_json::from_str(guard).map_err(io::Error::other)?;
    let before = cfg.clone();
    let obj = cfg
        .as_object_mut()
        .ok_or_else(|| io::Error::other("settings.json: top level is not an object"))?;
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| io::Error::other("settings.json: hooks is not an object"))?;
    let pre = hooks
        .entry("PreToolUse")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| io::Error::other("settings.json: hooks.PreToolUse is not an array"))?;
    let at = pre.iter().position(is_guard_group).unwrap_or(pre.len());
    pre.retain(|g| !is_guard_group(g));
    for (i, group) in guard_cfg["hooks"]["PreToolUse"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        pre.insert(at + i, group.clone());
    }
    if cfg == before {
        return Ok(false);
    }
    let mut text = serde_json::to_string_pretty(&cfg).map_err(io::Error::other)?;
    text.push('\n');
    fs::write(dst, text)?;
    Ok(true)
}

/// True when a PreToolUse group runs one of the chaude guard scripts, whatever its matcher.
fn is_guard_group(group: &Value) -> bool {
    let text: String = group["hooks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(hook_text)
        .collect();
    GUARD_SCRIPTS.iter().any(|s| text.contains(s))
}

/// Hook interpreter for the windows guard command: the py launcher (`py -3`) when on PATH, else `python`.
/// python3 is never used: on windows the python3 PATH entry is the Microsoft Store alias stub, which exits without
/// running the hook, so the guard fails open.
pub fn hook_python() -> &'static str {
    hook_python_in(&osenv::path_dirs())
}

/// `hook_python` over the given PATH directories.
pub fn hook_python_in(dirs: &[PathBuf]) -> &'static str {
    for (exe, cmd) in [("py", "py -3"), ("python", "python")] {
        if osenv::find_on_path(exe, dirs).is_some() {
            return cmd;
        }
    }
    "python"
}

/// Adds the guard PreToolUse groups from `guard` to an existing settings body. Keys and order of `old` are kept; a
/// group is added only when no group with the same matcher already runs the guard script. None when nothing was
/// added. Text that is not a JSON object (or whose hooks/PreToolUse have the wrong shape) is an error.
fn merge_guard(old: &[u8], guard: &str) -> Result<Option<String>, String> {
    let mut cfg: Value = serde_json::from_slice(old).map_err(|e| e.to_string())?;
    let guard_cfg: Value = serde_json::from_str(guard).map_err(|e| e.to_string())?;
    let obj = cfg
        .as_object_mut()
        .ok_or("settings.json: top level is not an object")?;
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or("settings.json: hooks is not an object")?;
    let pre = hooks
        .entry("PreToolUse")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or("settings.json: hooks.PreToolUse is not an array")?;
    let mut changed = false;
    for group in guard_cfg["hooks"]["PreToolUse"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let matcher = group["matcher"].as_str().unwrap_or_default();
        let script = guard_script(group);
        if pre.iter().any(|g| runs_guard(g, matcher, script)) {
            continue;
        }
        pre.push(group.clone());
        changed = true;
    }
    if !changed {
        return Ok(None);
    }
    let mut text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    text.push('\n');
    Ok(Some(text))
}

/// The guard script a generated PreToolUse group runs (main-thread-guard.py when none is named).
fn guard_script(group: &Value) -> &'static str {
    let text: String = group["hooks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(hook_text)
        .collect();
    GUARD_SCRIPTS
        .iter()
        .copied()
        .find(|s| text.contains(s))
        .unwrap_or(GUARD_SCRIPTS[0])
}

/// True when a PreToolUse group with this matcher already runs the guard script.
fn runs_guard(group: &Value, matcher: &str, script: &str) -> bool {
    group["matcher"].as_str() == Some(matcher)
        && group["hooks"]
            .as_array()
            .is_some_and(|hs| hs.iter().any(|h| hook_text(h).contains(script)))
}

/// Command line of a hook entry: the command, plus the args of the windows exec form.
fn hook_text(h: &Value) -> String {
    let mut s = h["command"].as_str().unwrap_or_default().to_string();
    for a in h["args"].as_array().into_iter().flatten() {
        s.push(' ');
        s.push_str(a.as_str().unwrap_or_default());
    }
    s
}

/// True when both paths name the same existing directory.
fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Writes `body` to `dst` unless `dst` already exists.
fn copy_new(body: &[u8], dst: &Path) -> io::Result<()> {
    if fs::symlink_metadata(dst).is_ok() {
        return Ok(());
    }
    write_new(dst, body)
}

/// Creates `dst` (and its folder) without overwriting an existing file.
fn write_new(dst: &Path, body: &[u8]) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(dst)?;
    file.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn creates_guard() {
        let root = TempDir::new("guard-create");
        let cwd = root.path().join("cwd");
        let home = root.path().join("home");
        fs::create_dir_all(&cwd).expect("mkdir cwd");
        fs::create_dir_all(&home).expect("mkdir home");
        assert!(ensure_project_settings(&cwd, Some(&home)).expect("ensure"));

        let settings =
            fs::read_to_string(cwd.join(".claude").join("settings.json")).expect("read settings");
        assert!(
            settings.contains("\"PreToolUse\""),
            "guard hook missing:\n{settings}"
        );
        if !osenv::is_windows() {
            assert!(
                settings.contains(
                    r#"python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\""#
                ),
                "guard command missing:\n{settings}"
            );
        }
        for name in GUARD_FILES {
            let got = fs::read(cwd.join(".claude").join("hooks").join(name)).expect("hook written");
            assert_eq!(
                got.as_slice(),
                embedded_hook(name).expect("embedded"),
                "hook {name} differs"
            );
        }
    }

    #[test]
    fn creates_claude_folders_when_missing() {
        let root = TempDir::new("guard-folders");
        let cwd = root.path().join("fresh");
        fs::create_dir_all(&cwd).expect("mkdir cwd");
        assert!(ensure_project_settings(&cwd, None).expect("ensure"));
        for d in [cwd.join(".claude"), cwd.join(".claude").join("hooks")] {
            assert!(d.is_dir(), "folder not created: {}", d.display());
        }
    }

    #[test]
    fn embedded_hooks_match_repo() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.claude/hooks");
        if !repo.is_dir() {
            eprintln!("skip: repo .claude/hooks not reachable from here");
            return;
        }
        for (name, body) in HOOKS {
            let repo_body = fs::read(repo.join(name))
                .unwrap_or_else(|e| panic!("{name}: repo copy missing: {e}"));
            assert_eq!(
                *body,
                repo_body.as_slice(),
                "{name}: embedded copy differs from .claude/hooks"
            );
        }
    }

    #[test]
    fn merges_existing_settings() {
        let root = TempDir::new("guard-merge");
        let cwd = root.path().join("cwd");
        let home = root.path().join("home");
        let existing = cwd.join(".claude").join("settings.json");
        fs::create_dir_all(existing.parent().expect("parent")).expect("mkdir");
        fs::create_dir_all(&home).expect("mkdir home");
        const USER: &str = r#"{"model":"sonnet","hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo mine"}]}]}}"#;
        fs::write(&existing, USER).expect("write user settings");

        assert!(!ensure_project_settings(&cwd, Some(&home)).expect("ensure"));
        let first = fs::read_to_string(&existing).expect("read");
        assert!(
            first.contains(r#""model": "sonnet""#),
            "user key lost:\n{first}"
        );
        assert!(first.contains("echo mine"), "user hook lost:\n{first}");
        assert!(
            first.contains("Agent|Task"),
            "Agent|Task guard missing:\n{first}"
        );
        assert!(
            first.contains("main-thread-guard.py"),
            "guard missing:\n{first}"
        );
        assert!(
            !ensure_project_settings(&cwd, Some(&home)).expect("ensure"),
            "second run should not create"
        );
        assert_eq!(
            fs::read_to_string(&existing).expect("read"),
            first,
            "second run changed settings"
        );
    }

    #[test]
    fn rejects_non_object_settings() {
        let root = TempDir::new("guard-bad");
        let cwd = root.path().join("cwd");
        let existing = cwd.join(".claude").join("settings.json");
        fs::create_dir_all(existing.parent().expect("parent")).expect("mkdir");
        fs::write(&existing, "[1, 2]").expect("write");
        assert!(ensure_project_settings(&cwd, None).is_err());
        assert_eq!(fs::read_to_string(&existing).expect("read"), "[1, 2]");
    }

    #[test]
    fn agents_written_and_existing_kept() {
        let root = TempDir::new("guard-agents");
        let cwd = root.path().join("cwd");
        fs::create_dir_all(&cwd).expect("mkdir cwd");
        let mine = cwd.join(".claude").join("agents").join("haiku-coder.md");
        fs::create_dir_all(mine.parent().expect("parent")).expect("mkdir");
        fs::write(&mine, "mine").expect("write");
        assert!(ensure_project_settings(&cwd, None).expect("ensure"));
        assert_eq!(fs::read_to_string(&mine).expect("read"), "mine");
        let (_, body) = AGENTS
            .iter()
            .find(|(n, _)| *n == "opus-architect.md")
            .expect("opus-architect embedded");
        let got = fs::read(cwd.join(".claude").join("agents").join("opus-architect.md"))
            .expect("opus-architect written");
        assert_eq!(got.as_slice(), *body);
    }

    #[test]
    fn embedded_agents_match_repo() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.claude/agents");
        if !repo.is_dir() {
            eprintln!("skip: repo .claude/agents not reachable from here");
            return;
        }
        for (name, body) in AGENTS {
            let repo_body = fs::read(repo.join(name))
                .unwrap_or_else(|e| panic!("{name}: repo copy missing: {e}"));
            assert_eq!(
                *body,
                repo_body.as_slice(),
                "{name}: embedded copy differs from .claude/agents"
            );
        }
    }

    #[test]
    fn skips_home() {
        let root = TempDir::new("guard-home");
        let home = root.path().join("home");
        fs::create_dir_all(&home).expect("mkdir home");
        assert!(!ensure_project_settings(&home, Some(&home)).expect("ensure"));
        assert!(!home.join(".claude").exists(), "home was written");
    }

    // Guard groups of a settings text, in order (any group that runs a chaude guard script).
    fn guard_groups(text: &str) -> Vec<Value> {
        let cfg: Value = serde_json::from_str(text).expect("json");
        cfg["hooks"]["PreToolUse"]
            .as_array()
            .expect("PreToolUse")
            .iter()
            .filter(|g| is_guard_group(g))
            .cloned()
            .collect()
    }

    // User-owned keys and groups plus stale chaude guard groups (an old matcher and an old command).
    const STALE_USER: &str = r#"{
  "model": "sonnet",
  "hooks": {
    "PreToolUse": [
      {"matcher": "Bash", "hooks": [{"type": "command", "command": "echo mine"}]},
      {"matcher": "Bash", "hooks": [{"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/bash-py-guard.py\" --old"}]},
      {"matcher": "Agent|Task", "hooks": [{"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\" --old"}]}
    ]
  }
}"#;

    #[test]
    fn same_version_changes_nothing() {
        let root = TempDir::new("reset-same");
        let cwd = root.path().join("cwd");
        let state = root.path().join("state");
        let settings = cwd.join(".claude").join("settings.json");
        fs::create_dir_all(settings.parent().expect("parent")).expect("mkdir");
        fs::create_dir_all(&state).expect("mkdir state");
        fs::write(&settings, STALE_USER).expect("write");
        fs::write(state.join(VERSION_FILE), "0.11.7\n").expect("record");

        assert!(
            !refresh_on_version_change(&state, &cwd, None, "0.11.7", GUARD_HOOKS).expect("refresh")
        );
        assert_eq!(fs::read_to_string(&settings).expect("read"), STALE_USER);
    }

    #[test]
    fn new_version_replaces_guard_and_keeps_user() {
        let root = TempDir::new("reset-new");
        let cwd = root.path().join("cwd");
        let state = root.path().join("state");
        let settings = cwd.join(".claude").join("settings.json");
        fs::create_dir_all(settings.parent().expect("parent")).expect("mkdir");
        fs::create_dir_all(&state).expect("mkdir state");
        fs::write(&settings, STALE_USER).expect("write");
        fs::write(state.join(VERSION_FILE), "0.11.6\n").expect("record");

        assert!(
            refresh_on_version_change(&state, &cwd, None, "0.11.7", GUARD_HOOKS).expect("refresh")
        );
        let text = fs::read_to_string(&settings).expect("read");
        let cfg: Value = serde_json::from_str(&text).expect("json");
        assert_eq!(cfg["model"], "sonnet", "user key lost:\n{text}");
        assert!(text.contains("echo mine"), "user hook lost:\n{text}");
        assert!(!text.contains("--old"), "stale guard kept:\n{text}");
        assert_eq!(
            guard_groups(&text),
            guard_groups(GUARD_HOOKS),
            "guard not regenerated:\n{text}"
        );
        let pre = cfg["hooks"]["PreToolUse"].as_array().expect("PreToolUse");
        assert_eq!(
            pre[0]["hooks"][0]["command"], "echo mine",
            "user group moved:\n{text}"
        );
        assert_eq!(
            fs::read_to_string(state.join(VERSION_FILE))
                .expect("record")
                .trim(),
            "0.11.7"
        );
    }

    #[test]
    fn no_record_counts_as_changed() {
        let root = TempDir::new("reset-none");
        let cwd = root.path().join("cwd");
        let state = root.path().join("state");
        let settings = cwd.join(".claude").join("settings.json");
        fs::create_dir_all(settings.parent().expect("parent")).expect("mkdir");
        fs::write(&settings, STALE_USER).expect("write");

        assert!(
            refresh_on_version_change(&state, &cwd, None, "0.11.7", GUARD_HOOKS).expect("refresh")
        );
        let text = fs::read_to_string(&settings).expect("read");
        assert!(!text.contains("--old"), "stale guard kept:\n{text}");
        assert_eq!(guard_groups(&text), guard_groups(GUARD_HOOKS));
        assert!(state.join(VERSION_FILE).is_file(), "version not recorded");
    }

    #[test]
    fn windows_python_pick_is_regenerated() {
        let root = TempDir::new("reset-win");
        let cwd = root.path().join("cwd");
        let state = root.path().join("state");
        let settings = cwd.join(".claude").join("settings.json");
        fs::create_dir_all(settings.parent().expect("parent")).expect("mkdir");
        fs::write(&settings, STALE_USER).expect("write");
        let win = windows(GUARD_HOOKS.as_bytes(), "py -3").expect("convert");

        assert!(refresh_on_version_change(&state, &cwd, None, "0.11.7", &win).expect("refresh"));
        let text = fs::read_to_string(&settings).expect("read");
        assert_eq!(
            guard_groups(&text),
            guard_groups(&win),
            "windows guard not written:\n{text}"
        );
        assert!(text.contains("\"-3\""), "py -3 pick missing:\n{text}");
    }

    #[test]
    fn home_is_recorded_but_not_reset() {
        let root = TempDir::new("reset-home");
        let home = root.path().join("home");
        let state = root.path().join("state");
        fs::create_dir_all(&home).expect("mkdir home");

        assert!(
            !refresh_on_version_change(&state, &home, Some(&home), "0.11.7", GUARD_HOOKS)
                .expect("refresh")
        );
        assert!(!home.join(".claude").exists(), "home was reset");
        assert!(state.join(VERSION_FILE).is_file(), "version not recorded");
    }

    #[test]
    fn stale_shipped_files_overwritten_on_version_change() {
        let root = TempDir::new("refresh-files");
        let cwd = root.path().join("cwd");
        let state = root.path().join("state");
        let claude = cwd.join(".claude");
        fs::create_dir_all(claude.join("hooks")).expect("mkdir hooks");
        fs::create_dir_all(claude.join("agents")).expect("mkdir agents");
        fs::create_dir_all(&state).expect("mkdir state");
        fs::write(
            claude.join("hooks").join("main-thread-guard.py"),
            "old hook",
        )
        .expect("write");
        fs::write(claude.join("agents").join("haiku-coder.md"), "old agent").expect("write");
        fs::write(state.join(VERSION_FILE), "0.11.6\n").expect("record");

        assert!(
            refresh_on_version_change(&state, &cwd, None, "0.11.7", GUARD_HOOKS).expect("refresh")
        );
        assert_eq!(
            fs::read(claude.join("hooks").join("main-thread-guard.py")).expect("read"),
            embedded_hook("main-thread-guard.py").expect("embedded"),
            "stale hook not overwritten"
        );
        let (_, agent) = AGENTS
            .iter()
            .find(|(n, _)| *n == "haiku-coder.md")
            .expect("haiku-coder embedded");
        assert_eq!(
            fs::read(claude.join("agents").join("haiku-coder.md")).expect("read"),
            *agent,
            "stale agent not overwritten"
        );
    }

    #[test]
    fn user_extra_files_kept_on_version_change() {
        let root = TempDir::new("refresh-extra");
        let cwd = root.path().join("cwd");
        let state = root.path().join("state");
        let claude = cwd.join(".claude");
        fs::create_dir_all(claude.join("hooks")).expect("mkdir hooks");
        fs::create_dir_all(claude.join("agents")).expect("mkdir agents");
        fs::create_dir_all(&state).expect("mkdir state");
        let my_hook = claude.join("hooks").join("my-hook.py");
        let my_agent = claude.join("agents").join("mine.md");
        fs::write(&my_hook, "mine").expect("write");
        fs::write(&my_agent, "mine").expect("write");
        fs::write(state.join(VERSION_FILE), "0.11.6\n").expect("record");

        assert!(
            refresh_on_version_change(&state, &cwd, None, "0.11.7", GUARD_HOOKS).expect("refresh")
        );
        assert_eq!(fs::read_to_string(&my_hook).expect("read"), "mine");
        assert_eq!(fs::read_to_string(&my_agent).expect("read"), "mine");
    }

    #[test]
    fn same_version_keeps_modified_shipped_file() {
        let root = TempDir::new("refresh-same");
        let cwd = root.path().join("cwd");
        let state = root.path().join("state");
        let agent = cwd.join(".claude").join("agents").join("haiku-coder.md");
        fs::create_dir_all(agent.parent().expect("parent")).expect("mkdir");
        fs::create_dir_all(&state).expect("mkdir state");
        fs::write(&agent, "edited").expect("write");
        fs::write(state.join(VERSION_FILE), "0.11.7\n").expect("record");

        assert!(
            !refresh_on_version_change(&state, &cwd, None, "0.11.7", GUARD_HOOKS).expect("refresh")
        );
        assert_eq!(fs::read_to_string(&agent).expect("read"), "edited");
    }

    #[test]
    fn home_shipped_files_not_refreshed() {
        let root = TempDir::new("refresh-home-files");
        let home = root.path().join("home");
        let state = root.path().join("state");
        let agent = home.join(".claude").join("agents").join("haiku-coder.md");
        fs::create_dir_all(agent.parent().expect("parent")).expect("mkdir");

        fs::write(&agent, "old agent").expect("write");
        assert!(
            !refresh_on_version_change(&state, &home, Some(&home), "0.11.7", GUARD_HOOKS)
                .expect("refresh")
        );
        assert_eq!(fs::read_to_string(&agent).expect("read"), "old agent");
    }

    #[test]
    fn hook_python_order() {
        let cases: [(&[&str], &str); 5] = [
            (&["python3", "python", "py"], "py -3"),
            (&["python3", "python"], "python"),
            (&["python3"], "python"),
            (&["py"], "py -3"),
            (&[], "python"),
        ];
        for (have, want) in cases {
            let root = TempDir::new("hook-python");
            for name in have {
                let p = root.path().join(name);
                fs::write(&p, "#!/bin/sh\n").expect("write fake interpreter");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod");
                }
            }
            let dirs = [root.path().to_path_buf()];
            assert_eq!(hook_python_in(&dirs), want, "have {have:?}");
        }
    }

    #[test]
    fn windows_guard_has_no_python3() {
        let root = TempDir::new("win-guard");
        let stub = root.path().join("python3");
        fs::write(&stub, "").expect("write python3 stub");
        let dirs = [root.path().to_path_buf()];
        let py = hook_python_in(&dirs);
        let text = windows(GUARD_HOOKS.as_bytes(), py).expect("convert");
        assert!(
            !text.contains("python3"),
            "python3 left in windows guard:\n{text}"
        );
        let v: Value = serde_json::from_str(&text).expect("valid json");
        let h = &v["hooks"]["PreToolUse"][0]["hooks"][0];
        assert_eq!(h["command"], "python", "only the python3 stub is on PATH");
    }

    /// Runs the embedded bash-py-guard.py on one Bash command; true when it denies.
    fn bash_guard_denies(hook: &Path, command: &str) -> bool {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let input = serde_json::json!({"tool_name": "Bash", "tool_input": {"command": command}})
            .to_string();
        let mut child = Command::new("python3")
            .arg("-I")
            .arg(hook)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 runs");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
        let out = child.wait_with_output().expect("hook exits");
        assert!(out.status.success(), "hook failed on {command:?}");
        let text = String::from_utf8_lossy(&out.stdout);
        if text.trim().is_empty() {
            return false;
        }
        assert!(
            text.contains(r#""permissionDecision": "deny""#)
                && (text.contains("Edit files with the Edit/Write tools, not Python.")
                    || text.contains("Unbounded wait loop denied")),
            "unexpected hook output for {command:?}: {text}"
        );
        true
    }

    #[test]
    fn bash_py_guard_samples() {
        let root = TempDir::new("bash-py-guard");
        let hook = root.path().join("bash-py-guard.py");
        fs::write(&hook, embedded_hook("bash-py-guard.py").expect("embedded")).expect("write hook");
        let deny = [
            "python3 -I - <<'EOF'\nimport json\np = 'a.md'\ns = open(p).read()\nopen(p, 'w').write(s.replace('a', 'b'))\nEOF",
            "python3 -c \"from pathlib import Path; Path('a.md').write_text('x')\"",
            "python3 -c \"import os; os.replace('a.tmp', 'a.md')\"",
            "node -e \"const fs = require('fs'); fs.writeFileSync('a.md', 'x')\"",
            "cat <<'EOF' | python3 -\nopen('a.md', 'w').write('x')\nEOF",
            "until [ -s /tmp/x ]; do sleep 1; done",
            "while true; do sleep 1; done",
        ];
        let allow = [
            "python3 -c \"print(1)\"",
            "python3 -m pytest -q tests/",
            "python3 -I - <<'EOF'\nprint(open('a.json').read())\nEOF",
            "node -e \"console.log(1)\"",
            // heredoc bodies of other commands are data, not code
            "cat <<'EOF' > notes/draft.md\nopen('a.md', 'w').write('x')\nEOF\npython3 -c \"print(1)\"",
            "cat <<EOF > a.md\nPath(p).write_text('x')\nEOF",
            "perl -e 'alarm shift; exec @ARGV' 60 sh -c 'until [ -s /tmp/x ]; do sleep 1; done'",
            "for f in *.md; do echo $f; done",
            "while read l; do echo $l; done < file",
            "i=0; while [ $i -lt 10 ]; do i=$((i+1)); done",
        ];
        for cmd in deny {
            assert!(bash_guard_denies(&hook, cmd), "should deny: {cmd:?}");
        }
        for cmd in allow {
            assert!(!bash_guard_denies(&hook, cmd), "should allow: {cmd:?}");
        }
    }

    /// Runs the embedded main-thread-guard.py on one tool call (JSON text) and returns its stdout.
    fn main_guard_output(root: &Path, payload: &str) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let hooks = root.join(".claude").join("hooks");
        fs::create_dir_all(&hooks).expect("mkdir hooks");
        for name in ["main-thread-guard.py", "_lean.py"] {
            fs::write(hooks.join(name), embedded_hook(name).expect("embedded"))
                .expect("write hook");
        }
        let mut child = Command::new("python3")
            .arg("-I")
            .arg(hooks.join("main-thread-guard.py"))
            .env("CLAUDE_PROJECT_DIR", root)
            .env_remove("CLAUDE_LEAN")
            .env_remove("CLAUDE_ALLOW_MAIN_TOOLS")
            .env_remove("CHAUDE_ORCH")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 runs");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(payload.as_bytes())
            .expect("write stdin");
        let out = child.wait_with_output().expect("hook exits");
        assert!(out.status.success(), "hook failed on {payload}");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    #[test]
    fn main_guard_powershell_main_denied_subagent_allowed() {
        let root = TempDir::new("main-guard-powershell");
        let main = serde_json::json!({"tool_name": "PowerShell", "tool_input": {"command": "Get-ChildItem"}})
            .to_string();
        let out = main_guard_output(root.path(), &main);
        assert!(
            out.contains(r#""permissionDecision": "deny""#)
                && out.contains("Main session does not use PowerShell"),
            "main PowerShell not denied: {out}"
        );
        let sub = serde_json::json!({
            "tool_name": "PowerShell",
            "tool_input": {"command": "Get-ChildItem"},
            "agent_id": "a1",
            "agent_type": "haiku-coder"
        })
        .to_string();
        let out = main_guard_output(root.path(), &sub);
        assert!(
            out.trim().is_empty(),
            "subagent PowerShell not allowed: {out}"
        );
    }
}
