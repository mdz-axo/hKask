//! Core loop types — identifiers, the Loop trait, and quality telemetry.

use super::actions::LoopAction;
use super::signals::{Deviation, DeviationDirection, Signal};

// LoopId is defined in hkask-types (canonical) — re-exported for local convenience.
pub use hkask_types::LoopId;

/// The Loop trait — sense → compare → compute → act.
///
/// Every loop implements this cycle. Authority flows downward
/// through the DAG: Curation → Cybernetics → domain loops.
///
/// Async trait (currently implemented via `async-trait` for object safety).
/// Native async traits are available in Rust 2024 edition; switching to them
/// would remove the `async-trait` dependency but must preserve `dyn Loop`
/// dispatch.
///
/// All async methods return `Send` futures so loops can run in
/// async tasks without `static bounds issues.

#[async_trait::async_trait]
pub trait Loop: Send + Sync {
    fn id(&self) -> LoopId;

    /// Sense: observe current state and produce afferent signals.
    async fn sense(&self) -> Vec<Signal>;

    /// Compare: detect deviations from set-points.
    async fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        signals.iter().filter_map(Deviation::from_signal).collect()
    }

    /// Compute: produce regulatory actions for detected deviations.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction>;

    /// Act: execute regulatory actions (route through Communication Loop).
    async fn act(&self, actions: &[LoopAction]);

    /// Full regulation cycle: sense → compare → compute → act.
    async fn tick(&self) {
        let signals = self.sense().await;
        let deviations = self.compare(&signals).await;
        let actions = self.compute(&deviations).await;
        self.act(&actions).await;
    }
}

/// Loop-quality telemetry — measures the loop's own performance.
///
/// These metrics are about the loop itself, not the signals it processes.
/// They enable CNS observability of loop health: is the loop responding
/// quickly enough? Is it producing appropriate actions for detected deviations?
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
}

impl Default for LoopQuality {
    fn default() -> Self {
        Self {
            delay_ms: 0,
            gain: 0.0,
            fidelity_score: 0.0,
        }
    }
}

impl LoopQuality {
    /// Compute loop quality from the cycle's inputs and outputs.
    ///
    /// - `elapsed_ms`: wall-clock time from sense start to act end
    /// - `deviations`: deviations detected during compare
    /// - `actions`: actions produced during compute
    pub fn from_cycle(elapsed_ms: u64, deviations: &[Deviation], actions: &[LoopAction]) -> Self {
        let total_deviations = deviations.len().max(1) as f64;
        let gain = actions.len() as f64 / total_deviations;

        // Fidelity: count how many deviations had a matching action.
        // A deviation is "matched" if any action's parameters reference
        // the same metric (via the "reason" field convention).
        let matched = deviations
            .iter()
            .filter(|d| {
                let metric_str = d.signal.metric.as_str();
                actions.iter().any(|a| {
                    a.parameters
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .is_some_and(|reason| {
                            // Heuristic: if the reason contains the metric name or
                            // the deviation direction, it's a match
                            reason.contains(metric_str)
                                || match d.direction {
                                    DeviationDirection::AboveSetPoint => {
                                        reason.contains("exceeded")
                                    }
                                    DeviationDirection::BelowSetPoint => {
                                        reason.contains("low") || reason.contains("depletion")
                                    }
                                }
                        })
                })
            })
            .count() as f64;
        let fidelity_score = matched / total_deviations;

        Self {
            delay_ms: elapsed_ms,
            gain,
            fidelity_score,
        }
    }
}
