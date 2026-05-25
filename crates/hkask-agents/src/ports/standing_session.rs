//! Standing Session Port — Hexagonal boundary for standing session persistence

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StandingSessionPortError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Session not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub session_id: String,
    pub config_yaml: String,
    pub created_at: String,
    pub last_active: String,
}

#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub id: i64,
    pub session_id: String,
    pub from_webid: String,
    pub content: String,
    pub timestamp: String,
    pub template_id: Option<String>,
}

/// Port trait for standing session persistence
///
/// Implementations:
/// - `StandingSessionStoreAdapter` — Production adapter via SQLite
/// - Mock implementations for testing
pub trait StandingSessionPort: Send + Sync {
    fn save_session(&self, session: &SessionRecord) -> Result<(), StandingSessionPortError>;

    fn get_session(&self, session_id: &str) -> Result<SessionRecord, StandingSessionPortError>;

    fn save_message(&self, message: &MessageRecord) -> Result<i64, StandingSessionPortError>;

    fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRecord>, StandingSessionPortError>;

    fn update_last_active(&self, session_id: &str) -> Result<(), StandingSessionPortError>;
}
