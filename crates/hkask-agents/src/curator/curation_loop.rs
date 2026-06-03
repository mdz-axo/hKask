//! Curation Loop — metacognitive observer (Loop 5)
//!
//! observe → evaluate → compose → regulate
//!
//! The Curation Loop is the ONLY loop that can override Cybernetics.
//! It observes system state and intervenes when Cybernetics
//! can't self-stabilize (e.g., alert cascade).

use crate::curator::metacognition::MetacognitionLoop;
use hkask_cns::allosteric::curation::{CurationConfidenceGate, CurationDecision};
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::{Deviation, HkaskLoop, LoopAction, LoopId, Signal};
use hkask_types::ports::ConsolidationPort;
use std::sync::Arc;

/// Curation Loop — metacognitive observer.
///
/// Wraps `MetacognitionLoop` and expresses its regulation cycle
/// through the `Loop` trait. The `CuratorContext` provides
/// capability-disciplined access to CNS, dispatch, and escalation.
pub struct CurationLoop {
    metacognition: Arc<MetacognitionLoop>,
    consolidation: Option<Arc<dyn ConsolidationPort>>,
}

impl CurationLoop {
    /// Create a new Curation Loop wrapping a MetacognitionLoop.
    pub fn new(metacognition: Arc<MetacognitionLoop>) -> Self {
        Self {
            metacognition,
            consolidation: None,
        }
    }

    /// Create a Curation Loop with a consolidation port.
    ///
    /// When episodic budget pressure triggers escalation, the consolidation
    /// bridge will fire to migrate episodic triples into semantic memory.
    pub fn with_consolidation(
        metacognition: Arc<MetacognitionLoop>,
        consolidation: Arc<dyn ConsolidationPort>,
    ) -> Self {
        Self {
            metacognition,
            consolidation: Some(consolidation),
        }
    }

    /// Access the underlying MetacognitionLoop for domain operations
    /// (generate_summary, etc.).
    pub fn metacognition(&self) -> &Arc<MetacognitionLoop> {
        &self.metacognition
    }

    /// Evaluate curation confidence using the ARL confidence gate.
    ///
    /// If the gate is in the transition zone (0.3 < R̄ < 0.8), returns a
    /// `CuratorDirective::SeekMoreEvidence` with the channel identified by
    /// sensitivity analysis as the most impactful to verify.
    ///
    /// This is the IP-3 metacognitive bridge: CurationConfidenceGate produces
    /// a `CurationDecision::SeekMoreEvidence`, which is translated into a
    /// `CuratorDirective` and routed through Cybernetics to Inference.
    pub fn evaluate_confidence(
        &self,
        gate: &mut CurationConfidenceGate,
        context: &str,
    ) -> Option<CuratorDirective> {
        let dist = gate.decide();
        let r_bar = gate.confidence();

        // Collapse the distribution to get the concrete decision
        // The gate returns Deterministic(CurationDecision) from decide()
        match dist {
            hkask_cns::allosteric::distribution::Distribution::Deterministic(
                CurationDecision::SeekMoreEvidence,
            ) => {
                // Sensitivity analysis: which channel to verify?
                let sensitivities = gate.sensitivity_analysis();
                let top_channel = sensitivities
                    .first()
                    .map(|(name, _)| name.as_str())
                    .unwrap_or("unknown");

                Some(CuratorDirective::SeekMoreEvidence {
                    context: context.to_string(),
                    channel: top_channel.to_string(),
                    confidence: format!("{r_bar:.3}"),
                })
            }
            _ => None, // Proceed or Suppress — no directive needed
        }
    }
}

#[async_trait::async_trait]
impl HkaskLoop for CurationLoop {
    fn id(&self) -> LoopId {
        LoopId::Curation
    }

    /// Sense: read CNS health snapshot, variety counters, bot reports.
    ///
    /// Produces signals for:
    /// - Variety deficit (total across all domains)
    /// - Critical alert count
    /// - Bot failure count
    async fn sense(&self) -> Vec<Signal> {
        let context = self.metacognition.context();

        let _cns_health = context.cns().health().await;
        let variety = context.cns().variety().await;
        let critical_alerts = context.cns().critical_alerts().await;
        let bot_reports = self.metacognition.get_bot_reports().await;

        let total_variety_deficit: u64 = variety
            .iter()
            .map(|(_, count)| *count)
            .fold(0u64, |acc, v| acc.saturating_add(v));

        let failed_bots = bot_reports
            .iter()
            .filter(|r| r.status == crate::curator::bot_metrics::BotHealthStatus::Critical)
            .count();

        vec![
            Signal::new(
                LoopId::Curation,
                "variety_deficit",
                total_variety_deficit as f64,
                self.metacognition.config().thresholds.variety_deficit as f64,
            ),
            Signal::new(
                LoopId::Curation,
                "critical_alerts",
                critical_alerts.len() as f64,
                self.metacognition.config().thresholds.critical_alerts as f64,
            ),
            Signal::new(
                LoopId::Curation,
                "bot_failures",
                failed_bots as f64,
                self.metacognition.config().thresholds.bot_failures as f64,
            ),
            {
                let pending_escalations = context
                    .escalation_queue()
                    .list_pending()
                    .map(|v| v.len())
                    .unwrap_or(0);
                Signal::new(
                    LoopId::Curation,
                    "pending_escalations",
                    pending_escalations as f64,
                    0.0, // set-point: zero pending escalations is healthy
                )
            },
        ]
    }

    /// Compare: detect thresholds exceeded.
    /// Uses the default implementation from the trait.
    /// Compute: produce CuratorDirectives as LoopActions.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            match dev.signal.metric.as_str() {
                "variety_deficit" if dev.signal.value > dev.signal.set_point => {
                    actions.push(LoopAction::new(
                        LoopId::Cybernetics,
                        hkask_types::loops::ActionType::Calibrate,
                        serde_json::json!({
                            "reason": "variety_deficit_exceeded",
                            "deficit": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ));
                }
                "critical_alerts" if dev.signal.value > dev.signal.set_point => {
                    actions.push(LoopAction::new(
                        LoopId::Cybernetics,
                        hkask_types::loops::ActionType::Escalate,
                        serde_json::json!({
                            "reason": "critical_alerts_exceeded",
                            "count": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ));
                }
                "bot_failures" if dev.signal.value > dev.signal.set_point => {
                    actions.push(LoopAction::new(
                        LoopId::Cybernetics,
                        hkask_types::loops::ActionType::Escalate,
                        serde_json::json!({
                            "reason": "bot_failures_exceeded",
                            "count": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ));
                }
                "pending_escalations" if dev.signal.value > 0.0 => {
                    // Pending escalations require Curator attention
                    actions.push(LoopAction::new(
                        LoopId::Curation,
                        hkask_types::loops::ActionType::Escalate,
                        serde_json::json!({
                            "reason": "pending_escalations_exist",
                            "count": dev.signal.value,
                        }),
                    ));
                }
                _ => {}
            }
        }

        actions
    }

    /// Act: issue directives through CuratorContext with DAMPEN filtering.
    ///
    /// Converts `LoopAction`s to `CuratorDirective`s and issues them
    /// through the dispatch. Dampening is applied automatically by
    /// `CuratorContext::issue_directive()`.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            tracing::info!(
                target: "curation.loop",
                action_type = ?action.action_type,
                target_loop = %action.target,
                "Curation Loop regulatory action"
            );

            // Convert LoopAction to CuratorDirective and issue
            let directive = match action.action_type {
                hkask_types::loops::ActionType::Calibrate
                    if action.parameters.get("reason").and_then(|v| v.as_str())
                        == Some("variety_deficit_exceeded") =>
                {
                    let deficit = action
                        .parameters
                        .get("deficit")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let threshold = action
                        .parameters
                        .get("threshold")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(100);
                    Some(CuratorDirective::CalibrateThreshold {
                        domain: "variety".to_string(),
                        new_threshold: deficit.saturating_add(threshold),
                    })
                }
                hkask_types::loops::ActionType::Escalate
                    if action.parameters.get("reason").and_then(|v| v.as_str())
                        == Some("pending_escalations_exist") =>
                {
                    // Process pending escalations from the queue
                    match self
                        .metacognition
                        .context()
                        .escalation_queue()
                        .list_pending()
                    {
                        Ok(entries) if !entries.is_empty() => {
                            tracing::warn!(
                                target: "curation.loop",
                                count = entries.len(),
                                "Processing pending escalations"
                            );
                            for entry in &entries {
                                tracing::info!(
                                    target: "curation.loop",
                                    escalation_id = %entry.id,
                                    confidence = entry.confidence,
                                    "Reviewing escalation entry"
                                );
                            }
                            // Issue directives for high-confidence escalations
                            // (adjust energy budgets for the associated bot)
                            for entry in entries.iter().filter(|e| e.confidence > 0.5) {
                                let directive = CuratorDirective::AdjustEnergyBudget {
                                    agent: entry.bot_id.into(), // BotID -> WebID
                                    new_budget: 5000, // Reduced budget for problematic bot
                                };
                                if let Some(trace_id) = self
                                    .metacognition
                                    .context()
                                    .issue_directive(directive)
                                    .await
                                {
                                    tracing::info!(
                                        target: "curation.loop",
                                        trace_id = %trace_id,
                                        escalation_id = %entry.id,
                                        "Issued AdjustEnergyBudget directive for escalated bot"
                                    );
                                }
                            }

                            // Trigger consolidation if a consolidation port is available
                            // and there are escalations (episodic budget pressure → consolidate)
                            if let Some(consolidation) = &self.consolidation {
                                let curator_id = self.metacognition.context().handle().curator_id();
                                match consolidation.consolidate(curator_id, 100) {
                                    Ok(outcome) if outcome.consolidated_count > 0 => {
                                        tracing::info!(
                                            target: "curation.loop",
                                            consolidated = outcome.consolidated_count,
                                            retracted = outcome.retracted_count,
                                            failed = outcome.failed_count,
                                            "Consolidation bridge fired for escalated system"
                                        );
                                    }
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::warn!(
                                            target: "curation.loop",
                                            error = %e,
                                            "Consolidation bridge failed"
                                        );
                                    }
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!(
                                target: "curation.loop",
                                error = %e,
                                "Failed to list pending escalations"
                            );
                        }
                    }
                    continue;
                }
                hkask_types::loops::ActionType::Escalate => {
                    // Other escalations go through the escalation queue
                    // (handled by CuratorContext internally)
                    None
                }
                _ => None,
            };

            if let Some(directive) = directive
                && let Some(trace_id) = self
                    .metacognition
                    .context()
                    .issue_directive(directive)
                    .await
            {
                tracing::info!(
                    target: "curation.loop",
                    trace_id = %trace_id,
                    "Directive issued through dispatch"
                );
            }
            // None means directive was dampened or issuance failed
        }
    }
}
