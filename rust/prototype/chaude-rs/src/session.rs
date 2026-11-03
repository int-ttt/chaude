// One `claude -p` child in stream-json mode. Argv mirrors sessionArgs in
// src/main/intt/internal/harness/core.go (default options: no Minimal/Tools/Extra).
// Owned types only: no lifetimes.

use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

// Same value as claudePermissionMode in core.go: chaude always launches claude in auto mode.
pub const PERMISSION_MODE: &str = "auto";

// Messages funnelled from reader threads into the main loop (std::thread + mpsc, no async).
pub enum Msg {
    Line(String),
    ChildEof,
    Prompt(String),
    StdinEof,
}

pub struct Opts {
    pub claude_bin: String,
    pub model: String,
    pub resume: Option<String>,
}

pub fn build_args(model: &str, resume: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = [
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--model",
        model,
        "--permission-mode",
        PERMISSION_MODE,
        "--permission-prompt-tool",
        "stdio",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    if let Some(id) = resume {
        if !id.is_empty() {
            args.push("--resume".to_string());
            args.push(id.to_string());
        }
    }
    args
}

pub struct Session {
    pub child: Child,
    pub stdin: Option<ChildStdin>,
}

// Spawns claude and a thread that forwards each stdout line as Msg::Line, then Msg::ChildEof.
pub fn spawn(opts: &Opts, tx: Sender<Msg>) -> io::Result<Session> {
    let mut child = Command::new(&opts.claude_bin)
        .args(build_args(&opts.model, opts.resume.as_deref()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdin = child.stdin.take();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("claude stdout not piped"))?;
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(l) => {
                    if tx.send(Msg::Line(l)).is_err() {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(Msg::ChildEof);
    });
    Ok(Session { child, stdin })
}

// Reads chaude's stdin lines as prompts on a thread; sends Msg::StdinEof at end.
pub fn spawn_prompt_reader(tx: Sender<Msg>) {
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(l) => {
                    let t = l.trim();
                    if !t.is_empty() && tx.send(Msg::Prompt(t.to_string())).is_err() {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(Msg::StdinEof);
    });
}

fn write_frame(stdin: &mut ChildStdin, v: &serde_json::Value) -> io::Result<()> {
    let mut s = serde_json::to_string(v).map_err(io::Error::other)?;
    s.push('\n');
    stdin.write_all(s.as_bytes())?;
    stdin.flush()
}

pub fn write_user(stdin: &mut ChildStdin, text: &str) -> io::Result<()> {
    write_frame(
        stdin,
        &serde_json::json!({"type": "user", "message": {"role": "user", "content": text}}),
    )
}

// Permission answer for a can_use_tool control_request (matches RespondPermission in core.go).
pub fn write_permission(stdin: &mut ChildStdin, request_id: &str, allow: bool, input: serde_json::Value, message: &str) -> io::Result<()> {
    let body = if allow {
        serde_json::json!({"behavior": "allow", "updatedInput": input})
    } else {
        serde_json::json!({"behavior": "deny", "message": message})
    };
    write_frame(
        stdin,
        &serde_json::json!({"type": "control_response", "response": {"subtype": "success", "request_id": request_id, "response": body}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_match_go_session_defaults() {
        let a = build_args("opus", None);
        let want = [
            "-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
            "--include-partial-messages", "--model", "opus", "--permission-mode", "auto",
            "--permission-prompt-tool", "stdio",
        ];
        assert_eq!(a, want);
    }

    #[test]
    fn resume_is_appended_only_when_set() {
        let a = build_args("opus", Some("abc"));
        assert_eq!(a[a.len() - 2..], ["--resume".to_string(), "abc".to_string()]);
        assert_eq!(build_args("opus", Some("")).len(), build_args("opus", None).len());
    }
}
