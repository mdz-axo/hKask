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

use chrono::Utc;
use hkask_memory::ConsolidationBridge;
use hkask_types::loops::curation::{CuratorDirective, CuratorHandle};
use hkask_types::loops::{
    CurationInput, Deviation, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use hkask_types::ports::ConsolidationRequest;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, mpsc};

use crate::curator::context::CuratorContext;

const CUR_TARGET: &str = "curation.loop";

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
    consolidation: Option<Arc<ConsolidationBridge>>,
    /// Cursor for incremental algedonic review.
    last_review_ms: AtomicU64,
    /// Inbox for receiving CurationInput messages from Cybernetics, SpecCurator,
    /// and GoalStore. Drained during the sense phase.
    inbox: Option<Arc<RwLock<mpsc::UnboundedReceiver<CurationInput>>>>,
}

impl CurationLoop {
    /// Create a new Curation Loop with a CuratorContext.
    ///
    /// The `curator_handle` is the single Curator capability handle for
    /// the system. Use `CuratorHandle::system()` to construct it.
    /// The `context` provides capability-disciplined access to CNS, dispatch,
    /// and escalation — the Curation Loop's only runtime dependencies.
    ///
    /// REQ: P9-agt-curator-loop-new
    /// [P9] Motivating: Homeostatic Self-Regulation — Curation Loop is the regulatory sense-act loop
    /// [P4] Constraining: Clear Boundaries — single CuratorHandle capability
    /// pre:  `curator_handle` is a valid `CuratorHandle` (singleton);
    ///       `context` is a valid `Arc<CuratorContext>`.
    /// post: Returns a `CurationLoop` with no consolidation, no inbox,
    ///       and `last_review_ms` initialized to 0.
    pub fn new(curator_handle: CuratorHandle, context: Arc<CuratorContext>) -> Self {
        Self {
            curator_handle,
            context,
            consolidation: None,
            last_review_ms: AtomicU64::new(0),
            inbox: None,
        }
    }

    /// REQ: P9-agt-curator-loop-new-with-consolidation
    /// [P9] Motivating: Homeostatic Self-Regulation — consolidation tunes the loop
    /// [P7] Constraining: Evolutionary Architecture — consolidation config emerged from usage
    /// pre:  `curator_handle` is a valid `CuratorHandle`; `context` is a
    ///       valid `Arc<CuratorContext>`; `consolidation` is a valid
    ///       `Arc<ConsolidationBridge>`.
    /// post: Returns a `CurationLoop` with consolidation set, no inbox,
    ///       and `last_review_ms` initialized to 0.
    pub fn with_consolidation(
        curator_handle: CuratorHandle,
        context: Arc<CuratorContext>,
        consolidation: Arc<ConsolidationBridge>,
    ) -> Self {
        Self {
            curator_handle,
            context,
            consolidation: Some(consolidation),
            last_review_ms: AtomicU64::new(0),
            inbox: None,
        }
    }

    /// Wire the unified inbox for CurationInput messages.
    ///
    /// REQ: P9-agt-curator-loop-inbox
    /// [P9] Motivating: Homeostatic Self-Regulation — unified inbox receives CurationInput
    /// pre:  `rx` is a valid `UnboundedReceiver<CurationInput>`.
    /// post: Returns `self` with `inbox` set to `Some(Arc<RwLock<rx>>)`.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_inbox(mut self, rx: mpsc::UnboundedReceiver<CurationInput>) -> Self {
        self.inbox = Some(Arc::new(RwLock::new(rx)));
        self
    }

    /// Access the CuratorContext (capability-disciplined runtime references).
    ///
    /// REQ: P9-agt-curator-loop-context
    /// [P9] Motivating: Homeostatic Self-Regulation — context exposes CNS and escalation
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `Arc<CuratorContext>`.
    pub fn context(&self) -> &Arc<CuratorContext> {
        &self.context
    }

    /// Access the CuratorHandle owned by this loop.
    ///
    /// Per the singleton invariant, this is the single CuratorHandle
    /// for the entire system.
    ///
    /// REQ: P9-agt-curator-loop-handle
    /// [P9] Motivating: Homeostatic Self-Regulation — handle is the capability to curate
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `CuratorHandle`.
    pub fn curator_handle(&self) -> &CuratorHandle {
        &self.curator_handle
    }

    /// Restore the last_review_ms cursor from persistent storage.
    ///
    /// Call this after construction and before the first tick to avoid
    /// re-processing all historical algedonic events on restart.
    ///
    /// REQ: P9-agt-curator-loop-restore-cursor
    /// [P9] Motivating: Homeostatic Self-Regulation — cursor restore avoids re-processing history
    /// pre:  `self.context.nu_event_store()` may be `Some` or `None`.
    /// post: If a persisted cursor exists, `last_review_ms` is updated;
    ///       otherwise it remains at 0. Logs the outcome at info/warn level.
    ///       Does not panic on storage errors.
    pub fn restore_cursor(&self) {
        if let Some(store) = self.context.nu_event_store() {
            match store.load_cursor("curation_last_review_ms") {
                Ok(Some(cursor_ms)) => {
                    self.last_review_ms
                        .store(cursor_ms as u64, Ordering::Relaxed);
                    tracing::info!(target: CUR_TARGET, cursor_ms = cursor_ms, "Restored curation review cursor from persistence");
                }
                Ok(None) => {
                    tracing::info!(target: CUR_TARGET, "No persisted cursor found — starting from epoch")
                }
                Err(e) => {
                    tracing::warn!(target: CUR_TARGET, error = %e, "Failed to load persisted curation cursor — starting from epoch")
                }
            }
        }
    }

    // Explicit 4-stage cycle: sense → compare → compute → act
    // Delegation methods removed — HkaskLoop trait impl provides tick().
}

#[async_trait::async_trait]
impl HkaskLoop for CurationLoop {
    fn id(&self) -> LoopId {
        LoopId::Curation
    }

    /// Sense: read algedonic-significant NuEvents from the persistent store.
    /// Falls back to live CNS reads if no NuEvent store is configured.
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
                    if let Some(latest) = events.last() {
                        let new_cursor = latest.timestamp.timestamp_millis() as u64;
                        self.last_review_ms.store(new_cursor, Ordering::Relaxed);
                        if let Err(e) =
                            store.persist_cursor("curation_last_review_ms", new_cursor as i64)
                        {
                            tracing::warn!(target: CUR_TARGET, error = %e, "Failed to persist curation review cursor");
                        }
                    }
                    tracing::info!(target: CUR_TARGET, since = %since.to_rfc3339(), event_count = count, "Curation read algedonic events from NuEvent store");
                    count
                }
                Err(e) => {
                    tracing::warn!(target: CUR_TARGET, error = %e, "Failed to query NuEvent store, falling back to live CNS reads");
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

        // Drain unified inbox for CurationInput messages.
        let mut goal_stale_count: u64 = 0;
        let mut goal_expired_count: u64 = 0;
        let mut spec_drift_alert_count: u64 = 0;
        let mut direct_alerts: u64 = 0;
        if let Some(inbox) = &self.inbox {
            let mut rx = inbox.write().await;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    CurationInput::Alert(alert) => {
                        direct_alerts += 1;
                        tracing::info!(
                            target: CUR_TARGET,
                            deficit = alert.deficit,
                            threshold = alert.threshold,
                            "RuntimeAlert received from Cybernetics"
                        );
                    }
                    CurationInput::SpecDrift(event) => {
                        spec_drift_alert_count += 1;
                        tracing::warn!(
                            target: CUR_TARGET,
                            spec_id = %event.spec_id,
                            drift_magnitude = event.drift_magnitude,
                            "SpecDrift received from SpecCurator"
                        );
                    }
                    CurationInput::GoalTransition(event) => match event.to_state.as_str() {
                        "stale" => {
                            goal_stale_count += 1;
                            tracing::debug!(target: CUR_TARGET, goal_id = %event.goal_id, "Goal stale");
                        }
                        "expired" => {
                            goal_expired_count += 1;
                            tracing::debug!(target: CUR_TARGET, goal_id = %event.goal_id, "Goal expired");
                        }
                        _ => {}
                    },
                    CurationInput::SpecDriftResolved {
                        spec_id,
                        resolved_at,
                    } => {
                        tracing::info!(
                            target: CUR_TARGET,
                            spec_id = %spec_id,
                            resolved_at = %resolved_at,
                            "SpecDriftResolved — human resolved spec drift (P1)"
                        );
                    }
                }
            }
        }

        let s = |metric, value: u64| Signal::new(LoopId::Curation, metric, value as f64, 0.0);
        vec![
            s(
                SignalMetric::AlgedonicEvents,
                algedonic_count + direct_alerts,
            ),
            s(SignalMetric::PendingEscalations, pending_escalations as u64),
            s(
                SignalMetric::ConsolidationCandidates,
                consolidation_candidates as u64,
            ),
            s(SignalMetric::GoalStaleCount, goal_stale_count),
            s(SignalMetric::GoalExpiredCount, goal_expired_count),
            s(SignalMetric::SpecDriftAlertCount, spec_drift_alert_count),
        ]
    }

    /// Compute: produce CuratorDirectives as LoopActions.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            match dev.signal.metric {
                SignalMetric::AlgedonicEvents if dev.signal.value > 0.0 => {
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
                SignalMetric::PendingEscalations if dev.signal.value > 0.0 => {
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
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            tracing::info!(target: CUR_TARGET, action_type = ?action.action_type, target_loop = %action.target, "Curation Loop regulatory action");
            let directive = match action.action_type {
                hkask_types::loops::ActionType::Escalate
                    if action.parameters.get("reason").and_then(|v| v.as_str())
                        == Some("algedonic_events_exceeded") =>
                {
                    let count = action
                        .parameters
                        .get("count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    tracing::warn!(target: CUR_TARGET, count = count, "Algedonic events exceeded threshold — Curation reviewing");
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
                                target: CUR_TARGET,
                                count = entries.len(),
                                "Processing pending escalations"
                            );
                            for entry in &entries {
                                tracing::info!(
                                    target: CUR_TARGET,
                                    escalation_id = %entry.id,
                                    confidence = entry.confidence,
                                    "Reviewing escalation entry"
                                );
                            }
                            // Issue directives for high-confidence escalations
                            // (adjust energy budgets for the associated bot)
                            for entry in entries.iter().filter(|e| e.confidence > 0.5) {
                                let directive = CuratorDirective::OverrideEnergyBudget {
                                    agent: entry.bot_id.into(),
                                    new_budget: 5000,
                                };
                                self.context.issue_directive(directive).await;
                                tracing::info!(target: CUR_TARGET, escalation_id = %entry.id, "Issued OverrideEnergyBudget directive for escalated bot");
                            }
                            if let Some(consolidation) = &self.consolidation {
                                let handle = self.context.handle();
                                let token = handle.issue_consolidation_token();
                                let curator_id = handle.curator_id();
                                match consolidation.consolidate(
                                    &token,
                                    curator_id,
                                    ConsolidationRequest {
                                        limit: 100,
                                        ..Default::default()
                                    },
                                ) {
                                    Ok(outcome) if outcome.consolidated_count > 0 => {
                                        tracing::info!(target: CUR_TARGET, consolidated = outcome.consolidated_count, failed = outcome.failed_count, "Consolidation bridge fired for escalated system")
                                    }
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::warn!(target: CUR_TARGET, error = %e, "Consolidation bridge failed")
                                    }
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!(
                                target: CUR_TARGET,
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

            if let Some(directive) = directive {
                self.context.issue_directive(directive).await;
                tracing::info!(
                    target: CUR_TARGET,
                    "Directive issued through dispatch"
                );
            }
            // None means directive was dampened or issuance failed
        }
    }
}
