//! Resident MCP server (Go resident_mcp.go, resident_coder.go mcpReply). The main child calls the
//! `resident_coder` tool through `chaude mcp-resident <addr>`, which copies MCP lines to this loopback socket.

use std::collections::hash_map::RandomState;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

pub const TOOL: &str = "resident_coder";

// Runs one resident_coder task: Ok is the coder summary, Err the reason (sent back as isError).
pub type CoderFn = dyn Fn(&str) -> Result<String, String> + Send + Sync;

// Go mcpReply: answers one JSON-RPC line. None for notifications (no id) and lines that do not parse.
pub fn reply(line: &str, run: &CoderFn) -> Option<String> {
    let req: Value = serde_json::from_str(line).ok()?;
    let id = req.get("id")?.clone();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let mut resp = json!({"jsonrpc": "2.0", "id": id});
    match method {
        "initialize" => {
            resp["result"] = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "chaude", "version": "0"},
            });
        }
        "tools/list" => {
            resp["result"] = json!({"tools": [{
                "name": TOOL,
                "description": "Run a bounded code or document task on the resident haiku coder and return only its change summary. Blocks until done.",
                "inputSchema": {
                    "type": "object",
                    "required": ["task"],
                    "properties": {"task": {"type": "string"}},
                },
            }]});
        }
        "tools/call" => {
            let name = req["params"]["name"].as_str().unwrap_or("");
            if name != TOOL {
                resp["error"] = json!({"code": -32602, "message": format!("unknown tool {name}")});
                return Some(resp.to_string());
            }
            let task = req["params"]["arguments"]["task"].as_str().unwrap_or("");
            let result = match run(task) {
                Ok(text) => json!({"content": [{"type": "text", "text": text}]}),
                Err(reason) => {
                    json!({"content": [{"type": "text", "text": reason}], "isError": true})
                }
            };
            resp["result"] = result;
        }
        _ => {
            resp["error"] =
                json!({"code": -32601, "message": format!("method not found: {method}")});
        }
    }
    Some(resp.to_string())
}

// Env var that carries the per-run token from mcp.json to the bridge (mcp.json is written 0600).
pub const TOKEN_ENV: &str = "CHAUDE_MCP_TOKEN";

// A 256-bit random token from std only: 32 bytes from /dev/urandom. Where that is missing (Windows), it
// falls back to RandomState, which is seeded from OS randomness once per thread (about 128 bits).
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    let os = fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut bytes));
    if os.is_err() {
        for chunk in bytes.chunks_mut(8) {
            let n = RandomState::new().build_hasher().finish().to_le_bytes();
            chunk.copy_from_slice(&n[..chunk.len()]);
        }
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// Go residentMCPConfig: the mcp.json the main child reads with --mcp-config. The token rides in env.
pub fn config_json(exe: &str, addr: &str, token: &str) -> String {
    json!({"mcpServers": {"chaude": {
        "command": exe,
        "args": ["mcp-resident", addr],
        "env": {TOKEN_ENV: token},
    }}})
    .to_string()
}

// Listens on 127.0.0.1 and answers each bridge connection on its own thread. Returns "tcp:<addr>".
// Each connection must open with the token line, or it is dropped without a reply.
// The listener lives until the process exits (Go serveMCP).
pub fn serve(run: Arc<CoderFn>, token: &str) -> io::Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = format!("tcp:{}", listener.local_addr()?);
    let token = token.to_string();
    thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(conn) = conn else { break };
            let run = Arc::clone(&run);
            let token = token.clone();
            thread::spawn(move || serve_conn(conn, &*run, &token));
        }
    });
    Ok(addr)
}

fn serve_conn(conn: TcpStream, run: &CoderFn, token: &str) {
    let Ok(reader_side) = conn.try_clone() else {
        return;
    };
    let mut writer = conn;
    let mut lines = BufReader::new(reader_side).lines();
    match lines.next() {
        Some(Ok(first)) if first == token => {}
        _ => return,
    }
    for line in lines {
        let Ok(line) = line else { break };
        if let Some(resp) = reply(&line, run)
            && writeln!(writer, "{resp}").is_err()
        {
            break;
        }
    }
}

// Starts the server and writes mcp.json into a fresh temp directory. Returns the config path.
pub fn start(run: Arc<CoderFn>) -> io::Result<PathBuf> {
    let token = new_token();
    let addr = serve(run, &token)?;
    let exe = std::env::current_exe()?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("chaude-mcp-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let path = dir.join("mcp.json");
    fs::write(
        &path,
        config_json(&exe.display().to_string(), &addr, &token),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(path)
}

// Go MCPResidentBridge: copies MCP lines between the main child's stdio and the socket at addr.
// The token comes from TOKEN_ENV. Input EOF closes the write side; the bridge returns when the server
// closes its side.
pub fn bridge(addr: &str, input: impl Read, output: impl Write + Send + 'static) -> io::Result<()> {
    let token = std::env::var(TOKEN_ENV).ok();
    bridge_with_token(addr, token.as_deref(), input, output)
}

// bridge with an explicit token (None sends no token line, for a server that expects none).
pub fn bridge_with_token(
    addr: &str,
    token: Option<&str>,
    mut input: impl Read,
    mut output: impl Write + Send + 'static,
) -> io::Result<()> {
    let addr = addr.strip_prefix("tcp:").unwrap_or(addr);
    let mut conn = TcpStream::connect(addr)?;
    if let Some(token) = token {
        writeln!(conn, "{token}")?;
    }
    let mut back = conn.try_clone()?;
    let pump = thread::spawn(move || {
        let _ = io::copy(&mut back, &mut output);
        let _ = output.flush();
    });
    let mut fwd = conn.try_clone()?;
    io::copy(&mut input, &mut fwd)?;
    let _ = conn.shutdown(Shutdown::Write);
    let _ = pump.join();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn ok_run(task: &str) -> Result<String, String> {
        Ok(format!("status: done ({task})"))
    }

    fn parse(s: Option<String>) -> Value {
        serde_json::from_str(&s.expect("reply")).expect("json")
    }

    #[test]
    fn initialize_and_tools_list() {
        let init = parse(reply(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
            &ok_run,
        ));
        assert_eq!(init["id"], 1);
        assert_eq!(init["result"]["serverInfo"]["name"], "chaude");
        let list = parse(reply(
            r#"{"jsonrpc":"2.0","id":"a","method":"tools/list"}"#,
            &ok_run,
        ));
        assert_eq!(list["id"], "a");
        assert_eq!(list["result"]["tools"][0]["name"], TOOL);
        assert_eq!(list["result"]["tools"].as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn tools_call_returns_summary_or_error() {
        let line = r#"{"id":2,"method":"tools/call","params":{"name":"resident_coder","arguments":{"task":"rename"}}}"#;
        let got = parse(reply(line, &ok_run));
        assert_eq!(got["result"]["content"][0]["text"], "status: done (rename)");
        assert!(got["result"].get("isError").is_none());
        let fail = |_: &str| -> Result<String, String> { Err("coder busy".to_string()) };
        let got = parse(reply(line, &fail));
        assert_eq!(got["result"]["isError"], true);
        assert_eq!(got["result"]["content"][0]["text"], "coder busy");
    }

    #[test]
    fn unknown_tool_method_and_notifications() {
        let tool = r#"{"id":3,"method":"tools/call","params":{"name":"nope"}}"#;
        assert_eq!(parse(reply(tool, &ok_run))["error"]["code"], -32602);
        let other = r#"{"id":4,"method":"resources/list"}"#;
        assert_eq!(parse(reply(other, &ok_run))["error"]["code"], -32601);
        assert!(
            reply(
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                &ok_run
            )
            .is_none()
        );
        assert!(reply("not json", &ok_run).is_none());
    }

    #[test]
    fn config_names_the_bridge_command_and_carries_token_in_env() {
        let got: Value =
            serde_json::from_str(&config_json("/bin/chaude", "tcp:127.0.0.1:9", "tok")).unwrap();
        let srv = &got["mcpServers"]["chaude"];
        assert_eq!(srv["command"], "/bin/chaude");
        assert_eq!(srv["args"][0], "mcp-resident");
        assert_eq!(srv["args"][1], "tcp:127.0.0.1:9");
        assert_eq!(srv["env"][TOKEN_ENV], "tok");
        assert!(
            !srv["args"].to_string().contains("tok"),
            "token stays out of argv"
        );
    }

    #[test]
    fn token_is_random_and_hex() {
        let a = new_token();
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, new_token());
    }

    struct Shared(Arc<Mutex<Vec<u8>>>);
    impl Write for Shared {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    const CALL: &[u8] = br#"{"id":5,"method":"tools/call","params":{"name":"resident_coder","arguments":{"task":"x"}}}
"#;

    #[test]
    fn loopback_round_trip_through_bridge() {
        let token = new_token();
        let addr = serve(Arc::new(ok_run), &token).expect("serve");
        let out = Arc::new(Mutex::new(Vec::<u8>::new()));
        bridge_with_token(&addr, Some(&token), CALL, Shared(Arc::clone(&out))).expect("bridge");
        let text = String::from_utf8(out.lock().unwrap().clone()).unwrap();
        let got: Value = serde_json::from_str(text.trim()).expect("one reply line");
        assert_eq!(got["id"], 5);
        assert_eq!(got["result"]["content"][0]["text"], "status: done (x)");
    }

    #[test]
    fn wrong_or_missing_token_gets_no_reply() {
        let addr = serve(Arc::new(ok_run), &new_token()).expect("serve");
        for token in [Some("not-the-token"), None] {
            let out = Arc::new(Mutex::new(Vec::<u8>::new()));
            // The server drops the connection, so the write side may fail; only the empty output matters.
            let _ = bridge_with_token(&addr, token, CALL, Shared(Arc::clone(&out)));
            assert!(out.lock().unwrap().is_empty(), "{token:?}");
        }
    }
}
