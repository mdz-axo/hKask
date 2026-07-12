//! Core loop types — identifiers, the Loop trait, and quality telemetry.
//!
//! Moved from hkask-cns to hkask-types to break the circular dependency
//! that prevented extracting CNS subcrates. The Loop trait uses async-trait
//! for object safety.

use super::actions::{ActionType, LoopAction};
use super::signals::{Deviation, DeviationDirection, SignalMetric};

/// Loop identifiers for the 6-loop model.
///
/// VSM correspondence:
/// - Loop 1:  Inference    (S1 Implementation)
/// - Loop 2a: Episodic     (S2 Coordination — private memory)
/// - Loop 2b: Semantic     (S2 Coordination — shared memory)
/// - Loop 5:  Curation     (S4 Intelligence — meta-observer)
/// - Loop 6:  Cybernetics  (S3 Control — homeostatic regulation)
/// - Loop 6b: Snapshot     (S3 Control — scheduled CAS snapshots)
/// - Loop 7: StorageGuard  (S3 Control — autonomous disk space management)
///
/// No Loop 3: Control absorbed into Cybernetics (intentional).
/// No Loop 4: VSM S4 = Curation (Loop 5).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LoopId {
    Inference,
    Episodic,
    Semantic,
    Curation,
    Cybernetics,
    Snapshot,
    StorageGuard,
    /// Loop 8: McpServerGuard (S3 Control — proactive MCP server health monitoring)
    McpServerGuard,
}

impl std::fmt::Display for LoopId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopId::Inference => write!(f, "inference"),
            LoopId::Episodic => write!(f, "episodic"),
            LoopId::Semantic => write!(f, "semantic"),
            LoopId::Curation => write!(f, "curation"),
            LoopId::Cybernetics => write!(f, "cybernetics"),
            LoopId::Snapshot => write!(f, "snapshot"),
            LoopId::StorageGuard => write!(f, "storage_guard"),
            LoopId::McpServerGuard => write!(f, "mcp_server_guard"),
        }
    }
}

/// What triggered this regulation cycle.
///
/// Adapted from Fermi's `TriggerReason` pattern — recording provenance
/// enables CNS to correlate trigger type with regulatory effectiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerOrigin {
    /// Regular scheduled tick (timer-driven).
    Scheduled,
    /// Triggered by an incoming algedonic alert.
    AlertDriven,
    /// Manually invoked via operator directive.
    Manual,
    /// Triggered by an external event (ν-event, goal transition, etc.).
    EventDriven,
}

/// Result of verifying whether a regulatory action improved its target metric.
///
/// Fermi pattern: the "impact gate" — after acting, re-sense the targeted
/// metric and compare against the pre-action value. This closes the cybernetic
/// feedback loop: sense → compare → compute → act → **verify**.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImpactReport {
    /// The action that was verified.
    pub action_type: ActionType,
    /// The metric the action targeted.
    pub metric: SignalMetric,
    /// Metric value before the action was applied.
    pub before: f64,
    /// Metric value after the action was applied (re-sensed).
    pub after: f64,
    /// Absolute change: after − before.
    pub delta: f64,
    /// Did the metric move in the intended direction?
    pub improved: bool,
    /// Classification decision based on the impact magnitude.
    pub decision: ActionDecision,
}

impl ImpactReport {
    /// Construct an ImpactReport, computing `improved` from the metric semantics.
    ///
    /// expect: "The system closes the cybernetic feedback loop by measuring action impact"
    /// [P9] Homeostatic Self-Regulation — impact verification closes the regulation cycle
    /// pre:  metric is a valid SignalMetric; before and after are sane numeric values
    /// post: returns ImpactReport with delta=after-before, improved computed per metric semantics
    ///
    /// `decision` should be computed via `RegulationRule::classify()` by the caller.
    pub fn new(
        action_type: ActionType,
        metric: SignalMetric,
        before: f64,
        after: f64,
        decision: ActionDecision,
    ) -> Self {
        let delta = after - before;
        let improved = match metric {
            SignalMetric::EnergyRemaining => delta > 0.0,
            SignalMetric::VarietyDeficit => delta < 0.0,
            _ => delta.abs() > f64::EPSILON,
        };
        Self {
            action_type,
            metric,
            before,
            after,
            delta,
            improved,
            decision,
        }
    }
}

/// Three-tier decision gate for verified actions (Fermi impact-gate pattern).
///
/// After re-sensing the target metric post-action, classify the outcome:
/// - **Accept** — action improved the metric or worsened within noise tolerance.
/// - **Stage** — action was moderately ineffective; escalate as Warning for review.
/// - **Block** — action was severely counterproductive; prevent re-use for this metric.
///
/// Thresholds are per-metric configurable via SetPoints. Defaults:
/// - Stage threshold: 5% relative worsening.
/// - Block threshold: 20% relative worsening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionDecision {
    /// Action was effective or within noise tolerance. Continue.
    Accept,
    /// Action was moderately ineffective — worth reviewing. Escalate as Warning.
    Stage,
    /// Action was severely counterproductive — prevent re-use. Escalate as Critical.
    Block,
}

/// Loop-quality telemetry — measures the loop's own performance.
///
/// These metrics are about the loop itself, not the signals it processes.
/// They enable CNS observability of loop health: is the loop responding
/// quickly enough? Is it producing appropriate actions for detected deviations?
/// Are those actions actually effective?
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LoopQuality {
    /// Milliseconds between sense start and act completion (loop latency).
    pub delay_ms: u64,
    /// Ratio of actions produced to deviations detected (responsiveness).
    /// 1.0 = every deviation produced an action. 0.0 = no actions produced.
    pub gain: f64,
    /// How well actions match deviations (0.0–1.0).
    /// 1.0 = every deviation had a corresponding action.
    /// Computed as: matched_deviations / total_deviations.
    pub fidelity_score: f64,
    /// Ratio of actions that actually improved their target metric (0.0–1.0).
    ///
    /// Fermi impact-gate pattern: 1.0 = every verified action moved its
    /// metric toward the set-point. 0.0 = no action had measurable impact.
    /// Only computed when `verify_impact` returns reports; defaults to 1.0
    /// for loops that don't implement verification.
    pub effectiveness_score: f64,
    /// Confidence in the fidelity_score computation (0.0–1.0).
    /// 1.0 = all deviation-to-action matches used `metric_name` directly.
    /// 0.6 = one or more matches fell back to string heuristics on `reason`.
    pub fidelity_confidence: f64,
    /// What triggered this tick.
    pub trigger: TriggerOrigin,
}

impl Default for LoopQuality {
    fn default() -> Self {
        Self {
            delay_ms: 0,
            gain: 0.0,
            fidelity_score: 0.0,
            fidelity_confidence: 1.0,
            effectiveness_score: 1.0,
            trigger: TriggerOrigin::Scheduled,
        }
    }
}

impl LoopQuality {
    /// Compute loop quality from the cycle's inputs and outputs.
    ///
    /// expect: "The system measures its own regulatory effectiveness"
    /// [P9] Homeostatic Self-Regulation — loop quality enables CNS self-observation
    /// pre:  elapsed_ms is measured wall-clock time; deviations and actions are from
    ///       the same regulation cycle
    /// post: returns LoopQuality with gain, fidelity_score, effectiveness_score, and
    ///       fidelity_confidence computed from cycle data
    ///
    /// - `elapsed_ms`: wall-clock time from sense start to act end
    /// - `deviations`: deviations detected during compare
    /// - `actions`: actions produced during compute
    /// - `impact_reports`: results from `verify_impact` (empty → effectiveness = 1.0)
    /// - `trigger`: what triggered this tick
    pub fn from_cycle(
        elapsed_ms: u64,
        deviations: &[Deviation],
        actions: &[LoopAction],
        impact_reports: &[ImpactReport],
        trigger: TriggerOrigin,
    ) -> Self {
        let gain = if deviations.is_empty() {
            0.0
        } else {
            actions.len() as f64 / deviations.len() as f64
        };

        // Fidelity: count how many deviations had a matching action.
        // Prefer matching via `metric_name` (type-safe). Only fall back
        // to string-matching on `reason` when no action carries a metric_name.
        let mut fidelity_fallback_used = false;
        let matched = deviations
            .iter()
            .filter(|d| {
                let metric_str = d.signal.metric.as_str();
                // Primary: match by metric_name if any action carries it.
                if actions
                    .iter()
                    .any(|a| a.metric_name.as_deref() == Some(metric_str))
                {
                    return true;
                }
                // Fallback: string-match on reason (less reliable).
                let fallback_match = actions.iter().any(|a| {
                    let reason = &a.parameters.reason;
                    reason.contains(metric_str)
                        || match d.direction {
                            DeviationDirection::AboveSetPoint => reason.contains("exceeded"),
                            DeviationDirection::BelowSetPoint => {
                                reason.contains("low") || reason.contains("depletion")
                            }
                        }
                });
                if fallback_match {
                    fidelity_fallback_used = true;
                }
                fallback_match
            })
            .count() as f64;
        let fidelity_score = if deviations.is_empty() {
            0.0
        } else {
            matched / deviations.len() as f64
        };
        let fidelity_confidence = if fidelity_fallback_used { 0.6 } else { 1.0 };

        // Effectiveness: percentage of verified actions that were Accepted
        // (i.e., either improved or within noise tolerance). Staged/Blocked
        // actions reduce the score.
        let effectiveness_score = if impact_reports.is_empty() {
            1.0
        } else {
            let accepted = impact_reports
                .iter()
                .filter(|r| r.decision == ActionDecision::Accept)
                .count() as f64;
            accepted / impact_reports.len() as f64
        };

        Self {
            delay_ms: elapsed_ms,
            gain,
            fidelity_score,
            fidelity_confidence,
            effectiveness_score,
            trigger,
        }
    }
}
