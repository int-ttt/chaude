// One `claude -p` child in stream-json mode. Argv mirrors sessionArgs, env mirrors cleanEnv and withCacheTTL,
// stderr is discarded like cmd.Stderr = nil, and the stdin frames match Send, RespondPermission and Interrupt
// in deprecated/src/main/intt/internal/harness/core.go. Owned types only: no lifetimes.

use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

// Same value as claudePermissionMode in core.go: chaude always launches claude in auto mode.
pub const PERMISSION_MODE: &str = "auto";

// What the stdout reader thread forwards: one trimmed non-empty line, then Eof when the child closes stdout.
#[derive(Debug)]
pub enum ProcMsg {
    Line(String),
    Eof,
}

#[derive(Debug, Clone)]
pub struct SpawnOpts {
    pub claude_bin: String,
    pub model: String,
    pub resume: Option<String>,
    pub cwd: Option<PathBuf>,
    // "1h" or "5m" pins the prompt cache TTL; anything else leaves the inherited env alone (Go: "" or "auto").
    pub cache_ttl: String,
    // Non-zero sets CLAUDE_CODE_AUTO_COMPACT_WINDOW for the child only.
    pub compact_window: u32,
    // Go --task-budget-usd; zero leaves the child without --max-budget-usd.
    pub max_budget_usd: f64,
}

// Go --task-budget-usd also passes --max-budget-usd to the child as a backstop. Zero adds nothing.
pub fn budget_args(cap: f64) -> Vec<String> {
    if cap > 0.0 {
        vec!["--max-budget-usd".to_string(), cap.to_string()]
    } else {
        Vec::new()
    }
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
    if let Some(id) = resume
        && !id.is_empty()
    {
        args.push("--resume".to_string());
        args.push(id.to_string());
    }
    args
}

// Env delta applied on top of the inherited environment (Go: cleanEnv + withCacheTTL).
fn apply_env(cmd: &mut Command, compact_window: u32, cache_ttl: &str) {
    // ANTHROPIC_API_KEY must be absent: with -p a key silently switches billing to the API.
    cmd.env_remove("ANTHROPIC_API_KEY");
    if compact_window != 0 {
        cmd.env(
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
            compact_window.to_string(),
        );
    }
    if cache_ttl == "1h" || cache_ttl == "5m" {
        cmd.env("CLAUDE_CODE_PROMPT_CACHE_TTL", cache_ttl);
    }
}

// Go /usage (usageRaw): a separate `claude -p /usage --output-format json` run in the temp dir, so it never
// touches the session. Returns the JSON `result` text. Not charged to the budget (Go never calls chargeOneShot).
pub fn one_shot_usage(claude_bin: &str) -> Result<String, String> {
    let mut cmd = Command::new(claude_bin);
    cmd.args([
        "-p",
        "/usage",
        "--output-format",
        "json",
        "--no-session-persistence",
    ])
    .current_dir(std::env::temp_dir())
    .stdin(Stdio::null())
    .stderr(Stdio::null());
    apply_env(&mut cmd, 0, "");
    let out = cmd.output().map_err(|e| format!("usage: {e}"))?;
    let v: Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("usage: bad json: {e}"))?;
    v.get("result")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "usage: no result".to_string())
}

pub struct ClaudeProc {
    child: Child,
    stdin: ChildStdin,
}

// Spawns claude and a thread that forwards each stdout line as wrap(ProcMsg::Line), then wrap(ProcMsg::Eof).
// The caller picks T so the forwarded messages can share the engine's input channel.
pub fn spawn<T: Send + 'static>(
    opts: &SpawnOpts,
    tx: Sender<T>,
    wrap: impl Fn(ProcMsg) -> T + Send + 'static,
) -> io::Result<ClaudeProc> {
    let mut cmd = Command::new(&opts.claude_bin);
    cmd.args(build_args(&opts.model, opts.resume.as_deref()))
        .args(budget_args(opts.max_budget_usd))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(dir) = &opts.cwd {
        cmd.current_dir(dir);
    }
    apply_env(&mut cmd, opts.compact_window, &opts.cache_ttl);
    let mut child = cmd.spawn()?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("claude stdin not piped"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("claude stdout not piped"))?;
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if tx.send(wrap(ProcMsg::Line(t.to_string()))).is_err() {
                return;
            }
        }
        let _ = tx.send(wrap(ProcMsg::Eof));
    });
    Ok(ClaudeProc { child, stdin })
}

impl ClaudeProc {
    // One user message frame (Go: sendFrame).
    pub fn write_user(&mut self, text: &str) -> io::Result<()> {
        self.write_frame(&json!({"type": "user", "message": {"role": "user", "content": text}}))
    }

    // Answer to a can_use_tool control_request (Go: RespondPermission). Empty message becomes "denied by user".
    pub fn write_permission(
        &mut self,
        request_id: &str,
        allow: bool,
        input: Value,
        message: &str,
    ) -> io::Result<()> {
        let body = if allow {
            let input = if input.is_null() { json!({}) } else { input };
            json!({"behavior": "allow", "updatedInput": input})
        } else {
            let msg = if message.is_empty() {
                "denied by user"
            } else {
                message
            };
            json!({"behavior": "deny", "message": msg})
        };
        self.write_frame(&json!({
            "type": "control_response",
            "response": {"subtype": "success", "request_id": request_id, "response": body}
        }))
    }

    // Interrupt control_request (Go: Interrupt). Returns the request_id that was sent.
    pub fn write_interrupt(&mut self) -> io::Result<String> {
        let id = new_request_id();
        self.write_frame(&json!({
            "type": "control_request",
            "request_id": id,
            "request": {"subtype": "interrupt"}
        }))?;
        Ok(id)
    }

    // Kills the child and reaps it. Errors are ignored: the child may already have exited.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn write_frame(&mut self, v: &Value) -> io::Result<()> {
        let mut s = serde_json::to_string(v).map_err(io::Error::other)?;
        s.push('\n');
        self.stdin.write_all(s.as_bytes())?;
        self.stdin.flush()
    }
}

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(0);

// UUID-shaped id like newUUID in core.go. Uniqueness per process is all the protocol needs.
fn new_request_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let seq = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let mix = nanos ^ seq.rotate_left(17) ^ u64::from(std::process::id()).rotate_left(41);
    format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        (mix >> 32) as u32,
        mix & 0xffff,
        seq & 0xfff,
        (seq >> 12) & 0xfff,
        mix & 0xffff_ffff_ffff
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_args_only_when_cap_is_set() {
        assert!(budget_args(0.0).is_empty());
        assert_eq!(budget_args(0.5), ["--max-budget-usd", "0.5"]);
    }

    #[test]
    fn args_match_go_session_defaults() {
        let a = build_args("opus", None);
        let want = [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--model",
            "opus",
            "--permission-mode",
            "auto",
            "--permission-prompt-tool",
            "stdio",
        ];
        assert_eq!(a, want);
    }

    #[test]
    fn resume_is_appended_only_when_set() {
        let a = build_args("opus", Some("abc"));
        assert_eq!(
            a[a.len() - 2..],
            ["--resume".to_string(), "abc".to_string()]
        );
        assert_eq!(
            build_args("opus", Some("")).len(),
            build_args("opus", None).len()
        );
    }

    #[test]
    fn request_ids_are_unique_and_uuid_shaped() {
        let a = new_request_id();
        let b = new_request_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(a.matches('-').count(), 4);
    }
}
