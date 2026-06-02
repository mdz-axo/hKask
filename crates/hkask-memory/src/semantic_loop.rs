//! Semantic Loop — knowledge → store → index → recall → dedup → combine → context (Loop 2b)
//!
//! Wraps `SemanticMemory` and provides loop-level observability for
//! triple count and consolidation signals.

use crate::semantic::SemanticMemory;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal,
};

/// Default set-point for semantic triple count.
const DEFAULT_TRIPLE_COUNT_SET_POINT: f64 = 10_000.0;

/// Semantic Loop — monitors semantic triple count against set-point.
///
/// Wraps `SemanticMemory` and reads triple count. When count exceeds the
/// set-point, it produces `Calibrate` actions suggesting consolidation/dedup.
pub struct SemanticLoop {
    memory: SemanticMemory,
}

impl SemanticLoop {
    /// Create a new Semantic Loop wrapping a SemanticMemory.
    pub fn new(memory: SemanticMemory) -> Self {
        Self { memory }
    }
}

impl HkaskLoop for SemanticLoop {
    fn id(&self) -> LoopId {
        LoopId::Semantic
    }

    /// Sense: read semantic triple count.
    ///
    /// Produces signals for:
    /// - `triple_count` — current count vs default set-point (10,000)
    async fn sense(&self) -> Vec<Signal> {
        let count = self.memory.triple_count().unwrap_or(0);

        vec![Signal::new(
            LoopId::Semantic,
            "triple_count",
            count as f64,
            DEFAULT_TRIPLE_COUNT_SET_POINT,
        )]
    }

    /// Compute: if triple count exceeds set-point, suggest consolidation.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            if dev.signal.metric == "triple_count"
                && dev.direction == DeviationDirection::AboveSetPoint
            {
                actions.push(LoopAction::new(
                    LoopId::Semantic,
                    ActionType::Calibrate,
                    serde_json::json!({
                        "reason": "semantic_triple_count_exceeded",
                        "count": dev.signal.value,
                        "set_point": dev.signal.set_point,
                    }),
                ));
            }
        }

        actions
    }

    /// Act: log regulatory actions.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            tracing::info!(
                target: "cns.semantic",
                action_type = ?action.action_type,
                target_loop = %action.target,
                "Semantic Loop regulatory action"
            );
        }
    }
}
