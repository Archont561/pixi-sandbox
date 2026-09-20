//! Error type for the core crate.
//!
//! The variants are chosen so that the messages an airlock operator sees are actionable:
//! which file, what was expected, what was found. Integrity problems are never warnings.

use std::path::Path;

/// Convenience alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("integrity check failed for {path}: expected sha256 {expected}, got {actual}")]
    Integrity {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("{path} has the wrong size: expected {expected} bytes, got {actual}")]
    SizeMismatch {
        path: String,
        expected: u64,
        actual: u64,
    },

    #[error("missing split part {0}")]
    MissingPart(String),

    #[error("{path} is not a valid manifest: {reason}")]
    InvalidManifest { path: String, reason: String },

    #[error("{0}")]
    Invalid(String),

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// `std::io::Error` + the path it happened on, in one call.
    pub fn io(path: &Path, source: std::io::Error) -> Self {
        Error::Io {
            path: path.display().to_string(),
            source,
        }
    }

    /// True for the errors that mean "this transport cannot be trusted".
    pub fn is_integrity(&self) -> bool {
        matches!(
            self,
            Error::Integrity { .. } | Error::SizeMismatch { .. } | Error::MissingPart(_)
        )
    }
}
