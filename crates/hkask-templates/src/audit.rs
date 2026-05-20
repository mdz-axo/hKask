//! Execution audit trail for template dispatch
//!
//! Logs bot ID, template ID, input hash, and outcome event ID for each dispatch.
//! Stored in SQLite for correlation with CNS ν-events.

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
    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
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
            duration_ms: 0,
            success: true,
            error_message: None,
            matroshka_depth,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_audit_with_outcome_event() {
        let bot_id = WebID::new();
        let event_id = Uuid::new_v4();
        let audit = ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc123".to_string(),
            1,
        )
        .with_outcome_event(event_id);

        assert_eq!(audit.outcome_event_id, Some(event_id));
    }

    #[test]
    fn test_execution_audit_with_error() {
        let bot_id = WebID::new();
        let audit = ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc123".to_string(),
            1,
        )
        .with_error("Template not found".to_string());

        assert!(!audit.success);
        assert_eq!(audit.error_message, Some("Template not found".to_string()));
    }

    #[test]
    fn test_execution_audit_hash_input() {
        let input = "test input";
        let hash = ExecutionAudit::hash_input(input);

        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars

        // Same input produces same hash
        assert_eq!(hash, ExecutionAudit::hash_input(input));

        // Different input produces different hash
        assert_ne!(hash, ExecutionAudit::hash_input("different input"));
    }

    #[test]
    fn test_audit_trail_record() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        let audit = ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc123".to_string(),
            1,
        );

        trail.record(audit);

        assert_eq!(trail.count(), 1);
    }

    #[test]
    fn test_audit_trail_get_by_bot() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();
        let other_bot = WebID::new();

        trail.record(ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc".to_string(),
            1,
        ));
        trail.record(ExecutionAudit::new(
            other_bot,
            "prompt/selector".to_string(),
            "def".to_string(),
            1,
        ));

        let by_bot = trail.get_by_bot(&bot_id);
        assert_eq!(by_bot.len(), 1);
        assert_eq!(by_bot[0].bot_id, bot_id);
    }

    #[test]
    fn test_audit_trail_get_by_template() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        trail.record(ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc".to_string(),
            1,
        ));
        trail.record(ExecutionAudit::new(
            bot_id,
            "process/dispatch".to_string(),
            "def".to_string(),
            1,
        ));

        let by_template = trail.get_by_template("prompt/selector");
        assert_eq!(by_template.len(), 1);
        assert_eq!(by_template[0].template_id, "prompt/selector");
    }

    #[test]
    fn test_audit_trail_trim_old_records() {
        let mut trail = AuditTrail::new(5);
        let bot_id = WebID::new();

        for i in 0..10 {
            trail.record(ExecutionAudit::new(
                bot_id,
                format!("template/{}", i),
                format!("hash{}", i),
                1,
            ));
        }

        assert_eq!(trail.count(), 5);
        // Oldest records should be trimmed
        assert!(trail.get_by_template("template/0").is_empty());
        assert!(!trail.get_by_template("template/9").is_empty());
    }

    #[test]
    fn test_audit_trail_get_failures() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        trail.record(ExecutionAudit::new(
            bot_id,
            "prompt/success".to_string(),
            "abc".to_string(),
            1,
        ));
        trail.record(
            ExecutionAudit::new(bot_id, "prompt/fail".to_string(), "def".to_string(), 1)
                .with_error("Failed".to_string()),
        );

        let failures = trail.get_failures();
        assert_eq!(failures.len(), 1);
        assert!(!failures[0].success);
    }

    #[test]
    fn test_audit_trail_get_stats() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        trail.record(
            ExecutionAudit::new(bot_id, "prompt/success".to_string(), "abc".to_string(), 1)
                .with_duration_ms(100),
        );

        trail.record(
            ExecutionAudit::new(bot_id, "prompt/success2".to_string(), "def".to_string(), 1)
                .with_duration_ms(200),
        );

        trail.record(
            ExecutionAudit::new(bot_id, "prompt/fail".to_string(), "ghi".to_string(), 1)
                .with_error("Failed".to_string())
                .with_duration_ms(50),
        );

        let stats = trail.get_stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.avg_duration, 116); // (100 + 200 + 50) / 3 = 116
    }

    #[test]
    fn test_audit_trail_clear() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        trail.record(ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc".to_string(),
            1,
        ));

        trail.clear();

        assert_eq!(trail.count(), 0);
    }
}
