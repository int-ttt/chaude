//! The core error type. Modules move from `Result<_, String>` to `Result<_, Error>` one at a time.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    /// A message shown to the user as is.
    Msg(String),
    /// An I/O failure that has no extra context.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Msg(m) => f.write_str(m),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}
