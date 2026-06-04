//! Semantic Loop — knowledge → store → index → recall → dedup → combine → context (Loop 2b)
//!
//! Wraps `SemanticMemory` and provides loop-level observability for
//! triple count and consolidation signals. When the triple count exceeds
//! the storage budget, the loop prunes low-confidence triples via
//! `bayesian::retract()` — mirroring `EpisodicLoop::act()`.

use std::sync::Arc;

use crate::semantic::SemanticMemory;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal,
};

/// Default set-point for semantic triple count.
const DEFAULT_TRIPLE_COUNT_SET_POINT: usize = 10_000;

/// Semantic Loop — monitors semantic triple count against set-point.
///
/// Wraps `SemanticMemory` and reads triple count. When count exceeds the
/// set-point, it produces `Calibrate` actions and enforces budget by
/// retracting lowest-confidence triples.
pub struct SemanticLoop {
    memory: Arc<SemanticMemory>,
    storage_budget: usize,
}

impl SemanticLoop {
    /// Create a new Semantic Loop wrapping a SemanticMemory.
    pub fn new(memory: Arc<SemanticMemory>) -> Self {
        Self {
            memory,
            storage_budget: DEFAULT_TRIPLE_COUNT_SET_POINT,
        }
    }

    /// Get the configured storage budget (set-point).
    pub fn storage_budget(&self) -> usize {
        self.storage_budget
    }
}

#[async_trait::async_trait]
impl HkaskLoop for SemanticLoop {
    fn id(&self) -> LoopId {
        LoopId::Semantic
    }

    /// Sense: read semantic triple count.
    ///
    /// Produces signals for:
    /// - `triple_count` — current count vs storage budget
    async fn sense(&self) -> Vec<Signal> {
        let count = self.memory.triple_count().unwrap_or(0);

        vec![Signal::new(
            LoopId::Semantic,
            "triple_count",
            count as f64,
            self.storage_budget as f64,
        )]
    }

    /// Compute: if triple count exceeds set-point, suggest consolidation.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            if dev.signal.metric == "triple_count"
                && dev.direction == DeviationDirection::AboveSetPoint
            {
                let overage = (dev.signal.value - dev.signal.set_point) as usize;
                actions.push(LoopAction::new(
                    LoopId::Semantic,
                    ActionType::Calibrate,
                    serde_json::json!({
                        "reason": "semantic_triple_count_exceeded",
                        "count": dev.signal.value,
                        "set_point": dev.signal.set_point,
                        "overage": overage,
                    }),
                ));
            }
        }

        actions
    }

    /// Act: enforce budget regulation.
    ///
    /// - `Calibrate` with reason `semantic_triple_count_exceeded`: retract
    ///   lowest-confidence semantic triples to bring count back within budget.
    ///   Semantic retraction reduces confidence via `bayesian::retract()`
    ///   rather than deleting, since shared knowledge should not be removed.
    /// - Other actions: logged at info level.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            match action.action_type {
                ActionType::Calibrate => {
                    let reason = action.parameters.get("reason").and_then(|v| v.as_str());
                    if reason == Some("semantic_triple_count_exceeded") {
                        let overage = action
                            .parameters
                            .get("overage")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;

                        // Prune lowest-confidence semantic triples
                        match self.memory.lowest_confidence_triples(overage) {
                            Ok(candidates) if !candidates.is_empty() => {
                                tracing::warn!(
                                    target: "cns.semantic",
                                    candidates = candidates.len(),
                                    overage = overage,
                                    "Retracting lowest-confidence semantic triples to enforce budget"
                                );
                                for triple in &candidates {
                                    if let Err(e) = self.memory.retract_triple(
                                        &triple.entity,
                                        &triple.attribute,
                                        0.5, // retraction confidence: halve the confidence
                                    ) {
                                        tracing::debug!(
                                            target: "cns.semantic",
                                            entity = %triple.entity,
                                            attribute = %triple.attribute,
                                            error = %e,
                                            "Failed to retract semantic triple"
                                        );
                                    }
                                }
                            }
                            Ok(_) => {
                                tracing::debug!(
                                    target: "cns.semantic",
                                    "No low-confidence semantic triples found for retraction"
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    target: "cns.semantic",
                                    error = %e,
                                    "Failed to query low-confidence semantic triples"
                                );
                            }
                        }
                    } else {
                        tracing::info!(
                            target: "cns.semantic",
                            action_type = ?action.action_type,
                            target_loop = %action.target,
                            "Semantic Loop calibration action"
                        );
                    }
                }
                _ => {
                    tracing::info!(
                        target: "cns.semantic",
                        action_type = ?action.action_type,
                        target_loop = %action.target,
                        "Semantic Loop regulatory action"
                    );
                }
            }
        }
    }
}
