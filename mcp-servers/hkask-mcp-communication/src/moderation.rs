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
    matrix: Arc<crate::matrix::MatrixClient>,
    /// Moderation queue for escalation output.
    queue: Arc<ModerationQueue>,
    /// Classification function (in production, would call an LLM).
    classifier: Box<dyn Classifier>,
}

impl SevenR7Bot {
    /// Create a new 7R7 bot.
    pub fn new(
        matrix: Arc<crate::matrix::MatrixClient>,
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
        rooms: &[crate::matrix::RoomIdStr],
    ) -> Result<usize, ModerationError> {
        let messages = self
            .matrix
            .poll_unread(rooms)
            .await
            .map_err(|e| ModerationError::Storage(format!("Matrix poll failed: {}", e)))?;

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
