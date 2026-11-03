//! Removes the claude-lean harness from a project (Go: internal/clean). The reverse of what the old install step
//! wrote: the package files under .claude/ (agents, hooks, skills), the claude-lean marker block in CLAUDE.md and
//! the package hook entries in .claude/settings.json. Everything else (notes/, record/, user files, user settings
//! keys, user hooks) is left alone. Dry-run unless --yes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chaude_core::settings::project::embedded_hook;
use serde_json::{Map, Value};

/// Package copy lookup for a hook file name (tests replace it).
type HookLookup = fn(&str) -> Option<&'static [u8]>;

const MARK_START: &str = "<!-- claude-lean:start -->";
const MARK_END: &str = "<!-- claude-lean:end -->";
const BOM: &[u8] = b"\xef\xbb\xbf";
const BACKUP_PREFIX: &str = ".chaude-backup-";
const HOOKS_DIR: &str = ".claude/hooks/";
const CLAUDE_MD: &str = "CLAUDE.md";
const SETTINGS_REL: &str = ".claude/settings.json";

/// The manifest: every file the harness installed under .claude/ (slash paths, relative to the project).
/// tests::manifest_matches_repo keeps it equal to the repo's .claude/ tree (minus __pycache__).
const PKG_FILES: &[&str] = &[
    ".claude/agents/haiku-batch-worker.md",
    ".claude/agents/haiku-checker.md",
    ".claude/agents/haiku-coder.md",
    ".claude/agents/haiku-drafter.md",
    ".claude/agents/haiku-extractor.md",
    ".claude/agents/haiku-prompter.md",
    ".claude/agents/haiku-researcher.md",
    ".claude/agents/haiku-runner.md",
    ".claude/agents/haiku-scout.md",
    ".claude/agents/haiku-tui.md",
    ".claude/agents/opus-architect.md",
    ".claude/hooks/_lean.py",
    ".claude/hooks/agent-model-pin.py",
    ".claude/hooks/caveman-format.py",
    ".claude/hooks/context-alert.py",
    ".claude/hooks/correction-guard.py",
    ".claude/hooks/haiku-context-guard.py",
    ".claude/hooks/history-export.py",
    ".claude/hooks/ledger.py",
    ".claude/hooks/main-thread-guard.py",
    ".claude/hooks/session-context.py",
    ".claude/hooks/style-adapter.py",
    ".claude/skills/check/SKILL.md",
    ".claude/skills/correction/SKILL.md",
    ".claude/skills/distill/SKILL.md",
    ".claude/skills/fact/SKILL.md",
    ".claude/skills/migrate-session/SKILL.md",
    ".claude/skills/research/SKILL.md",
    ".claude/skills/status/SKILL.md",
];

const CLEAN_USAGE: &str = "usage: chaude clean [dir] [--yes]
  claude-lean 하네스를 프로젝트에서 제거한다. 기본은 dry-run(대상만 출력, 변경 없음).
  제거 대상: CLAUDE.md의 claude-lean 정책 블록(마커 사이만; 남는 내용이 없으면 파일 삭제),
            .claude/{agents,hooks,skills}의 설치 파일, .claude/settings.json의 훅 항목(다른 키·사용자 훅 유지)
  건드리지 않음: notes/, record/, 목록에 없는 파일
  --yes                  실제로 제거. 먼저 바뀔 파일을 전부 .chaude-backup-<YYYYMMDD-HHMMSS>/ 에 복사한다";

/// Plan is what clean would change in one project. Nothing is written until apply.
#[derive(Debug, Default)]
pub struct Plan {
    pub dir: PathBuf,
    /// Slash paths under .claude/ that exist and are in the manifest.
    remove_files: Vec<String>,
    /// Hook files left alone: not byte-identical to the package copy (or no package copy).
    kept: Vec<String>,
    /// CLAUDE.md contains the marker block.
    md_touched: bool,
    /// Nothing but the block was in CLAUDE.md: delete the file.
    md_delete: bool,
    /// CLAUDE.md content without the block (when !md_delete).
    md_new: Vec<u8>,
    /// settings.json had package hook entries.
    set_touched: bool,
    /// settings.json without them.
    set_new: Vec<u8>,
    /// Nothing but the package hooks was in settings.json: delete the file.
    set_delete: bool,
    /// Number of package hook entries removed.
    hook_removed: usize,
    /// Slash path -> original bytes of every touched file (backup source).
    orig: BTreeMap<String, Vec<u8>>,
}

/// A failed apply, with the backup folder that was being written.
#[derive(Debug)]
pub struct ApplyError {
    pub backup: PathBuf,
    pub source: io::Error,
}

/// Decides every change in dir. It fails (and changes nothing) on files it cannot interpret, e.g. a CLAUDE.md with
/// only a start marker or a settings.json that is not JSON.
pub fn make_plan(dir: &Path) -> io::Result<Plan> {
    plan_with(dir, embedded_hook)
}

fn plan_with(dir: &Path, hook: HookLookup) -> io::Result<Plan> {
    let mut p = Plan {
        dir: dir.to_path_buf(),
        ..Plan::default()
    };
    for &rel in PKG_FILES {
        let full = dir.join(rel);
        let Ok(st) = fs::metadata(&full) else {
            continue;
        };
        if st.is_dir() {
            continue;
        }
        let b = fs::read(&full)?;
        if let Some(name) = rel.strip_prefix(HOOKS_DIR)
            && !hook(name).is_some_and(|pkg| pkg == b.as_slice())
        {
            p.kept.push(rel.to_string());
            continue;
        }
        p.remove_files.push(rel.to_string());
        p.orig.insert(rel.to_string(), b);
    }
    p.plan_claude_md()?;
    p.plan_settings()?;
    Ok(p)
}

impl Plan {
    /// Hook files that were not removed (slash paths under .claude/hooks/).
    pub fn kept(&self) -> &[String] {
        &self.kept
    }

    /// Number of files that apply would copy into the backup folder.
    pub fn backup_count(&self) -> usize {
        self.orig.len()
    }

    /// One line per planned change.
    pub fn lines(&self) -> Vec<String> {
        let mut ls: Vec<String> = self
            .remove_files
            .iter()
            .map(|rel| format!("remove  {rel}"))
            .collect();
        if self.md_touched {
            ls.push(if self.md_delete {
                "delete  CLAUDE.md (only the claude-lean block was in it)".to_string()
            } else {
                "policy  CLAUDE.md: claude-lean block removed, project content kept".to_string()
            });
        }
        if self.set_touched {
            ls.push(if self.set_delete {
                "delete  .claude/settings.json (only the claude-lean hook entries were in it)"
                    .to_string()
            } else {
                format!(
                    "settings .claude/settings.json: {} claude-lean hook entries removed (other keys kept)",
                    self.hook_removed
                )
            });
        }
        ls
    }

    /// Backs up every touched file to <dir>/.chaude-backup-<stamp>/ (same relative paths), then applies the plan.
    /// Returns the backup folder (empty when there was nothing to do).
    pub fn apply(&self, now: SystemTime) -> Result<PathBuf, ApplyError> {
        if self.lines().is_empty() {
            return Ok(PathBuf::new());
        }
        let bdir = self.dir.join(format!("{BACKUP_PREFIX}{}", stamp(now)));
        match self.apply_to(&bdir) {
            Ok(()) => Ok(bdir),
            Err(source) => Err(ApplyError {
                backup: bdir,
                source,
            }),
        }
    }

    fn apply_to(&self, bdir: &Path) -> io::Result<()> {
        for (rel, b) in &self.orig {
            write_file(&bdir.join(rel), b)?;
        }
        if self.md_touched {
            let md = self.dir.join(CLAUDE_MD);
            if self.md_delete {
                fs::remove_file(&md)?;
            } else {
                write_file(&md, &self.md_new)?;
            }
        }
        if self.set_touched && !self.set_delete {
            write_file(&self.dir.join(SETTINGS_REL), &self.set_new)?;
        }
        for rel in &self.remove_files {
            fs::remove_file(self.dir.join(rel))?;
            prune_empty_dirs(&self.dir, parent_of(rel));
        }
        if self.set_touched && self.set_delete {
            fs::remove_file(self.dir.join(SETTINGS_REL))?;
            prune_empty_dirs(&self.dir, ".claude");
        }
        Ok(())
    }

    fn plan_claude_md(&mut self) -> io::Result<()> {
        let b = match fs::read(self.dir.join(CLAUDE_MD)) {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let out = strip_policy(&b).map_err(|why| {
            io::Error::other(format!(
                "CLAUDE.md: {why} (fix by hand first; nothing changed)"
            ))
        })?;
        let Some(out) = out else {
            return Ok(());
        };
        self.orig.insert(CLAUDE_MD.to_string(), b);
        self.md_touched = true;
        if is_blank(&out) {
            self.md_delete = true;
        } else {
            self.md_new = out;
        }
        Ok(())
    }

    fn plan_settings(&mut self) -> io::Result<()> {
        let b = match fs::read(self.dir.join(SETTINGS_REL)) {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let mut m = Map::new();
        if !b.trim_ascii().is_empty() {
            let why = |why: String| {
                io::Error::other(format!(
                    "{SETTINGS_REL} is not valid JSON ({why}); fix it by hand first (nothing changed)"
                ))
            };
            let first = serde_json::Deserializer::from_slice(&b)
                .into_iter::<Value>()
                .next();
            let v = match first {
                Some(Ok(v)) => v,
                Some(Err(e)) => return Err(why(e.to_string())),
                None => Value::Null,
            };
            m = match v {
                Value::Object(o) => o,
                Value::Null => Map::new(),
                _ => return Err(why("expected a JSON object".to_string())),
            };
        }
        let n = strip_hooks(&mut m, &self.settings_hook_names());
        if n == 0 {
            return Ok(());
        }
        self.set_touched = true;
        self.hook_removed = n;
        if m.is_empty() {
            self.set_delete = true;
            self.orig.insert(SETTINGS_REL.to_string(), b);
            return Ok(());
        }
        let mut text = serde_json::to_string_pretty(&Value::Object(m)).map_err(io::Error::other)?;
        text.push('\n');
        self.set_new = text.into_bytes();
        self.orig.insert(SETTINGS_REL.to_string(), b);
        Ok(())
    }

    /// Package hook names whose settings entries are removed; a kept (user-edited) hook keeps its entry.
    fn settings_hook_names(&self) -> BTreeSet<String> {
        let mut names = pkg_hook_names();
        for k in &self.kept {
            names.remove(base_of(k));
        }
        names
    }
}

/// Package hook file names (.claude/hooks/<name>) from the manifest.
fn pkg_hook_names() -> BTreeSet<String> {
    PKG_FILES
        .iter()
        .filter(|rel| rel.ends_with(".py"))
        .filter_map(|rel| rel.strip_prefix(HOOKS_DIR))
        .map(str::to_string)
        .collect()
}

/// Removes the claude-lean block (markers included). Ok(None) when there is no block.
/// install put the block first and separated it from the old content by one blank line, so that blank line is
/// dropped when the block was at the top; a UTF-8 BOM and CRLF endings are kept.
fn strip_policy(b: &[u8]) -> Result<Option<Vec<u8>>, &'static str> {
    let has_bom = b.starts_with(BOM);
    let body = if has_bom { &b[BOM.len()..] } else { b };
    let Some(i) = find(body, MARK_START.as_bytes()) else {
        return if find(body, MARK_END.as_bytes()).is_some() {
            Err("end marker without start marker")
        } else {
            Ok(None)
        };
    };
    let Some(k) = find(&body[i..], MARK_END.as_bytes()) else {
        return Err("start marker without end marker");
    };
    let j = i + k + MARK_END.len();
    let head = &body[..i];
    let mut tail = &body[j..];
    if is_blank(head) {
        let n = tail
            .iter()
            .take_while(|c| matches!(c, b'\r' | b'\n'))
            .count();
        tail = &tail[n..];
    }
    let mut res = Vec::with_capacity(body.len());
    res.extend_from_slice(head);
    res.extend_from_slice(tail);
    if is_blank(&res) {
        return Ok(Some(Vec::new()));
    }
    if has_bom {
        let mut with_bom = BOM.to_vec();
        with_bom.extend_from_slice(&res);
        return Ok(Some(with_bom));
    }
    Ok(Some(res))
}

/// Removes the hook entries that run a package hook from m["hooks"]. A group keeps its other entries; a group left
/// empty is dropped; an event left empty is dropped; "hooks" is dropped when it ends up empty.
/// Returns the number of entries removed.
fn strip_hooks(m: &mut Map<String, Value>, names: &BTreeSet<String>) -> usize {
    let (total, empty) = {
        let Some(Value::Object(hooks)) = m.get_mut("hooks") else {
            return 0;
        };
        let mut total = 0;
        let events: Vec<String> = hooks.keys().cloned().collect();
        for ev in events {
            let Some(Value::Array(groups)) = hooks.get_mut(&ev) else {
                continue;
            };
            let mut removed = 0;
            let mut keep = Vec::new();
            for g in std::mem::take(groups) {
                let Value::Object(mut gm) = g else {
                    keep.push(g);
                    continue;
                };
                let Some(inner) = gm.get("hooks").and_then(Value::as_array).cloned() else {
                    keep.push(Value::Object(gm));
                    continue;
                };
                let before = inner.len();
                let kept: Vec<Value> = inner
                    .into_iter()
                    .filter(|h| !is_pkg_hook(h, names))
                    .collect();
                removed += before - kept.len();
                if kept.len() == before {
                    keep.push(Value::Object(gm));
                } else if !kept.is_empty() {
                    gm.insert("hooks".to_string(), Value::Array(kept));
                    keep.push(Value::Object(gm));
                }
            }
            if removed == 0 {
                *groups = keep;
                continue;
            }
            total += removed;
            if keep.is_empty() {
                hooks.remove(&ev);
            } else {
                *groups = keep;
            }
        }
        (total, hooks.is_empty())
    };
    if total > 0 && empty {
        m.remove("hooks");
    }
    total
}

/// A hook entry whose command (shell form) or args (exec form, windows) run .claude/hooks/<pkg>.py.
fn is_pkg_hook(h: &Value, names: &BTreeSet<String>) -> bool {
    let Value::Object(hm) = h else {
        return false;
    };
    let mut strs = vec![str_of(hm.get("command"))];
    if let Some(Value::Array(args)) = hm.get("args") {
        strs.extend(args.iter().map(|a| str_of(Some(a))));
    }
    strs.iter().any(|s| {
        let n = hook_name(s);
        !n.is_empty() && names.contains(&n)
    })
}

/// The file name after ".claude/hooks/" in a command string ("" when absent). Accepts / and \ separators.
fn hook_name(s: &str) -> String {
    let s = s.replace('\\', "/");
    let Some(i) = s.find(HOOKS_DIR) else {
        return String::new();
    };
    let rest = &s[i + HOOKS_DIR.len()..];
    let end = rest.find(['"', '\'', ' ']).unwrap_or(rest.len());
    rest[..end].to_string()
}

fn str_of(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_string()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// True for an empty or whitespace-only text. Invalid UTF-8 is never blank.
fn is_blank(b: &[u8]) -> bool {
    std::str::from_utf8(b).is_ok_and(|s| s.trim().is_empty())
}

/// Slash-path parent, "." when there is none (path.Dir).
fn parent_of(rel: &str) -> &str {
    rel.rsplit_once('/').map_or(".", |(p, _)| p)
}

/// Slash-path last element (path.Base).
fn base_of(rel: &str) -> &str {
    rel.rsplit_once('/').map_or(rel, |(_, b)| b)
}

/// Removes rel and its parents while they are empty, up to and including .claude. remove_dir fails on non-empty
/// dirs, so user files are never deleted here.
fn prune_empty_dirs(root: &Path, rel: &str) {
    let mut rel = rel;
    while rel != "." && rel.starts_with(".claude") {
        let _ = fs::remove_dir(root.join(rel));
        rel = parent_of(rel);
    }
}

fn write_file(p: &Path, b: &[u8]) -> io::Result<()> {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(p, b)
}

/// Backup folder stamp, YYYYMMDD-HHMMSS in UTC (Go uses local time; no tz dependency here).
fn stamp(now: SystemTime) -> String {
    let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Proleptic Gregorian (year, month, day) for a day count since 1970-01-01.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// `clean [dir] [--yes]`. Returns the process exit code.
pub fn main(args: &[String]) -> i32 {
    let mut yes = false;
    let mut pos: Vec<String> = Vec::new();
    let mut only_pos = false;
    for arg in args {
        if only_pos || !arg.starts_with('-') || arg == "-" {
            pos.push(arg.clone());
            continue;
        }
        if arg == "--" {
            only_pos = true;
            continue;
        }
        let body = arg
            .strip_prefix("--")
            .or_else(|| arg.strip_prefix('-'))
            .unwrap_or(arg);
        let (name, value) = match body.split_once('=') {
            Some((n, v)) => (n, Some(v)),
            None => (body, None),
        };
        match name {
            "h" | "help" => {
                eprintln!("{CLEAN_USAGE}");
                return 0;
            }
            "yes" => match value.map_or(Some(true), parse_bool) {
                Some(v) => yes = v,
                None => {
                    eprintln!(
                        "invalid boolean value {:?} for -yes: parse error",
                        value.unwrap_or("")
                    );
                    eprintln!("{CLEAN_USAGE}");
                    return 2;
                }
            },
            _ => {
                eprintln!("flag provided but not defined: -{name}");
                eprintln!("{CLEAN_USAGE}");
                return 2;
            }
        }
    }
    if pos.len() > 1 {
        eprintln!("{CLEAN_USAGE}");
        return 2;
    }
    let dir = pos.first().map_or(".", String::as_str);
    let abs = match std::path::absolute(dir) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("clean: {e}");
            return 1;
        }
    };
    if !abs.is_dir() {
        eprintln!("not a folder: {}", abs.display());
        return 1;
    }
    let plan = match make_plan(&abs) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("clean: {e}");
            return 1;
        }
    };
    let lines = plan.lines();
    for k in plan.kept() {
        println!("keep    {k} (differs from the package copy, or no package copy to compare)");
    }
    if lines.is_empty() {
        println!(
            "nothing to remove: no claude-lean harness files in {}",
            abs.display()
        );
        return 0;
    }
    for l in &lines {
        println!("{l}");
    }
    if !yes {
        println!(
            "dry-run: {} targets listed, nothing changed. Run with --yes to remove (backup first to {BACKUP_PREFIX}<time>/)",
            lines.len()
        );
        return 0;
    }
    match plan.apply(SystemTime::now()) {
        Ok(bdir) => {
            let name = bdir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            println!(
                "backup  {name} ({} files)\ndone    {} targets applied",
                plan.backup_count(),
                lines.len()
            );
            0
        }
        Err(e) => {
            eprintln!("clean: {} (backup: {})", e.source, e.backup.display());
            1
        }
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Some(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
