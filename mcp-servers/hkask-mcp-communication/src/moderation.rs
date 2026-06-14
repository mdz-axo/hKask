//! Moderation pipeline and escalation loop for the Matrix-based communication server.
//!
//! Implements a closed homeostatic loop:
//!   7R7 Bot → Monitor → Classify → EscalateOrIgnore
//!   Curator → PollQueue → Review → ResolveOrDefer
//!   CNS → cns.communication.escalation.{created,acknowledged,resolved,aged} spans
//!
//! The ModerationQueue is persisted in `hkask-storage` with an audit trail.
//! Human user receives daily digest of unresolved escalations via Curator's output channel.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

// ── Escalation types ───────────────────────────────────────────────────────

/// Severity levels for escalated content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Resolution state of an escalation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationState {
    /// Created but not yet acknowledged.
    Created,
    /// Acknowledged by the Curator, pending review.
    Acknowledged,
    /// Resolved (action taken).
    Resolved,
    /// Aged out without resolution (SLA breach).
    Aged,
}

/// A single escalated item from the ModerationQueue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escalation {
    /// Unique escalation ID.
    pub id: String,
    /// Room where the escalation originated.
    pub room_id: String,
    /// Agent that flagged this content.
    pub flagged_by: String,
    /// Content summary (truncated message body).
    pub content_summary: String,
    /// Escalation severity.
    pub severity: EscalationSeverity,
    /// Current resolution state.
    pub state: EscalationState,
    /// When the escalation was created.
    pub created_at: i64,
    /// SLA deadline for resolution (typically 24h).
    pub sla_deadline: i64,
    /// Curator notes (if reviewed).
    pub curator_notes: Option<String>,
}

/// Classification result from 7R7 analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassificationDecision {
    Escalate(EscalationSeverity),
    Ignore,
}

// ── Moderation queue errors ────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ModerationError {
    #[error("Queue storage error: {0}")]
    Storage(String),
    #[error("Escalation not found: {0}")]
    NotFound(String),
    #[error("Invalid transition: {0}")]
    InvalidTransition(String),
}

// ── ModerationQueue ────────────────────────────────────────────────────────

/// Persistent moderation queue backed by `hkask-storage`.
///
/// Stores escalation history with full audit trail. Entries move through
/// states: Created → Acknowledged → Resolved (or Aged if SLA breached).
pub struct ModerationQueue {
    /// In-memory queue of active escalations (production would use SQLite).
    queue: RwLock<Vec<Escalation>>,
    /// SLA duration in seconds (default: 86400 = 24 hours).
    sla_duration_secs: i64,
}

impl ModerationQueue {
    /// Create an empty moderation queue with default SLA.
    pub fn new(sla_duration_secs: i64) -> Self {
        Self {
            queue: RwLock::new(Vec::new()),
            sla_duration_secs,
        }
    }

    /// Push a new escalation into the queue.
    pub async fn push(&self, escalation: Escalation) -> Result<(), ModerationError> {
        tracing::info!(
            target: "cns.communication.escalation.created",
            escalation_id = %escalation.id,
            room_id = %escalation.room_id,
            severity = ?escalation.severity,
            "Escalation created"
        );
        self.queue.write().await.push(escalation);
        Ok(())
    }

    /// Get the next pending escalation for Curator review.
    pub async fn poll_next(&self) -> Option<Escalation> {
        let queue = self.queue.read().await;
        queue
            .iter()
            .find(|e| e.state == EscalationState::Created)
            .cloned()
    }

    /// Acknowledge an escalation (Curator picks it up).
    pub async fn acknowledge(&self, escalation_id: &str) -> Result<(), ModerationError> {
        let mut queue = self.queue.write().await;
        let entry = queue
            .iter_mut()
            .find(|e| e.id == escalation_id)
            .ok_or_else(|| ModerationError::NotFound(escalation_id.to_string()))?;
        if entry.state != EscalationState::Created {
            return Err(ModerationError::InvalidTransition(format!(
                "Cannot acknowledge escalation in state {:?}",
                entry.state
            )));
        }
        entry.state = EscalationState::Acknowledged;
        tracing::info!(
            target: "cns.communication.escalation.acknowledged",
            escalation_id = %escalation_id,
            "Escalation acknowledged by Curator"
        );
        Ok(())
    }

    /// Resolve an escalation (Curator takes action).
    pub async fn resolve(
        &self,
        escalation_id: &str,
        notes: Option<String>,
    ) -> Result<(), ModerationError> {
        let mut queue = self.queue.write().await;
        let entry = queue
            .iter_mut()
            .find(|e| e.id == escalation_id)
            .ok_or_else(|| ModerationError::NotFound(escalation_id.to_string()))?;
        if entry.state == EscalationState::Resolved {
            return Err(ModerationError::InvalidTransition(
                "Escalation already resolved".to_string(),
            ));
        }
        entry.state = EscalationState::Resolved;
        entry.curator_notes = notes;
        tracing::info!(
            target: "cns.communication.escalation.resolved",
            escalation_id = %escalation_id,
            "Escalation resolved by Curator"
        );
        Ok(())
    }

    /// Age out escalations past their SLA deadline.
    pub async fn age_expired(&self) -> Vec<Escalation> {
        let now = chrono::Utc::now().timestamp();
        let mut queue = self.queue.write().await;
        let mut aged = Vec::new();
        for entry in queue.iter_mut() {
            if entry.state == EscalationState::Created && now > entry.sla_deadline {
                entry.state = EscalationState::Aged;
                tracing::warn!(
                    target: "cns.communication.escalation.aged",
                    escalation_id = %entry.id,
                    "Escalation aged out — SLA breached"
                );
                aged.push(entry.clone());
            }
        }
        aged
    }

    /// Get the count of active escalations by state.
    pub async fn counts_by_state(&self) -> HashMap<EscalationState, usize> {
        let queue = self.queue.read().await;
        let mut counts = HashMap::new();
        for entry in queue.iter() {
            *counts.entry(entry.state).or_default() += 1;
        }
        counts
    }

    /// Generate daily digest of unresolved escalations.
    pub async fn daily_digest(&self) -> String {
        let queue = self.queue.read().await;
        let unresolved: Vec<&Escalation> = queue
            .iter()
            .filter(|e| e.state != EscalationState::Resolved)
            .collect();
        if unresolved.is_empty() {
            return "No unresolved escalations today.".to_string();
        }
        let mut digest = format!(
            "Daily Escalation Digest — {} unresolved\n\n",
            unresolved.len()
        );
        for e in &unresolved {
            digest.push_str(&format!(
                "- [{}] {}: {} (SLA: {})\n",
                e.severity_to_str(),
                e.room_id,
                &e.content_summary[..e.content_summary.len().min(80)],
                e.sla_deadline,
            ));
        }
        digest
    }
}

impl Default for ModerationQueue {
    fn default() -> Self {
        Self::new(86400)
    }
}

// ── SQLite-backed ModerationQueue ──────────────────────────────────────

/// SQLite-backed moderation queue using `hkask-storage::Database`.
///
/// Persists escalations in an `escalations` table with full audit trail.
/// Survives restarts. The caller must run `migrate()` once during server
/// startup to create the table (idempotent via `IF NOT EXISTS`).
pub struct SqliteModerationQueue {
    conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    sla_duration_secs: i64,
}

impl SqliteModerationQueue {
    /// Create a new SQLite-backed queue using an existing `hkask_storage::Database`.
    pub fn new(db: hkask_storage::Database, sla_duration_secs: i64) -> Self {
        Self {
            conn: db.conn_arc(),
            sla_duration_secs,
        }
    }

    /// Initialize the `escalations` table.
    ///
    /// Call once during server startup. Idempotent — uses `IF NOT EXISTS`.
    pub fn migrate(&self) -> Result<(), ModerationError> {
        let conn = self.conn.lock().map_err(|e| {
            ModerationError::Storage(format!("Lock error: {}", e))
        })?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS escalations (
                id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL,
                flagged_by TEXT NOT NULL,
                content_summary TEXT NOT NULL,
                severity TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                sla_deadline INTEGER NOT NULL,
                curator_notes TEXT
            )",
        )
        .map_err(|e| ModerationError::Storage(format!("Migration failed: {}", e)))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, ModerationError> {
        self.conn
            .lock()
            .map_err(|e| ModerationError::Storage(format!("Lock error: {}", e)))
    }

    /// Push a new escalation into the queue.
    pub fn push(&self, escalation: &Escalation) -> Result<(), ModerationError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO escalations (id, room_id, flagged_by, content_summary, severity, state, created_at, sla_deadline, curator_notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                escalation.id,
                escalation.room_id,
                escalation.flagged_by,
                escalation.content_summary,
                serde_json::to_string(&escalation.severity).unwrap_or_default(),
                serde_json::to_string(&escalation.state).unwrap_or_default(),
                escalation.created_at,
                escalation.sla_deadline,
                escalation.curator_notes,
            ],
        )
        .map(|_| ())
        .map_err(|e| ModerationError::Storage(format!("Insert failed: {}", e)))?;

        tracing::info!(
            target: "cns.communication.escalation.created",
            escalation_id = %escalation.id,
            room_id = %escalation.room_id,
            severity = ?escalation.severity,
            "Escalation persisted"
        );
        Ok(())
    }

    /// Get the next pending escalation for Curator review.
    pub fn poll_next(&self) -> Result<Option<Escalation>, ModerationError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, flagged_by, content_summary, severity, state, created_at, sla_deadline, curator_notes
                 FROM escalations WHERE state = '\"created\"' ORDER BY created_at ASC LIMIT 1",
            )
            .map_err(|e| ModerationError::Storage(format!("Query failed: {}", e)))?;

        match stmt.query_row([], |row| row_to_escalation(row)) {
            Ok(esc) => Ok(Some(esc)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ModerationError::Storage(format!("Query failed: {}", e))),
        }
    }

    /// Acknowledge an escalation (Curator picks it up).
    pub fn acknowledge(&self, escalation_id: &str) -> Result<(), ModerationError> {
        let conn = self.lock()?;
        let state_json = serde_json::to_string(&EscalationState::Acknowledged).unwrap_or_default();
        let rows = conn
            .execute(
                "UPDATE escalations SET state = ?1 WHERE id = ?2 AND state = '\"created\"'",
                rusqlite::params![state_json, escalation_id],
            )
            .map_err(|e| ModerationError::Storage(format!("Update failed: {}", e)))?;

        if rows == 0 {
            return Err(ModerationError::NotFound(escalation_id.to_string()));
        }

        tracing::info!(
            target: "cns.communication.escalation.acknowledged",
            escalation_id = %escalation_id,
            "Escalation acknowledged by Curator"
        );
        Ok(())
    }

    /// Resolve an escalation (Curator takes action).
    pub fn resolve(&self, escalation_id: &str, notes: Option<&str>) -> Result<(), ModerationError> {
        let conn = self.lock()?;
        let state_json = serde_json::to_string(&EscalationState::Resolved).unwrap_or_default();
        let rows = conn
            .execute(
                "UPDATE escalations SET state = ?1, curator_notes = ?2 WHERE id = ?3",
                rusqlite::params![state_json, notes, escalation_id],
            )
            .map_err(|e| ModerationError::Storage(format!("Update failed: {}", e)))?;

        if rows == 0 {
            return Err(ModerationError::NotFound(escalation_id.to_string()));
        }

        tracing::info!(
            target: "cns.communication.escalation.resolved",
            escalation_id = %escalation_id,
            "Escalation resolved by Curator"
        );
        Ok(())
    }

    /// Age out escalations past their SLA deadline.
    pub fn age_expired(&self) -> Result<Vec<Escalation>, ModerationError> {
        let now = chrono::Utc::now().timestamp();
        let conn = self.lock()?;
        let aged_state = serde_json::to_string(&EscalationState::Aged).unwrap_or_default();
        let created_state = serde_json::to_string(&EscalationState::Created).unwrap_or_default();

        // Find expired escalations
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, flagged_by, content_summary, severity, state, created_at, sla_deadline, curator_notes
                 FROM escalations WHERE state = ?1 AND sla_deadline < ?2",
            )
            .map_err(|e| ModerationError::Storage(format!("Query failed: {}", e)))?;

        let aged: Vec<Escalation> = stmt
            .query_map(rusqlite::params![created_state, now], |row| row_to_escalation(row))
            .map_err(|e| ModerationError::Storage(format!("Query failed: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        // Mark them as aged
        for esc in &aged {
            conn.execute(
                "UPDATE escalations SET state = ?1 WHERE id = ?2",
                rusqlite::params![aged_state, esc.id],
            )
            .map_err(|e| ModerationError::Storage(format!("Update failed: {}", e)))?;
            tracing::warn!(
                target: "cns.communication.escalation.aged",
                escalation_id = %esc.id,
                "Escalation aged out — SLA breached"
            );
        }

        Ok(aged)
    }

    /// Get the count of active escalations by state.
    pub fn counts_by_state(&self) -> Result<HashMap<EscalationState, usize>, ModerationError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT state, COUNT(*) FROM escalations GROUP BY state")
            .map_err(|e| ModerationError::Storage(format!("Query failed: {}", e)))?;

        let mut counts = HashMap::new();
        let rows = stmt
            .query_map([], |row| {
                let state_str: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((state_str, count as usize))
            })
            .map_err(|e| ModerationError::Storage(format!("Query failed: {}", e)))?;

        for row in rows {
            let (state_str, count) = row.map_err(|e| ModerationError::Storage(format!("Row error: {}", e)))?;
            if let Ok(state) = serde_json::from_str(&state_str) {
                counts.insert(state, count);
            }
        }
        Ok(counts)
    }

    /// Generate daily digest of unresolved escalations.
    pub fn daily_digest(&self) -> Result<String, ModerationError> {
        let conn = self.lock()?;
        let resolved_state = serde_json::to_string(&EscalationState::Resolved).unwrap_or_default();
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, flagged_by, content_summary, severity, state, created_at, sla_deadline, curator_notes
                 FROM escalations WHERE state != ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| ModerationError::Storage(format!("Query failed: {}", e)))?;

        let unresolved: Vec<Escalation> = stmt
            .query_map(rusqlite::params![resolved_state], |row| row_to_escalation(row))
            .map_err(|e| ModerationError::Storage(format!("Query failed: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        if unresolved.is_empty() {
            return Ok("No unresolved escalations today.".to_string());
        }
        let mut digest = format!("Daily Escalation Digest — {} unresolved\n\n", unresolved.len());
        for e in &unresolved {
            digest.push_str(&format!(
                "- [{}] {}: {} (SLA: {})\n",
                e.severity_to_str(),
                e.room_id,
                &e.content_summary[..e.content_summary.len().min(80)],
                e.sla_deadline,
            ));
        }
        Ok(digest)
    }
}

/// Convert a rusqlite Row into an Escalation.
fn row_to_escalation(row: &rusqlite::Row) -> rusqlite::Result<Escalation> {
    let severity_str: String = row.get(4)?;
    let state_str: String = row.get(5)?;
    Ok(Escalation {
        id: row.get(0)?,
        room_id: row.get(1)?,
        flagged_by: row.get(2)?,
        content_summary: row.get(3)?,
        severity: serde_json::from_str(&severity_str).unwrap_or(EscalationSeverity::Low),
        state: serde_json::from_str(&state_str).unwrap_or(EscalationState::Created),
        created_at: row.get(6)?,
        sla_deadline: row.get(7)?,
        curator_notes: row.get(8)?,
    })
}

impl Escalation {
    fn severity_to_str(&self) -> &str {
        match self.severity {
            EscalationSeverity::Low => "LOW",
            EscalationSeverity::Medium => "MED",
            EscalationSeverity::High => "HIGH",
            EscalationSeverity::Critical => "CRIT",
        }
    }
}

// ── 7R7 Bot ───────────────────────────────────────────────────────────────

/// 7R7 moderation bot — polls Matrix for unread content, classifies severity,
/// and produces Escalation entries for the ModerationQueue.
///
/// The 7R7 bot is a lightweight daemon process within the communication server.
/// It runs a polling loop: Monitor → Classify → EscalateOrIgnore.
pub struct SevenR7Bot {
    /// Matrix client for polling.
    matrix: Arc<crate::matrix::MatrixTransport>,
    /// Moderation queue for escalation output.
    queue: Arc<ModerationQueue>,
    /// Classification function (in production, would call an LLM).
    classifier: Box<dyn Classifier>,
}

impl SevenR7Bot {
    /// Create a new 7R7 bot.
    pub fn new(
        matrix: Arc<crate::matrix::MatrixTransport>,
        queue: Arc<ModerationQueue>,
        classifier: Box<dyn Classifier>,
    ) -> Self {
        Self {
            matrix,
            queue,
            classifier,
        }
    }

    /// Run a single moderation check cycle.
    ///
    /// 1. Poll Matrix for unread messages in monitored rooms
    /// 2. Classify each message for escalation potential
    /// 3. Push escalations to the ModerationQueue
    pub async fn check_cycle(
        &self,
        _rooms: &[crate::matrix::RoomId],
    ) -> Result<usize, ModerationError> {
        let messages = self.matrix.pending_messages().await;

        let mut escalated = 0;
        for msg in &messages {
            let classification = self.classifier.classify(&msg.body);
            if let ClassificationDecision::Escalate(severity) = classification {
                let escalation = Escalation {
                    id: uuid::Uuid::new_v4().to_string(),
                    room_id: "unknown".to_string(), // Would extract from Matrix event
                    flagged_by: msg.sender.as_str().to_string(),
                    content_summary: msg.body[..msg.body.len().min(200)].to_string(),
                    severity,
                    state: EscalationState::Created,
                    created_at: chrono::Utc::now().timestamp(),
                    sla_deadline: chrono::Utc::now().timestamp() + self.queue.sla_duration_secs,
                    curator_notes: None,
                };
                self.queue.push(escalation).await?;
                escalated += 1;
            }
        }
        if escalated > 0 {
            tracing::info!(
                target: "cns.communication.escalation.batch",
                escalated = %escalated,
                total_checked = %messages.len(),
                "7R7 moderation cycle complete"
            );
        }
        Ok(escalated)
    }
}

// ── Content classifier ─────────────────────────────────────────────────────

/// Classification strategy for messages.
///
/// In production, this would wrap an LLM call for nuanced classification.
/// For now, a simple keyword-based mock is provided.
pub trait Classifier: Send + Sync {
    fn classify(&self, content: &str) -> ClassificationDecision;
}

/// Naive keyword-based classifier for demonstration/testing.
///
/// Flags content containing known escalation triggers.
/// Production would replace this with an LLM-based classifier.
pub struct NaiveKeywordClassifier;

impl Classifier for NaiveKeywordClassifier {
    fn classify(&self, content: &str) -> ClassificationDecision {
        let lower = content.to_lowercase();

        // Critical: explicit policy violations
        let critical_triggers = ["violence", "illegal", "exploit", "malware"];
        for trigger in &critical_triggers {
            if lower.contains(trigger) {
                return ClassificationDecision::Escalate(EscalationSeverity::Critical);
            }
        }

        // High: potential harm
        let high_triggers = ["threat", "attack", "breach", "unauthorized"];
        for trigger in &high_triggers {
            if lower.contains(trigger) {
                return ClassificationDecision::Escalate(EscalationSeverity::High);
            }
        }

        // Medium: policy questions
        let medium_triggers = ["error", "broken", "failed", "timeout", "permission"];
        for trigger in &medium_triggers {
            if lower.contains(trigger) {
                return ClassificationDecision::Escalate(EscalationSeverity::Medium);
            }
        }

        // Low: general noise
        let low_triggers = ["help", "support", "question"];
        for trigger in &low_triggers {
            if lower.contains(trigger) {
                return ClassificationDecision::Escalate(EscalationSeverity::Low);
            }
        }

        ClassificationDecision::Ignore
    }
}
