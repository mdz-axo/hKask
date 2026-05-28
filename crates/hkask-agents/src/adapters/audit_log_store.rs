//! Audit Log Store Adapter — Bridges hkask_storage::AuditLogStore to AuditLogPort
//!
//! This adapter implements the canonical `AuditLogPort` trait from `hkask-types`
//! using the SQL-backed `AuditLogStore` from `hkask-storage`.

use hkask_storage::AuditLogStore;
use hkask_types::{AuditEntry, AuditLogPort, WebID};
use std::sync::Arc;

pub struct AuditLogStoreAdapter {
    store: Arc<AuditLogStore>,
}

impl AuditLogStoreAdapter {
    pub fn new(store: Arc<AuditLogStore>) -> Self {
        Self { store }
    }
}

impl AuditLogPort for AuditLogStoreAdapter {
    fn log(&self, entry: AuditEntry) {
        let storage_entry: hkask_storage::AuditEntry = entry.into();
        if let Err(e) = self.store.insert(&storage_entry) {
            tracing::error!(error = %e, "Failed to insert audit entry");
        }
    }

    fn query_recent(&self, limit: usize) -> Vec<AuditEntry> {
        self.store
            .query_recent(limit)
            .map(|v| v.into_iter().map(|e| e.into()).collect())
            .unwrap_or_default()
    }

    fn query_by_actor(&self, actor: &WebID, limit: usize) -> Vec<AuditEntry> {
        self.store
            .query_by_actor(&actor.to_string(), limit)
            .map(|v| v.into_iter().map(|e| e.into()).collect())
            .unwrap_or_default()
    }

    fn query_by_correlation(&self, _correlation_id: &str) -> Vec<AuditEntry> {
        // Storage layer doesn't currently support correlation queries
        // This would require adding a correlation_id column to the schema
        Vec::new()
    }
}
