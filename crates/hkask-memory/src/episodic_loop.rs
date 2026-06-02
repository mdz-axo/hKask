//! Episodic Loop — experience → encode → store → recall → temporal weight → context (Loop 2a)
//!
//! Wraps `EpisodicMemory` and provides loop-level budget observability AND
//! enforcement. Budget regulation is now owned by this loop (Cybernetics
//! concern), not by domain code in `EpisodicMemory`.

use crate::episodic::EpisodicMemory;
use hkask_types::WebID;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal,
};

/// Episodic Loop — monitors episodic storage usage against budget and enforces limits.
///
/// Wraps `EpisodicMemory` and reads storage usage per perspective. When
/// usage exceeds 80% of budget, it produces `Throttle` actions targeting
/// itself. When usage exceeds 100%, it escalates to the Curation loop.
///
/// In `act()`, when budget is exceeded, the loop prunes consolidation
/// candidates (lowest-confidence, oldest triples) to bring usage back
/// within budget. This replaces `EpisodicMemory::check_budget()` as the
/// authority for budget enforcement.
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
    /// - >100% of budget → `Escalate` to Curation AND `Calibrate` self (prune triples)
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            if dev.signal.metric == "storage_usage"
                && dev.direction == DeviationDirection::AboveSetPoint
            {
                let ratio = dev.signal.value / dev.signal.set_point;

                if ratio > 1.0 {
                    // Budget exceeded — prune consolidation candidates
                    let overage = (dev.signal.value - dev.signal.set_point) as usize;
                    actions.push(LoopAction::new(
                        LoopId::Episodic,
                        ActionType::Calibrate,
                        serde_json::json!({
                            "reason": "episodic_budget_exceeded_prune",
                            "usage": dev.signal.value,
                            "budget": dev.signal.set_point,
                            "overage": overage,
                        }),
                    ));
                    // Also escalate to Curation (budget exceeded)
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

    /// Act: enforce budget regulation.
    ///
    /// - `Calibrate` with reason `episodic_budget_exceeded_prune`: prune
    ///   consolidation candidates (lowest-confidence, oldest triples) to
    ///   bring storage back within budget.
    /// - `Throttle`: log warning (no direct enforcement — ingestion rate
    ///   limiting is handled by the caller checking storage usage).
    /// - `Escalate`: logged (Curation loop handles escalation).
    ///
    /// This replaces `EpisodicMemory::check_budget()` as the authority
    /// for budget enforcement. Domain code should query
    /// `storage_usage()` instead of calling `check_budget()`.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            match action.action_type {
                ActionType::Calibrate => {
                    let reason = action.parameters.get("reason").and_then(|v| v.as_str());
                    if reason == Some("episodic_budget_exceeded_prune") {
                        let overage = action
                            .parameters
                            .get("overage")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;

                        // Prune lowest-confidence, oldest triples
                        match self
                            .memory
                            .consolidation_candidates(self.perspective, overage)
                        {
                            Ok(candidates) if !candidates.is_empty() => {
                                tracing::warn!(
                                    target: "cns.episodic",
                                    perspective = %self.perspective,
                                    candidates = candidates.len(),
                                    overage = overage,
                                    "Pruning consolidation candidates to enforce budget"
                                );
                                // Retract each candidate via bayesian::retract
                                for triple in &candidates {
                                    let perspective =
                                        triple.perspective.unwrap_or(self.perspective);
                                    if let Err(e) = self.memory.retract_triple(
                                        &triple.entity,
                                        &triple.attribute,
                                        0.5, // retraction confidence: halve the confidence
                                        perspective,
                                    ) {
                                        tracing::debug!(
                                            target: "cns.episodic",
                                            entity = %triple.entity,
                                            attribute = %triple.attribute,
                                            error = %e,
                                            "Failed to retract consolidation candidate"
                                        );
                                    }
                                }
                            }
                            Ok(_) => {
                                tracing::debug!(
                                    target: "cns.episodic",
                                    "No consolidation candidates found for pruning"
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    target: "cns.episodic",
                                    error = %e,
                                    "Failed to query consolidation candidates"
                                );
                            }
                        }
                    } else {
                        tracing::info!(
                            target: "cns.episodic",
                            action_type = ?action.action_type,
                            target_loop = %action.target,
                            "Episodic Loop calibration action"
                        );
                    }
                }
                _ => {
                    tracing::info!(
                        target: "cns.episodic",
                        action_type = ?action.action_type,
                        target_loop = %action.target,
                        "Episodic Loop regulatory action"
                    );
                }
            }
        }
    }
}
