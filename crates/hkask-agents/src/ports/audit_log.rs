//! Audit Log Port — Hexagonal boundary for audit persistence

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditLogPortError {
    #[error("Storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub id: String,
    pub agent_webid: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub timestamp: i64,
    pub details: Option<serde_json::Value>,
}

/// Port trait for audit log persistence
///
/// Implementations:
/// - `AuditLogStore` — Production adapter via SQLite
/// - Mock implementations for testing
pub trait AuditLogPort: Send + Sync {
    fn log(&self, entry: AuditEntry) -> Result<(), AuditLogPortError>;

    fn get_recent(&self, limit: usize) -> Result<Vec<AuditEntry>, AuditLogPortError>;

    fn get_by_webid(&self, webid: &str, limit: usize) -> Result<Vec<AuditEntry>, AuditLogPortError>;
}
