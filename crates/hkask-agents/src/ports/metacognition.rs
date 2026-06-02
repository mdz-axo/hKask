//! Metacognition Port — Curator metacognition types

use hkask_types::WebID;

// Re-export bot metrics types used in our public interface
pub use crate::curator::bot_metrics::{
    BotEvaluationMetrics, BotHealthStatus, CapabilityGap, GapType,
};

/// Kata type for coaching protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum KataType {
    /// Systematic capability development (4-step cycle)
    Improvement,
    /// Teaching scientific thinking patterns (5-question dialogue)
    Coaching,
    /// Building foundational habits (3 practice routines)
    Starter,
}

/// A directive to execute a Kata coaching cycle
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct KataDirective {
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
pub(crate) struct EvaluationResult {
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
pub(crate) enum RecommendedAction {
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
