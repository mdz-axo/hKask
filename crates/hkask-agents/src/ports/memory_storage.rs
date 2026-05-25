//! Memory Storage Port — Hexagonal boundary for artifact persistence

use hkask_types::{CapabilityToken, WebID};

/// Port trait for memory storage operations
///
/// Implementations:
/// - `MemoryStorageAdapter` — Production adapter via SQLite
/// - Mock implementations for testing
pub trait MemoryStoragePort: Send + Sync {
    fn store_artifact(
        &self,
        producer_webid: WebID,
        artifact_type: &str,
        content: serde_json::Value,
        visibility: &str,
        token: &CapabilityToken,
    ) -> Result<String, crate::error::MemoryError>;

    fn recall(
        &self,
        query: &str,
        token: &CapabilityToken,
    ) -> Result<Vec<serde_json::Value>, crate::error::MemoryError>;
}
