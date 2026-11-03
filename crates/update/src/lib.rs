//! Self-update helpers (Go: update command).

pub mod archive;
pub mod fsutil;
pub mod http;

use std::fmt;

/// Update failure. `Offline` means GitHub could not be reached; `Failed` is any other error.
#[derive(Debug)]
pub enum Error {
    Offline(String),
    Failed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Offline(m) | Error::Failed(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}
