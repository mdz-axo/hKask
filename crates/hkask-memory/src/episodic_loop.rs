//! Episodic Loop — experience → encode → store → recall → temporal weight → context (Loop 2a)
//!
//! Wraps `EpisodicMemory` and provides loop-level budget observability AND
//! enforcement. Budget regulation is now owned by this loop (Cybernetics
//! concern), not by domain code in `EpisodicMemory`.

use std::sync::Arc;

use crate::consolidation::ConsolidationBridge;
use crate::episodic::EpisodicMemory;
use hkask_types::WebID;
use hkask_types::capability::tokens::ConsolidationToken;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use hkask_types::ports::ConsolidationRequest;

/// Episodic Loop — monitors episodic storage usage against budget and enforces limits.
///
/// Wraps `EpisodicMemory` and reads storage usage per perspective. When
/// usage exceeds 80% of budget, it produces `Throttle` actions targeting
/// itself. When usage exceeds 100%, it escalates to the Curation loop
/// and consolidates lowest-confidence episodic triples to semantic memory
/// to free storage.
pub struct EpisodicLoop {
    memory: Arc<EpisodicMemory>,
    perspective: WebID,
    storage_budget: usize,
    /// Consolidation bridge for promoting episodic triples to semantic memory
    /// when budget pressure requires it.
    consolidation: Option<Arc<ConsolidationBridge>>,
    /// OCAP token proving consolidation authority (issued by Curator/Cybernetics).
    consolidation_token: Option<ConsolidationToken>,
}

impl EpisodicLoop {
    /// Create a new Episodic Loop wrapping an EpisodicMemory.
    ///
    /// The `perspective` identifies which agent's episodic storage to monitor.
    /// The `storage_budget` is the set-point for the regulation signal.
    ///
    /// REQ: MEM-006
    /// pre:  memory is initialized, perspective is valid, storage_budget > 0
    /// post: returns EpisodicLoop without consolidation bridge
    pub fn new(memory: Arc<EpisodicMemory>, perspective: WebID, storage_budget: usize) -> Self {
        Self {
            memory,
            perspective,
            storage_budget,
            consolidation: None,
            consolidation_token: None,
        }
    }

    /// Create an Episodic Loop with a consolidation bridge.
    ///
    /// When budget pressure requires it, the consolidation bridge fires
    /// to promote episodic triples into semantic memory. The token proves
    /// Curator/Cybernetics authority for the one-way bridge.
    ///
    /// REQ: MEM-007
    /// pre:  memory is initialized, perspective is valid, storage_budget > 0
    /// pre:  consolidation_token.issuer() == expected curator
    /// post: returns EpisodicLoop with consolidation bridge and token
    pub fn with_consolidation(
        memory: Arc<EpisodicMemory>,
        perspective: WebID,
        storage_budget: usize,
        consolidation: Arc<ConsolidationBridge>,
        consolidation_token: ConsolidationToken,
    ) -> Self {
        Self {
            memory,
            perspective,
            storage_budget,
            consolidation: Some(consolidation),
            consolidation_token: Some(consolidation_token),
        }
    }

    /// Get the configured storage budget (set-point).
    ///
    /// REQ: MEM-008
    /// post: returns the storage_budget value set at construction
    pub fn storage_budget(&self) -> usize {
        self.storage_budget
    }
}

#[async_trait::async_trait]
impl HkaskLoop for EpisodicLoop {
    fn id(&self) -> LoopId {
        LoopId::Memory
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
                LoopId::Memory,
                SignalMetric::StorageUsage,
                usage as f64,
                self.storage_budget as f64,
            ),
            Signal::new(
                LoopId::Memory,
                SignalMetric::DecayRate,
                decay_rate,
                decay_rate, // set-point = current (no deviation expected)
            ),
        ]
    }

    /// Compute: produce regulatory actions based on storage usage thresholds.
    ///
    /// - >80% of budget → `Throttle` self (reduce ingestion rate)
    /// - >100% of budget → `Escalate` to Curation AND `Calibrate` self (consolidate triples)
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            if dev.signal.metric == SignalMetric::StorageUsage
                && dev.direction == DeviationDirection::AboveSetPoint
            {
                let ratio = dev.signal.value / dev.signal.set_point;

                if ratio > 1.0 {
                    // Budget exceeded — consolidate to free space
                    let overage = (dev.signal.value - dev.signal.set_point) as usize;
                    actions.push(LoopAction::new(
                        LoopId::Memory,
                        ActionType::Calibrate,
                        serde_json::json!({
                            "reason": "episodic_budget_exceeded_consolidate",
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
                        LoopId::Memory,
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

    /// Act: enforce budget regulation via consolidation.
    ///
    /// - `Calibrate` with reason `episodic_budget_exceeded_consolidate`: fire
    ///   the consolidation bridge to promote lowest-confidence, oldest triples
    ///   from episodic to semantic memory, freeing storage.
    /// - `Throttle`: log warning (no direct enforcement — ingestion rate
    ///   limiting is handled by the caller checking storage usage).
    /// - `Escalate`: logged (Curation loop handles escalation).
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            match action.action_type {
                ActionType::Calibrate => {
                    let reason = action.parameters.get("reason").and_then(|v| v.as_str());
                    if reason == Some("episodic_budget_exceeded_consolidate") {
                        let overage = action
                            .parameters
                            .get("overage")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;

                        // Fire consolidation bridge: promote lowest-confidence,
                        // oldest episodic triples to semantic memory
                        if let (Some(consolidation), Some(token)) =
                            (&self.consolidation, &self.consolidation_token)
                        {
                            match consolidation.consolidate(
                                token,
                                &self.perspective,
                                ConsolidationRequest {
                                    limit: overage,
                                    ..Default::default()
                                },
                            ) {
                                Ok(outcome) if outcome.consolidated_count > 0 => {
                                    tracing::info!(
                                        target: "cns.episodic",
                                        perspective = %self.perspective,
                                        consolidated = outcome.consolidated_count,
                                        failed = outcome.failed_count,
                                        "Consolidation bridge fired for episodic budget enforcement"
                                    );
                                }
                                Ok(_) => {
                                    tracing::debug!(
                                        target: "cns.episodic",
                                        perspective = %self.perspective,
                                        "Consolidation fired but no triples consolidated"
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        target: "cns.episodic",
                                        perspective = %self.perspective,
                                        error = %e,
                                        "Consolidation bridge failed during budget enforcement"
                                    );
                                }
                            }
                        } else {
                            tracing::warn!(
                                target: "cns.episodic",
                                perspective = %self.perspective,
                                overage = overage,
                                "Episodic budget exceeded but no consolidation bridge available"
                            );
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
