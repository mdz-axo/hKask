//! Escalation Queue — Persistent queue for escalated alerts requiring human review.
//!
//! The escalation queue is a Cybernetics (Loop 6) algedonic regulation mechanism.
//! Governed by the Cybernetics loop, which receives CuratorDirectives from Curation
//! and escalation signals from algedonic variety deficit detection.

use crate::{Store, now_rfc3339};
use chrono::{DateTime, Utc};
use hkask_types::{BotID, InfrastructureError, TemplateID};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationEntry {
    pub id: String,
    pub template_id: TemplateID,
    pub bot_id: BotID,
    pub output: String,
    pub confidence: f64,
    pub retry_count: u32,
    pub error_context: String,
    pub created_at: DateTime<Utc>,
    pub status: EscalationStatus,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}

impl EscalationEntry {
    /// Create a pending escalation entry with auto-generated id, timestamps, and defaults.
    pub fn pending(output: String, confidence: f64, error_context: String) -> Self {
        Self {
            id: format!("esc_{}", Uuid::new_v4().simple()),
            template_id: TemplateID::new(),
            bot_id: BotID::new(),
            output,
            confidence,
            retry_count: 0,
            error_context,
            created_at: Utc::now(),
            status: EscalationStatus::Pending,
            resolved_at: None,
            resolved_by: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationStatus {
    Pending,
    Resolved,
    Dismissed,
}

pub struct EscalationQueue {
    conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
}

#[derive(Error, Debug)]
pub enum EscalationError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    #[error("Escalation not found: {0}")]
    NotFound(String),
}

impl_from_rusqlite!(EscalationError, Infra);

impl Store for EscalationQueue {
    fn conn_arc(&self) -> Arc<std::sync::Mutex<rusqlite::Connection>> {
        Arc::clone(&self.conn)
    }

    fn lock_conn(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, InfrastructureError> {
        crate::lock_helpers::lock_mutex(&self.conn)
    }
}

impl EscalationQueue {
    pub fn new(conn: Arc<std::sync::Mutex<rusqlite::Connection>>) -> Result<Self, EscalationError> {
        let queue = Self { conn };
        queue.init()?;
        Ok(queue)
    }

    fn init(&self) -> Result<(), EscalationError> {
        self.lock_conn()?.execute_batch(
            r#"CREATE TABLE IF NOT EXISTS escalations (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                bot_id TEXT NOT NULL,
                output TEXT NOT NULL,
                confidence REAL NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                error_context TEXT,
                created_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                resolved_at TEXT,
                resolved_by TEXT
            )
        "#,
        )?;
        Ok(())
    }

    pub fn add(
        &self,
        template_id: TemplateID,
        bot_id: BotID,
        output: String,
        confidence: f64,
        retry_count: u32,
        error_context: String,
    ) -> Result<String, EscalationError> {
        let id = format!("esc_{}", Uuid::new_v4().simple());
        let now = now_rfc3339();

        self.lock_conn()?.execute(
            r#"INSERT INTO escalations (id, template_id, bot_id, output, confidence, retry_count, error_context, created_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')"#,
            params![
                id,
                template_id.to_string(),
                bot_id.as_uuid().to_string(),
                output,
                confidence,
                retry_count,
                error_context,
                now
            ],
        )?;

        Ok(id)
    }

    pub fn list_pending(&self) -> Result<Vec<EscalationEntry>, EscalationError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"SELECT id, template_id, bot_id, output, confidence, retry_count, error_context, created_at, status, resolved_at, resolved_by
             FROM escalations WHERE status = 'pending' ORDER BY created_at ASC"#
        )?;

        let rows = stmt.query_map([], |row| {
            let bot_uuid_str: String = row.get(2)?;
            let bot_id: BotID = bot_uuid_str.parse().unwrap_or_else(|_| BotID::new());

            Ok(EscalationEntry {
                id: row.get(0)?,
                template_id: row
                    .get::<_, String>(1)?
                    .parse()
                    .unwrap_or_else(|_| TemplateID::new()),
                bot_id,
                output: row.get(3)?,
                confidence: row.get(4)?,
                retry_count: row.get(5)?,
                error_context: row.get(6)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
                status: EscalationStatus::Pending,
                resolved_at: None,
                resolved_by: None,
            })
        })?;

        let mut escalations = Vec::new();
        for esc in rows {
            escalations.push(esc?);
        }
        Ok(escalations)
    }

    pub fn get(&self, id: &str) -> Result<Option<EscalationEntry>, EscalationError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, template_id, bot_id, output, confidence, retry_count, error_context, created_at, status, resolved_at, resolved_by
             FROM escalations WHERE id = ?1"
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            let status_str: String = row.get(8)?;
            let status = match status_str.as_str() {
                "pending" => EscalationStatus::Pending,
                "resolved" => EscalationStatus::Resolved,
                "dismissed" => EscalationStatus::Dismissed,
                _ => EscalationStatus::Pending,
            };

            let bot_uuid_str: String = row.get(2)?;
            let bot_id: BotID = bot_uuid_str.parse().unwrap_or_else(|_| BotID::new());

            let resolved_at: Option<String> = row.get(9)?;
            let resolved_at = resolved_at.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

            Ok(Some(EscalationEntry {
                id: row.get(0)?,
                template_id: row
                    .get::<_, String>(1)?
                    .parse()
                    .unwrap_or_else(|_| TemplateID::new()),
                bot_id,
                output: row.get(3)?,
                confidence: row.get(4)?,
                retry_count: row.get(5)?,
                error_context: row.get(6)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
                status,
                resolved_at,
                resolved_by: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn resolve(&self, id: &str, resolved_by: &str) -> Result<(), EscalationError> {
        let now = now_rfc3339();
        let affected = self.lock_conn()?.execute(
            r#"UPDATE escalations SET status = 'resolved', resolved_at = ?1, resolved_by = ?2 WHERE id = ?3"#,
            params![now, resolved_by, id],
        )?;
        if affected == 0 {
            return Err(EscalationError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn dismiss(&self, id: &str, resolved_by: &str) -> Result<(), EscalationError> {
        let now = now_rfc3339();
        let affected = self.lock_conn()?.execute(
            r#"UPDATE escalations SET status = 'dismissed', resolved_at = ?1, resolved_by = ?2 WHERE id = ?3"#,
            params![now, resolved_by, id],
        )?;
        if affected == 0 {
            return Err(EscalationError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn stats(&self) -> Result<EscalationStats, EscalationError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN status = 'resolved' THEN 1 ELSE 0 END) as resolved,
                SUM(CASE WHEN status = 'dismissed' THEN 1 ELSE 0 END) as dismissed
             FROM escalations"#,
        )?;

        let row = stmt.query_row([], |row| {
            Ok(EscalationStats {
                total: row.get(0)?,
                pending: row.get(1)?,
                resolved: row.get(2)?,
                dismissed: row.get(3)?,
            })
        })?;

        Ok(row)
    }
}

/// Aggregated stats over escalation queue.
///
/// The algedonic channel's value is inversely proportional to its traffic
/// (VSM algedonic paradox). Batching reduces noise while preserving signal fidelity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationBatch {
    pub id: String,
    pub entries: Vec<EscalationEntry>,
    pub domain: String,
    pub created_at: DateTime<Utc>,
    pub threshold: usize,
}

impl EscalationBatch {
    pub fn new(entries: Vec<EscalationEntry>, domain: &str, threshold: usize) -> Self {
        Self {
            id: format!("batch_{}", uuid::Uuid::new_v4().simple()),
            entries,
            domain: domain.to_string(),
            created_at: Utc::now(),
            threshold,
        }
    }

    pub fn summary(&self) -> String {
        let count = self.entries.len();
        let domains: std::collections::HashSet<&str> = self
            .entries
            .iter()
            .map(|e| e.output.split(':').next().unwrap_or("unknown"))
            .collect();
        format!(
            "System attention required: {} escalation(s) across {} domain(s) [{}]",
            count,
            domains.len(),
            self.domain
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationStats {
    pub total: i64,
    pub pending: i64,
    pub resolved: i64,
    pub dismissed: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_queue() -> EscalationQueue {
        let conn = Arc::new(Mutex::new(
            rusqlite::Connection::open_in_memory().expect("in-memory DB"),
        ));
        EscalationQueue::new(conn).expect("init queue")
    }

    // REQ: escalation-rows-001 — resolve on a missing id returns NotFound, not Ok
    #[test]
    fn resolve_missing_id_returns_not_found() {
        let q = make_queue();
        let result = q.resolve("no-such-id", "tester");
        assert!(
            matches!(result, Err(EscalationError::NotFound(_))),
            "expected NotFound, got {:?}",
            result
        );
    }

    // REQ: escalation-rows-002 — dismiss on a missing id returns NotFound
    #[test]
    fn dismiss_missing_id_returns_not_found() {
        let q = make_queue();
        let result = q.dismiss("no-such-id", "tester");
        assert!(
            matches!(result, Err(EscalationError::NotFound(_))),
            "expected NotFound, got {:?}",
            result
        );
    }

    // REQ: escalation-rows-003 — resolve on an existing entry succeeds
    #[test]
    fn resolve_existing_id_succeeds() {
        let q = make_queue();
        let id = q
            .add(
                TemplateID::new(),
                BotID::new(),
                "output".into(),
                0.9,
                0,
                "ctx".into(),
            )
            .expect("add escalation");
        assert!(q.resolve(&id, "tester").is_ok());
    }

    // REQ: escalation-rows-004 — dismiss on an existing entry succeeds
    #[test]
    fn dismiss_existing_id_succeeds() {
        let q = make_queue();
        let id = q
            .add(
                TemplateID::new(),
                BotID::new(),
                "output".into(),
                0.8,
                0,
                "ctx".into(),
            )
            .expect("add escalation");
        assert!(q.dismiss(&id, "tester").is_ok());
    }
}
