//! Self-update helpers (Go: update command).

pub mod archive;
pub mod cleanup;
pub mod fsutil;
pub mod http;
pub mod update;

#[cfg(test)]
mod tests;

use std::fmt;

pub use cleanup::cleanup_old;
pub use update::{
    Fetch, Options, Version, check_latest, check_latest_with, default_fetch, run, run_cli, text,
};

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
