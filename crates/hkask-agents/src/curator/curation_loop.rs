//! Curation Loop — pure regulatory observer (Loop 5)
//!
//! observe → evaluate → compose → regulate
//!
//! The Curation Loop is the ONLY loop that can override Cybernetics.
//! It observes system state and intervenes when Cybernetics
//! can't self-stabilize (e.g., alert cascade).
//!
//! # Curation / Agent Separation (Task 6)
//!
//! The Curation Loop is pure regulatory code — no persona, no chat behavior,
//! no memory. The Curator Agent (`crate::curator_agent::CuratorAgent`) is the
//! persona layer that holds metacognition, bot orchestration, and human-facing
//! reporting. The Curation Loop reads from the NuEvent store and produces
//! `CuratorDirective`s; the Curator Agent *consumes* those directives and
//! formats them for human operators.

use crate::curator::curation_gate::{CurationConfidenceGate, CurationDecision};
use chrono::Utc;
use hkask_types::loops::curation::{CuratorDirective, CuratorHandle};
use hkask_types::loops::{Deviation, HkaskLoop, LoopAction, LoopId, Signal};
use hkask_types::ports::ConsolidationPort;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::curator::context::CuratorContext;

/// Curation Loop — pure regulatory observer.
///
/// Reads from the NuEvent store and produces `CuratorDirective`s through
/// Communication dispatch. All persona concerns (metacognition, bot
/// orchestration, human-facing reporting) belong in `CuratorAgent`.
///
/// **Singleton invariant:** There is exactly one `CurationLoop` per hKask
/// system. It owns the single `CuratorHandle`; all Curator capability access
/// flows through this instance via `CuratorContext::handle()`. The
/// `curator_handle` field makes the singleton relationship explicit at the
/// type level — it is not a runtime enforcement, but a structural guarantee
/// that the loop and its handle are co-located.
pub struct CurationLoop {
    curator_handle: CuratorHandle,
    context: Arc<CuratorContext>,
    consolidation: Option<Arc<dyn ConsolidationPort>>,
    /// Cursor for incremental algedonic review.
    /// Stores the Unix timestamp (milliseconds) of the last reviewed event.
    /// Curation reads from the persistent NuEvent log, not live CNS state.
    last_review_ms: AtomicU64,
}

impl CurationLoop {
    /// Create a new Curation Loop with a CuratorContext.
    ///
    /// The `curator_handle` is the single Curator capability handle for
    /// the system. Use `CuratorHandle::system()` to construct it.
    /// The `context` provides capability-disciplined access to CNS, dispatch,
    /// and escalation — the Curation Loop's only runtime dependencies.
    pub fn new(curator_handle: CuratorHandle, context: Arc<CuratorContext>) -> Self {
        Self {
            curator_handle,
            context,
            consolidation: None,
            last_review_ms: AtomicU64::new(0),
        }
    }

    /// Create a Curation Loop with a consolidation port.
    ///
    /// When episodic budget pressure triggers escalation, the consolidation
    /// bridge will fire to migrate episodic triples into semantic memory.
    pub fn with_consolidation(
        curator_handle: CuratorHandle,
        context: Arc<CuratorContext>,
        consolidation: Arc<dyn ConsolidationPort>,
    ) -> Self {
        Self {
            curator_handle,
            context,
            consolidation: Some(consolidation),
            last_review_ms: AtomicU64::new(0),
        }
    }

    /// Access the CuratorContext (capability-disciplined runtime references).
    pub fn context(&self) -> &Arc<CuratorContext> {
        &self.context
    }

    /// Access the CuratorHandle owned by this loop.
    ///
    /// Per the singleton invariant, this is the single CuratorHandle
    /// for the entire system.
    pub fn curator_handle(&self) -> &CuratorHandle {
        &self.curator_handle
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
        let decision = gate.decide();
        let r_bar = gate.confidence();

        match decision {
            CurationDecision::SeekMoreEvidence => {
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

    /// Sense: read algedonic-significant NuEvents from the persistent store.
    ///
    /// Per Fowler's Gateway pattern: Curation reads from the NuEvent store
    /// (the canonical alerts log), not from live CNS state. This makes Curation
    /// a deliberative reviewer, not a real-time monitor.
    ///
    /// Falls back to live CNS reads if no NuEvent store is configured.
    ///
    /// Produces signals for:
    /// - Algedonic event count (from NuEvent store)
    /// - Escalation queue size
    async fn sense(&self) -> Vec<Signal> {
        // Primary: Read from NuEvent store using cursor-based algedonic review
        let since_ms = self.last_review_ms.load(Ordering::Relaxed);
        let since = chrono::DateTime::<Utc>::from_timestamp_millis(since_ms as i64)
            .unwrap_or(chrono::DateTime::<Utc>::UNIX_EPOCH);

        let algedonic_count = if let Some(store) = self.context.nu_event_store() {
            // Read algedonic-significant events since last review cursor
            match store.query_algedonic(since, 1000) {
                Ok(events) => {
                    let count = events.len() as u64;
                    // Advance cursor to latest event timestamp
                    if let Some(latest) = events.last() {
                        self.last_review_ms.store(
                            latest.timestamp.timestamp_millis() as u64,
                            Ordering::Relaxed,
                        );
                    }
                    tracing::info!(
                        target: "curation.loop",
                        since = %since.to_rfc3339(),
                        event_count = count,
                        "Curation read algedonic events from NuEvent store"
                    );
                    count
                }
                Err(e) => {
                    tracing::warn!(
                        target: "curation.loop",
                        error = %e,
                        "Failed to query NuEvent store, falling back to live CNS reads"
                    );
                    // Fallback: count critical alerts from live CNS
                    self.context.cns().critical_alerts().await.len() as u64
                }
            }
        } else {
            // No NuEvent store configured: fall back to live CNS reads
            self.context.cns().critical_alerts().await.len() as u64
        };

        let pending_escalations = self
            .context
            .escalation_queue()
            .list_pending()
            .map(|v| v.len())
            .unwrap_or(0);

        let consolidation_candidates = self
            .consolidation
            .as_ref()
            .map(|port| port.consolidation_candidate_count(self.context.handle().curator_id()))
            .unwrap_or(0);

        vec![
            Signal::new(
                LoopId::Curation,
                "algedonic_events",
                algedonic_count as f64,
                0.0, // set-point: zero algedonic events is healthy
            ),
            Signal::new(
                LoopId::Curation,
                "pending_escalations",
                pending_escalations as f64,
                0.0, // set-point: zero pending escalations is healthy
            ),
            Signal::new(
                LoopId::Curation,
                "consolidation_candidates",
                consolidation_candidates as f64,
                0.0, // set-point: zero pending consolidation candidates is healthy
            ),
        ]
    }

    /// Compute: produce CuratorDirectives as LoopActions.
    ///
    /// Now that Curation reads from the NuEvent store (not live CNS),
    /// the signals are algedonic_event_count and pending_escalations.
    /// When algedonic events exceed zero, Curation produces escalation directives.
    /// When pending_escalations exist, Curation processes them.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            match dev.signal.metric.as_str() {
                "algedonic_events" if dev.signal.value > 0.0 => {
                    // Algedonic events from NuEvent store require Curation review
                    actions.push(LoopAction::new(
                        LoopId::Cybernetics,
                        hkask_types::loops::ActionType::Escalate,
                        serde_json::json!({
                            "reason": "algedonic_events_exceeded",
                            "count": dev.signal.value,
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
                hkask_types::loops::ActionType::Escalate
                    if action.parameters.get("reason").and_then(|v| v.as_str())
                        == Some("algedonic_events_exceeded") =>
                {
                    // Algedonic events from NuEvent store — review and override if needed
                    let count = action
                        .parameters
                        .get("count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    tracing::warn!(
                        target: "curation.loop",
                        count = count,
                        "Algedonic events exceeded threshold — Curation reviewing"
                    );
                    // Curation may issue OverrideGasBudget or CalibrateThreshold directives
                    // based on review. For now, log and escalate.
                    None
                }
                hkask_types::loops::ActionType::Escalate
                    if action.parameters.get("reason").and_then(|v| v.as_str())
                        == Some("pending_escalations_exist") =>
                {
                    // Process pending escalations from the queue
                    match self.context.escalation_queue().list_pending() {
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
                            // (adjust gas budgets for the associated bot)
                            for entry in entries.iter().filter(|e| e.confidence > 0.5) {
                                let directive = CuratorDirective::OverrideGasBudget {
                                    agent: entry.bot_id.into(), // BotID -> WebID
                                    new_budget: 5000, // Reduced budget for problematic bot
                                };
                                if let Some(trace_id) =
                                    self.context.issue_directive(directive).await
                                {
                                    tracing::info!(
                                        target: "curation.loop",
                                        trace_id = %trace_id,
                                        escalation_id = %entry.id,
                                        "Issued OverrideGasBudget directive for escalated bot"
                                    );
                                }
                            }

                            // Trigger consolidation if a consolidation port is available
                            // and there are escalations (episodic budget pressure → consolidate)
                            if let Some(consolidation) = &self.consolidation {
                                let handle = self.context.handle();
                                let token = handle.issue_consolidation_token();
                                let curator_id = handle.curator_id();
                                match consolidation.consolidate(&token, curator_id, 100) {
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
                && let Some(trace_id) = self.context.issue_directive(directive).await
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
