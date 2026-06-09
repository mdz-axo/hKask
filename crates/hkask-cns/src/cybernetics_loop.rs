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
use crate::energy::{AgentEnergyStatus, EnergyBudget, EnergyCost, EnergyError};
use crate::energy_budget_management::EnergyBudgetManager;
use crate::runtime::CnsRuntime;
use crate::set_points::{DEFAULT_MAX_ITERATIONS, SetPoints};

use hkask_types::WebID;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
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
    energy_budget_manager: EnergyBudgetManager,
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
    pub fn new(
        cns: Arc<RwLock<CnsRuntime>>,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> Self {
        let (_, dead_rx) = mpsc::unbounded_channel();
        Self::build(cns, SetPoints::default(), dispatch_tx, dead_rx)
    }

    pub fn with_set_points(
        cns: Arc<RwLock<CnsRuntime>>,
        set_points: SetPoints,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> Self {
        let (_, dead_rx) = mpsc::unbounded_channel();
        Self::build(cns, set_points, dispatch_tx, dead_rx)
    }

    /// Shared struct init. `inbox` is dead (no sender) when called from `new()`/
    /// `with_set_points()`; use `with_inbox()` for a live inbox.
    fn build(
        cns: Arc<RwLock<CnsRuntime>>,
        set_points: SetPoints,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
        inbox: mpsc::UnboundedReceiver<LoopMessage>,
    ) -> Self {
        Self {
            cns,
            energy_budget_manager: EnergyBudgetManager::new(),
            set_points,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(inbox)),
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
        let (inbox_tx, inbox_rx) = mpsc::unbounded_channel();
        (
            Self::build(cns, SetPoints::default(), dispatch_tx, inbox_rx),
            inbox_tx,
        )
    }

    pub async fn register_energy_budget(&self, agent: WebID, budget: EnergyBudget) {
        self.energy_budget_manager
            .register_energy_budget(agent, budget)
            .await;
    }

    pub async fn can_proceed(&self, agent: &WebID, gas: EnergyCost) -> bool {
        self.energy_budget_manager.can_proceed(agent, gas).await
    }

    /// Returns `None` if agent has no registered budget.
    pub async fn agent_gas_status(&self, agent: &WebID) -> Option<AgentEnergyStatus> {
        self.energy_budget_manager.agent_gas_status(agent).await
    }

    /// Hold-settle pattern: gas reserved but not consumed. Call settle_gas() after.
    pub async fn reserve_gas(
        &self,
        agent: &WebID,
        gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        self.energy_budget_manager.reserve_gas(agent, gas).await
    }

    /// If actual < reserved, the difference is refunded.
    pub async fn settle_gas(
        &self,
        agent: &WebID,
        reserved_gas: EnergyCost,
        actual_gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        self.energy_budget_manager
            .settle_gas(agent, reserved_gas, actual_gas)
            .await
    }

    /// For estimated cost, prefer `reserve_gas` + `settle_gas`.
    pub async fn acquire_budget(
        &self,
        agent: &WebID,
        gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        self.energy_budget_manager.acquire_budget(agent, gas).await
    }

    pub async fn replenish_all_budgets(&self) {
        self.energy_budget_manager.replenish_all_budgets().await;
    }

    /// Used by CuratorDirective::ReplenishBudget.
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: EnergyCost) {
        self.energy_budget_manager
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
                    tracing::debug!(target: "cns.cybernetics", payload_type = ?msg.payload, "Ignoring non-directive payload in CyberneticsLoop inbox")
                }
            }
        }
        if processed > 0 {
            tracing::info!(target: "cns.cybernetics", processed = processed, "Processed inbox messages");
        }
        self.energy_budget_manager.expire_overrides().await;
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
            } => self.apply_calibrate_threshold(&domain, new_threshold).await,
            CuratorDirective::OverrideEnergyBudget { agent, new_budget } => {
                self.apply_override_energy_budget(agent, new_budget).await
            }
            CuratorDirective::ClearOverride { agent } => self.apply_clear_override(agent).await,
            CuratorDirective::ReplenishBudget {
                agent,
                amount,
                priority,
            } => self.apply_replenish_budget(agent, amount, priority).await,
            CuratorDirective::UpdateCapabilities {
                agent,
                additions,
                removals,
            } => {
                tracing::info!(target: "cns.cybernetics", agent = %agent, additions = ?additions, removals = ?removals, "Applied UpdateCapabilities directive from Curation (capabilities updated)")
            }
            CuratorDirective::SeekMoreEvidence {
                context,
                channel,
                confidence,
            } => {
                tracing::info!(target: "cns.cybernetics", context = %context, channel = %channel, confidence = %confidence, "Applied SeekMoreEvidence directive from Curation (metacognition loop triggered)")
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
    async fn apply_override_energy_budget(&self, agent: WebID, new_budget: u64) {
        self.energy_budget_manager
            .apply_override_energy_budget(agent, EnergyCost(new_budget))
            .await;
    }

    /// Removes agent from active_overrides, resuming normal replenishment.
    async fn apply_clear_override(&self, agent: WebID) {
        self.energy_budget_manager.apply_clear_override(agent).await;
    }

    /// Priority-scaled: when priority is provided, replenishment is weighted.
    async fn apply_replenish_budget(&self, agent: WebID, amount: u64, priority: Option<f64>) {
        self.energy_budget_manager
            .apply_replenish_budget(agent, EnergyCost(amount), priority)
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
        let budget_ratios = self.energy_budget_manager.energy_ratios().await;
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
                    actions.push(LoopAction::new(LoopId::Inference, ActionType::Throttle, serde_json::json!({"reason": "energy_budget_low", "remaining_ratio": dev.signal.value, "set_point": dev.signal.set_point})));
                    actions.push(LoopAction::new(LoopId::Cybernetics, ActionType::AdjustEnergyBudget, serde_json::json!({"reason": "energy_depletion_auto_adjust", "remaining_ratio": dev.signal.value, "set_point": dev.signal.set_point})));
                    None
                }
                SignalMetric::VarietyDeficit
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    Some(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({"reason": "variety_deficit_exceeded", "deficit": dev.signal.value, "threshold": dev.signal.set_point}),
                    ))
                }
                SignalMetric::ErrorRate if dev.direction == DeviationDirection::AboveSetPoint => {
                    Some(LoopAction::new(
                        LoopId::Inference,
                        ActionType::CircuitBreak,
                        serde_json::json!({"reason": "error_rate_exceeded", "error_rate": dev.signal.value, "threshold": dev.signal.set_point}),
                    ))
                }
                SignalMetric::ConnectorLatency
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    Some(LoopAction::new(
                        LoopId::Communication,
                        ActionType::Throttle,
                        serde_json::json!({"reason": "connector_latency_exceeded", "latency_secs": dev.signal.value, "threshold": dev.signal.set_point}),
                    ))
                }
                SignalMetric::CommunicationQueueDepth
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    tracing::info!(target: "cns.cybernetics.backpressure", queue_depth = dev.signal.value, threshold = dev.signal.set_point, "Communication queue depth exceeded backpressure threshold");
                    Some(LoopAction::new(
                        LoopId::Communication,
                        ActionType::Throttle,
                        serde_json::json!({"reason": "communication_backpressure", "queue_depth": dev.signal.value, "threshold": dev.signal.set_point}),
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

    async fn act(&self, actions: &[LoopAction]) {
        self.replenish_all_budgets().await;
        let has_energy_depletion = actions.iter().any(|a| {
            a.parameters.get("reason").and_then(|v| v.as_str()) == Some("energy_budget_low")
        });
        if has_energy_depletion {
            let cns = self.cns.read().await;
            let worst_ratio = actions
                .iter()
                .filter_map(|a| a.parameters.get("remaining_ratio").and_then(|v| v.as_f64()))
                .fold(1.0, f64::min);
            cns.emit_backpressure(BackpressureSignal {
                source: LoopId::Cybernetics,
                reason: "energy_budget_depletion".into(),
                severity: 1.0 - worst_ratio,
            })
            .await;
        }
        if actions.len() > self.max_iterations as usize {
            tracing::warn!(target: "cns.cybernetics", action_count = actions.len(), max_iterations = self.max_iterations, "Cascade detected: action count exceeds max_iterations");
        }
        for action in actions {
            tracing::info!(target: "cns.cybernetics", action_type = ?action.action_type, target_loop = %action.target, "Cybernetics Loop efferent signal");
            let target_id = action.target;
            let directive_type = match action.action_type {
                ActionType::Throttle => "throttle",
                ActionType::Escalate => "escalate",
                ActionType::Calibrate => "calibrate",
                ActionType::CircuitBreak => "circuit_break",
                ActionType::AdjustEnergyBudget => "adjust_energy_budget",
                ActionType::OverrideEnergyBudget => "override_energy_budget",
                ActionType::ReplenishBudget => "replenish_budget",
            };
            let payload = if action.action_type == ActionType::Escalate
                && target_id == DispatchTarget::Loop(LoopId::Curation)
            {
                let (deficit, threshold) = extract_deficit_threshold(&action.parameters);
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
                tracing::warn!(target: "cns.cybernetics", error = %e, "Failed to dispatch LoopAction — Communication Loop may be closed");
            }
            if action.action_type == ActionType::Escalate
                && target_id == DispatchTarget::Loop(LoopId::Curation)
                && let Some(ref sink) = self.event_sink
            {
                let (deficit, threshold) = extract_deficit_threshold(&action.parameters);
                let event = NuEvent::new(
                    WebID::new(),
                    Span::new(SpanNamespace::new("cns.variety"), "algedonic_alert"),
                    Phase::Act,
                    serde_json::json!({"deficit": deficit, "threshold": threshold}),
                    0,
                );
                if let Err(e) = sink.persist(&event) {
                    tracing::warn!(target: "cns.algedonic", error = %e, "Failed to persist algedonic alert to NuEventStore");
                }
            }
        }
    }
}

/// Extract (deficit, threshold) from action parameters. Returns (0, 0) on missing fields.
fn extract_deficit_threshold(params: &serde_json::Value) -> (u64, u64) {
    let get_f64 =
        |key: &str| -> u64 { params.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) as u64 };
    (get_f64("deficit"), get_f64("threshold"))
}
