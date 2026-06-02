//! Canonical audit entry types for hKask
//!
//! This module provides unified audit logging types used across all crates,
//! eliminating duplication in:
//! - hkask-agents/src/acp/audit.rs (AuditLogEntry)
//! - hkask-agents/src/ports/audit_log.rs (AuditEntry)
//! - hkask-agents/src/ports/audit_log_storage.rs (AuditStorageEntry)
//! - hkask-storage/src/audit_log.rs (AuditEntry)
//! - hkask-mcp/src/security.rs (AuditEntry)

use crate::WebID;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unified audit entry for all hKask operations
///
/// This consolidates 5 duplicate audit entry types into a single canonical structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry identifier
    pub id: String,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Actor WebID (who performed the action)
    pub actor: WebID,
    /// Action performed (e.g., "template_dispatch", "tool_invoke")
    pub action: String,
    /// Resource affected (e.g., template ID, tool name)
    pub resource: String,
    /// Outcome (success, failure, denied)
    pub outcome: AuditOutcome,
    /// Additional context/metadata
    pub context: AuditContext,
}

/// Audit outcome classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
    Denied,
    Error,
}

impl std::fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditOutcome::Success => write!(f, "success"),
            AuditOutcome::Failure => write!(f, "failure"),
            AuditOutcome::Denied => write!(f, "denied"),
            AuditOutcome::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for AuditOutcome {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "success" => Ok(AuditOutcome::Success),
            "failure" => Ok(AuditOutcome::Failure),
            "denied" => Ok(AuditOutcome::Denied),
            "error" => Ok(AuditOutcome::Error),
            _ => Err(format!("Invalid audit outcome: {}", s)),
        }
    }
}

/// Additional context for audit entries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditContext {
    /// Correlation ID for distributed tracing
    pub correlation_id: Option<String>,
    /// Recipient WebID (for message passing)
    pub recipient: Option<WebID>,
    /// IP address (for network operations)
    pub ip_address: Option<String>,
    /// Error message (if outcome is failure/error)
    pub error_message: Option<String>,
    /// Arbitrary metadata
    pub metadata: serde_json::Value,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(
        actor: WebID,
        action: impl Into<String>,
        resource: impl Into<String>,
        outcome: AuditOutcome,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor,
            action: action.into(),
            resource: resource.into(),
            outcome,
            context: AuditContext::default(),
        }
    }

    /// Add correlation ID
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.context.correlation_id = Some(correlation_id.into());
        self
    }

    /// Add recipient
    pub fn with_recipient(mut self, recipient: WebID) -> Self {
        self.context.recipient = Some(recipient);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.context.metadata = metadata;
        self
    }
}

/// Audit log port trait (hexagonal architecture boundary)
///
/// Implementations:
/// - In-memory buffer (for testing)
/// - SQLite persistence (production)
/// - External audit systems
pub trait AuditLogPort: Send + Sync {
    /// Record an audit entry
    fn log(&self, entry: AuditEntry);

    /// Query recent entries
    fn query_recent(&self, limit: usize) -> Vec<AuditEntry>;

    /// Query entries by actor
    fn query_by_actor(&self, actor: &WebID, limit: usize) -> Vec<AuditEntry>;

    /// Query entries by correlation ID
    fn query_by_correlation(&self, correlation_id: &str) -> Vec<AuditEntry>;
}
