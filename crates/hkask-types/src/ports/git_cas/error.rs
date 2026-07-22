//! Git CAS error type — distinct recovery paths for each variant.

use crate::NotFound;
use serde::{Deserialize, Serialize};

/// Errors from Git CAS port operations.
///
/// Each variant has a distinct recovery path (C5: every error variant = unique recovery):
/// - `CrateNotFound` → create the repo
/// - `Io` → retry or check filesystem permissions
/// - `Git` → inspect git state, possibly reinitialize
/// - `PathValidation` → reject the request, possible attack
/// - `ContentHashMismatch` → re-download or restore from backup
/// - `NotFound` → create the blob first, or check the hash
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitCasError {
    #[error("Crate not found: {0}")]
    CrateNotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Path validation error: {0}")]
    PathValidation(String),

    #[error("Content hash mismatch: expected {expected}, got {actual}")]
    ContentHashMismatch { expected: String, actual: String },

    #[error("Not found: {0}")]
    NotFound(NotFound),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl From<NotFound> for GitCasError {
    fn from(nf: NotFound) -> Self {
        GitCasError::NotFound(nf)
    }
}
