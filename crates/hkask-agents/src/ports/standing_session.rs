//! Standing Session Port — Hexagonal boundary for standing session persistence

// StandingSessionPortError is cut. The port now returns the storage error directly.
// This eliminates the shallow string-wrapper that duplicated hkask_storage::StandingSessionError.
pub use hkask_storage::standing_session::StandingSessionError as StandingSessionPortError;

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

    fn get_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<MessageRecord>, StandingSessionPortError>;

    fn update_last_active(&self, session_id: &str) -> Result<(), StandingSessionPortError>;
}
