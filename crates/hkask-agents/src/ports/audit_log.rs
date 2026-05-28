//! Audit Log Port — Re-exports canonical types from hkask-types
//!
//! This module re-exports the canonical `AuditEntry` and `AuditLogPort` from
//! `hkask-types::audit` to eliminate duplication across the codebase.

pub use hkask_types::audit::{AuditContext, AuditEntry, AuditLogPort, AuditOutcome};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditLogPortError {
    #[error("Storage error: {0}")]
    Storage(String),
}
