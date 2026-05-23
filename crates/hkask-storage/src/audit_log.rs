//! AuditLogStore — Persistent SQL-backed audit log

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum AuditLogError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

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

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_ip(mut self, ip: &str) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }
}

pub struct AuditLogStore {
    conn: Arc<Mutex<Connection>>,
    max_entries: usize,
}

impl AuditLogStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            max_entries: 100_000,
        }
    }

    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    pub fn insert(&self, entry: &AuditEntry) -> Result<(), AuditLogError> {
        let conn = self.conn.lock().unwrap();
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
                entry.details.as_ref().map(|v| serde_json::to_string(v).ok()).flatten(),
                entry.ip_address,
            ],
        )?;
        Ok(())
    }

    pub fn query_recent(&self, limit: usize) -> Result<Vec<AuditEntry>, AuditLogError> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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

    pub fn prune_retain_last(&self, keep: usize) -> Result<usize, AuditLogError> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute(
            "DELETE FROM audit_log WHERE id NOT IN (SELECT id FROM audit_log ORDER BY timestamp DESC LIMIT ?1)",
            rusqlite::params![keep as i64],
        )?;
        Ok(deleted)
    }

    pub fn count(&self) -> Result<usize, AuditLogError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))?;
        Ok(count as usize)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> AuditLogStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_log (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, actor_webid TEXT NOT NULL, action TEXT NOT NULL, resource TEXT NOT NULL, outcome TEXT NOT NULL, details TEXT, ip_address TEXT, created_at TEXT DEFAULT (datetime('now')));",
        ).unwrap();
        AuditLogStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_insert_and_query() {
        let store = test_store();
        let entry = AuditEntry::new("webid-1", "tool:invoke", "tool:search", "success");
        store.insert(&entry).unwrap();

        let results = store.query_recent(10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, "tool:invoke");
    }

    #[test]
    fn test_query_by_actor() {
        let store = test_store();
        store
            .insert(&AuditEntry::new("webid-1", "read", "file:a", "ok"))
            .unwrap();
        store
            .insert(&AuditEntry::new("webid-2", "write", "file:b", "ok"))
            .unwrap();
        store
            .insert(&AuditEntry::new("webid-1", "delete", "file:c", "ok"))
            .unwrap();

        let results = store.query_by_actor("webid-1", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_prune_retain_last() {
        let store = test_store();
        for i in 0..20 {
            store
                .insert(&AuditEntry::new(
                    "webid-1",
                    &format!("action-{}", i),
                    "res",
                    "ok",
                ))
                .unwrap();
        }
        assert_eq!(store.count().unwrap(), 20);

        let deleted = store.prune_retain_last(10).unwrap();
        assert_eq!(deleted, 10);
        assert_eq!(store.count().unwrap(), 10);
    }
}
