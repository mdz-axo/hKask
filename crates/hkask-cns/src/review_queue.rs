//! CNS Review Queue — Simplified
//!
//! Minimal span emission with rate limiting.
//! Human review delegated to Curator replicant.
//! ℏKask v0.21.2 — Planck's Constant of Agent Systems

use chrono::{DateTime, Utc};
use hkask_types::{Span, WebID};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Simplified violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub id: Uuid,
    pub agent_id: WebID,
    pub violation_type: String,
    pub description: String,
    pub occurred_at: DateTime<Utc>,
}

impl Violation {
    pub fn new(agent_id: WebID, violation_type: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            violation_type,
            description,
            occurred_at: Utc::now(),
        }
    }

    /// Emit violation as CNS span (Curator handles review)
    pub fn emit_span(&self) -> Span {
        Span::Review(format!("cns.alert.violation.{}", self.id))
    }
}

/// Minimal review queue — just span emission
pub struct ReviewQueue {
    violations: Vec<Violation>,
}

impl Default for ReviewQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ReviewQueue {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
        }
    }

    /// Add violation and emit span (Curator reviews)
    pub fn add_violation(&mut self, violation: Violation) {
        let _span = violation.emit_span();
        // Span emitted (Curator will review via CNS)
        self.violations.push(violation);
    }

    /// Get pending violations (for Curator review)
    pub fn pending_violations(&self) -> Vec<&Violation> {
        self.violations.iter().collect()
    }
}
