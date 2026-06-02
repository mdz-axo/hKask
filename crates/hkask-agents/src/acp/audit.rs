//! Audit log for A2A message tracking
//!
//! Provides in-memory and persistent audit logging for ACP messages,
//! with storage port integration for SQLite persistence.
//!
//! Uses canonical `AuditEntry` from `hkask-types`.

pub use hkask_types::{AuditEntry, AuditLogPort};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Audit log for A2A message tracking
pub(crate) struct AuditLog {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    max_entries: usize,
    store: Option<Arc<dyn AuditLogPort>>,
}

impl AuditLog {
    /// Create new audit log with default max entries
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000,
            store: None,
        }
    }

    pub async fn log(&self, entry: AuditEntry) {
        if let Some(ref store) = self.store {
            store.log(entry.clone());
        }

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
