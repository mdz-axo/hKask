//! Execution audit trail for template dispatch
//!
//! Logs bot ID, template ID, input hash, and outcome event ID for each dispatch.
//! Stored in SQLite for correlation with CNS ν-events.
//!
//! **Bitemporal semantics:**
//! - `executed_at` is transaction-time: when the render actually completed.
//! - `context_timestamp` is valid-time: when the rendering context was assembled
//!   (may predate `executed_at` if the template used stale or cached data).
//! - The `triple()` method produces a subject-predicate-object representation
//!   suitable for insertion into the bitemporal triple store.

use chrono::{DateTime, Utc};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Execution audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionAudit {
    /// Unique audit record ID
    pub id: Uuid,
    /// Bot WebID that executed the dispatch
    pub bot_id: WebID,
    /// Template ID that was executed
    pub template_id: String,
    /// SHA-256 hash of input (for privacy, not stored raw)
    pub input_hash: String,
    /// CNS ν-event ID for outcome correlation
    pub outcome_event_id: Option<Uuid>,
    /// Execution timestamp (transaction-time — when the render completed)
    pub executed_at: DateTime<Utc>,
    /// Context assembly timestamp (valid-time — when the rendering context
    /// was assembled; may predate `executed_at` for stale/cached contexts)
    pub context_timestamp: Option<DateTime<Utc>>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Success or failure
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Matroshka depth at execution time
    pub matroshka_depth: u8,
}

impl ExecutionAudit {
    /// Create new audit record
    pub fn new(
        bot_id: WebID,
        template_id: String,
        input_hash: String,
        matroshka_depth: u8,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            bot_id,
            template_id,
            input_hash,
            outcome_event_id: None,
            executed_at: Utc::now(),
            context_timestamp: None,
            duration_ms: 0,
            success: true,
            error_message: None,
            matroshka_depth,
        }
    }

    /// Set context assembly timestamp (valid-time).
    /// When the rendering context was assembled — may predate `executed_at`.
    pub fn with_context_timestamp(mut self, ts: DateTime<Utc>) -> Self {
        self.context_timestamp = Some(ts);
        self
    }

    /// Produce a bitemporal triple representation of this audit.
    ///
    /// Returns a tuple of `(subject, predicate, object, valid_from, tx_from)`
    /// suitable for insertion into the `TripleStore`. The `valid_from` is
    /// the context assembly moment (or `executed_at` if not recorded);
    /// the `tx_from` is when the render completed.
    pub fn triple(&self) -> BitemporalTriple {
        BitemporalTriple {
            subject: format!(
                "template:{}:render:invocation:{}",
                self.template_id,
                self.id.simple()
            ),
            predicate: "rendered".to_string(),
            object: self.input_hash.clone(),
            valid_from: self.context_timestamp.unwrap_or(self.executed_at),
            tx_from: self.executed_at,
        }
    }

    /// Set outcome event ID
    pub fn with_outcome_event(mut self, event_id: Uuid) -> Self {
        self.outcome_event_id = Some(event_id);
        self
    }

    /// Mark as failed
    pub fn with_error(mut self, error: String) -> Self {
        self.success = false;
        self.error_message = Some(error);
        self
    }

    /// Set duration
    pub fn with_duration_ms(mut self, duration: u64) -> Self {
        self.duration_ms = duration;
        self
    }

    /// Compute SHA-256 hash of input
    pub fn hash_input(input: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// A bitemporal triple — subject-predicate-object with valid-time and transaction-time.
///
/// Mirrors the schema in `hkask-storage::triples` for interop between
/// the template audit layer and the bitemporal triple store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitemporalTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from: DateTime<Utc>,
    pub tx_from: DateTime<Utc>,
}

/// Audit trail manager
pub struct AuditTrail {
    records: Vec<ExecutionAudit>,
    max_records: usize,
}

impl AuditTrail {
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Vec::new(),
            max_records,
        }
    }

    /// Record an execution
    pub fn record(&mut self, audit: ExecutionAudit) {
        self.records.push(audit);

        // Trim old records if exceeding limit
        if self.records.len() > self.max_records {
            let drain_count = self.records.len() - self.max_records;
            self.records.drain(0..drain_count);
        }
    }

    /// Get audit records for a bot
    pub fn get_by_bot(&self, bot_id: &WebID) -> Vec<&ExecutionAudit> {
        self.records
            .iter()
            .filter(|r| r.bot_id == *bot_id)
            .collect()
    }

    /// Get audit records for a template
    pub fn get_by_template(&self, template_id: &str) -> Vec<&ExecutionAudit> {
        self.records
            .iter()
            .filter(|r| r.template_id == template_id)
            .collect()
    }

    /// Get audit record by ID
    pub fn get_by_id(&self, id: &Uuid) -> Option<&ExecutionAudit> {
        self.records.iter().find(|r| r.id == *id)
    }

    /// Get recent failed executions
    pub fn get_failures(&self) -> Vec<&ExecutionAudit> {
        self.records.iter().filter(|r| !r.success).collect()
    }

    /// Get all audit records
    pub fn get_all(&self) -> &[ExecutionAudit] {
        &self.records
    }

    /// Get record count
    pub fn count(&self) -> usize {
        self.records.len()
    }

    /// Clear audit trail
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Get statistics
    pub fn get_stats(&self) -> AuditStats {
        let total = self.records.len();
        let successes = self.records.iter().filter(|r| r.success).count();
        let failures = total - successes;
        let avg_duration = if total > 0 {
            self.records.iter().map(|r| r.duration_ms).sum::<u64>() / total as u64
        } else {
            0
        };

        AuditStats {
            total,
            successes,
            failures,
            avg_duration,
        }
    }
}

/// Audit statistics
#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total: usize,
    pub successes: usize,
    pub failures: usize,
    pub avg_duration: u64,
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new(10000) // Default: keep last 10,000 records
    }
}
