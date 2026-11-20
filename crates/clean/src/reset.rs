//! `chaude reset`: a full reset of the current project folder. Removes what chaude put there: the whole .claude/ dir,
//! the shipped scripts/ files (only byte-identical copies; scripts/ too when it ends up empty), the claude-lean block
//! in CLAUDE.md (the file too when nothing else is in it) and this folder's row in ~/.chaude/recent.json. notes/,
//! record/ and logs/ are kept unless --with-notes. Links are removed, never followed. Dry-run unless --yes or the
//! answer to the prompt is yes.

use std::env;
use std::fs;
use std::io::{self, BufRead, ErrorKind, Write};
use std::path::{Path, PathBuf};

use chaude_core::file_lock::FileLock;
use chaude_core::osenv;
use chaude_core::recent_folders;
use chaude_core::settings::project::SCRIPTS;
use serde_json::value::RawValue;

use crate::{CLAUDE_MD, is_blank, strip_policy};

const RESET_USAGE: &str = "usage: chaude reset [--yes] [--dry-run] [--with-notes]
  Fully resets the current folder: removes what chaude installed here. Prints the exact list first, then asks.
  Removes: .claude/ and .chaude/ (whole dirs), the shipped scripts/ files (only identical copies; scripts/ when then empty),
           the claude-lean block in CLAUDE.md (the file when nothing else is in it),
           this folder's row in ~/.chaude/recent.json and in clients.json
  Keeps:   notes/, record/, logs/ (removed with --with-notes), agents.json (no per-folder key), other folders, the global install
  --yes          remove without asking
  --dry-run      only print the list
  Refuses to run in $HOME or in a filesystem root.";

const WITH_NOTES_DIRS: &[&str] = &["notes", "record", "logs"];

/// One planned change: the line shown to the user and what to do.
#[derive(Debug)]
enum Op {
    /// A file, a link or an empty dir.
    Remove(PathBuf),
    /// A real dir tree (std remove_dir_all removes links inside it, never follows them).
    RemoveTree(PathBuf),
    /// Replaces a file with new bytes (CLAUDE.md without the block).
    Write(PathBuf, Vec<u8>),
    /// Drops the rows of these folder names from a recent-folders file.
    Forget(PathBuf, Vec<String>),
    /// Drops the rows whose cwd is one of these folders from the global clients.json. Other rows stay byte-identical.
    Clients(PathBuf, Vec<String>),
}

#[derive(Debug)]
struct Step {
    line: String,
    op: Op,
}

/// What reset would change in one folder. Nothing is written until apply.
#[derive(Debug, Default)]
pub struct Plan {
    dir: PathBuf,
    steps: Vec<Step>,
    /// Paths left alone, with the reason (shown before the list).
    keep: Vec<String>,
}

/// Plans the reset of dir. config is chaude's config dir (~/.chaude); state is the shared state dir (clients.json,
/// agents.json). Fails, changing nothing, on a CLAUDE.md with a broken marker pair or a clients.json that is not JSON.
pub fn make_plan(dir: &Path, config: &Path, state: &Path, with_notes: bool) -> io::Result<Plan> {
    let mut p = Plan {
        dir: dir.to_path_buf(),
        ..Plan::default()
    };
    p.plan_claude_dir()?;
    p.plan_local_state()?;
    p.plan_scripts()?;
    p.plan_claude_md()?;
    p.plan_notes(with_notes)?;
    p.plan_recent(config);
    p.plan_clients(state)?;
    Ok(p)
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Paths left alone, with the reason.
    pub fn keep(&self) -> &[String] {
        &self.keep
    }

    /// One line per planned change, in the order apply runs them.
    pub fn lines(&self) -> Vec<String> {
        self.steps.iter().map(|s| s.line.clone()).collect()
    }

    /// Runs every step in order. Stops at the first error; earlier steps stay done. Returns the number of steps run.
    pub fn apply(&self) -> io::Result<usize> {
        for s in &self.steps {
            match &s.op {
                Op::Remove(p) => remove_entry(p)?,
                Op::RemoveTree(p) => fs::remove_dir_all(p)?,
                Op::Write(p, b) => fs::write(p, b)?,
                Op::Forget(p, folders) => {
                    recent_folders::forget(p, folders)?;
                }
                Op::Clients(p, folders) => {
                    forget_clients(p, folders)?;
                }
            }
        }
        Ok(self.steps.len())
    }

    fn push(&mut self, line: String, op: Op) {
        self.steps.push(Step { line, op });
    }

    /// .claude/ and everything under it, deepest first, so every entry is removed while its parent still exists.
    fn plan_claude_dir(&mut self) -> io::Result<()> {
        let root = self.dir.join(".claude");
        if lstat(&root)?.is_none() {
            return Ok(());
        }
        let mut entries = Vec::new();
        walk(&root, &mut entries)?;
        for (path, link) in entries {
            let shown = rel(&self.dir, &path);
            let suffix = if link { " (link only)" } else { "" };
            self.push(format!("remove  {shown}{suffix}"), Op::Remove(path));
        }
        Ok(())
    }

    /// .chaude/ (chaude's project-local state) as a whole. A link is removed, never followed.
    fn plan_local_state(&mut self) -> io::Result<()> {
        let path = self.dir.join(".chaude");
        let Some(st) = lstat(&path)? else {
            return Ok(());
        };
        let op = if st.is_dir() {
            Op::RemoveTree(path)
        } else {
            Op::Remove(path)
        };
        self.push(String::from("remove  .chaude/ (whole dir)"), op);
        Ok(())
    }

    /// scripts/<name> only when it is the shipped copy; scripts/ itself only when nothing is left in it.
    fn plan_scripts(&mut self) -> io::Result<()> {
        let sdir = self.dir.join("scripts");
        match lstat(&sdir)? {
            Some(st) if st.is_dir() => {}
            _ => return Ok(()),
        }
        let mut names: Vec<PathBuf> = fs::read_dir(&sdir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<io::Result<_>>()?;
        names.sort();
        let mut removed = 0;
        for path in &names {
            let shown = rel(&self.dir, path);
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let shipped = SCRIPTS.iter().find(|(n, _)| *n == name).map(|(_, b)| *b);
            match shipped {
                Some(body) if is_plain_copy(path, body)? => {
                    removed += 1;
                    self.push(format!("remove  {shown}"), Op::Remove(path.clone()));
                }
                _ => self.keep.push(format!(
                    "{shown} (not the shipped copy, or not a chaude script)"
                )),
            }
        }
        if removed == names.len() {
            let shown = rel(&self.dir, &sdir);
            self.push(format!("remove  {shown}/ (empty after)"), Op::Remove(sdir));
        }
        Ok(())
    }

    /// The claude-lean block only. A file with nothing else left is deleted; a symlink is never edited through.
    fn plan_claude_md(&mut self) -> io::Result<()> {
        let path = self.dir.join(CLAUDE_MD);
        let Some(st) = lstat(&path)? else {
            return Ok(());
        };
        if st.file_type().is_symlink() {
            self.keep.push(format!(
                "{CLAUDE_MD} (symlink; not edited through the link)"
            ));
            return Ok(());
        }
        if !st.is_file() {
            return Ok(());
        }
        let b = fs::read(&path)?;
        let out = strip_policy(&b).map_err(|why| {
            io::Error::other(format!(
                "CLAUDE.md: {why} (fix by hand first; nothing changed)"
            ))
        })?;
        let Some(out) = out else {
            return Ok(());
        };
        if is_blank(&out) {
            self.push(
                format!("delete  {CLAUDE_MD} (only the claude-lean block was in it)"),
                Op::Remove(path),
            );
        } else {
            self.push(
                format!("policy  {CLAUDE_MD}: claude-lean block removed, project content kept"),
                Op::Write(path, out),
            );
        }
        Ok(())
    }

    fn plan_notes(&mut self, with_notes: bool) -> io::Result<()> {
        for name in WITH_NOTES_DIRS {
            let path = self.dir.join(name);
            let Some(st) = lstat(&path)? else {
                continue;
            };
            if !with_notes {
                self.keep
                    .push(format!("{name}/ (kept; --with-notes removes it)"));
                continue;
            }
            if st.is_dir() {
                self.push(
                    format!("remove  {name}/ (whole folder)"),
                    Op::RemoveTree(path),
                );
            } else {
                self.push(format!("remove  {name}"), Op::Remove(path));
            }
        }
        Ok(())
    }

    /// The row(s) for this folder in ~/.chaude/recent.json. Other folders and the rest of the file stay.
    fn plan_recent(&mut self, config: &Path) {
        let file = recent_folders::recent_file(config);
        let folders = folder_names(&self.dir);
        let hits = recent_folders::all_rows(&file)
            .iter()
            .filter(|r| folders.contains(&r.path))
            .count();
        if hits > 0 {
            self.push(
                format!("forget  {}: {hits} row(s) for this folder", file.display()),
                Op::Forget(file, folders),
            );
        }
    }

    /// The rows of this folder in the global clients.json. agents.json has no per-folder key, so it is only listed as kept.
    fn plan_clients(&mut self, state: &Path) -> io::Result<()> {
        let file = state.join("clients.json");
        let folders = folder_names(&self.dir);
        if let Some(text) = read_optional(&file)? {
            let (_, gone) = clients_kept(&text, &folders).map_err(|why| {
                io::Error::other(format!(
                    "{}: {why} (fix by hand first; nothing changed)",
                    file.display()
                ))
            })?;
            if gone > 0 {
                self.push(
                    format!("entry in {}: {gone} row(s) for this folder", file.display()),
                    Op::Clients(file, folders),
                );
            }
        }
        let agents = state.join("agents.json");
        if lstat(&agents)?.is_some() {
            self.keep.push(format!(
                "{} (no per-folder key; left as is)",
                agents.display()
            ));
        }
        Ok(())
    }
}

/// Splits a clients.json array into its raw rows. Keeps the rows whose cwd is not one of folders, and counts the rest.
/// Errors when the text is not a JSON array.
fn clients_kept<'a>(
    text: &'a str,
    folders: &[String],
) -> Result<(Vec<&'a RawValue>, usize), String> {
    let rows: Vec<&RawValue> =
        serde_json::from_str(text).map_err(|e| format!("not a JSON array ({e})"))?;
    let mut kept = Vec::new();
    let mut gone = 0;
    for r in rows {
        let v: serde_json::Value =
            serde_json::from_str(r.get()).map_err(|e| format!("bad row ({e})"))?;
        match v.get("cwd").and_then(|c| c.as_str()) {
            Some(cwd) if folders.iter().any(|f| f == cwd) => gone += 1,
            _ => kept.push(r),
        }
    }
    Ok((kept, gone))
}

/// The kept rows as a JSON array. Each row's bytes are copied as they were.
fn join_rows(rows: &[&RawValue]) -> String {
    if rows.is_empty() {
        return String::from("[]");
    }
    let body: Vec<&str> = rows.iter().map(|r| r.get()).collect();
    format!("[\n {}\n]", body.join(",\n "))
}

/// Re-reads clients.json under its lock and drops this folder's rows. Returns how many were dropped. A missing file is 0.
fn forget_clients(file: &Path, folders: &[String]) -> io::Result<usize> {
    let _lock = FileLock::acquire(file);
    let Some(text) = read_optional(file)? else {
        return Ok(0);
    };
    let (kept, gone) = clients_kept(&text, folders)
        .map_err(|why| io::Error::other(format!("{}: {why}", file.display())))?;
    if gone > 0 {
        fs::write(file, join_rows(&kept))?;
    }
    Ok(gone)
}

/// The file's text, or None when it does not exist.
fn read_optional(p: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(p) {
        Ok(t) => Ok(Some(t)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// True when the answer to "Delete these?" means yes.
pub fn answer_is_yes(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// Why dir must not be reset, if it must not: a filesystem root or the home folder.
pub fn refusal(dir: &Path, home: Option<&Path>) -> Option<&'static str> {
    if dir.parent().is_none() {
        return Some("refusing to reset a filesystem root");
    }
    if home.is_some_and(|h| same_dir(dir, h)) {
        return Some("refusing to reset the home folder");
    }
    None
}

/// `reset [--yes] [--dry-run] [--with-notes]`. Runs in the current folder. Returns the process exit code.
pub fn main(args: &[String]) -> i32 {
    let mut yes = false;
    let mut dry = false;
    let mut with_notes = false;
    for arg in args {
        match arg.as_str() {
            "--yes" | "-yes" => yes = true,
            "--dry-run" | "-dry-run" => dry = true,
            "--with-notes" | "-with-notes" => with_notes = true,
            "-h" | "--help" | "help" => {
                eprintln!("{RESET_USAGE}");
                return 0;
            }
            _ => {
                eprintln!("chaude reset: unknown argument {arg:?}\n{RESET_USAGE}");
                return 2;
            }
        }
    }
    let dir = match env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("reset: {e}");
            return 1;
        }
    };
    let home = osenv::home_dir();
    if let Some(why) = refusal(&dir, Some(&home)) {
        eprintln!("reset: {why}: {}", dir.display());
        return 1;
    }
    let plan = match make_plan(
        &dir,
        &recent_folders::config_dir(),
        &osenv::state_dir(),
        with_notes,
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("reset: {e}");
            return 1;
        }
    };
    println!("reset folder: {}", dir.display());
    for k in plan.keep() {
        println!("keep    {k}");
    }
    if plan.is_empty() {
        println!("nothing to reset: no chaude files in this folder");
        return 0;
    }
    let lines = plan.lines();
    for l in &lines {
        println!("{l}");
    }
    if dry {
        println!("dry-run: {} steps listed, nothing changed", lines.len());
        return 0;
    }
    if !yes {
        print!("Delete these? [y/N] ");
        let _ = io::stdout().flush();
        let mut answer = String::new();
        let _ = io::stdin().lock().read_line(&mut answer);
        if !answer_is_yes(&answer) {
            println!("cancelled, nothing changed");
            return 0;
        }
    }
    match plan.apply() {
        Ok(n) => {
            println!("done    {n} steps applied");
            0
        }
        Err(e) => {
            eprintln!("reset: {e} (stopped; steps before it are done)");
            1
        }
    }
}

/// lstat: None when nothing is there. Links are never followed.
fn lstat(p: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(p) {
        Ok(st) => Ok(Some(st)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// p and everything under it, children before parents. The bool marks a link (listed, never followed).
fn walk(p: &Path, out: &mut Vec<(PathBuf, bool)>) -> io::Result<()> {
    let st = fs::symlink_metadata(p)?;
    if st.is_dir() {
        let mut kids: Vec<PathBuf> = fs::read_dir(p)?
            .map(|e| e.map(|e| e.path()))
            .collect::<io::Result<_>>()?;
        kids.sort();
        for k in &kids {
            walk(k, out)?;
        }
    }
    out.push((p.to_path_buf(), st.file_type().is_symlink()));
    Ok(())
}

/// A regular file (not a link) whose bytes equal body.
fn is_plain_copy(p: &Path, body: &[u8]) -> io::Result<bool> {
    let st = fs::symlink_metadata(p)?;
    if !st.is_file() {
        return Ok(false);
    }
    Ok(fs::read(p)? == body)
}

/// Removes one entry: a dir (must be empty), or a file or a link (the link itself). Missing is fine.
fn remove_entry(p: &Path) -> io::Result<()> {
    match fs::symlink_metadata(p) {
        Ok(st) if st.is_dir() => fs::remove_dir(p),
        Ok(_) => fs::remove_file(p),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Path shown relative to the project root when it is inside it.
fn rel(dir: &Path, p: &Path) -> String {
    p.strip_prefix(dir).unwrap_or(p).display().to_string()
}

/// The names a recent-folders row may use for dir: as given and canonical.
fn folder_names(dir: &Path) -> Vec<String> {
    let mut v = vec![dir.display().to_string()];
    if let Ok(c) = dir.canonicalize() {
        let s = c.display().to_string();
        if !v.contains(&s) {
            v.push(s);
        }
    }
    v
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let ca = a.canonicalize().unwrap_or_else(|_| a.to_path_buf());
    let cb = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());
    ca == cb
}

#[cfg(test)]
#[path = "reset_tests.rs"]
mod tests;
