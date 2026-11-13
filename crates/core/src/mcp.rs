//! Resident MCP server (Go resident_mcp.go, resident_coder.go mcpReply). The main child calls the
//! `resident_coder` tool through `chaude mcp-resident <addr>`, which copies MCP lines to this server.
//! Unix serves a unix socket in a 0700 temp dir (Go listenMCP); Windows serves loopback TCP with a token.

use std::collections::hash_map::{DefaultHasher, RandomState};
use std::fs;
use std::hash::{BuildHasher, Hash, Hasher};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::mem::ManuallyDrop;
#[cfg(not(unix))]
use std::net::TcpListener;
use std::net::{Shutdown, TcpStream};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use crate::error::Error;
use crate::tmp_layout;
use serde_json::{Value, json};

pub const TOOL: &str = "resident_coder";

// Runs one resident_coder task: Ok is the coder summary, Err the reason (sent back as isError).
pub type CoderFn = dyn Fn(&str) -> Result<String, Error> + Send + Sync;

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
                    json!({"content": [{"type": "text", "text": reason.to_string()}], "isError": true})
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

// A bridge connection: a unix socket on unix, TCP (and tcp: addresses) everywhere.
trait Conn: Read + Write + Send + Sized + 'static {
    fn dup(&self) -> io::Result<Self>;
    fn close_write(&self) -> io::Result<()>;
}

impl Conn for TcpStream {
    fn dup(&self) -> io::Result<Self> {
        self.try_clone()
    }
    fn close_write(&self) -> io::Result<()> {
        self.shutdown(Shutdown::Write)
    }
}

#[cfg(unix)]
impl Conn for UnixStream {
    fn dup(&self) -> io::Result<Self> {
        self.try_clone()
    }
    fn close_write(&self) -> io::Result<()> {
        self.shutdown(Shutdown::Write)
    }
}

// Serves the unix socket at sock (Go listenMCP). The socket sits in a 0700 dir and is chmod 0600, so
// only this user reaches it; no token line is read. Returns the bare socket path for the bridge arg.
// The listener lives until the process exits (Go serveMCP).
#[cfg(unix)]
pub fn serve(run: Arc<CoderFn>, _token: &str, sock: &Path) -> io::Result<String> {
    use std::os::unix::fs::PermissionsExt;
    let listener = UnixListener::bind(sock)?;
    fs::set_permissions(sock, fs::Permissions::from_mode(0o600))?;
    let addr = sock.display().to_string();
    thread::spawn(move || accept_loop(listener.incoming(), run, None));
    Ok(addr)
}

// Windows has no unix socket here: listens on 127.0.0.1 and returns "tcp:<addr>". Each connection must
// open with the token line, or it is dropped without a reply. The sock argument is unused.
#[cfg(not(unix))]
pub fn serve(run: Arc<CoderFn>, token: &str, _sock: &Path) -> io::Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = format!("tcp:{}", listener.local_addr()?);
    let token = token.to_string();
    thread::spawn(move || accept_loop(listener.incoming(), run, Some(token)));
    Ok(addr)
}

fn accept_loop<S: Conn>(
    incoming: impl Iterator<Item = io::Result<S>>,
    run: Arc<CoderFn>,
    token: Option<String>,
) {
    for conn in incoming {
        let Ok(conn) = conn else { break };
        let run = Arc::clone(&run);
        let token = token.clone();
        thread::spawn(move || serve_conn(conn, &*run, token.as_deref()));
    }
}

fn serve_conn<S: Conn>(conn: S, run: &CoderFn, token: Option<&str>) {
    let Ok(reader_side) = conn.dup() else {
        return;
    };
    let mut writer = conn;
    let mut lines = BufReader::new(reader_side).lines();
    if let Some(token) = token {
        match lines.next() {
            Some(Ok(first)) if first == token => {}
            _ => return,
        }
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

// Owns the dirs that hold mcp.json and the socket. Dropping it removes both (the listener thread stays up until exit).
pub struct McpConfig {
    dir: PathBuf,
    sock_dir: PathBuf,
    path: PathBuf,
}

impl McpConfig {
    pub fn path(&self) -> &Path {
        &self.path
    }

    // Gives up the cleanup: the dir stays on disk. Only for callers that cannot hold the guard yet.
    pub fn into_path(self) -> PathBuf {
        let this = ManuallyDrop::new(self);
        this.path.clone()
    }
}

impl Drop for McpConfig {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
        let _ = fs::remove_dir_all(&self.sock_dir);
    }
}

// macOS sun_path is 104 bytes (NUL included), so a longer socket path cannot bind.
const SUN_PATH_MAX: usize = 104;

// <system temp>/chaude-<hash of cwd>: the socket dirs. Sockets are not data, and the system temp path stays short,
// where a cwd under notes/ can push the socket path past SUN_PATH_MAX.
fn sock_root(cwd: &Path) -> PathBuf {
    let mut h = DefaultHasher::new();
    cwd.to_string_lossy().hash(&mut h);
    std::env::temp_dir().join(format!("chaude-{:08x}", h.finish() as u32))
}

// Starts the server and writes mcp.json into a fresh dir under cwd's notes/tmp/test, and the unix socket
// mcp.sock into a fresh dir under sock_root(cwd). The guard removes both dirs on drop.
pub fn start_owned(cwd: &Path, run: Arc<CoderFn>) -> io::Result<McpConfig> {
    let token = new_token();
    let exe = std::env::current_exe()?;
    // The pid and a process-wide sequence name the dirs, so parallel callers in one process never share them.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let name = format!("{}-{seq}", std::process::id());
    let sock_dir = sock_root(cwd).join(&name);
    let sock = sock_dir.join("mcp.sock");
    if sock.as_os_str().len() >= SUN_PATH_MAX {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("socket path too long: {}", sock.display()),
        ));
    }
    let parent = tmp_layout::test_dir(cwd);
    fs::create_dir_all(&parent)?;
    let dir = parent.join(&name);
    // A leftover with the same name can only come from a dead process, so it is cleared first.
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&sock_dir);
    fs::create_dir_all(sock_root(cwd))?;
    make_private_dir(&dir)?;
    // Created before serve, so a failed serve still removes both dirs.
    let path = dir.join("mcp.json");
    let cfg = McpConfig {
        dir: dir.clone(),
        sock_dir: sock_dir.clone(),
        path: path.clone(),
    };
    make_private_dir(&sock_dir)?;
    let addr = serve(run, &token, &sock)?;
    fs::write(
        &path,
        config_json(&exe.display().to_string(), &addr, &token),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(cfg)
}

#[cfg(unix)]
fn make_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(dir)
}

#[cfg(not(unix))]
fn make_private_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)
}

// Same as start_owned, but the dir is never removed (the old behaviour; api.rs still uses it).
pub fn start(cwd: &Path, run: Arc<CoderFn>) -> io::Result<PathBuf> {
    start_owned(cwd, run).map(McpConfig::into_path)
}

// Go MCPResidentBridge: copies MCP lines between the main child's stdio and the socket at addr
// (a unix socket path, or tcp:<addr>). The token comes from TOKEN_ENV and is sent only on tcp:.
// Input EOF closes the write side; the bridge returns when the server closes its side.
pub fn bridge(addr: &str, input: impl Read, output: impl Write + Send + 'static) -> io::Result<()> {
    let token = std::env::var(TOKEN_ENV).ok();
    bridge_with_token(addr, token.as_deref(), input, output)
}

// bridge with an explicit token (None sends no token line, for a server that expects none).
pub fn bridge_with_token(
    addr: &str,
    token: Option<&str>,
    input: impl Read,
    output: impl Write + Send + 'static,
) -> io::Result<()> {
    #[cfg(unix)]
    {
        if !addr.starts_with("tcp:") {
            // The socket file is 0600 in a 0700 dir, so no token line is sent.
            return relay(UnixStream::connect(addr)?, None, input, output);
        }
    }
    let addr = addr.strip_prefix("tcp:").unwrap_or(addr);
    relay(TcpStream::connect(addr)?, token, input, output)
}

fn relay<S: Conn>(
    mut conn: S,
    token: Option<&str>,
    mut input: impl Read,
    mut output: impl Write + Send + 'static,
) -> io::Result<()> {
    if let Some(token) = token {
        writeln!(conn, "{token}")?;
    }
    let mut back = conn.dup()?;
    let pump = thread::spawn(move || {
        let _ = io::copy(&mut back, &mut output);
        let _ = output.flush();
    });
    let mut fwd = conn.dup()?;
    io::copy(&mut input, &mut fwd)?;
    let _ = conn.close_write();
    let _ = pump.join();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn ok_run(task: &str) -> Result<String, Error> {
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
        let fail = |_: &str| -> Result<String, Error> { Err(Error::Msg("coder busy".to_string())) };
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
    fn owned_config_removes_its_dir_on_drop() {
        let tmp = crate::testutil::TempDir::new("mcp-drop");
        let cfg = start_owned(tmp.path(), Arc::new(ok_run)).expect("start");
        let path = cfg.path().to_path_buf();
        let dir = path.parent().expect("dir").to_path_buf();
        assert!(path.is_file());
        drop(cfg);
        assert!(!dir.exists(), "{}", dir.display());
    }

    // Back-to-back callers (same pid, clock may tie) each get their own dir, so neither fails or loses its socket.
    #[test]
    fn owned_configs_started_together_use_distinct_dirs() {
        let tmp = crate::testutil::TempDir::new("mcp-distinct");
        let a = start_owned(tmp.path(), Arc::new(ok_run)).expect("a");
        let b = start_owned(tmp.path(), Arc::new(ok_run)).expect("b");
        assert_ne!(a.path(), b.path());
        assert!(a.path().is_file() && b.path().is_file());
    }

    // A long cwd (deep checkout under notes/) must still get a socket path that binds on macOS (sun_path 104).
    #[test]
    fn socket_path_stays_under_sun_path_for_a_long_cwd() {
        let tmp = crate::testutil::TempDir::new("mcp-long-cwd");
        let cwd = tmp.path().join("long-dir-".repeat(20));
        assert!(cwd.as_os_str().len() > 150);
        let cfg = start_owned(&cwd, Arc::new(ok_run)).expect("start");
        assert!(cfg.path().starts_with(tmp_layout::test_dir(&cwd)));
        let got: Value = serde_json::from_str(&fs::read_to_string(cfg.path()).unwrap()).unwrap();
        let args = &got["mcpServers"]["chaude"]["args"];
        let addr = args[1].as_str().expect("addr");
        #[cfg(unix)]
        {
            assert!(Path::new(addr).is_absolute());
            assert!(addr.len() < SUN_PATH_MAX, "{} bytes: {addr}", addr.len());
            assert!(Path::new(addr).exists(), "socket file: {addr}");
            let sock_dir = Path::new(addr).parent().expect("sock dir").to_path_buf();
            drop(cfg);
            assert!(!sock_dir.exists(), "{}", sock_dir.display());
        }
        // Windows serves loopback TCP (see serve), so addr is "tcp:127.0.0.1:<port>" and no socket file exists.
        #[cfg(not(unix))]
        {
            assert!(addr.starts_with("tcp:127.0.0.1:"), "{addr}");
            drop(cfg);
        }
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

    // A socket path unique to this test process; the test removes it.
    fn sock_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("chaude-mcp-t-{}-{name}.sock", std::process::id()))
    }

    #[test]
    fn loopback_round_trip_through_bridge() {
        let token = new_token();
        let sock = sock_path("roundtrip");
        let addr = serve(Arc::new(ok_run), &token, &sock).expect("serve");
        let out = Arc::new(Mutex::new(Vec::<u8>::new()));
        bridge_with_token(&addr, Some(&token), CALL, Shared(Arc::clone(&out))).expect("bridge");
        let _ = fs::remove_file(&sock);
        let text = String::from_utf8(out.lock().unwrap().clone()).unwrap();
        let got: Value = serde_json::from_str(text.trim()).expect("one reply line");
        assert_eq!(got["id"], 5);
        assert_eq!(got["result"]["content"][0]["text"], "status: done (x)");
    }

    #[cfg(unix)]
    #[test]
    fn unix_socket_is_private_and_needs_no_token() {
        use std::os::unix::fs::PermissionsExt;
        let sock = sock_path("private");
        let addr = serve(Arc::new(ok_run), "unused", &sock).expect("serve");
        assert_eq!(addr, sock.display().to_string());
        let mode = fs::metadata(&sock).expect("socket").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let out = Arc::new(Mutex::new(Vec::<u8>::new()));
        bridge_with_token(&addr, None, CALL, Shared(Arc::clone(&out))).expect("bridge");
        let _ = fs::remove_file(&sock);
        let text = String::from_utf8(out.lock().unwrap().clone()).unwrap();
        assert!(text.contains("status: done (x)"), "{text}");
    }

    #[cfg(not(unix))]
    #[test]
    fn wrong_or_missing_token_gets_no_reply() {
        let addr = serve(Arc::new(ok_run), &new_token(), Path::new("")).expect("serve");
        for token in [Some("not-the-token"), None] {
            let out = Arc::new(Mutex::new(Vec::<u8>::new()));
            // The server drops the connection, so the write side may fail; only the empty output matters.
            let _ = bridge_with_token(&addr, token, CALL, Shared(Arc::clone(&out)));
            assert!(out.lock().unwrap().is_empty(), "{token:?}");
        }
    }
}
