//! Audit Log Store Adapter — Bridges hkask_storage::AuditLogStore to AuditLogStoragePort

use crate::ports::{AuditLogStoragePort, AuditLogStoragePortError, AuditStorageEntry};
use hkask_storage::AuditLogStore;
use std::sync::Arc;

pub struct AuditLogStoreAdapter {
    store: Arc<AuditLogStore>,
}

impl AuditLogStoreAdapter {
    pub fn new(store: Arc<AuditLogStore>) -> Self {
        Self { store }
    }
}

impl AuditLogStoragePort for AuditLogStoreAdapter {
    fn insert(&self, entry: &AuditStorageEntry) -> Result<(), AuditLogStoragePortError> {
        let mut storage_entry = hkask_storage::AuditEntry::new(
            &entry.actor_webid,
            &entry.action,
            &entry.resource,
            &entry.outcome,
        );
        if let Some(ref details) = entry.details {
            storage_entry = storage_entry.with_details(details.clone());
        }
        if let Some(ref ip) = entry.ip_address {
            storage_entry = storage_entry.with_ip(ip);
        }
        self.store
            .insert(&storage_entry)
            .map_err(|e| AuditLogStoragePortError::Storage(e.to_string()))
    }

    fn query_recent(
        &self,
        limit: usize,
    ) -> Result<Vec<AuditStorageEntry>, AuditLogStoragePortError> {
        self.store
            .query_recent(limit)
            .map(|v| v.into_iter().map(storage_to_domain).collect())
            .map_err(|e| AuditLogStoragePortError::Storage(e.to_string()))
    }

    fn query_by_actor(
        &self,
        actor_webid: &str,
        limit: usize,
    ) -> Result<Vec<AuditStorageEntry>, AuditLogStoragePortError> {
        self.store
            .query_by_actor(actor_webid, limit)
            .map(|v| v.into_iter().map(storage_to_domain).collect())
            .map_err(|e| AuditLogStoragePortError::Storage(e.to_string()))
    }
}

fn storage_to_domain(e: hkask_storage::AuditEntry) -> AuditStorageEntry {
    AuditStorageEntry {
        id: e.id,
        timestamp: e.timestamp.timestamp(),
        actor_webid: e.actor_webid,
        action: e.action,
        resource: e.resource,
        outcome: e.outcome,
        details: e.details,
        ip_address: e.ip_address,
    }
}
