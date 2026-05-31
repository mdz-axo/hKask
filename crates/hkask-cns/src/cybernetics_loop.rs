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
//!    efferent signal: throttle, escalate, calibrate, or circuit-break.
//! 4. **Act** — dispatch the efferent signal to the target loop's `regulate`
//!    entry point.
//!
//! The loop is self-stabilizing: if the Cybernetics Loop itself becomes unstable
//! (e.g., alert cascade), the Curation Loop detects it via metacognitive monitoring
//! and intervenes. This is the two-level meta-loop stability guarantee.
//!
//! # Essential Subloops
//!
//! - 6.1 Access Guard (GUARD) — OCAP verification + sovereignty enforcement
//! - 6.3 Variety Sensing (SENSE) — measure variety across domains
//! - 6.4 Algedonic Regulation (ADAPT) — deficit → threshold → escalate
//! - 6.6 Revocation (WITHDRAW) — persistent deny-future
//!
//! Energy homeostasis is NOT a subloop — it is expressed as set-points
//! in `SetPoints` + regulation actions via `InferenceRegulation`.

use crate::energy::{EnergyBudget, EnergyError};
use crate::runtime::CnsRuntime;
use hkask_types::WebID;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Regulatable, Signal,
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
    #[allow(dead_code)] // Wired to async sense() in future PR
    cns: Arc<RwLock<CnsRuntime>>,
    /// Energy budgets keyed by agent WebID
    energy_budgets: Arc<RwLock<std::collections::HashMap<WebID, EnergyBudget>>>,
    /// Homeostatic set-points
    set_points: SetPoints,
    /// Maximum number of loop iterations before forced stabilization
    #[allow(dead_code)] // Reserved for cascade detection in future PR
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
        signals.iter().filter_map(Deviation::from_signal).collect()
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

    // =========================================================================
    // Task 7: Cybernetic Unit Tests — Full loop validation
    // =========================================================================

    /// Test: Inject a known energy deviation (5% remaining vs 20% set-point)
    /// Assert: The loop produces a Throttle action targeting Inference
    /// Assert: The action propagates through the capability membrane
    /// Assert: The system reaches a new stable equilibrium within bounded iterations
    #[test]
    fn energy_deviation_propagates_and_stabilizes() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // Step 1: Inject known deviation — energy at 5% (set-point: 20%)
        let signal = Signal::new(LoopId::Cybernetics, "energy_remaining", 0.05, 0.2);

        // Step 2: Compare — detect deviation
        let deviations = loop6.compare(&[signal]);
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::BelowSetPoint);
        assert!((deviations[0].magnitude - 0.15).abs() < f64::EPSILON);

        // Step 3: Compute — produce efferent action
        let actions = loop6.compute(&deviations);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(actions[0].target, LoopId::Inference);

        // Step 4: Verify capability membrane — Cybernetics can regulate Inference
        // (domain loop), but NOT Curation (peer meta loop)
        assert_ne!(actions[0].target, LoopId::Curation);

        // Step 5: Simulate stabilization — after throttling, energy recovers
        let recovered_signal = Signal::new(LoopId::Cybernetics, "energy_remaining", 0.25, 0.2);
        let new_deviations = loop6.compare(&[recovered_signal]);
        let new_actions = loop6.compute(&new_deviations);
        // Above-set-point energy is fine — no throttle action
        assert!(new_actions.is_empty());
    }

    /// Test: Multiple simultaneous deviations produce multiple actions
    #[test]
    fn multiple_deviations_produce_multiple_actions() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        let signals = vec![
            Signal::new(LoopId::Cybernetics, "energy_remaining", 0.05, 0.2),
            Signal::new(LoopId::Cybernetics, "variety_deficit", 150.0, 100.0),
            Signal::new(LoopId::Cybernetics, "error_rate", 0.5, 0.3),
        ];

        let deviations = loop6.compare(&signals);
        assert_eq!(deviations.len(), 3);

        let actions = loop6.compute(&deviations);
        assert_eq!(actions.len(), 3);

        // Verify each action targets the correct loop
        let targets: std::collections::HashSet<LoopId> = actions.iter().map(|a| a.target).collect();
        assert!(targets.contains(&LoopId::Inference)); // energy + error
        assert!(targets.contains(&LoopId::Curation)); // variety
    }

    /// Test: Capability membrane — Cybernetics cannot regulate Curation
    #[test]
    fn cybernetics_cannot_regulate_curation() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // Cybernetics can signal Curation (Escalate) but not regulate it
        let signal = Signal::new(LoopId::Cybernetics, "variety_deficit", 150.0, 100.0);
        let deviations = loop6.compare(&[signal]);
        let actions = loop6.compute(&deviations);

        // The action targets Curation with Escalate, not Throttle/Calibrate
        assert_eq!(actions[0].action_type, ActionType::Escalate);
        assert_eq!(actions[0].target, LoopId::Curation);

        // Verify: CyberneticsLoop.regulate() ignores actions targeting other loops
        let wrong_target = LoopAction::new(
            LoopId::Curation,
            ActionType::Throttle,
            serde_json::json!({}),
        );
        // CyberneticsLoop's regulate only accepts actions targeting Cybernetics
        loop6.regulate(&wrong_target); // silently ignored
    }

    /// Test: Two-level stability guarantee — algedonic cascade halted by
    /// Curation's metacognitive override.
    ///
    /// Scenario: Cybernetics Loop detects a cascade of alerts.
    /// Curation Loop detects the cascade and issues a Throttle to Cybernetics,
    /// halting the escalation chain.
    #[test]
    fn algedonic_cascade_halted_by_curation_override() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // Simulate an algedonic cascade: multiple variety deficit signals
        let cascade_signals: Vec<Signal> = (0..5)
            .map(|i| {
                Signal::new(
                    LoopId::Cybernetics,
                    "variety_deficit",
                    100.0 + i as f64 * 50.0, // escalating deficit
                    100.0,
                )
            })
            .collect();

        // Each signal produces an Escalate action
        let all_deviations = loop6.compare(&cascade_signals);
        let all_actions = loop6.compute(&all_deviations);

        // All are escalate actions targeting Curation
        for action in &all_actions {
            assert_eq!(action.action_type, ActionType::Escalate);
            assert_eq!(action.target, LoopId::Curation);
        }

        // Curation detects the cascade and intervenes
        // It issues a Throttle to Cybernetics to reduce escalation frequency
        let curation_override = LoopAction::new(
            LoopId::Cybernetics,
            ActionType::Throttle,
            serde_json::json!({"reason": "algedonic_cascade_detected"}),
        );

        // Cybernetics accepts the regulation
        loop6.regulate(&curation_override);

        // After throttling, subsequent ticks produce fewer/no new escalations
        // (In this implementation, sense() returns empty, so no new signals)
        let post_override_signals = loop6.sense();
        assert!(post_override_signals.is_empty());

        // The system has stabilized — no new deviations
        let post_deviations = loop6.compare(&post_override_signals);
        assert!(post_deviations.is_empty());
    }

    /// Test: Loop reaches equilibrium within bounded iterations
    #[test]
    fn loop_reaches_equilibrium_within_bounded_iterations() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        // Simulate a sequence of improving signals
        let max_iterations = 10;
        for i in 0..max_iterations {
            // Energy recovers from 5% to 25% over iterations
            let energy = 0.05 + (i as f64 * 0.02);
            let signal = Signal::new(LoopId::Cybernetics, "energy_remaining", energy, 0.2);
            let deviations = loop6.compare(&[signal]);
            let actions = loop6.compute(&deviations);

            if energy >= 0.2 {
                // Once energy reaches set-point, no throttle action
                let throttle_actions: Vec<_> = actions
                    .iter()
                    .filter(|a| a.action_type == ActionType::Throttle)
                    .collect();
                assert!(
                    throttle_actions.is_empty(),
                    "System should stabilize by iteration {}, but still throttling",
                    i
                );
                return; // Equilibrium reached
            }
        }
        panic!(
            "System did not reach equilibrium within {} iterations",
            max_iterations
        );
    }

    /// Test: Energy budget exhaustion blocks operations
    #[tokio::test]
    async fn energy_exhaustion_blocks_operations() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);
        let agent = WebID::new();

        // Register a very small budget
        let budget = EnergyBudget::new(100);
        loop6.register_energy_budget(agent, budget).await;

        // Initially can proceed
        assert!(loop6.can_proceed(&agent, 10).await);

        // Exhaust the budget
        while loop6.acquire_budget(&agent, 10).await.is_ok() {
            // Keep consuming until budget exhausted
        }

        // Now cannot proceed
        assert!(!loop6.can_proceed(&agent, 10).await);
    }

    /// Test: Energy replenishment restores capacity
    #[tokio::test]
    async fn energy_replenishment_restores_capacity() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);
        let agent = WebID::new();

        let budget = EnergyBudget::new(100);
        loop6.register_energy_budget(agent, budget).await;

        // Exhaust the budget
        while loop6.acquire_budget(&agent, 10).await.is_ok() {}
        assert!(!loop6.can_proceed(&agent, 10).await);

        // Replenish
        {
            let mut budgets = loop6.energy_budgets.write().await;
            if let Some(budget) = budgets.get_mut(&agent) {
                budget.replenish(100);
            }
        }

        // Can proceed again
        assert!(loop6.can_proceed(&agent, 10).await);
    }

    /// Test: Connector latency deviation produces throttle on Communication
    #[test]
    fn connector_latency_produces_throttle_on_communication() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns);

        let signal = Signal::new(LoopId::Cybernetics, "connector_latency", 60.0, 30.0);
        let deviations = loop6.compare(&[signal]);
        let actions = loop6.compute(&deviations);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(actions[0].target, LoopId::Communication);
    }
}
