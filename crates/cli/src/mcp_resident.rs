//! `chaude mcp-resident <socket>`: the stdio side of the resident coder MCP tool (Go MCPResidentBridge).
//! The main child runs this as its MCP server; it copies lines between its stdio and the socket that
//! core's mcp::serve listens on: a unix socket path on unix, or tcp:<addr> (Windows).

use std::io;

use chaude_core::mcp;

// Go MCPResidentBridge exit codes: 2 for a usage error, 1 when the socket cannot be reached, 0 otherwise.
pub fn main(args: &[String]) -> i32 {
    let [addr] = args else {
        eprintln!("usage: chaude mcp-resident <socket>");
        return 2;
    };
    match mcp::bridge(addr, io::stdin(), io::stdout()) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("mcp-resident: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    // Output sink that stays readable after bridge consumes it (Send + 'static, unlike a borrowed buffer).
    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn usage_errors_exit_2() {
        assert_eq!(main(&[]), 2);
        assert_eq!(main(&["a".to_string(), "b".to_string()]), 2);
    }

    #[test]
    fn missing_unix_socket_exits_1() {
        let sock = std::env::temp_dir().join(format!("chaude-no-such-{}.sock", std::process::id()));
        assert_eq!(main(&[sock.display().to_string()]), 1);
    }

    #[test]
    fn unreachable_socket_exits_1() {
        // Bind then drop, so the port is closed when the bridge dials it.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        assert_eq!(main(&[format!("tcp:127.0.0.1:{port}")]), 1);
    }

    // Stdin lines reach the server, and the server reply comes back out the bridge.
    #[test]
    fn bridge_copies_lines_both_ways() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut got = String::new();
            conn.read_to_string(&mut got).unwrap(); // EOF from the bridge's write shutdown
            conn.write_all(format!("reply:{got}").as_bytes()).unwrap();
        });
        let out = Sink::default();
        let input: &[u8] = b"{\"id\":1}\n";
        mcp::bridge(&format!("tcp:{addr}"), input, out.clone()).unwrap();
        server.join().unwrap();
        let text = String::from_utf8(out.0.lock().unwrap().clone()).unwrap();
        assert_eq!(text, "reply:{\"id\":1}\n");
    }
}
