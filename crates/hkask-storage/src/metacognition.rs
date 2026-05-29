//! MetacognitionStore — Persistent storage for metacognition snapshots

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetacognitionError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Snapshot not found: {0}")]
    NotFound(i64),
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSnapshot {
    pub id: i64,
    pub timestamp: String,
    pub cns_health: String,
    pub critical_alerts: i32,
    pub total_alerts: i32,
    pub variety_counters_json: String,
    pub bot_reports_json: String,
}

#[derive(Clone)]
pub struct MetacognitionStore {
    conn: Arc<Mutex<Connection>>,
}

impl MetacognitionStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<(), MetacognitionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MetacognitionError::LockPoisoned(e.to_string()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS metacognition_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                cns_health TEXT NOT NULL,
                critical_alerts INTEGER NOT NULL,
                total_alerts INTEGER NOT NULL,
                variety_counters_json TEXT NOT NULL,
                bot_reports_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_metacognition_timestamp ON metacognition_snapshots(timestamp);",
        )?;
        Ok(())
    }

    pub fn save_snapshot(&self, snapshot: &StoredSnapshot) -> Result<i64, MetacognitionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MetacognitionError::LockPoisoned(e.to_string()))?;
        conn.execute(
            "INSERT INTO metacognition_snapshots (timestamp, cns_health, critical_alerts, total_alerts, variety_counters_json, bot_reports_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                snapshot.timestamp,
                snapshot.cns_health,
                snapshot.critical_alerts,
                snapshot.total_alerts,
                snapshot.variety_counters_json,
                snapshot.bot_reports_json,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_snapshot(&self, id: i64) -> Result<StoredSnapshot, MetacognitionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MetacognitionError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, cns_health, critical_alerts, total_alerts, variety_counters_json, bot_reports_json
             FROM metacognition_snapshots WHERE id = ?1",
        )?;

        let snapshot = stmt
            .query_row(rusqlite::params![id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, i32>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|_| MetacognitionError::NotFound(id))?;

        Ok(StoredSnapshot {
            id: snapshot.0,
            timestamp: snapshot.1,
            cns_health: snapshot.2,
            critical_alerts: snapshot.3,
            total_alerts: snapshot.4,
            variety_counters_json: snapshot.5,
            bot_reports_json: snapshot.6,
        })
    }

    pub fn list_snapshots(&self, limit: usize) -> Result<Vec<StoredSnapshot>, MetacognitionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MetacognitionError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, cns_health, critical_alerts, total_alerts, variety_counters_json, bot_reports_json
             FROM metacognition_snapshots ORDER BY timestamp DESC LIMIT ?1",
        )?;

        let snapshots = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, i32>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|s| StoredSnapshot {
                id: s.0,
                timestamp: s.1,
                cns_health: s.2,
                critical_alerts: s.3,
                total_alerts: s.4,
                variety_counters_json: s.5,
                bot_reports_json: s.6,
            })
            .collect();

        Ok(snapshots)
    }

    pub fn delete_old_snapshots(&self, days_to_keep: i64) -> Result<usize, MetacognitionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MetacognitionError::LockPoisoned(e.to_string()))?;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days_to_keep);
        let deleted = conn.execute(
            "DELETE FROM metacognition_snapshots WHERE timestamp < ?1",
            rusqlite::params![cutoff.to_rfc3339()],
        )?;
        Ok(deleted)
    }
}
