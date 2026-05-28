//! Audit log for A2A message tracking
//!
//! Provides in-memory and persistent audit logging for ACP messages,
//! with storage port integration for SQLite persistence.

use crate::ports::{AuditLogStoragePort, AuditStorageEntry};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Audit log entry for A2A messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique entry identifier
    pub id: String,
    /// Timestamp of the event
    pub timestamp: i64,
    /// Sender WebID
    pub from: WebID,
    /// Recipient WebID (if any)
    pub to: Option<WebID>,
    /// Message type
    pub message_type: String,
    /// Correlation ID
    pub correlation_id: String,
    /// Event type (sent, received, verified, denied)
    pub event_type: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Audit log for A2A message tracking
pub struct AuditLog {
    entries: Arc<RwLock<Vec<AuditLogEntry>>>,
    max_entries: usize,
    store: Option<Arc<dyn AuditLogStoragePort>>,
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

    pub fn with_store(store: Arc<dyn AuditLogStoragePort>) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000,
            store: Some(store),
        }
    }

    pub fn with_max_entries_and_store(
        max_entries: usize,
        store: Arc<dyn AuditLogStoragePort>,
    ) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
            store: Some(store),
        }
    }

    pub async fn log(&self, entry: AuditLogEntry) {
        if let Some(ref store) = self.store {
            let storage_entry = AuditStorageEntry {
                id: entry.id.clone(),
                timestamp: 0,
                actor_webid: entry.from.to_string(),
                action: entry.event_type.clone(),
                resource: entry.message_type.clone(),
                outcome: "success".to_string(),
                details: Some(serde_json::json!({
                    "correlation_id": entry.correlation_id,
                    "to": entry.to.map(|t| t.to_string()),
                    "metadata": entry.metadata,
                })),
                ip_address: None,
            };
            if let Err(e) = store.insert(&storage_entry) {
                tracing::error!(
                    target: "cns.audit.write_failed",
                    error = %e,
                    event_type = %entry.event_type,
                    from = %entry.from,
                    "Audit log storage write failed"
                );
            }
        }

        let mut entries = self.entries.write().await;
        entries.push(entry);

        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }
    }

    pub async fn get_recent(&self, count: usize) -> Vec<AuditLogEntry> {
        if let Some(ref store) = self.store
            && let Ok(storage_entries) = store.query_recent(count)
        {
            return storage_entries
                .into_iter()
                .filter_map(audit_entry_from_port)
                .collect();
        }
        let entries = self.entries.read().await;
        entries.iter().rev().take(count).cloned().collect()
    }

    pub async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry> {
        if let Some(ref store) = self.store
            && let Ok(storage_entries) = store.query_by_actor(&webid.to_string(), count)
        {
            return storage_entries
                .into_iter()
                .filter_map(audit_entry_from_port)
                .collect();
        }
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.from == *webid || e.to == Some(*webid))
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    pub async fn get_by_correlation(&self, correlation_id: &str) -> Vec<AuditLogEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.correlation_id == correlation_id)
            .cloned()
            .collect()
    }
}

fn audit_entry_from_port(e: AuditStorageEntry) -> Option<AuditLogEntry> {
    let details = e.details.as_ref()?;
    let correlation_id = details.get("correlation_id")?.as_str()?.to_string();
    let to = details
        .get("to")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(WebID::from_string);
    let metadata = details
        .get("metadata")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    Some(AuditLogEntry {
        id: e.id,
        timestamp: e.timestamp,
        from: WebID::from_string(&e.actor_webid),
        to,
        message_type: e.resource,
        correlation_id,
        event_type: e.action,
        metadata,
    })
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit log port for external audit systems
#[async_trait::async_trait]
pub trait AuditLogPort: Send + Sync {
    /// Log an A2A message event
    async fn log(&self, entry: AuditLogEntry);

    /// Get recent audit entries
    async fn get_recent(&self, count: usize) -> Vec<AuditLogEntry>;

    /// Query audit log by WebID
    async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry>;
}

#[async_trait::async_trait]
impl AuditLogPort for AuditLog {
    async fn log(&self, entry: AuditLogEntry) {
        if let Some(ref store) = self.store {
            let storage_entry = AuditStorageEntry {
                id: entry.id.clone(),
                timestamp: 0,
                actor_webid: entry.from.to_string(),
                action: entry.event_type.clone(),
                resource: entry.message_type.clone(),
                outcome: "success".to_string(),
                details: Some(serde_json::json!({
                    "correlation_id": entry.correlation_id,
                    "to": entry.to.map(|t| t.to_string()),
                    "metadata": entry.metadata,
                })),
                ip_address: None,
            };
            if let Err(e) = store.insert(&storage_entry) {
                tracing::error!(
                    target: "cns.audit.write_failed",
                    error = %e,
                    event_type = %entry.event_type,
                    from = %entry.from,
                    "Audit log storage write failed (port impl)"
                );
            }
        }

        let mut entries = self.entries.write().await;
        entries.push(entry);

        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }
    }

    async fn get_recent(&self, count: usize) -> Vec<AuditLogEntry> {
        if let Some(ref store) = self.store
            && let Ok(entries) = store.query_recent(count)
        {
            return entries
                .into_iter()
                .filter_map(audit_entry_from_port)
                .collect();
        }

        let entries = self.entries.read().await;
        entries.iter().rev().take(count).cloned().collect()
    }

    async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry> {
        if let Some(ref store) = self.store
            && let Ok(entries) = store.query_by_actor(&webid.to_string(), count)
        {
            return entries
                .into_iter()
                .filter_map(audit_entry_from_port)
                .collect();
        }

        let entries = self.entries.read().await;
        entries
            .iter()
            .rev()
            .filter(|e| &e.from == webid || e.to.as_ref() == Some(webid))
            .take(count)
            .cloned()
            .collect()
    }
}
