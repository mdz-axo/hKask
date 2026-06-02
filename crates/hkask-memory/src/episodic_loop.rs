//! Episodic Loop — experience → encode → store → recall → temporal weight → context (Loop 2a)
//!
//! Wraps `EpisodicMemory` and provides loop-level budget observability.
//! Budget enforcement is already done in `EpisodicMemory::check_budget()` —
//! the loop provides observability and regulatory signals, not direct enforcement.

use crate::episodic::EpisodicMemory;
use hkask_types::WebID;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal,
};

/// Episodic Loop — monitors episodic storage usage against budget.
///
/// Wraps `EpisodicMemory` and reads storage usage per perspective. When
/// usage exceeds 80% of budget, it produces `Throttle` actions targeting
/// itself. When usage exceeds 100%, it escalates to the Curation loop.
pub struct EpisodicLoop {
    memory: EpisodicMemory,
    perspective: WebID,
    storage_budget: usize,
}

impl EpisodicLoop {
    /// Create a new Episodic Loop wrapping an EpisodicMemory.
    ///
    /// The `perspective` identifies which agent's episodic storage to monitor.
    /// The `storage_budget` is the set-point for the regulation signal.
    pub fn new(memory: EpisodicMemory, perspective: WebID, storage_budget: usize) -> Self {
        Self {
            memory,
            perspective,
            storage_budget,
        }
    }

    /// Get the configured storage budget (set-point).
    pub fn storage_budget(&self) -> usize {
        self.storage_budget
    }
}

#[async_trait::async_trait]
impl HkaskLoop for EpisodicLoop {
    fn id(&self) -> LoopId {
        LoopId::Episodic
    }

    /// Sense: read storage usage and decay rate.
    ///
    /// Produces signals for:
    /// - `storage_usage` — current triple count vs storage budget
    /// - `decay_rate` — current confidence decay rate
    async fn sense(&self) -> Vec<Signal> {
        let usage = self.memory.storage_usage(&self.perspective).unwrap_or(0);
        let decay_rate = self.memory.decay_rate();

        vec![
            Signal::new(
                LoopId::Episodic,
                "storage_usage",
                usage as f64,
                self.storage_budget as f64,
            ),
            Signal::new(
                LoopId::Episodic,
                "decay_rate",
                decay_rate,
                decay_rate, // set-point = current (no deviation expected)
            ),
        ]
    }

    /// Compute: produce regulatory actions based on storage usage thresholds.
    ///
    /// - >80% of budget → `Throttle` self (reduce ingestion rate)
    /// - >100% of budget → `Escalate` to Curation (budget exceeded)
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            if dev.signal.metric == "storage_usage"
                && dev.direction == DeviationDirection::AboveSetPoint
            {
                let ratio = dev.signal.value / dev.signal.set_point;

                if ratio > 1.0 {
                    // Budget exceeded — escalate to Curation
                    actions.push(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({
                            "reason": "episodic_budget_exceeded",
                            "usage": dev.signal.value,
                            "budget": dev.signal.set_point,
                        }),
                    ));
                } else if ratio > 0.8 {
                    // Approaching budget — throttle self
                    actions.push(LoopAction::new(
                        LoopId::Episodic,
                        ActionType::Throttle,
                        serde_json::json!({
                            "reason": "episodic_budget_approaching",
                            "usage": dev.signal.value,
                            "budget": dev.signal.set_point,
                            "ratio": ratio,
                        }),
                    ));
                }
            }
        }

        actions
    }

    /// Act: log regulatory actions.
    ///
    /// Budget enforcement is already done in `EpisodicMemory::check_budget()`.
    /// The loop provides observability and regulatory signals, not direct enforcement.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            tracing::info!(
                target: "cns.episodic",
                action_type = ?action.action_type,
                target_loop = %action.target,
                "Episodic Loop regulatory action"
            );
        }
    }
}
