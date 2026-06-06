//! Audit Log Port — Re-exports canonical types from hkask-types
//!
//! This module re-exports the canonical `AuditEntry` and `AuditOutcome` from
//! `hkask-types::audit` to eliminate duplication across the codebase.

pub use hkask_types::audit::{AuditEntry, AuditOutcome};
