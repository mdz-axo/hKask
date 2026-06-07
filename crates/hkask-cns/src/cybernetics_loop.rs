//! Cybernetics Loop — Homeostatic self-regulation (Loop 6)
//!
//! The Cybernetics Loop is a closed-loop controller, not a passive observer.
//! Its functional contract:
//!
//! 1. **Sense** — receive `cns.*` spans from all loops (tool invocations,
//!    prompt outcomes, agent pod lifecycle, connector I/O).
//! 2. **Compare** — evaluate each signal against homeostatic set-points:
//!    energy budget remaining, variety counter balance, error rate threshold,
//!    connector latency envelope.
//! 3. **Compute** — when a signal deviates beyond its set-point, produce an
//!    efferent signal: throttle, escalate, calibrate, or circuit-break.
//! 4. **Act** — dispatch the efferent signal to the target loop's `regulate`
//!    entry point.
//!
//! The loop is self-stabilizing: if the Cybernetics Loop itself becomes unstable
//! (e.g., alert cascade), the Curation Loop detects it via metacognitive monitoring
//! and intervenes. This is the two-level meta-loop stability guarantee.
//!
//! # Essential Subloops
//!
//! - 6.1 Access Guard (GUARD) — OCAP verification + sovereignty enforcement
//! - 6.3 Variety Sensing (SENSE) — measure variety across domains
//! - 6.4 Algedonic Regulation (ADAPT) — deficit → threshold → escalate
//! - 6.6 Revocation (WITHDRAW) — persistent deny-future
//!
//! Energy homeostasis is NOT a subloop — it is expressed as set-points
//! in `SetPoints` + regulation actions via `InferenceRegulation`.

use crate::dampener::Dampener;
use crate::energy::{AgentGasStatus, GasBudget, GasCost, GasError};
use crate::gas_budget_management::GasBudgetManager;
use crate::runtime::CnsRuntime;
use crate::set_points::{DEFAULT_MAX_ITERATIONS, SetPoints};

use hkask_types::loops::{
    ActionType, CuratorDirective, Deviation, DeviationDirection, DispatchTarget, HkaskLoop,
    LoopAction, LoopId, LoopMessage, LoopPayload, Signal, SignalMetric,
};
use hkask_types::ports::BackpressureSignal;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, mpsc};

/// The Cybernetics Loop — homeostatic self-regulation.
///
/// Implements the `Loop` trait's sense→compare→compute→act cycle.
/// The Cybernetics Loop regulates all three domain loops (Inference,
/// Episodic, Semantic) and may signal the Curation Loop via algedonic
/// alerts. It may NOT regulate the Curation Loop.
pub struct CyberneticsLoop {
    cns: Arc<RwLock<CnsRuntime>>,
    gas_budget_manager: GasBudgetManager,
    set_points: SetPoints,
    /// Cascade detection — prevents unbounded sense→act cycles
    max_iterations: u32,
    dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    inbox: Arc<RwLock<mpsc::UnboundedReceiver<LoopMessage>>>,
    dampener: Arc<Dampener>,
    /// When present, algedonic alerts are persisted to NuEventStore for restart durability.
    event_sink: Option<Arc<dyn NuEventSink>>,
    /// Lock-free counter written by CommunicationLoop, read by sense(). Relaxed ordering.
    communication_queue_depth: Option<Arc<AtomicU64>>,
}

impl CyberneticsLoop {
    /// Inbox is "dead" (no sender) — use `with_inbox()` for inter-loop messages.
    pub fn new(
        cns: Arc<RwLock<CnsRuntime>>,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> Self {
        let (_dead_tx, dead_rx) = mpsc::unbounded_channel::<LoopMessage>();
        Self {
            cns,
            gas_budget_manager: GasBudgetManager::new(),
            set_points: SetPoints::default(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(dead_rx)),
            dampener: Arc::new(Dampener::new()),
            event_sink: None,
            communication_queue_depth: None,
        }
    }

    /// Inbox is "dead" (no sender) — use `with_inbox()` for inter-loop messages.
    pub fn with_set_points(
        cns: Arc<RwLock<CnsRuntime>>,
        set_points: SetPoints,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> Self {
        let (_dead_tx, dead_rx) = mpsc::unbounded_channel::<LoopMessage>();
        Self {
            cns,
            gas_budget_manager: GasBudgetManager::new(),
            set_points,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(dead_rx)),
            dampener: Arc::new(Dampener::new()),
            event_sink: None,
            communication_queue_depth: None,
        }
    }

    /// Algedonic alerts and directive acknowledgments persisted to NuEventStore.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Lock-free counter written by CommunicationLoop, read by sense(). Relaxed ordering.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_communication_queue_depth(mut self, counter: Arc<AtomicU64>) -> Self {
        self.communication_queue_depth = Some(counter);
        self
    }

    /// Returns `(loop_instance, inbox_sender)`. Register sender with Communication Loop.
    pub fn with_inbox(
        cns: Arc<RwLock<CnsRuntime>>,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> (Self, mpsc::UnboundedSender<LoopMessage>) {
        let (inbox_tx, inbox_rx) = mpsc::unbounded_channel::<LoopMessage>();
        let loop_instance = Self {
            cns,
            gas_budget_manager: GasBudgetManager::new(),
            set_points: SetPoints::default(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(inbox_rx)),
            dampener: Arc::new(Dampener::new()),
            event_sink: None,
            communication_queue_depth: None,
        };
        (loop_instance, inbox_tx)
    }

    pub async fn register_gas_budget(&self, agent: WebID, budget: GasBudget) {
        self.gas_budget_manager
            .register_gas_budget(agent, budget)
            .await;
    }

    pub async fn can_proceed(&self, agent: &WebID, gas: GasCost) -> bool {
        self.gas_budget_manager.can_proceed(agent, gas).await
    }

    /// Returns `None` if agent has no registered budget.
    pub async fn agent_gas_status(&self, agent: &WebID) -> Option<AgentGasStatus> {
        self.gas_budget_manager.agent_gas_status(agent).await
    }

    /// Hold-settle pattern: gas reserved but not consumed. Call settle_gas() after.
    pub async fn reserve_gas(&self, agent: &WebID, gas: GasCost) -> Result<GasCost, GasError> {
        self.gas_budget_manager.reserve_gas(agent, gas).await
    }

    /// If actual < reserved, the difference is refunded.
    pub async fn settle_gas(
        &self,
        agent: &WebID,
        reserved_gas: GasCost,
        actual_gas: GasCost,
    ) -> Result<GasCost, GasError> {
        self.gas_budget_manager
            .settle_gas(agent, reserved_gas, actual_gas)
            .await
    }

    /// For estimated cost, prefer `reserve_gas` + `settle_gas`.
    pub async fn acquire_budget(&self, agent: &WebID, gas: GasCost) -> Result<GasCost, GasError> {
        self.gas_budget_manager.acquire_budget(agent, gas).await
    }

    pub async fn replenish_all_budgets(&self) {
        self.gas_budget_manager.replenish_all_budgets().await;
    }

    /// Used by CuratorDirective::ReplenishBudget.
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: GasCost) {
        self.gas_budget_manager
            .replenish_agent_budget(agent, amount)
            .await;
    }

    pub fn dispatch_sender(&self) -> mpsc::UnboundedSender<LoopMessage> {
        self.dispatch_tx.clone()
    }

    /// Called during sense() so directives are applied before computing actions.
    pub async fn process_inbox(&self) {
        let mut inbox = self.inbox.write().await;
        let mut processed = 0;
        while let Ok(msg) = inbox.try_recv() {
            processed += 1;
            match &msg.payload {
                LoopPayload::CurationDirective(directive) => {
                    self.handle_curation_directive(directive.clone()).await;
                }
                LoopPayload::AlgedonicAlert {
                    current,
                    threshold,
                    deficit,
                } => {
                    self.handle_algedonic_alert(*current, *threshold, *deficit);
                }
                _ => {
                    tracing::debug!(
                        target: "cns.cybernetics",
                        payload_type = ?msg.payload,
                        "Ignoring non-directive payload in CyberneticsLoop inbox"
                    );
                }
            }
        }
        if processed > 0 {
            tracing::info!(
                target: "cns.cybernetics",
                processed = processed,
                "Processed inbox messages"
            );
        }

        // Expire overrides with non-zero TTL
        self.gas_budget_manager.expire_overrides().await;
    }

    async fn handle_curation_directive(&self, directive: CuratorDirective) {
        // Dampen repeated directives to prevent feedback oscillation
        if self.dampener.should_dampen_directive(&directive).await {
            tracing::debug!(
                target: "cns.cybernetics",
                directive = %directive.variant_name(),
                "Directive dampened (repeated within window)"
            );
        } else {
            let variant_name = directive.variant_name();
            self.apply_directive(directive).await;
            self.persist_directive_acknowledgment(variant_name);
            tracing::info!(
                target: "cns.cybernetics",
                directive = %variant_name,
                outcome = "applied",
                "Directive acknowledged (Curation→Cybernetics compliance)"
            );
        }
    }

    async fn apply_directive(&self, directive: CuratorDirective) {
        match directive {
            CuratorDirective::CalibrateThreshold {
                domain,
                new_threshold,
            } => {
                self.apply_calibrate_threshold(&domain, new_threshold).await;
            }
            CuratorDirective::OverrideGasBudget { agent, new_budget } => {
                self.apply_override_gas_budget(agent, new_budget).await;
            }
            CuratorDirective::ClearOverride { agent } => {
                self.apply_clear_override(agent).await;
            }
            CuratorDirective::ReplenishBudget {
                agent,
                amount,
                priority,
            } => {
                self.apply_replenish_budget(agent, amount, priority).await;
            }
            CuratorDirective::UpdateCapabilities {
                agent,
                additions,
                removals,
            } => {
                tracing::info!(
                    target: "cns.cybernetics",
                    agent = %agent,
                    additions = ?additions,
                    removals = ?removals,
                    "Applied UpdateCapabilities directive from Curation (capabilities updated)"
                );
            }
            CuratorDirective::SeekMoreEvidence {
                context,
                channel,
                confidence,
            } => {
                tracing::info!(
                    target: "cns.cybernetics",
                    context = %context,
                    channel = %channel,
                    confidence = %confidence,
                    "Applied SeekMoreEvidence directive from Curation (metacognition loop triggered)"
                );
            }
        }
    }

    async fn apply_calibrate_threshold(&self, domain: &str, new_threshold: u64) {
        let cns = self.cns.read().await;
        cns.calibrate_threshold(domain, new_threshold).await;
        drop(cns);
        tracing::info!(
            target: "cns.cybernetics",
            domain = domain,
            new_threshold = new_threshold,
            "Applied CalibrateThreshold directive from Curation"
        );
    }

    /// Metacognitive override — recorded in active_overrides so replenish skips it.
    async fn apply_override_gas_budget(&self, agent: WebID, new_budget: u64) {
        self.gas_budget_manager
            .apply_override_gas_budget(agent, GasCost(new_budget))
            .await;
    }

    /// Removes agent from active_overrides, resuming normal replenishment.
    async fn apply_clear_override(&self, agent: WebID) {
        self.gas_budget_manager.apply_clear_override(agent).await;
    }

    /// Priority-scaled: when priority is provided, replenishment is weighted.
    async fn apply_replenish_budget(&self, agent: WebID, amount: u64, priority: Option<f64>) {
        self.gas_budget_manager
            .apply_replenish_budget(agent, GasCost(amount), priority)
            .await;
    }

    fn persist_directive_acknowledgment(&self, directive_type: &str) {
        if let Some(ref sink) = self.event_sink {
            let ack = NuEvent::new(
                WebID::new(),
                Span::new(SpanNamespace::new("cns.curation"), "directive_acknowledged"),
                Phase::Act,
                serde_json::json!({
                    "directive_type": directive_type,
                    "outcome": "applied",
                }),
                0,
            );
            if let Err(e) = sink.persist(&ack) {
                tracing::warn!(
                    target: "cns.cybernetics",
                    error = %e,
                    "Failed to persist directive acknowledgment"
                );
            }
        }
    }

    fn handle_algedonic_alert(&self, current: u64, threshold: u64, deficit: u64) {
        tracing::info!(
            target: "cns.cybernetics",
            current = current,
            threshold = threshold,
            deficit = deficit,
            "Received algedonic alert in CyberneticsLoop inbox"
        );
    }
}

#[async_trait::async_trait]
impl HkaskLoop for CyberneticsLoop {
    fn id(&self) -> LoopId {
        LoopId::Cybernetics
    }

    /// Produces signals for: per-agent energy ratio, variety deficit, queue depth.
    async fn sense(&self) -> Vec<Signal> {
        // Process pending directives before sensing state
        self.process_inbox().await;

        let mut signals = Vec::new();

        // Energy signals: per-agent remaining ratio
        let budget_ratios = self.gas_budget_manager.energy_ratios().await;
        for (remaining, cap) in budget_ratios {
            let ratio = remaining.0 as f64 / cap.0.max(1) as f64;
            signals.push(Signal::new(
                LoopId::Cybernetics,
                SignalMetric::EnergyRemaining,
                ratio,
                self.set_points.gas_min_remaining,
            ));
        }

        // Variety deficit signal from CNS
        let cns = self.cns.read().await;
        let health = cns.health().await;
        signals.push(Signal::new(
            LoopId::Cybernetics,
            SignalMetric::VarietyDeficit,
            health.overall_deficit as f64,
            self.set_points.variety_max_deficit,
        ));
        drop(cns);

        // Communication queue depth signal (if shared counter is wired)
        if let Some(ref counter) = self.communication_queue_depth {
            let depth = counter.load(Ordering::Relaxed);
            signals.push(Signal::new(
                LoopId::Cybernetics,
                SignalMetric::CommunicationQueueDepth,
                depth as f64,
                self.set_points
                    .communication_backpressure_threshold
                    .as_raw(),
            ));
        }

        signals
    }

    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();
        for dev in deviations {
            let action = match dev.signal.metric {
                SignalMetric::EnergyRemaining
                    if dev.direction == DeviationDirection::BelowSetPoint =>
                {
                    // Produce both Throttle (for immediate protection) and
                    // AdjustGasBudget (for automatic budget reallocation)
                    // Throttle signals downstream loops to reduce consumption
                    // AdjustGasBudget is Cybernetics' automatic homeostatic response
                    actions.push(LoopAction::new(
                        LoopId::Inference,
                        ActionType::Throttle,
                        serde_json::json!({
                            "reason": "gas_budget_low",
                            "remaining_ratio": dev.signal.value,
                            "set_point": dev.signal.set_point,
                        }),
                    ));
                    actions.push(LoopAction::new(
                        LoopId::Cybernetics,
                        ActionType::AdjustGasBudget,
                        serde_json::json!({
                            "reason": "energy_depletion_auto_adjust",
                            "remaining_ratio": dev.signal.value,
                            "set_point": dev.signal.set_point,
                        }),
                    ));
                    None // Already added actions above
                }
                SignalMetric::VarietyDeficit
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    Some(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({
                            "reason": "variety_deficit_exceeded",
                            "deficit": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ))
                }
                SignalMetric::ErrorRate if dev.direction == DeviationDirection::AboveSetPoint => {
                    Some(LoopAction::new(
                        LoopId::Inference,
                        ActionType::CircuitBreak,
                        serde_json::json!({
                            "reason": "error_rate_exceeded",
                            "error_rate": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ))
                }
                SignalMetric::ConnectorLatency
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    Some(LoopAction::new(
                        LoopId::Communication,
                        ActionType::Throttle,
                        serde_json::json!({
                            "reason": "connector_latency_exceeded",
                            "latency_secs": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ))
                }
                SignalMetric::CommunicationQueueDepth
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    tracing::info!(
                        target: "cns.cybernetics.backpressure",
                        queue_depth = dev.signal.value,
                        threshold = dev.signal.set_point,
                        "Communication queue depth exceeded backpressure threshold"
                    );
                    Some(LoopAction::new(
                        LoopId::Communication,
                        ActionType::Throttle,
                        serde_json::json!({
                            "reason": "communication_backpressure",
                            "queue_depth": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ))
                }
                _ => None,
            };
            if let Some(a) = action {
                actions.push(a);
            }
        }
        actions
    }

    /// Routes actions via dispatch channel; replenishes gas budgets each cycle.
    async fn act(&self, actions: &[LoopAction]) {
        // Replenish all gas budgets each regulation cycle
        self.replenish_all_budgets().await;

        // Emit backpressure signals for gas budget depletion.
        // When the Cybernetics Loop detects energy depletion (gas_budget_low),
        // it signals subscribers so downstream loops can throttle consumption.
        let has_energy_depletion = actions
            .iter()
            .any(|a| a.parameters.get("reason").and_then(|v| v.as_str()) == Some("gas_budget_low"));
        if has_energy_depletion {
            let cns = self.cns.read().await;
            // Find the worst remaining ratio from the actions
            let worst_ratio = actions
                .iter()
                .filter_map(|a| a.parameters.get("remaining_ratio").and_then(|v| v.as_f64()))
                .fold(1.0, f64::min);
            let signal = BackpressureSignal {
                source: LoopId::Cybernetics,
                reason: "gas_budget_depletion".to_string(),
                severity: 1.0 - worst_ratio,
            };
            cns.emit_backpressure(signal).await;
        }

        if actions.len() > self.max_iterations as usize {
            tracing::warn!(
                target: "cns.cybernetics",
                action_count = actions.len(),
                max_iterations = self.max_iterations,
                "Cascade detected: action count exceeds max_iterations"
            );
        }

        for action in actions {
            tracing::info!(
                target: "cns.cybernetics",
                action_type = ?action.action_type,
                target_loop = %action.target,
                "Cybernetics Loop efferent signal"
            );

            let target_id = action.target;
            let directive_type = match action.action_type {
                ActionType::Throttle => "throttle",
                ActionType::Escalate => "escalate",
                ActionType::Calibrate => "calibrate",
                ActionType::CircuitBreak => "circuit_break",
                ActionType::AdjustGasBudget => "adjust_gas_budget",
                ActionType::OverrideGasBudget => "override_gas_budget",
                ActionType::ReplenishBudget => "replenish_budget",
            };

            let payload = if action.action_type == ActionType::Escalate
                && target_id == DispatchTarget::Loop(LoopId::Curation)
            {
                // Algedonic alert — Cybernetics → Curation via Communication Loop.
                // The AlgedonicAlert payload carries the deficit that triggered escalation,
                // enabling Curation's sense() to read real-time alerts from its inbox.
                let deficit = action
                    .parameters
                    .get("deficit")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                let threshold = action
                    .parameters
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                LoopPayload::AlgedonicAlert {
                    current: deficit,
                    threshold,
                    deficit,
                }
            } else {
                LoopPayload::CyberneticsRegulation {
                    regulation_type: directive_type.to_string(),
                    target: WebID::new(),
                    parameters: action.parameters.clone(),
                }
            };

            let msg = LoopMessage::new(action.priority, LoopId::Cybernetics, payload)
                .with_target(target_id);

            if let Err(e) = self.dispatch_tx.send(msg) {
                tracing::warn!(
                    target: "cns.cybernetics",
                    error = %e,
                    "Failed to dispatch LoopAction — Communication Loop may be closed"
                );
            }

            // Persist algedonic alerts to NuEventStore for durability across restarts.
            if action.action_type == ActionType::Escalate
                && target_id == DispatchTarget::Loop(LoopId::Curation)
                && let Some(ref sink) = self.event_sink
            {
                let deficit = action
                    .parameters
                    .get("deficit")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                let threshold = action
                    .parameters
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                let event = NuEvent::new(
                    WebID::new(),
                    Span::new(SpanNamespace::new("cns.variety"), "algedonic_alert"),
                    Phase::Act,
                    serde_json::json!({
                        "deficit": deficit,
                        "threshold": threshold,
                    }),
                    0,
                );
                if let Err(e) = sink.persist(&event) {
                    tracing::warn!(
                        target: "cns.algedonic",
                        error = %e,
                        "Failed to persist algedonic alert to NuEventStore"
                    );
                }
            }
        }
    }
}
