//! Single-instance guard. The first launch takes an exclusive lock on a lock file in the user's state folder and
//! listens on a local socket next to it (a unix socket; a named pipe on Windows). A second launch that cannot take the
//! lock sends "focus" over that socket and exits. No TCP port is used, so a busy port or another user cannot break it.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const LOCK_FILE: &str = "gui.lock";

// The first launch's hold: the lock (open until the process exits) and the listener that serves focus requests.
pub struct Server {
    lock: File,
    listener: Listener,
}

pub enum Claim {
    // This is the first launch; hand the server to `serve`.
    First(Server),
    // Another launch holds the guard; it has been asked to focus.
    Second,
}

// The guard folder of this user: the chaude state folder's gui subfolder.
pub fn dir() -> PathBuf {
    chaude_core::osenv::state_dir().join("gui")
}

// Takes the guard in `dir`, or asks the running launch to focus. Err is a real failure (no lock file, no socket).
pub fn claim(dir: &Path) -> io::Result<Claim> {
    fs::create_dir_all(dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(dir.join(LOCK_FILE))?;
    match lock.try_lock() {
        Ok(()) => Ok(Claim::First(Server {
            listener: bind(dir)?,
            lock,
        })),
        Err(TryLockError::WouldBlock) => {
            focus(dir)?;
            Ok(Claim::Second)
        }
        Err(TryLockError::Error(e)) => Err(e),
    }
}

// The first launch may hold the lock a moment before it listens, so the focus request is retried briefly.
fn focus(dir: &Path) -> io::Result<()> {
    let mut last = io::Error::other("no answer from the running launch");
    for _ in 0..20 {
        match send_focus(dir) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = e;
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Err(last)
}

// Runs on a thread: calls `on_focus` for each focus request until the process exits. The lock stays with the thread.
pub fn serve(server: Server, on_focus: impl Fn() + Send + 'static) {
    let Server { lock, listener } = server;
    std::thread::spawn(move || {
        let _lock = lock;
        accept_loop(listener, on_focus);
    });
}

// True when the stream's first line is the focus request.
fn reads_focus(stream: impl Read) -> bool {
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).is_ok() && line.trim() == "focus"
}

#[cfg(unix)]
const SOCKET_FILE: &str = "gui.sock";

#[cfg(unix)]
type Listener = std::os::unix::net::UnixListener;

#[cfg(unix)]
fn bind(dir: &Path) -> io::Result<Listener> {
    // Only the lock holder gets here, so a socket file left by a killed launch is stale.
    let path = dir.join(SOCKET_FILE);
    let _ = fs::remove_file(&path);
    Listener::bind(path)
}

#[cfg(unix)]
fn send_focus(dir: &Path) -> io::Result<()> {
    std::os::unix::net::UnixStream::connect(dir.join(SOCKET_FILE))?.write_all(b"focus\n")
}

#[cfg(unix)]
fn accept_loop(listener: Listener, on_focus: impl Fn()) {
    for stream in listener.incoming().flatten() {
        if reads_focus(stream) {
            on_focus();
        }
    }
}

// The pipe name is per user, since the pipe namespace is shared by the whole machine.
#[cfg(windows)]
pub struct Listener {
    path: String,
}

#[cfg(windows)]
fn bind(_dir: &Path) -> io::Result<Listener> {
    let user: String = std::env::var("USERNAME")
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    Ok(Listener {
        path: format!(r"\\.\pipe\chaude-gui-{user}"),
    })
}

#[cfg(windows)]
fn send_focus(dir: &Path) -> io::Result<()> {
    let path = bind(dir)?.path;
    OpenOptions::new()
        .write(true)
        .open(path)?
        .write_all(b"focus\n")
}

#[cfg(windows)]
fn accept_loop(listener: Listener, on_focus: impl Fn()) {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_INBOUND;
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
        PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };

    let name: Vec<u16> = listener.path.encode_utf16().chain(Some(0)).collect();
    loop {
        // SAFETY: `name` is NUL-terminated and outlives the call; the security attributes are null (the default).
        let h = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_INBOUND,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                0,
                64,
                0,
                std::ptr::null(),
            )
        };
        if h == INVALID_HANDLE_VALUE {
            eprintln!("guard: could not create the focus pipe");
            return;
        }
        // SAFETY: `h` is a pipe handle this loop owns; the File closes it when dropped. A client that connected
        // before this call is reported as an error here, and its data is still readable.
        unsafe { ConnectNamedPipe(h, std::ptr::null_mut()) };
        let pipe = unsafe { File::from_raw_handle(h) };
        if reads_focus(pipe) {
            on_focus();
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn second_launch_focuses_the_first() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "cg-{}-{}",
            std::process::id(),
            nanos % 1_000_000_000
        ));
        let Claim::First(server) = claim(&dir).unwrap() else {
            panic!("the first launch should take the guard");
        };
        let (tx, rx) = std::sync::mpsc::channel();
        serve(server, move || {
            let _ = tx.send(());
        });
        assert!(matches!(claim(&dir).unwrap(), Claim::Second));
        assert!(rx.recv_timeout(Duration::from_secs(2)).is_ok());
        // One connection is one focus request: no extra callbacks follow.
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(rx.try_iter().count(), 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
