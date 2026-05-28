//! Audit Log Storage Port — Re-exports canonical types from hkask-types
//!
//! This port re-exports the canonical `AuditEntry` and `AuditLogPort` from
//! `hkask-types::audit` to eliminate duplication across the codebase.

pub use hkask_types::audit::{AuditContext, AuditEntry, AuditLogPort, AuditOutcome};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditLogStoragePortError {
    #[error("Storage error: {0}")]
    Storage(String),
}
