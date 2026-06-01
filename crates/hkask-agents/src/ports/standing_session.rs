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

/// Bot status report submitted to the standing session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BotReport {
    /// Bot's WebID
    pub bot_id: String,
    /// Bot name (e.g., "cns-curator-bot")
    pub bot_name: String,
    /// Health status
    pub health_status: String,
    /// Current issues
    pub issues: Vec<String>,
    /// CNS spans emitted since last report
    pub span_count: u64,
    /// Energy consumed since last report
    pub energy_consumed: u64,
    /// Algedonic alerts received
    pub alert_count: u32,
    /// Report timestamp
    pub timestamp: String,
}

/// ACP message for standing session routing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcpSessionMessage {
    /// Message ID
    pub id: String,
    /// Sender WebID
    pub from_webid: String,
    /// Target (bot name or "Curator")
    pub target: String,
    /// Message type
    pub message_type: SessionMessageType,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: String,
}

/// Types of messages in the standing session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SessionMessageType {
    /// Hourly status report
    StatusReport,
    /// Event-driven alert
    Alert,
    /// Curator directive
    Directive,
    /// Kata coaching message
    KataDirective,
    /// Metacognition summary
    MetacognitionSummary,
    /// System state update
    SystemState,
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

    /// Submit a bot status report
    fn submit_bot_report(&self, report: &BotReport) -> Result<(), StandingSessionPortError>;

    /// Get bot reports for a specific session
    fn get_bot_reports(
        &self,
        session_id: &str,
        bot_name: &str,
    ) -> Result<Vec<BotReport>, StandingSessionPortError>;

    /// Route an ACP message to the standing session
    fn route_acp_message(
        &self,
        message: &AcpSessionMessage,
    ) -> Result<(), StandingSessionPortError>;
}
