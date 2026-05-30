//! Metacognition Port — Hexagonal boundary for metacognition snapshot persistence
//!
//!

use hkask_types::WebID;
use thiserror::Error;

// Re-export CNS types used in our public interface
pub use hkask_cns::bot_metrics::{BotEvaluationMetrics, BotHealthStatus, CapabilityGap, GapType};

#[derive(Debug, Error)]
pub enum MetacognitionPortError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Snapshot not found: {0}")]
    NotFound(i64),
}

/// **Deprecated:** Use `HealthSnapshot` instead.
///
/// `StoredHealthSnapshot` used JSON strings for storage serialization.
/// Migrate to using `HealthSnapshot` directly and call `.into()` when
/// you need storage-compatible fields.
#[derive(Debug, Clone)]
#[deprecated(
    since = "0.21.0",
    note = "Use `HealthSnapshot` instead. StoredHealthSnapshot has been collapsed into HealthSnapshot."
)]
pub struct StoredHealthSnapshot {
    pub timestamp: String,
    pub cns_health: String,
    pub critical_alerts: i32,
    pub total_alerts: i32,
    pub variety_counters_json: String,
    pub bot_reports_json: String,
}

#[allow(deprecated)]
impl From<crate::curator::metacognition::HealthSnapshot> for StoredHealthSnapshot {
    fn from(snapshot: crate::curator::metacognition::HealthSnapshot) -> Self {
        Self {
            timestamp: snapshot.timestamp.to_rfc3339(),
            cns_health: snapshot.cns_health,
            critical_alerts: snapshot.critical_alerts as i32,
            total_alerts: snapshot.total_alerts as i32,
            variety_counters_json: serde_json::to_string(&snapshot.variety_counters)
                .unwrap_or_else(|_| "{}".to_string()),
            bot_reports_json: serde_json::to_string(&snapshot.bot_status_reports)
                .unwrap_or_else(|_| "[]".to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// Curator Metacognition — Evaluate, Coach, Direct
// ---------------------------------------------------------------------------

/// Kata type for coaching protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KataType {
    /// Systematic capability development (4-step cycle)
    Improvement,
    /// Teaching scientific thinking patterns (5-question dialogue)
    Coaching,
    /// Building foundational habits (3 practice routines)
    Starter,
}

/// Types of directives the Curator can issue
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DirectiveType {
    /// Adjust a CNS threshold
    CalibrateThreshold { domain: String, new_threshold: u64 },
    /// Adjust an energy budget
    AdjustEnergyBudget { new_budget: u64 },
    /// Trigger a Kata coaching cycle
    TriggerKata {
        kata_type: KataType,
        gap_description: String,
    },
    /// Update capability boundaries
    UpdateCapabilities {
        additions: Vec<String>,
        removals: Vec<String>,
    },
    /// Escalate to human administrator
    EscalateToHuman { message: String },
}

/// Directive from Curator to an R7 bot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BotDirective {
    /// Target bot WebID
    pub bot_id: WebID,
    /// Bot name (human-readable)
    pub bot_name: String,
    /// Type of directive
    pub directive_type: DirectiveType,
    /// Directive payload
    pub payload: serde_json::Value,
    /// Reason for the directive
    pub reason: String,
    /// Timestamp
    pub timestamp: String,
}

/// A directive to execute a Kata coaching cycle
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KataDirective {
    /// Bot to coach
    pub bot_id: WebID,
    /// Bot name
    pub bot_name: String,
    /// Type of Kata to execute
    pub kata_type: KataType,
    /// Description of the capability gap
    pub gap_description: String,
    /// The gap that triggered this directive
    pub gap: CapabilityGap,
}

/// Result of evaluating a bot's performance
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvaluationResult {
    /// Bot being evaluated
    pub bot_id: WebID,
    /// Bot name
    pub bot_name: String,
    /// Health status
    pub health: BotHealthStatus,
    /// Capability gaps identified
    pub gaps: Vec<CapabilityGap>,
    /// Recommended action
    pub recommended_action: RecommendedAction,
    /// Evaluation timestamp
    pub timestamp: String,
}

/// Recommended action from evaluation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RecommendedAction {
    /// No action needed
    None,
    /// Monitor more closely
    Monitor,
    /// Trigger a Kata coaching cycle
    Coach(KataType),
    /// Calibrate thresholds
    Calibrate(String, u64),
    /// Escalate to human
    Escalate,
}
