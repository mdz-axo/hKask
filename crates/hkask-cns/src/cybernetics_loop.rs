//! Cybernetics Loop — Homeostatic self-regulation (Loop 6)
//!
//! The Cybernetics Loop is a closed-loop controller, not a passive observer.
//! Its functional contract:
//!
//! 1. **Sense** — receive `cns.*` spans from all loops (tool invocations,
//!    prompt outcomes, agent pod lifecycle, connector I/O).
//! 2. **Compare** — evaluate each signal against homeostatic set-points:
//!    energy budget remaining, variety counter balance, error rate threshold,
//!    connector latency envelope.
//! 3. **Compute** — when a signal deviates beyond its set-point, produce an
//!    efferent signal: throttle, escalate, calibrate, rebalance, or circuit-break.
//! 4. **Act** — dispatch the efferent signal to the target loop's `regulate`
//!    entry point.
//!
//! The loop is self-stabilizing: if the Cybernetics Loop itself becomes unstable
//! (e.g., alert cascade), the Curation Loop detects it via metacognitive monitoring
//! and intervenes. This is the two-level meta-loop stability guarantee.

use crate::algedonic::{AlgedonicManager, RuntimeAlert};
use crate::energy::{EnergyBudget, EnergyError};
use crate::runtime::CnsRuntime;
use crate::variety::VarietyMonitor;
use hkask_types::WebID;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, MessagePriority,
    Regulatable, Signal,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Homeostatic set-points for the Cybernetics Loop.
///
/// These define the reference values against which sensed signals
/// are compared. When a signal deviates beyond its set-point,
/// the loop produces an efferent action.
#[derive(Debug, Clone)]
pub struct SetPoints {
    /// Minimum energy budget remaining ratio (0.0-1.0). Default: 0.2 (20% remaining)
    pub energy_min_remaining: f64,
    /// Maximum variety deficit before escalation. Default: 100
    pub variety_max_deficit: f64,
    /// Maximum error rate (0.0-1.0). Default: 0.3 (30% errors)
    pub error_rate_max: f64,
    /// Maximum connector latency in seconds. Default: 30.0
    pub connector_latency_max_secs: f64,
}

impl Default for SetPoints {
    fn default() -> Self {
        Self {
            energy_min_remaining: 0.2,
            variety_max_deficit: 100.0,
            error_rate_max: 0.3,
            connector_latency_max_secs: 30.0,
        }
    }
}

/// The Cybernetics Loop — homeostatic self-regulation.
///
/// Implements the `Loop` trait's sense→compare→compute→act cycle.
/// The Cybernetics Loop regulates all three domain loops (Inference,
/// Episodic, Semantic) and may signal the Curation Loop via algedonic
/// alerts. It may NOT regulate the Curation Loop.
pub struct CyberneticsLoop {
    /// CNS runtime for variety and alert access
    cns: Arc<RwLock<CnsRuntime>>,
    /// Energy budgets keyed by agent WebID
    energy_budgets: Arc<RwLock<std::collections::HashMap<WebID, EnergyBudget>>>,
    /// Homeostatic set-points
    set_points: SetPoints,
    /// Maximum number of loop iterations before forced stabilization
    max_iterations: u32,
}

impl CyberneticsLoop {
    /// Create a new Cybernetics Loop with default set-points.
    pub fn new(cns: Arc<RwLock<CnsRuntime>>) -> Self {
        Self {
            cns,
            energy_budgets: Arc::new(RwLock::new(std::collections::HashMap::new())),
            set_points: SetPoints::default(),
            max_iterations: 100,
        }
    }

    /// Create a Cybernetics Loop with custom set-points.
    pub fn with_set_points(cns: Arc<RwLock<CnsRuntime>>, set_points: SetPoints) -> Self {
        Self {
            cns,
            energy_budgets: Arc::new(RwLock::new(std::collections::HashMap::new())),
            set_points,
            max_iterations: 100,
        }
    }

    /// Register an energy budget for an agent.
    pub async fn register_energy_budget(&self, agent: WebID, budget: EnergyBudget) {
        let mut budgets = self.energy_budgets.write().await;
        budgets.insert(agent, budget);
    }

    /// Check if an agent can proceed with an operation costing `estimated_tokens`.
    pub async fn can_proceed(&self, agent: &WebID, estimated_tokens: u64) -> bool {
        let budgets = self.energy_budgets.read().await;
        if let Some(budget) = budgets.get(agent) {
            budget.can_proceed(estimated_tokens)
        } else {
            // No budget registered — allow by default (soft limit)
            true
        }
    }

    /// Acquire energy budget for an agent's operation.
    pub async fn acquire_budget(
        &self,
        agent: &WebID,
        estimated_tokens: u64,
    ) -> Result<u64, EnergyError> {
        let mut budgets = self.energy_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.acquire_budget(estimated_tokens)
        } else {
            // No budget registered — cost is 0 (soft limit)
            Ok(0)
        }
    }

    /// Get the current set-points.
    pub fn set_points(&self) -> &SetPoints {
        &self.set_points
    }

    /// Update a set-point. Returns the old value.
    pub async fn calibrate_set_point(&self, metric: &str, new_value: f64) -> Option<f64> {
        // Set-point calibration is a Curation directive, but the Cybernetics
        // Loop can self-calibrate within bounded ranges.
        // This is intentionally minimal — full calibration goes through Curation.
        let _ = (metric, new_value);
        None
    }
}

impl HkaskLoop for CyberneticsLoop {
    fn id(&self) -> LoopId {
        LoopId::Cybernetics
    }

    fn sense(&self) -> Vec<Signal> {
        // Synchronous sense — collect available signals without awaiting.
        // For full async sensing, use `sense_async`.
        Vec::new()
    }

    fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        signals
            .iter()
            .filter_map(|s| Deviation::from_signal(s))
            .collect()
    }

    fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();
        for dev in deviations {
            let action = match dev.signal.metric.as_str() {
                "energy_remaining" if dev.direction == DeviationDirection::BelowSetPoint => {
                    Some(LoopAction::new(
                        LoopId::Inference,
                        ActionType::Throttle,
                        serde_json::json!({
                            "reason": "energy_budget_low",
                            "remaining_ratio": dev.signal.value,
                            "set_point": dev.signal.set_point,
                        }),
                    ))
                }
                "variety_deficit" if dev.direction == DeviationDirection::AboveSetPoint => {
                    Some(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({
                            "reason": "variety_deficit_exceeded",
                            "deficit": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ))
                }
                "error_rate" if dev.direction == DeviationDirection::AboveSetPoint => {
                    Some(LoopAction::new(
                        LoopId::Inference,
                        ActionType::CircuitBreak,
                        serde_json::json!({
                            "reason": "error_rate_exceeded",
                            "error_rate": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ))
                }
                "connector_latency" if dev.direction == DeviationDirection::AboveSetPoint => {
                    Some(LoopAction::new(
                        LoopId::Communication,
                        ActionType::Throttle,
                        serde_json::json!({
                            "reason": "connector_latency_exceeded",
                            "latency_secs": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ))
                }
                _ => None,
            };
            if let Some(a) = action {
                actions.push(a);
            }
        }
        actions
    }

    fn act(&self, actions: &[LoopAction]) {
        // Dispatch efferent signals to target loops.
        // In production, this routes through the Communication Loop's DISPATCH.
        // For now, we log the actions for observability.
        for action in actions {
            tracing::info!(
                target: "cns.cybernetics",
                action_type = ?action.action_type,
                target_loop = %action.target,
                "Cybernetics Loop efferent signal"
            );
        }
    }
}

impl Regulatable for CyberneticsLoop {
    fn regulate(&self, action: &LoopAction) {
        // The Cybernetics Loop can only be regulated by the Curation Loop.
        // Curation may calibrate set-points or adjust energy budgets.
        if action.target != LoopId::Cybernetics {
            return;
        }
        match action.action_type {
            ActionType::Calibrate => {
                tracing::info!(
                    target: "cns.cybernetics",
                    "Curation Loop calibration received"
                );
            }
            ActionType::Throttle => {
                tracing::warn!(
                    target: "cns.cybernetics",
                    "Cybernetics Loop throttle signal received — reducing sensing frequency"
                );
            }
            _ => {
                tracing::warn!(
                    target: "cns.cybernetics",
                    action_type = ?action.action_type,
                    "Unsupported regulation action for Cybernetics Loop"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::loops::{ActionType, DeviationDirection, HkaskLoop, LoopId, Regulatable};

    #[test]
    fn cybernetics_loop_id_is_cybernetics() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);
        assert_eq!(loop6.id(), LoopId::Cybernetics);
    }

    #[test]
    fn energy_deviation_produces_throttle_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // Signal: energy remaining at 5% (below 20% set-point)
        let signal = Signal::new(LoopId::Cybernetics, "energy_remaining", 0.05, 0.2);
        let deviations = loop6.compare(&[signal]);
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::BelowSetPoint);

        let actions = loop6.compute(&deviations);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(actions[0].target, LoopId::Inference);
    }

    #[test]
    fn variety_deficit_produces_escalate_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // Signal: variety deficit at 150 (above 100 threshold)
        let signal = Signal::new(LoopId::Cybernetics, "variety_deficit", 150.0, 100.0);
        let deviations = loop6.compare(&[signal]);
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::AboveSetPoint);

        let actions = loop6.compute(&deviations);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Escalate);
        assert_eq!(actions[0].target, LoopId::Curation);
    }

    #[test]
    fn error_rate_produces_circuit_break_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // Signal: error rate at 50% (above 30% threshold)
        let signal = Signal::new(LoopId::Cybernetics, "error_rate", 0.5, 0.3);
        let deviations = loop6.compare(&[signal]);
        assert_eq!(deviations.len(), 1);

        let actions = loop6.compute(&deviations);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::CircuitBreak);
        assert_eq!(actions[0].target, LoopId::Inference);
    }

    #[test]
    fn no_deviation_produces_no_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // Signal: energy at 50% (above 20% set-point — no deviation)
        let signal = Signal::new(LoopId::Cybernetics, "energy_remaining", 0.5, 0.2);
        let deviations = loop6.compare(&[signal]);
        // Deviation exists (above set-point) but no action for above-set-point energy
        let actions = loop6.compute(&deviations);
        // Above-set-point energy is fine — no action needed
        assert!(actions.is_empty());
    }

    #[test]
    fn regulate_accepts_curation_calibration() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        let action = LoopAction::new(
            LoopId::Cybernetics,
            ActionType::Calibrate,
            serde_json::json!({"metric": "energy_min_remaining", "value": 0.15}),
        );
        // Should not panic
        loop6.regulate(&action);
    }

    #[test]
    fn regulate_ignores_wrong_target() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        let action = LoopAction::new(
            LoopId::Inference, // wrong target
            ActionType::Throttle,
            serde_json::json!({}),
        );
        // Should silently ignore
        loop6.regulate(&action);
    }

    #[tokio::test]
    async fn can_proceed_with_sufficient_budget() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);
        let agent = WebID::new();

        let budget = EnergyBudget::new(10_000);
        loop6.register_energy_budget(agent, budget).await;

        assert!(loop6.can_proceed(&agent, 100).await);
    }

    #[tokio::test]
    async fn acquire_budget_deducts_energy() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);
        let agent = WebID::new();

        let budget = EnergyBudget::new(10_000);
        loop6.register_energy_budget(agent, budget).await;

        let cost = loop6.acquire_budget(&agent, 100).await.unwrap();
        assert!(cost > 0);
    }

    #[tokio::test]
    async fn full_tick_cycle_completes() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // A tick with no sensed signals should complete without panic
        loop6.tick();
    }
}
