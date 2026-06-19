//! Audit log for A2A message tracking
//
//! Provides in-memory audit logging for A2A messages.
//! Uses canonical `AuditEntry` from `hkask-types`.

use hkask_rsolidity as rs;
pub use hkask_types::AuditEntry;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Audit log for A2A message tracking
pub(crate) struct AuditLog {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    max_entries: usize,
}

impl AuditLog {
    /// Create new audit log with default max entries.
    ///
    /// \[P4\] Motivating: Clear Boundaries — audit log attests capability actions
    /// \[P1\] Constraining: User Sovereignty — every action is attributable to an agent
    ///       `max_entries = 10000`.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000,
        }
    }

    /// \[P4\] Motivating: Clear Boundaries — append-only audit preserves OCAP evidence
    /// \[P8\] Constraining: Semantic Grounding — entries are structured and traceable
    ///       `max_entries`, the oldest entries are drained to stay
    ///       within the limit.
    pub async fn log(&self, entry: AuditEntry) {
        let mut entries = self.entries.write().await;
        entries.push(entry);

        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}
