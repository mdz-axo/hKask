//! AuditLogStore — Persistent SQL-backed audit log
//!
//! This module provides SQL persistence for audit entries.
//! It maintains a storage-layer `AuditEntry` type optimized for SQL serialization,
//! with conversion methods to/from the canonical `hkask_types::AuditEntry`.

use chrono::{DateTime, Utc};
use hkask_types::InfrastructureError;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum AuditLogError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
}

impl From<rusqlite::Error> for AuditLogError {
    fn from(e: rusqlite::Error) -> Self {
        AuditLogError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

/// Storage-layer audit entry optimized for SQL serialization
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor_webid: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub details: Option<Value>,
    pub ip_address: Option<String>,
}

impl AuditEntry {
    pub fn new(actor_webid: &str, action: &str, resource: &str, outcome: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor_webid: actor_webid.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            outcome: outcome.to_string(),
            details: None,
            ip_address: None,
        }
    }
}

/// Convert canonical AuditEntry to storage AuditEntry
impl From<hkask_types::AuditEntry> for AuditEntry {
    fn from(canonical: hkask_types::AuditEntry) -> Self {
        Self {
            id: canonical.id,
            timestamp: canonical.timestamp,
            actor_webid: canonical.actor.to_string(),
            action: canonical.action,
            resource: canonical.resource,
            outcome: canonical.outcome.to_string(),
            details: if canonical.context.metadata.is_null() {
                None
            } else {
                Some(canonical.context.metadata)
            },
            ip_address: canonical.context.ip_address,
        }
    }
}

/// Convert storage AuditEntry to canonical AuditEntry
impl From<AuditEntry> for hkask_types::AuditEntry {
    fn from(storage: AuditEntry) -> Self {
        use hkask_types::{AuditContext, AuditOutcome, WebID};

        Self {
            id: storage.id,
            timestamp: storage.timestamp,
            actor: WebID::from_string(&storage.actor_webid),
            action: storage.action,
            resource: storage.resource,
            outcome: storage.outcome.parse().unwrap_or(AuditOutcome::Success),
            context: AuditContext {
                correlation_id: None,
                recipient: None,
                ip_address: storage.ip_address,
                error_message: None,
                metadata: storage.details.unwrap_or(Value::Null),
            },
        }
    }
}

pub struct AuditLogStore {
    conn: Arc<Mutex<Connection>>,
}

impl AuditLogStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn insert(&self, entry: &AuditEntry) -> Result<(), AuditLogError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        conn.execute(
            "INSERT INTO audit_log (id, timestamp, actor_webid, action, resource, outcome, details, ip_address)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.id,
                entry.timestamp.to_rfc3339(),
                entry.actor_webid,
                entry.action,
                entry.resource,
                entry.outcome,
                entry.details.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                entry.ip_address,
            ],
        )?;
        Ok(())
    }

    pub fn query_recent(&self, limit: usize) -> Result<Vec<AuditEntry>, AuditLogError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, actor_webid, action, resource, outcome, details, ip_address
             FROM audit_log
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let entries = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(AuditRow {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    actor_webid: row.get(2)?,
                    action: row.get(3)?,
                    resource: row.get(4)?,
                    outcome: row.get(5)?,
                    details: row.get(6)?,
                    ip_address: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .filter_map(|row| row_to_entry(row).ok())
            .collect();

        Ok(entries)
    }

    pub fn query_by_actor(
        &self,
        actor_webid: &str,
        limit: usize,
    ) -> Result<Vec<AuditEntry>, AuditLogError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, actor_webid, action, resource, outcome, details, ip_address
             FROM audit_log
             WHERE actor_webid = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(rusqlite::params![actor_webid, limit as i64], |row| {
                Ok(AuditRow {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    actor_webid: row.get(2)?,
                    action: row.get(3)?,
                    resource: row.get(4)?,
                    outcome: row.get(5)?,
                    details: row.get(6)?,
                    ip_address: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .filter_map(|row| row_to_entry(row).ok())
            .collect();

        Ok(entries)
    }
}

fn row_to_entry(row: AuditRow) -> Result<AuditEntry, AuditLogError> {
    let timestamp = DateTime::parse_from_rfc3339(&row.timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let details: Option<Value> = row.details.and_then(|s| serde_json::from_str(&s).ok());

    Ok(AuditEntry {
        id: row.id,
        timestamp,
        actor_webid: row.actor_webid,
        action: row.action,
        resource: row.resource,
        outcome: row.outcome,
        details,
        ip_address: row.ip_address,
    })
}

struct AuditRow {
    id: String,
    timestamp: String,
    actor_webid: String,
    action: String,
    resource: String,
    outcome: String,
    details: Option<String>,
    ip_address: Option<String>,
}
