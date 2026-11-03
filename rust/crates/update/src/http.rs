//! HTTP GET through the system `curl` (Go net/http behaviour: redirects followed, status code returned to the caller,
//! transport failures reported as Offline). The request is passed to curl as a config file on stdin, so the token never
//! shows up in the process list. curl is preferred over an HTTP crate because it ships with macOS, Windows 10+ and
//! practically every Linux, and it keeps the dependency tree small.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::Error;

/// One GET. Owned fields only.
pub struct Request {
    pub url: String,
    pub accept: String,
    /// Sent as `Authorization: Bearer <token>` when non-empty.
    pub token: String,
    /// When set, the body is written to this file and `Response::body` stays empty.
    pub output: Option<PathBuf>,
    /// Truncate the body to this many bytes (Go io.LimitReader); None = no limit.
    pub max_bytes: Option<usize>,
}

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Runs the GET. `deadline` bounds the whole request (curl --max-time is set to what is left).
/// A transport failure (no connection, DNS, timeout, TLS) is `Error::Offline`.
pub fn fetch(req: &Request, deadline: Instant) -> Result<Response, Error> {
    let left = deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| {
            Error::Offline("cannot reach GitHub: timed out (10 minute limit)".to_string())
        })?;
    let secs = left.as_secs().max(1);

    let config = config_for(req, secs);
    let mut child = Command::new("curl")
        .args(["--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Failed(format!("cannot run curl (needed to reach GitHub): {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        // A write error only means curl exited early; its exit status and stderr below explain why.
        let _ = stdin.write_all(config.as_bytes());
    }
    let out = child
        .wait_with_output()
        .map_err(|e| Error::Failed(format!("curl failed: {e}")))?;

    // The write-out format appends the 3-digit status after the body (or alone, when writing to a file).
    let split = out.stdout.len().saturating_sub(3);
    let status = std::str::from_utf8(&out.stdout[split..])
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if status == 0 {
            return Err(Error::Offline(format!("cannot reach GitHub: {stderr}")));
        }
        return Err(Error::Failed(stderr));
    }
    let mut body = if req.output.is_some() {
        Vec::new()
    } else {
        out.stdout[..split].to_vec()
    };
    if let Some(max) = req.max_bytes {
        body.truncate(max);
    }
    Ok(Response { status, body })
}

/// The curl config (one option per line). Values are double-quoted with curl's escapes.
fn config_for(req: &Request, max_time_secs: u64) -> String {
    let mut c = String::new();
    c.push_str(&format!("url = {}\n", quote(&req.url)));
    c.push_str(&format!(
        "header = {}\n",
        quote(&format!("Accept: {}", req.accept))
    ));
    if !req.token.is_empty() {
        c.push_str(&format!(
            "header = {}\n",
            quote(&format!("Authorization: Bearer {}", req.token))
        ));
    }
    if let Some(p) = &req.output {
        c.push_str(&format!("output = {}\n", quote(&p.to_string_lossy())));
    }
    c.push_str("silent\nshow-error\nlocation\n");
    c.push_str("max-redirs = 10\n");
    c.push_str(&format!("max-time = {max_time_secs}\n"));
    c.push_str("write-out = \"%{http_code}\"\n");
    c
}

/// Quotes a value for a curl config file: backslash, double quote, tab, CR and LF are escaped.
fn quote(v: &str) -> String {
    let mut s = String::with_capacity(v.len() + 2);
    s.push('"');
    for ch in v.chars() {
        match ch {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '\t' => s.push_str("\\t"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_escapes_config_specials() {
        assert_eq!(quote(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(quote("x\ny\tz\r"), r#""x\ny\tz\r""#);
    }

    #[test]
    fn config_carries_token_and_output_without_argv() {
        let req = Request {
            url: "https://example.test/x".to_string(),
            accept: "application/octet-stream".to_string(),
            token: "tok".to_string(),
            output: Some(PathBuf::from("/tmp/o")),
            max_bytes: None,
        };
        let c = config_for(&req, 42);
        assert!(c.contains("header = \"Authorization: Bearer tok\"\n"));
        assert!(c.contains("output = \"/tmp/o\"\n"));
        assert!(c.contains("max-time = 42\n"));
    }
}
