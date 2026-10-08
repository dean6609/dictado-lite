//! Parakeet-only engine ownership and cache lifetime.
mod ffi;
mod worker;
pub use ffi::{Backend, Cancellation, Engine};
pub use worker::{Completion, Worker};

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

pub type Result<T> = std::result::Result<T, Error>;
pub const MODEL_BYTES: u64 = 739_508_576;
pub const MODEL_SHA256: &str = "5859f77944efcd8eafa23a6350731960b2b55b2203df51f319665c807d802cc7";

#[derive(Debug)]
pub enum Error {
    Cancelled,
    Message(String),
}
impl Error {
    pub fn message(value: impl std::fmt::Display) -> Self {
        Self::Message(value.to_string())
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("Cancelled"),
            Self::Message(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for Error {}

/// Package/smoke validation. Kept separate from measured native load time.
pub fn verify_model(path: &Path) -> Result<()> {
    let mut file = std::fs::File::open(path).map_err(Error::message)?;
    if file.metadata().map_err(Error::message)?.len() != MODEL_BYTES {
        return Err(Error::message("Unexpected Parakeet model size"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(Error::message)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if format!("{:x}", hash.finalize()) != MODEL_SHA256 {
        return Err(Error::message("Parakeet model SHA-256 mismatch"));
    }
    Ok(())
}
