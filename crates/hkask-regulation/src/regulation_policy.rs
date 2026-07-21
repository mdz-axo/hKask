//! RegulationPolicy — data-driven per-metric regulation rules
//!
//! Consolidates the per-metric action mappings, severity thresholds,
//! and classification thresholds that were previously scattered across
//! `cybernetics_loop.rs`. Each `RegulationRule` defines what actions
//! to take when a specific metric deviates in a specific direction.

use crate::types::loops::{
    ActionDecision, ActionType, Deviation, DeviationDirection, LoopId, RegulationData, SignalMetric,
};

/// A proposed action before substitution and mode-specific filtering.
///
/// The `compute()` method applies `try_substitute` and mode checks
/// to finalize these into `RegulatoryAction` instances.
///
/// Fields are read by `build_regulation_action` via string matching
/// on `reason`; `target`, `action_type`, `data`, and `metric_name`
/// serve as documentation of each rule's intent. `data` is `None` in
/// the policy table because concrete values come from the `Deviation`
/// at runtime.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProposedAction {
    pub target: LoopId,
    pub action_type: ActionType,
    pub reason: &'static str,
    pub data: Option<RegulationData>,
    pub metric_name: Option<&'static str>,
}

/// A single regulation rule: when `metric` deviates in `direction`,
/// produce `proposed` actions with the given severity classification.
pub struct RegulationRule {
    pub metric: SignalMetric,
    pub direction: DeviationDirection,
    /// The proposed actions for this rule. A single rule can produce
    /// multiple proposed actions (e.g., EnergyRemaining triggers both
    /// Throttle and AdjustEnergyBudget).
    pub proposed: &'static [ProposedAction],
}

/// Consolidates all per-metric regulation rules.
///
/// Fuel source: declaration of what actions to propose when a metric
/// deviates. Runtime concerns (substitution ladders, throttle modes)
/// are handled by the caller in `compute()`.
pub struct RegulationPolicy {
    rules: Vec<RegulationRule>,
}

impl RegulationPolicy {
    /// Build the default regulation policy with all currently-supported rules.
    pub fn default() -> Self {
        use ActionType::*;
        use DeviationDirection::*;
        use LoopId::*;
        use SignalMetric::*;

        Self {
            rules: vec![
                // 1. EnergyRemaining BelowSetPoint → Throttle (for Autonomous mode)
                RegulationRule {
                    metric: EnergyRemaining,
                    direction: BelowSetPoint,
                    proposed: &[ProposedAction {
                        target: Inference,
                        action_type: Throttle,
                        reason: "energy_budget_low",
                        data: None,
                        metric_name: Some("energy_remaining"),
                    }],
                },
                // 2. EnergyRemaining BelowSetPoint → Escalate (for CuratorMediated mode)
                RegulationRule {
                    metric: EnergyRemaining,
                    direction: BelowSetPoint,
                    proposed: &[ProposedAction {
                        target: Curation,
                        action_type: Escalate,
                        reason: "budget_guard_escalation",
                        data: None,
                        metric_name: Some("energy_remaining"),
                    }],
                },
                // 3. EnergyRemaining BelowSetPoint → AdjustEnergyBudget (for non-Off modes)
                RegulationRule {
                    metric: EnergyRemaining,
                    direction: BelowSetPoint,
                    proposed: &[ProposedAction {
                        target: Cybernetics,
                        action_type: AdjustEnergyBudget,
                        reason: "energy_depletion_auto_adjust",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 4. VarietyDeficit AboveSetPoint → Escalate
                RegulationRule {
                    metric: VarietyDeficit,
                    direction: AboveSetPoint,
                    proposed: &[ProposedAction {
                        target: Curation,
                        action_type: Escalate,
                        reason: "variety_deficit_exceeded",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 5. ErrorRate AboveSetPoint → CircuitBreak
                RegulationRule {
                    metric: ErrorRate,
                    direction: AboveSetPoint,
                    proposed: &[ProposedAction {
                        target: Inference,
                        action_type: CircuitBreak,
                        reason: "error_rate_exceeded",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 6. ConnectorLatency AboveSetPoint → Throttle
                RegulationRule {
                    metric: ConnectorLatency,
                    direction: AboveSetPoint,
                    proposed: &[ProposedAction {
                        target: Cybernetics,
                        action_type: Throttle,
                        reason: "connector_latency_exceeded",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 7. CommunicationQueueDepth AboveSetPoint → Throttle
                RegulationRule {
                    metric: CommunicationQueueDepth,
                    direction: AboveSetPoint,
                    proposed: &[ProposedAction {
                        target: Cybernetics,
                        action_type: Throttle,
                        reason: "communication_backpressure",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 8. WalletBalanceRatio BelowSetPoint → Escalate
                RegulationRule {
                    metric: WalletBalanceRatio,
                    direction: BelowSetPoint,
                    proposed: &[ProposedAction {
                        target: Curation,
                        action_type: Escalate,
                        reason: "wallet_balance_low",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 9. WalletKeyHealth AboveSetPoint → Escalate
                RegulationRule {
                    metric: WalletKeyHealth,
                    direction: AboveSetPoint,
                    proposed: &[ProposedAction {
                        target: Curation,
                        action_type: Escalate,
                        reason: "wallet_key_unhealthy",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 10. SeamCoverage BelowSetPoint → Escalate
                RegulationRule {
                    metric: SeamCoverage,
                    direction: BelowSetPoint,
                    proposed: &[ProposedAction {
                        target: Curation,
                        action_type: Escalate,
                        reason: "seam_coverage_degraded",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 11. SeamCoverage AboveSetPoint → Notify
                RegulationRule {
                    metric: SeamCoverage,
                    direction: AboveSetPoint,
                    proposed: &[ProposedAction {
                        target: Curation,
                        action_type: Notify,
                        reason: "seam_coverage_improved",
                        data: None,
                        metric_name: None,
                    }],
                },
                // 12. ToolReliability BelowSetPoint → Escalate
                RegulationRule {
                    metric: ToolReliability,
                    direction: BelowSetPoint,
                    proposed: &[ProposedAction {
                        target: Curation,
                        action_type: Escalate,
                        reason: "tool_reliability_degraded",
                        data: None,
                        metric_name: None,
                    }],
                },
            ],
        }
    }

    /// Find all proposed actions for a given deviation.
    ///
    /// Returns a flat list of `ProposedAction` references matching
    /// the deviation's `(metric, direction)`. The caller applies
    /// `try_substitute`, mode filtering, and data population.
    pub fn decide(&self, dev: &Deviation) -> Vec<&ProposedAction> {
        self.rules
            .iter()
            .filter(|r| r.metric == dev.signal.metric && r.direction == dev.direction)
            .flat_map(|r| r.proposed.iter())
            .collect()
    }
}

/// Extract (deficit, threshold) from a `RegulationData` variant.
/// Returns (0, 0) when the variant doesn't carry deficit/threshold.
pub fn extract_deficit_threshold(data: &RegulationData) -> (u64, u64) {
    match data {
        RegulationData::VarietyDeficitExceeded { deficit, threshold } => {
            (*deficit as u64, *threshold as u64)
        }
        _ => (0, 0),
    }
}

/// Classify an action's impact decision using Fermi's three-tier gate.
///
/// - `worsening`: absolute value of the negative delta (0.0 if improved).
/// - `stage_ratio`: below this → Accept (noise).
/// - `block_ratio`: at or above this → Block (hard reject).
/// - Between → Stage (escalate for review).
pub fn classify_decision(worsening: f64, stage_ratio: f64, block_ratio: f64) -> ActionDecision {
    debug_assert!(
        stage_ratio <= block_ratio,
        "stage_worsening_ratio ({stage_ratio}) must be <= block_worsening_ratio ({block_ratio})"
    );
    if worsening >= block_ratio {
        ActionDecision::Block
    } else if worsening < stage_ratio {
        ActionDecision::Accept
    } else {
        ActionDecision::Stage
    }
}

/// Return the default substitution ladder for a metric.
///
/// These are the built-in ladders used when no custom ladders are configured
/// via `SetPoints.action_substitutions`. Each ladder is an ordered list of
/// action types to try when the primary action is repeatedly ineffective.
pub fn default_substitution_ladder(metric: SignalMetric) -> &'static [ActionType] {
    use ActionType::*;
    match metric {
        SignalMetric::EnergyRemaining => &[Throttle, AdjustEnergyBudget, Escalate],
        SignalMetric::VarietyDeficit => &[Escalate, Calibrate, OverrideEnergyBudget],
        SignalMetric::ErrorRate => &[CircuitBreak, Calibrate, Escalate],
        SignalMetric::ConnectorLatency => &[Throttle, Calibrate, Escalate],
        SignalMetric::CommunicationQueueDepth => &[Throttle, Escalate],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_deviation(metric: SignalMetric, value: f64, set_point: f64) -> Deviation {
        use crate::types::loops::Signal;
        let signal = Signal {
            source: LoopId::Cybernetics,
            metric,
            value,
            set_point,
            timestamp: Utc::now(),
        };
        Deviation::from_signal(&signal).unwrap()
    }

    #[test]
    fn policy_matches_energy_below_setpoint() {
        let policy = RegulationPolicy::default();
        let dev = make_deviation(SignalMetric::EnergyRemaining, 0.3, 0.5);
        let proposed = policy.decide(&dev);
        assert_eq!(proposed.len(), 3);
        assert_eq!(proposed[0].action_type, ActionType::Throttle);
        assert_eq!(proposed[1].action_type, ActionType::Escalate);
        assert_eq!(proposed[2].action_type, ActionType::AdjustEnergyBudget);
    }

    #[test]
    fn policy_matches_variety_above_setpoint() {
        let policy = RegulationPolicy::default();
        let dev = make_deviation(SignalMetric::VarietyDeficit, 15.0, 10.0);
        let proposed = policy.decide(&dev);
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0].action_type, ActionType::Escalate);
        assert_eq!(proposed[0].target, LoopId::Curation);
    }

    #[test]
    fn policy_matches_error_rate_above_setpoint() {
        let policy = RegulationPolicy::default();
        let dev = make_deviation(SignalMetric::ErrorRate, 0.15, 0.05);
        let proposed = policy.decide(&dev);
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0].action_type, ActionType::CircuitBreak);
    }

    #[test]
    fn policy_matches_seam_coverage_below_setpoint() {
        let policy = RegulationPolicy::default();
        let dev = make_deviation(SignalMetric::SeamCoverage, 80.0, 90.0);
        let proposed = policy.decide(&dev);
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0].action_type, ActionType::Escalate);
    }

    #[test]
    fn policy_matches_seam_coverage_above_setpoint() {
        let policy = RegulationPolicy::default();
        let dev = make_deviation(SignalMetric::SeamCoverage, 95.0, 90.0);
        let proposed = policy.decide(&dev);
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0].action_type, ActionType::Notify);
    }

    #[test]
    fn policy_no_match_for_unregistered_metric() {
        let policy = RegulationPolicy::default();
        let dev = make_deviation(SignalMetric::DiskUsagePct, 85.0, 80.0);
        let proposed = policy.decide(&dev);
        assert!(proposed.is_empty());
    }

    #[test]
    fn classify_decision_accept_noise() {
        assert_eq!(classify_decision(0.03, 0.05, 0.20), ActionDecision::Accept);
    }

    #[test]
    fn classify_decision_stage_moderate() {
        assert_eq!(classify_decision(0.10, 0.05, 0.20), ActionDecision::Stage);
    }

    #[test]
    fn classify_decision_block_severe() {
        assert_eq!(classify_decision(0.25, 0.05, 0.20), ActionDecision::Block);
    }

    #[test]
    fn default_substitution_ladders_are_nonempty_for_regulated_metrics() {
        assert!(!default_substitution_ladder(SignalMetric::EnergyRemaining).is_empty());
        assert!(!default_substitution_ladder(SignalMetric::VarietyDeficit).is_empty());
        assert!(!default_substitution_ladder(SignalMetric::ErrorRate).is_empty());
        assert!(!default_substitution_ladder(SignalMetric::ConnectorLatency).is_empty());
        assert!(!default_substitution_ladder(SignalMetric::CommunicationQueueDepth).is_empty());
    }

    #[test]
    fn default_substitution_ladders_empty_for_unregulated_metrics() {
        assert!(default_substitution_ladder(SignalMetric::DiskUsagePct).is_empty());
        assert!(default_substitution_ladder(SignalMetric::MemoryLife).is_empty());
    }
}
