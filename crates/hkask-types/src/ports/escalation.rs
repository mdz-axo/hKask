//! Escalation port — decouples agent pods from the concrete EscalationQueue.
//!
//! Lives in hkask-ports because both hkask-agents (consumer) and
//! hkask-storage-escalation (implementor) depend on hkask-ports.

use crate::{BotID, EscalationID, InfrastructureError, TemplateID};
use chrono::Utc;

/// An entry in the escalation queue awaiting human review.
///
/// Mirrors `hkask_storage_escalation::EscalationEntry` but lives at the port
/// boundary so consumers do not depend on hkask-storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EscalationEntry {
    pub id: EscalationID,
    pub template_id: TemplateID,
    pub bot_id: BotID,
    pub output: String,
    pub confidence: f64,
    pub retry_count: u32,
    pub error_context: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: EscalationStatus,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub resolved_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationStatus {
    Pending,
    Resolved,
    Dismissed,
}

/// A batch of escalation entries sharing a domain and trigger threshold.
#[derive(Debug, Clone)]
pub struct EscalationBatch {
    pub id: EscalationID,
    pub entries: Vec<EscalationEntry>,
    pub domain: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub threshold: usize,
}

impl EscalationEntry {
    /// Create a pending escalation entry with auto-generated IDs.
    pub fn pending(output: String, confidence: f64, error_context: String) -> Self {
        Self {
            id: EscalationID::new(),
            template_id: TemplateID::new(),
            bot_id: BotID::new(),
            output,
            confidence,
            retry_count: 0,
            error_context,
            created_at: Utc::now(),
            status: EscalationStatus::Pending,
            resolved_at: None,
            resolved_by: None,
        }
    }
}

impl EscalationBatch {
    pub fn new(entries: Vec<EscalationEntry>, domain: &str, threshold: usize) -> Self {
        Self {
            id: EscalationID::new(),
            entries,
            domain: domain.to_string(),
            created_at: Utc::now(),
            threshold,
        }
    }

    pub fn summary(&self) -> String {
        let count = self.entries.len();
        format!(
            "System attention required: {} escalation(s) across domain [{}]",
            count, self.domain
        )
    }
}

/// Port trait for escalation queue operations.
pub trait EscalationPort: Send + Sync {
    fn list_pending(&self) -> Result<Vec<EscalationEntry>, InfrastructureError>;

    fn get(&self, id: &str) -> Result<Option<EscalationEntry>, InfrastructureError>;

    fn resolve(&self, id: &str, resolved_by: &str) -> Result<(), InfrastructureError>;

    fn dismiss(&self, id: &str, resolved_by: &str) -> Result<(), InfrastructureError>;

    fn persist_batch(&self, batch: &EscalationBatch) -> Result<(), InfrastructureError>;

    fn add(
        &self,
        template_id: TemplateID,
        bot_id: BotID,
        output: String,
        confidence: f64,
        retry_count: u32,
        error_context: String,
    ) -> Result<EscalationID, InfrastructureError>;
}
