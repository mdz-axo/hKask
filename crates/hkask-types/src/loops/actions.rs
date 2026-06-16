//! Loop action types — efferent actions and their type classification.

use super::core::LoopId;

/// Efferent action produced by a loop's compute phase.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopAction {
    pub target: LoopId,
    pub action_type: ActionType,
    pub parameters: serde_json::Value,
}

impl LoopAction {
    pub fn new(target: LoopId, action_type: ActionType, parameters: serde_json::Value) -> Self {
        Self {
            target,
            action_type,
            parameters,
        }
    }
}

/// Types of regulatory actions a loop can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionType {
    /// Reduce resource allocation to a target loop
    Throttle,
    /// Escalate an alert to the Curation loop
    Escalate,
    /// Adjust a threshold or set-point
    Calibrate,
    /// Open a circuit breaker on a target
    CircuitBreak,
    /// Adjust energy budget within set-point bounds (Cybernetics automatic regulation)
    ///
    /// This is a *weaker* capability than `OverrideEnergyBudget`.
    /// Cybernetics can adjust within its set-point range.
    /// Only Curation can override set-points themselves.
    AdjustEnergyBudget,
    /// Override energy budget beyond set-point bounds (Curation metacognitive override)
    ///
    /// This is a *stronger* capability than `AdjustEnergyBudget`.
    /// Only Curation can issue this — it can exceed Cybernetics' set-point range.
    OverrideEnergyBudget,
    /// Replenish an agent's energy budget (Curation directive)
    ///
    /// \[NORMATIVE\] Used when an agent has exhausted its budget but should continue. (P9 — Homeostatic Self-Regulation).
    /// This is the Curator's ability to inject gas into the system.
    ReplenishBudget,
    /// Informational notification — no action required, positive signal.
    /// Used for non-urgent health improvements (e.g., seam coverage increased).
    Notify,
}
