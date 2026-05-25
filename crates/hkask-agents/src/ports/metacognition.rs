//! Metacognition Port — Hexagonal boundary for metacognition snapshot persistence

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetacognitionPortError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Snapshot not found: {0}")]
    NotFound(i64),
}

#[derive(Debug, Clone)]
pub struct StoredHealthSnapshot {
    pub timestamp: String,
    pub cns_health: String,
    pub critical_alerts: i32,
    pub total_alerts: i32,
    pub variety_counters_json: String,
    pub bot_reports_json: String,
}

/// Port trait for metacognition snapshot persistence
///
/// Implementations:
/// - `MetacognitionStoreAdapter` — Production adapter via SQLite
/// - Mock implementations for testing
pub trait MetacognitionPort: Send + Sync {
    fn save_snapshot(&self, snapshot: &StoredHealthSnapshot)
    -> Result<i64, MetacognitionPortError>;

    fn list_snapshots(
        &self,
        limit: usize,
    ) -> Result<Vec<StoredHealthSnapshot>, MetacognitionPortError>;
}
