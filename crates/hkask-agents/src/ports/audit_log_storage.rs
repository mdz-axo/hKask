//! Audit Log Storage Port — Hexagonal boundary for audit log persistence
//!
//! This port abstracts the persistence layer for audit entries,
//! decoupling `AuditLog` from the concrete `hkask_storage::AuditLogStore`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditLogStoragePortError {
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Domain-native audit entry for storage port
#[derive(Debug, Clone)]
pub struct AuditStorageEntry {
    pub id: String,
    pub timestamp: i64,
    pub actor_webid: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
}

/// Port trait for audit log persistence
///
/// Implementations:
/// - `AuditLogStoreAdapter` — Production adapter via SQLite
/// - Mock implementations for testing
pub trait AuditLogStoragePort: Send + Sync {
    fn insert(&self, entry: &AuditStorageEntry) -> Result<(), AuditLogStoragePortError>;

    fn query_recent(
        &self,
        limit: usize,
    ) -> Result<Vec<AuditStorageEntry>, AuditLogStoragePortError>;

    fn query_by_actor(
        &self,
        actor_webid: &str,
        limit: usize,
    ) -> Result<Vec<AuditStorageEntry>, AuditLogStoragePortError>;
}
