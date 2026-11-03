//! claude-history (Go: claude_history.go). Finds a project's transcript folder.

use std::fs;
use std::path::{Path, PathBuf};

use chaude_core::osenv;

use crate::common::{
    abs_path, default_project, each_file_line, fail, fget, glob_dir, norm_case, parse, parse_line,
    sval, usage,
};

/// ~/.claude/projects
pub fn projects_root() -> PathBuf {
    osenv::home_dir().join(".claude").join("projects")
}

/// Whether one of the first lines of a transcript records `want` as its cwd.
fn has_cwd(path: &Path, want: &str) -> bool {
    let mut found = false;
    let mut n = 0;
    let _ = each_file_line(path, |line| {
        n += 1;
        if found || n > 50 || !line.contains("\"cwd\"") {
            return;
        }
        let Some(e) = parse_line(line) else { return };
        let cwd = sval(fget(&e, "cwd"));
        if !cwd.is_empty() && norm_case(Path::new(cwd)) == want {
            found = true;
        }
    });
    found
}

/// The transcript folder for `project`, or None.
pub fn history_dir(project: &str) -> Option<PathBuf> {
    let proj = abs_path(project);
    let root = projects_root();
    let encoded: String = proj
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let d = root.join(encoded);
    if d.is_dir() {
        return Some(d);
    }
    let want = norm_case(&proj);
    let mut names: Vec<String> = fs::read_dir(&root)
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        if name.starts_with('.') {
            continue;
        }
        let cand = root.join(&name);
        if glob_dir(&cand, ".jsonl")
            .iter()
            .take(3)
            .any(|f| has_cwd(f, &want))
        {
            return Some(cand);
        }
    }
    None
}

pub fn run(args: &[String]) -> i32 {
    let flags = match parse("claude-history", &[], args) {
        Ok(f) => f,
        Err(rc) => return rc,
    };
    if flags.pos.len() > 1 {
        usage("claude-history");
        return 2;
    }
    let proj = flags.pos.first().cloned().unwrap_or_else(default_project);
    match history_dir(&proj) {
        Some(d) => {
            println!("{}", d.display());
            0
        }
        None => fail(&format!(
            "기록 없음: {} 에 대한 대화 기록 폴더를 찾지 못했습니다 (확인 위치: {})",
            abs_path(&proj).display(),
            projects_root().display()
        )),
    }
}
