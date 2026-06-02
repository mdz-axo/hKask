//! Audit log for A2A message tracking
//!
//! Provides in-memory and persistent audit logging for ACP messages,
//! with storage port integration for SQLite persistence.
//!
//! Uses canonical `AuditEntry` from `hkask-types`.

pub use hkask_types::{AuditEntry, AuditLogPort, WebID};
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

    /// Create audit log with custom max entries
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
            store: None,
        }
    }

    pub fn with_store(store: Arc<dyn AuditLogPort>) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000,
            store: Some(store),
        }
    }

    pub fn with_max_entries_and_store(max_entries: usize, store: Arc<dyn AuditLogPort>) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
            store: Some(store),
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

    pub async fn get_recent(&self, count: usize) -> Vec<AuditEntry> {
        if let Some(ref store) = self.store {
            return store.query_recent(count);
        }
        let entries = self.entries.read().await;
        entries.iter().rev().take(count).cloned().collect()
    }

    pub async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditEntry> {
        if let Some(ref store) = self.store {
            return store.query_by_actor(webid, count);
        }
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.actor == *webid || e.context.recipient == Some(*webid))
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    pub async fn get_by_correlation(&self, correlation_id: &str) -> Vec<AuditEntry> {
        if let Some(ref store) = self.store {
            return store.query_by_correlation(correlation_id);
        }
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.context.correlation_id.as_deref() == Some(correlation_id))
            .cloned()
            .collect()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}
