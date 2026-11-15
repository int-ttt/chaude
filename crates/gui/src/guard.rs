//! Single-instance guard. The first launch holds a loopback port; a second launch sends "focus" to it and exits.

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpListener, TcpStream};

// The loopback port of the guard. Fixed, so a second launch finds the first one.
pub const GUARD_ADDR: &str = "127.0.0.1:47613";

pub enum Claim {
    // This is the first launch; the listener must be handed to `serve`.
    First(TcpListener),
    // Another launch holds the guard; it has been asked to focus.
    Second,
}

// Takes the guard, or asks the running launch to focus. Err is a real failure (the port is taken by something else).
pub fn claim() -> std::io::Result<Claim> {
    match TcpListener::bind(GUARD_ADDR) {
        Ok(listener) => Ok(Claim::First(listener)),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            TcpStream::connect(GUARD_ADDR)?.write_all(b"focus\n")?;
            Ok(Claim::Second)
        }
        Err(e) => Err(e),
    }
}

// Runs on a thread: calls `on_focus` for each focus request until the process exits.
pub fn serve(listener: TcpListener, on_focus: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(stream).read_line(&mut line).is_ok() && line.trim() == "focus" {
                on_focus();
            }
        }
    });
}
