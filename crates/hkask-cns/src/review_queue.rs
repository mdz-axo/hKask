//! Sandbox Review Queue
//!
//! Implements Q8 decision: Temporary block + human review for sandbox violations.

use chrono::{DateTime, Duration, Utc};
use hkask_types::{NuEvent, Phase, Span, WebID};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

/// Sandbox violation severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Warning,
    Moderate,
    Severe,
    Critical,
}

impl ViolationSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViolationSeverity::Warning => "warning",
            ViolationSeverity::Moderate => "moderate",
            ViolationSeverity::Severe => "severe",
            ViolationSeverity::Critical => "critical",
        }
    }

    pub fn default_block_duration(&self) -> Duration {
        match self {
            ViolationSeverity::Warning => Duration::minutes(0),
            ViolationSeverity::Moderate => Duration::minutes(15),
            ViolationSeverity::Severe => Duration::hours(2),
            ViolationSeverity::Critical => Duration::hours(24),
        }
    }
}

/// Sandbox violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub id: Uuid,
    pub agent_id: WebID,
    pub severity: ViolationSeverity,
    pub violation_type: String,
    pub description: String,
    pub occurred_at: DateTime<Utc>,
    pub reviewed: bool,
    pub review_decision: Option<ReviewDecision>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<WebID>,
}

impl Violation {
    pub fn new(agent_id: WebID, severity: ViolationSeverity, violation_type: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            severity,
            violation_type,
            description,
            occurred_at: Utc::now(),
            reviewed: false,
            review_decision: None,
            reviewed_at: None,
            reviewed_by: None,
        }
    }

    pub fn mark_reviewed(&mut self, decision: ReviewDecision, operator: WebID) {
        self.reviewed = true;
        self.review_decision = Some(decision);
        self.reviewed_at = Some(Utc::now());
        self.reviewed_by = Some(operator);
    }
}

/// Human operator review decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewDecision {
    FalsePositive,
    Acceptable,
    UpholdBlock,
    Escalate,
}

impl ReviewDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewDecision::FalsePositive => "false_positive",
            ReviewDecision::Acceptable => "acceptable",
            ReviewDecision::UpholdBlock => "uphold_block",
            ReviewDecision::Escalate => "escalate",
        }
    }
}

/// Temporary block on an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryBlock {
    pub id: Uuid,
    pub agent_id: WebID,
    pub violation_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub active: bool,
    pub reason: String,
}

impl TemporaryBlock {
    pub fn new(agent_id: WebID, violation_id: Uuid, duration: Duration, reason: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            agent_id,
            violation_id,
            started_at: now,
            ends_at: Some(now + duration),
            active: true,
            reason,
        }
    }

    pub fn new_indefinite(agent_id: WebID, violation_id: Uuid, reason: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            violation_id,
            started_at: Utc::now(),
            ends_at: None,
            active: true,
            reason,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.ends_at.map(|end| Utc::now() > end).unwrap_or(false)
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn remaining_duration(&self) -> Option<Duration> {
        self.ends_at.map(|end| end - Utc::now())
    }
}

/// Review queue for sandbox violations
pub struct ReviewQueue {
    violations: VecDeque<Violation>,
    blocks: Vec<TemporaryBlock>,
    max_size: usize,
    stats: ReviewQueueStats,
}

/// Review queue statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewQueueStats {
    pub total_violations: usize,
    pub total_reviewed: usize,
    pub total_blocks: usize,
    pub total_released: usize,
    pub false_positives: usize,
    pub escalations: usize,
}

impl ReviewQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            violations: VecDeque::new(),
            blocks: Vec::new(),
            max_size,
            stats: ReviewQueueStats::default(),
        }
    }

    pub fn add_violation(&mut self, violation: Violation) -> (Uuid, Option<Uuid>) {
        self.stats.total_violations += 1;
        if self.violations.len() >= self.max_size {
            self.violations.pop_front();
        }
        let violation_id = violation.id;
        let severity = violation.severity;
        let agent_id = violation.agent_id;
        let description = violation.description.clone();
        self.violations.push_back(violation);
        let block_id = if severity >= ViolationSeverity::Moderate {
            let duration = severity.default_block_duration();
            let reason = format!("Sandbox violation: {}", description);
            let block = TemporaryBlock::new(agent_id, violation_id, duration, reason);
            let block_id = block.id;
            self.blocks.push(block);
            self.stats.total_blocks += 1;
            Some(block_id)
        } else {
            None
        };
        (violation_id, block_id)
    }

    pub fn pending_violations(&self) -> Vec<&Violation> {
        self.violations.iter().filter(|v| !v.reviewed).collect()
    }

    pub fn get_violation(&self, id: Uuid) -> Option<&Violation> {
        self.violations.iter().find(|v| v.id == id)
    }

    pub fn review_violation(&mut self, violation_id: Uuid, decision: ReviewDecision, operator: WebID) -> Option<&Violation> {
        let violation = self.violations.iter_mut().find(|v| v.id == violation_id)?;
        if violation.reviewed {
            return None;
        }
        violation.mark_reviewed(decision, operator);
        self.stats.total_reviewed += 1;
        match decision {
            ReviewDecision::FalsePositive => {
                self.stats.false_positives += 1;
                if let Some(block) = self.blocks.iter_mut().find(|b| b.violation_id == violation_id) {
                    block.deactivate();
                    self.stats.total_released += 1;
                }
            }
            ReviewDecision::Acceptable => {
                if let Some(block) = self.blocks.iter_mut().find(|b| b.violation_id == violation_id) {
                    block.deactivate();
                    self.stats.total_released += 1;
                }
            }
            ReviewDecision::UpholdBlock => {}
            ReviewDecision::Escalate => {
                self.stats.escalations += 1;
                if let Some(block) = self.blocks.iter_mut().find(|b| b.violation_id == violation_id) {
                    block.ends_at = None;
                }
            }
        }
        Some(violation)
    }

    pub fn is_agent_blocked(&self, agent_id: &WebID) -> bool {
        self.blocks.iter().any(|b| b.agent_id == *agent_id && b.active && !b.is_expired())
    }

    pub fn get_agent_block(&self, agent_id: &WebID) -> Option<&TemporaryBlock> {
        self.blocks.iter().find(|b| b.agent_id == *agent_id && b.active && !b.is_expired())
    }

    pub fn cleanup_expired_blocks(&mut self) -> usize {
        let mut cleaned = 0;
        for block in self.blocks.iter_mut() {
            if block.active && block.is_expired() {
                block.deactivate();
                self.stats.total_released += 1;
                cleaned += 1;
            }
        }
        cleaned
    }

    pub fn stats(&self) -> &ReviewQueueStats {
        &self.stats
    }

    pub fn len(&self) -> usize {
        self.violations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.violations.is_empty()
    }
}

/// CNS span emitter for review queue events
pub struct ReviewQueueObserver {
    observer_webid: WebID,
}

impl ReviewQueueObserver {
    pub fn new(observer_webid: WebID) -> Self {
        Self { observer_webid }
    }

    pub fn emit_violation(&self, violation: &Violation, block_id: Option<Uuid>) -> NuEvent {
        let observation = serde_json::json!({
            "violation_id": violation.id.to_string(),
            "agent_id": violation.agent_id.to_string(),
            "severity": violation.severity.as_str(),
            "violation_type": violation.violation_type,
            "description": violation.description,
            "block_applied": block_id.is_some(),
            "block_id": block_id.map(|id| id.to_string()),
        });
        NuEvent::new(
            self.observer_webid.clone(),
            Span::review("violation"),
            Phase::Observe,
            observation,
            0,
        )
    }

    pub fn emit_block(&self, block: &TemporaryBlock) -> NuEvent {
        let observation = serde_json::json!({
            "block_id": block.id.to_string(),
            "agent_id": block.agent_id.to_string(),
            "violation_id": block.violation_id.to_string(),
            "reason": block.reason,
            "started_at": block.started_at.to_rfc3339(),
            "ends_at": block.ends_at.map(|dt| dt.to_rfc3339()),
            "duration_minutes": block.ends_at.map(|end| (end - block.started_at).num_minutes()),
        });
        NuEvent::new(
            self.observer_webid.clone(),
            Span::review("block"),
            Phase::Observe,
            observation,
            0,
        )
    }

    pub fn emit_decision(&self, violation: &Violation, block: Option<&TemporaryBlock>) -> NuEvent {
        let observation = serde_json::json!({
            "violation_id": violation.id.to_string(),
            "agent_id": violation.agent_id.to_string(),
            "decision": violation.review_decision.map(|d| d.as_str()),
            "reviewed_by": violation.reviewed_by.map(|id| id.to_string()),
            "reviewed_at": violation.reviewed_at.map(|dt| dt.to_rfc3339()),
            "block_status": block.map(|b| if b.active { "active" } else { "released" }),
        });
        NuEvent::new(
            self.observer_webid.clone(),
            Span::review("decision"),
            Phase::Observe,
            observation,
            0,
        )
    }

    pub fn emit_release(&self, agent_id: &WebID, block: &TemporaryBlock) -> NuEvent {
        let observation = serde_json::json!({
            "agent_id": agent_id.to_string(),
            "block_id": block.id.to_string(),
            "violation_id": block.violation_id.to_string(),
            "block_duration_minutes": (block.ends_at.unwrap_or(block.started_at) - block.started_at).num_minutes(),
            "reason": block.reason,
        });
        NuEvent::new(
            self.observer_webid.clone(),
            Span::review("release"),
            Phase::Observe,
            observation,
            0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_violation_creation() {
        let agent_id = WebID::new();
        let violation = Violation::new(
            agent_id.clone(),
            ViolationSeverity::Moderate,
            "memory_access".to_string(),
            "Attempted unauthorized memory access".to_string(),
        );
        assert!(!violation.reviewed);
        assert_eq!(violation.severity, ViolationSeverity::Moderate);
        assert_eq!(violation.agent_id, agent_id);
    }

    #[test]
    fn test_violation_review() {
        let agent_id = WebID::new();
        let mut violation = Violation::new(
            agent_id.clone(),
            ViolationSeverity::Moderate,
            "memory_access".to_string(),
            "Test violation".to_string(),
        );
        let operator = WebID::new();
        violation.mark_reviewed(ReviewDecision::FalsePositive, operator.clone());
        assert!(violation.reviewed);
        assert_eq!(violation.review_decision, Some(ReviewDecision::FalsePositive));
        assert_eq!(violation.reviewed_by, Some(operator));
        assert!(violation.reviewed_at.is_some());
    }

    #[test]
    fn test_violation_severity_block_duration() {
        assert_eq!(ViolationSeverity::Warning.default_block_duration(), Duration::minutes(0));
        assert_eq!(ViolationSeverity::Moderate.default_block_duration(), Duration::minutes(15));
        assert_eq!(ViolationSeverity::Severe.default_block_duration(), Duration::hours(2));
        assert_eq!(ViolationSeverity::Critical.default_block_duration(), Duration::hours(24));
    }

    #[test]
    fn test_temporary_block_expiry() {
        let agent_id = WebID::new();
        let violation_id = Uuid::new_v4();
        let block = TemporaryBlock::new(agent_id.clone(), violation_id, Duration::seconds(-1), "Test".to_string());
        assert!(block.is_expired());
        let block2 = TemporaryBlock::new(agent_id, violation_id, Duration::hours(1), "Test".to_string());
        assert!(!block2.is_expired());
    }

    #[test]
    fn test_review_queue_add_violation() {
        let mut queue = ReviewQueue::new(100);
        let agent_id = WebID::new();
        let violation1 = Violation::new(agent_id.clone(), ViolationSeverity::Warning, "minor".to_string(), "Minor".to_string());
        let (id1, block1) = queue.add_violation(violation1);
        assert!(!id1.is_nil());
        assert!(block1.is_none());
        let violation2 = Violation::new(agent_id.clone(), ViolationSeverity::Moderate, "moderate".to_string(), "Moderate".to_string());
        let (id2, block2) = queue.add_violation(violation2);
        assert!(!id2.is_nil());
        assert!(block2.is_some());
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.stats().total_violations, 2);
        assert_eq!(queue.stats().total_blocks, 1);
    }

    #[test]
    fn test_review_queue_review_violation() {
        let mut queue = ReviewQueue::new(100);
        let agent_id = WebID::new();
        let operator = WebID::new();
        let violation = Violation::new(agent_id.clone(), ViolationSeverity::Moderate, "test".to_string(), "Test".to_string());
        let (violation_id, _block_id) = queue.add_violation(violation);
        let result = queue.review_violation(violation_id, ReviewDecision::FalsePositive, operator);
        assert!(result.is_some());
        let stats = queue.stats();
        assert_eq!(stats.total_reviewed, 1);
        assert_eq!(stats.false_positives, 1);
    }

    #[test]
    fn test_review_queue_agent_blocked() {
        let mut queue = ReviewQueue::new(100);
        let agent_id = WebID::new();
        let violation = Violation::new(agent_id.clone(), ViolationSeverity::Moderate, "test".to_string(), "Test".to_string());
        queue.add_violation(violation);
        assert!(queue.is_agent_blocked(&agent_id));
        let other_agent = WebID::new();
        assert!(!queue.is_agent_blocked(&other_agent));
    }

    #[test]
    fn test_review_queue_cleanup_expired() {
        let mut queue = ReviewQueue::new(100);
        let agent_id = WebID::new();
        let violation = Violation::new(agent_id.clone(), ViolationSeverity::Moderate, "test".to_string(), "Test".to_string());
        queue.add_violation(violation);
        if let Some(block) = queue.blocks.first_mut() {
            block.ends_at = Some(Utc::now() - Duration::hours(1));
        }
        let cleaned = queue.cleanup_expired_blocks();
        assert_eq!(cleaned, 1);
        assert_eq!(queue.stats().total_released, 1);
    }

    #[test]
    fn test_review_queue_fifo_eviction() {
        let mut queue = ReviewQueue::new(3);
        let agent_id = WebID::new();
        for i in 0..5 {
            let violation = Violation::new(agent_id.clone(), ViolationSeverity::Warning, format!("v_{}", i), format!("Violation {}", i));
            queue.add_violation(violation);
        }
        assert_eq!(queue.len(), 3);
        let violations = queue.pending_violations();
        assert_eq!(violations.len(), 3);
    }

    #[test]
    fn test_review_decision_strings() {
        assert_eq!(ReviewDecision::FalsePositive.as_str(), "false_positive");
        assert_eq!(ReviewDecision::Acceptable.as_str(), "acceptable");
        assert_eq!(ReviewDecision::UpholdBlock.as_str(), "uphold_block");
        assert_eq!(ReviewDecision::Escalate.as_str(), "escalate");
    }
}
