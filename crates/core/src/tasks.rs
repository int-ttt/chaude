//! Agent task file: notes/tasks/<ws>.md, the steps of one long agent job, run one at a time. A step is
//! `## <title>` followed by `- key: value` lines (files, gate, commit, status, hash). Any other non-blank
//! line in a step is a note and is kept as written. The scheduler (plan/agent-scheduler.md, step 2) picks the
//! next step with `next_todo` and records the result with `mark_done` / `mark_failed`. An agent does the same
//! by editing the file at each checkpoint.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Todo,
    Done,
    Failed,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Todo => "todo",
            Status::Done => "done",
            Status::Failed => "failed",
        }
    }

    fn parse(s: &str) -> Option<Status> {
        match s {
            "todo" => Some(Status::Todo),
            "done" => Some(Status::Done),
            "failed" => Some(Status::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Step {
    pub title: String,
    pub files: Vec<String>,
    pub gate: String,
    pub commit: String,
    pub status: Status,
    pub hash: String,
    // Lines that are not fields, kept as written. Failure logs go here as "> " lines.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskFile {
    // Lines before the first step (title, intro), kept as written.
    pub preamble: Vec<String>,
    pub steps: Vec<Step>,
}

// Parses a task file. A missing status means todo; an unknown status is an error.
pub fn parse(text: &str) -> Result<TaskFile, Error> {
    let mut file = TaskFile::default();
    for line in text.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            file.steps.push(Step {
                title: title.trim().to_string(),
                ..Step::default()
            });
            continue;
        }
        let n = file.steps.len();
        let Some(step) = file.steps.last_mut() else {
            file.preamble.push(line.to_string());
            continue;
        };
        let field = line
            .strip_prefix("- ")
            .and_then(|rest| rest.split_once(':'))
            .map(|(k, v)| (k.trim(), v.trim()));
        match field {
            Some(("files", v)) => step.files = split_files(v),
            Some(("gate", v)) => step.gate = v.to_string(),
            Some(("commit", v)) => step.commit = v.to_string(),
            Some(("hash", v)) => step.hash = v.to_string(),
            Some(("status", v)) => {
                step.status = Status::parse(v).ok_or_else(|| {
                    Error::Msg(format!(
                        "step {n}: unknown status \"{v}\" (todo, done or failed)"
                    ))
                })?;
            }
            _ => {
                if !line.trim().is_empty() {
                    step.notes.push(line.trim_end().to_string());
                }
            }
        }
    }
    Ok(file)
}

// Renders the canonical form: preamble, then each step (fields in fixed order, then notes), one blank line
// between steps.
pub fn render(file: &TaskFile) -> String {
    let mut out = String::new();
    for line in &file.preamble {
        out.push_str(line);
        out.push('\n');
    }
    for (i, step) in file.steps.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let _ = writeln!(out, "## {}", step.title);
        let fields = [
            ("files", step.files.join(", ")),
            ("gate", step.gate.clone()),
            ("commit", step.commit.clone()),
            ("status", step.status.as_str().to_string()),
            ("hash", step.hash.clone()),
        ];
        for (key, value) in fields {
            let line = format!("- {key}: {value}");
            let _ = writeln!(out, "{}", line.trim_end());
        }
        for note in &step.notes {
            let _ = writeln!(out, "{note}");
        }
    }
    out
}

// Index (0-based) of the first step whose status is todo.
pub fn next_todo(file: &TaskFile) -> Option<usize> {
    file.steps.iter().position(|s| s.status == Status::Todo)
}

// Marks the todo step at idx done and records its commit hash.
pub fn mark_done(file: &mut TaskFile, idx: usize, hash: &str) -> Result<(), Error> {
    let step = todo_step(file, idx)?;
    step.status = Status::Done;
    step.hash = hash.trim().to_string();
    Ok(())
}

// Marks the todo step at idx failed and keeps the failure log as "> " note lines.
pub fn mark_failed(file: &mut TaskFile, idx: usize, log: &str) -> Result<(), Error> {
    let step = todo_step(file, idx)?;
    step.status = Status::Failed;
    for line in log.lines().filter(|l| !l.trim().is_empty()) {
        step.notes.push(format!("> {}", line.trim_end()));
    }
    Ok(())
}

fn todo_step(file: &mut TaskFile, idx: usize) -> Result<&mut Step, Error> {
    let n = idx + 1;
    let step = file
        .steps
        .get_mut(idx)
        .ok_or_else(|| Error::Msg(format!("no step {n}")))?;
    if step.status != Status::Todo {
        return Err(Error::Msg(format!(
            "step {n}: status is {}, not todo",
            step.status.as_str()
        )));
    }
    Ok(step)
}

fn split_files(v: &str) -> Vec<String> {
    v.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

// notes/tasks/<ws>.md under the project dir. The workspace id is letters, digits, '-' or '_'.
pub fn task_file(project_dir: &Path, ws: &str) -> Result<PathBuf, Error> {
    let ok = !ws.is_empty()
        && ws
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(Error::Msg(format!("invalid workspace id \"{ws}\"")));
    }
    Ok(project_dir
        .join("notes")
        .join("tasks")
        .join(format!("{ws}.md")))
}

pub fn load(path: &Path) -> Result<TaskFile, Error> {
    parse(&fs::read_to_string(path)?)
}

pub fn save(path: &Path, file: &TaskFile) -> Result<(), Error> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, render(file))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    const SAMPLE: &str = "# Task: demo\n\nIntro line.\n\n## Add parser\n- files: crates/core/src/a.rs, crates/core/src/b.rs\n- gate: cargo test -p chaude-core\n- commit: core: add parser\n- status: todo\n- hash:\n\n## Wire UI\n- files: crates/cli/src/x.rs\n- gate: cargo test -p chaude-cli\n- commit: cli: wire ui\n- status: todo\n- hash:\n";

    #[test]
    fn canonical_text_round_trips() {
        let file = parse(SAMPLE).unwrap();
        assert_eq!(file.preamble, vec!["# Task: demo", "", "Intro line.", ""]);
        assert_eq!(file.steps.len(), 2);
        assert_eq!(file.steps[0].title, "Add parser");
        assert_eq!(
            file.steps[0].files,
            vec!["crates/core/src/a.rs", "crates/core/src/b.rs"]
        );
        assert_eq!(file.steps[0].commit, "core: add parser");
        assert_eq!(render(&file), SAMPLE);
    }

    #[test]
    fn next_todo_walks_steps_in_order() {
        let mut file = parse(SAMPLE).unwrap();
        assert_eq!(next_todo(&file), Some(0));
        mark_done(&mut file, 0, " abc1234 ").unwrap();
        assert_eq!(next_todo(&file), Some(1));
        mark_failed(&mut file, 1, "error[E0425]: cannot find\n\nnext line").unwrap();
        assert_eq!(next_todo(&file), None);
        assert_eq!(file.steps[0].status, Status::Done);
        assert_eq!(file.steps[0].hash, "abc1234");
        assert_eq!(file.steps[1].status, Status::Failed);
        assert_eq!(
            file.steps[1].notes,
            vec!["> error[E0425]: cannot find", "> next line"]
        );
    }

    #[test]
    fn marked_file_renders_and_parses_back() {
        let mut file = parse(SAMPLE).unwrap();
        mark_done(&mut file, 0, "abc1234").unwrap();
        mark_failed(&mut file, 1, "## not a heading\n- files: x").unwrap();
        let text = render(&file);
        assert!(text.contains("- status: done\n- hash: abc1234\n\n## Wire UI\n"));
        assert!(text.contains("- status: failed\n- hash:\n> ## not a heading\n> - files: x\n"));
        assert_eq!(parse(&text).unwrap(), file);
    }

    #[test]
    fn only_todo_steps_can_be_marked() {
        let mut file = parse(SAMPLE).unwrap();
        mark_done(&mut file, 0, "abc1234").unwrap();
        assert!(mark_done(&mut file, 0, "def").is_err());
        assert!(mark_failed(&mut file, 0, "log").is_err());
        assert!(mark_done(&mut file, 2, "def").is_err());
        assert_eq!(file.steps[0].hash, "abc1234");
    }

    #[test]
    fn unknown_status_is_an_error_and_missing_status_is_todo() {
        assert!(parse("## A\n- status: doing\n").is_err());
        let file = parse("## A\n- gate: make\n").unwrap();
        assert_eq!(file.steps[0].status, Status::Todo);
        assert_eq!(file.steps[0].gate, "make");
    }

    #[test]
    fn task_file_path_and_save_load() {
        let dir = TempDir::new("tasks-io");
        assert!(task_file(dir.path(), "../x").is_err());
        assert!(task_file(dir.path(), "").is_err());
        let path = task_file(dir.path(), "ws-1").unwrap();
        assert_eq!(path, dir.path().join("notes").join("tasks").join("ws-1.md"));
        let file = parse(SAMPLE).unwrap();
        save(&path, &file).unwrap();
        assert_eq!(load(&path).unwrap(), file);
    }
}
