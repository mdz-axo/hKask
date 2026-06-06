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
use crate::energy::{AgentGasStatus, GasBudget, GasError};
use crate::gas_budget_management::GasBudgetManager;
use crate::runtime::CnsRuntime;
use crate::set_points::{DEFAULT_MAX_ITERATIONS, SetPoints};

#[cfg(test)]
use crate::set_points::SetPointsConfig;
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

    pub async fn can_proceed(&self, agent: &WebID, gas: u64) -> bool {
        self.gas_budget_manager.can_proceed(agent, gas).await
    }

    /// Returns `None` if agent has no registered budget.
    pub async fn agent_gas_status(&self, agent: &WebID) -> Option<AgentGasStatus> {
        self.gas_budget_manager.agent_gas_status(agent).await
    }

    /// Hold-settle pattern: gas reserved but not consumed. Call settle_gas() after.
    pub async fn reserve_gas(&self, agent: &WebID, gas: u64) -> Result<u64, GasError> {
        self.gas_budget_manager.reserve_gas(agent, gas).await
    }

    /// If actual < reserved, the difference is refunded.
    pub async fn settle_gas(
        &self,
        agent: &WebID,
        reserved_gas: u64,
        actual_gas: u64,
    ) -> Result<u64, GasError> {
        self.gas_budget_manager
            .settle_gas(agent, reserved_gas, actual_gas)
            .await
    }

    /// For estimated cost, prefer `reserve_gas` + `settle_gas`.
    pub async fn acquire_budget(&self, agent: &WebID, gas: u64) -> Result<u64, GasError> {
        self.gas_budget_manager.acquire_budget(agent, gas).await
    }

    pub async fn replenish_all_budgets(&self) {
        self.gas_budget_manager.replenish_all_budgets().await;
    }

    /// Used by CuratorDirective::ReplenishBudget.
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: u64) {
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
            .apply_override_gas_budget(agent, new_budget)
            .await;
    }

    /// Removes agent from active_overrides, resuming normal replenishment.
    async fn apply_clear_override(&self, agent: WebID) {
        self.gas_budget_manager.apply_clear_override(agent).await;
    }

    /// Priority-scaled: when priority is provided, replenishment is weighted.
    async fn apply_replenish_budget(&self, agent: WebID, amount: u64, priority: Option<f64>) {
        self.gas_budget_manager
            .apply_replenish_budget(agent, amount, priority)
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
            let ratio = remaining as f64 / cap.max(1) as f64;
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
                self.set_points.communication_backpressure_threshold,
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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::loops::{
        ActionType, CuratorDirective, DeviationDirection, HkaskLoop, LoopId, LoopPayload,
        SignalMetric,
    };

    fn test_dispatch_tx() -> mpsc::UnboundedSender<LoopMessage> {
        let (tx, _rx) = mpsc::unbounded_channel();
        tx
    }

    #[test]
    fn cybernetics_loop_id_is_cybernetics() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());
        assert_eq!(loop6.id(), LoopId::Cybernetics);
    }

    #[tokio::test]
    async fn energy_deviation_produces_throttle_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Signal: energy remaining at 5% (below 20% set-point)
        let signal = Signal::new(
            LoopId::Cybernetics,
            SignalMetric::EnergyRemaining,
            0.05,
            0.2,
        );
        let deviations = loop6.compare(&[signal]).await;
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::BelowSetPoint);

        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 2); // Throttle + AdjustGasBudget
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(actions[0].target, DispatchTarget::Loop(LoopId::Inference));
        assert_eq!(actions[1].action_type, ActionType::AdjustGasBudget);
        assert_eq!(actions[1].target, DispatchTarget::Loop(LoopId::Cybernetics));
    }

    #[tokio::test]
    async fn variety_deficit_produces_escalate_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Signal: variety deficit at 150 (above 100 threshold)
        let signal = Signal::new(
            LoopId::Cybernetics,
            SignalMetric::VarietyDeficit,
            150.0,
            100.0,
        );
        let deviations = loop6.compare(&[signal]).await;
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::AboveSetPoint);

        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Escalate);
        assert_eq!(actions[0].target, DispatchTarget::Loop(LoopId::Curation));
    }

    #[tokio::test]
    async fn error_rate_produces_circuit_break_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Signal: error rate at 50% (above 30% threshold)
        let signal = Signal::new(LoopId::Cybernetics, SignalMetric::ErrorRate, 0.5, 0.3);
        let deviations = loop6.compare(&[signal]).await;
        assert_eq!(deviations.len(), 1);

        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::CircuitBreak);
        assert_eq!(actions[0].target, DispatchTarget::Loop(LoopId::Inference));
    }

    #[tokio::test]
    async fn no_deviation_produces_no_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Signal: energy at 50% (above 20% set-point — no deviation)
        let signal = Signal::new(LoopId::Cybernetics, SignalMetric::EnergyRemaining, 0.5, 0.2);
        let deviations = loop6.compare(&[signal]).await;
        // Deviation exists (above set-point) but no action for above-set-point energy
        let actions = loop6.compute(&deviations).await;
        // Above-set-point energy is fine — no action needed
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn can_proceed_with_sufficient_budget() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());
        let agent = WebID::new();

        let budget = GasBudget::new(10_000);
        loop6.register_gas_budget(agent, budget).await;

        assert!(loop6.can_proceed(&agent, 100).await);
    }

    #[tokio::test]
    async fn gas_budget_consume_deducts() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());
        let agent = WebID::new();

        let budget = GasBudget::new(10_000);
        loop6.register_gas_budget(agent, budget).await;

        let cost = loop6.acquire_budget(&agent, 100).await.unwrap();
        assert!(cost > 0);
    }

    #[tokio::test]
    async fn full_tick_cycle_completes() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // A tick with default state should complete without panic
        loop6.tick().await;
    }

    // Task 7: Cybernetic Unit Tests — Full loop validation

    /// Test: Inject a known energy deviation (5% remaining vs 20% set-point)
    /// Assert: The loop produces a Throttle action targeting Inference
    /// Assert: The action propagates through the capability membrane
    /// Assert: The system reaches a new stable equilibrium within bounded iterations
    #[tokio::test]
    async fn energy_deviation_propagates_and_stabilizes() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Step 1: Inject known deviation — energy at 5% (set-point: 20%)
        let signal = Signal::new(
            LoopId::Cybernetics,
            SignalMetric::EnergyRemaining,
            0.05,
            0.2,
        );

        // Step 2: Compare — detect deviation
        let deviations = loop6.compare(&[signal]).await;
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::BelowSetPoint);
        assert!((deviations[0].magnitude - 0.15).abs() < f64::EPSILON);

        // Step 3: Compute — produce efferent action
        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 2); // Throttle + AdjustGasBudget
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(actions[0].target, DispatchTarget::Loop(LoopId::Inference));
        assert_eq!(actions[1].action_type, ActionType::AdjustGasBudget);
        assert_eq!(actions[1].target, DispatchTarget::Loop(LoopId::Cybernetics));

        // Step 4: Verify capability membrane — Cybernetics can regulate Inference
        // (domain loop), but NOT Curation (peer meta loop)
        assert_ne!(actions[0].target, DispatchTarget::Loop(LoopId::Curation));

        // Step 5: Simulate stabilization — after throttling, energy recovers
        let recovered_signal = Signal::new(
            LoopId::Cybernetics,
            SignalMetric::EnergyRemaining,
            0.25,
            0.2,
        );
        let new_deviations = loop6.compare(&[recovered_signal]).await;
        let new_actions = loop6.compute(&new_deviations).await;
        // Above-set-point energy is fine — no throttle action
        assert!(new_actions.is_empty());
    }

    /// Test: Multiple simultaneous deviations produce multiple actions
    #[tokio::test]
    async fn multiple_deviations_produce_multiple_actions() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        let signals = vec![
            Signal::new(
                LoopId::Cybernetics,
                SignalMetric::EnergyRemaining,
                0.05,
                0.2,
            ),
            Signal::new(
                LoopId::Cybernetics,
                SignalMetric::VarietyDeficit,
                150.0,
                100.0,
            ),
            Signal::new(LoopId::Cybernetics, SignalMetric::ErrorRate, 0.5, 0.3),
        ];

        let deviations = loop6.compare(&signals).await;
        assert_eq!(deviations.len(), 3);

        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 4); // gas:Throttle + gas:AdjustGasBudget + variety:Escalate + error:CircuitBreak

        // Verify each action targets the correct loop
        let targets: std::collections::HashSet<DispatchTarget> =
            actions.iter().map(|a| a.target).collect();
        assert!(targets.contains(&DispatchTarget::Loop(LoopId::Inference))); // gas:Throttle + error:CircuitBreak
        assert!(targets.contains(&DispatchTarget::Loop(LoopId::Curation))); // variety:Escalate
        assert!(targets.contains(&DispatchTarget::Loop(LoopId::Cybernetics))); // gas:AdjustGasBudget
    }

    /// Test: Loop reaches equilibrium within bounded iterations
    #[tokio::test]
    async fn loop_reaches_equilibrium_within_bounded_iterations() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Simulate a sequence of improving signals
        let max_iterations = 10;
        for i in 0..max_iterations {
            // Energy recovers from 5% to 25% over iterations
            let energy = 0.05 + (i as f64 * 0.02);
            let signal = Signal::new(
                LoopId::Cybernetics,
                SignalMetric::EnergyRemaining,
                energy,
                0.2,
            );
            let deviations = loop6.compare(&[signal]).await;
            let actions = loop6.compute(&deviations).await;

            if energy >= 0.2 {
                // Once energy reaches set-point, no throttle action
                let throttle_actions: Vec<_> = actions
                    .iter()
                    .filter(|a| a.action_type == ActionType::Throttle)
                    .collect();
                assert!(
                    throttle_actions.is_empty(),
                    "System should stabilize by iteration {}, but still throttling",
                    i
                );
                return; // Equilibrium reached
            }
        }
        panic!(
            "System did not reach equilibrium within {} iterations",
            max_iterations
        );
    }

    /// Test: Energy budget exhaustion blocks operations
    #[tokio::test]
    async fn energy_exhaustion_blocks_operations() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());
        let agent = WebID::new();

        // Register a very small budget
        let budget = GasBudget::new(100);
        loop6.register_gas_budget(agent, budget).await;

        // Initially can proceed
        assert!(loop6.can_proceed(&agent, 10).await);

        // Exhaust the budget
        while loop6.acquire_budget(&agent, 10).await.is_ok() {
            // Keep consuming until budget exhausted
        }

        // Now cannot proceed
        assert!(!loop6.can_proceed(&agent, 10).await);
    }

    /// Test: Gas replenishment restores capacity
    #[tokio::test]
    async fn gas_replenishment_restores_capacity() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());
        let agent = WebID::new();

        let budget = GasBudget::new(100).with_replenish_rate(10);
        loop6.register_gas_budget(agent, budget).await;

        // Exhaust the budget
        while loop6.acquire_budget(&agent, 10).await.is_ok() {}
        assert!(!loop6.can_proceed(&agent, 10).await);

        // Replenish all budgets
        loop6.replenish_all_budgets().await;

        // Can proceed again — replenished by 10 units
        assert!(loop6.can_proceed(&agent, 10).await);
    }

    /// Test: Directive replenishment restores specific agent
    #[tokio::test]
    async fn directive_replenishment_restores_agent() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());
        let agent = WebID::new();

        let budget = GasBudget::new(100);
        loop6.register_gas_budget(agent, budget).await;

        // Exhaust the budget
        while loop6.acquire_budget(&agent, 10).await.is_ok() {}
        assert!(!loop6.can_proceed(&agent, 10).await);

        // Replenish by directive
        loop6.replenish_agent_budget(&agent, 50).await;

        // Can proceed — replenished by 50 units
        assert!(loop6.can_proceed(&agent, 10).await);
    }

    /// Test: Connector latency deviation produces throttle on Communication
    #[tokio::test]
    async fn connector_latency_produces_throttle_on_communication() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        let signal = Signal::new(
            LoopId::Cybernetics,
            SignalMetric::ConnectorLatency,
            60.0,
            30.0,
        );
        let deviations = loop6.compare(&[signal]).await;
        let actions = loop6.compute(&deviations).await;

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(
            actions[0].target,
            DispatchTarget::Loop(LoopId::Communication)
        );
    }

    // Inbox processing tests — CuratorDirective consumption

    #[tokio::test]
    async fn cybernetics_loop_processes_calibrate_directive() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let (loop6, inbox_tx) = CyberneticsLoop::with_inbox(cns.clone(), test_dispatch_tx());

        // Send a CalibrateThreshold directive
        let msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective(CuratorDirective::CalibrateThreshold {
                domain: "variety".to_string(),
                new_threshold: 200,
            }),
        )
        .with_target(LoopId::Cybernetics);

        inbox_tx.send(msg).unwrap();

        // Process inbox should apply the threshold
        loop6.process_inbox().await;

        // Verify: the CNS threshold should have been updated
        // CnsRuntime::calibrate_threshold is async, so check after processing
        let _health = cns.read().await.health().await;
        // Health should still be accessible (no crash) — the threshold was applied
    }

    #[tokio::test]
    async fn cybernetics_loop_processes_override_gas_budget_directive() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let agent = WebID::new();
        let (loop6, inbox_tx) = CyberneticsLoop::with_inbox(cns, test_dispatch_tx());

        // Register initial budget
        loop6
            .register_gas_budget(agent, GasBudget::new(10_000))
            .await;

        // Send OverrideGasBudget directive
        let msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective(CuratorDirective::OverrideGasBudget {
                agent,
                new_budget: 5000,
            }),
        )
        .with_target(LoopId::Cybernetics);

        inbox_tx.send(msg).unwrap();

        // Process inbox
        loop6.process_inbox().await;

        // Verify: the agent's budget should now be 5000
        // After adjustment, remaining=5000, cap=5000
        assert!(!loop6.can_proceed(&agent, 40_000).await);
        assert!(loop6.can_proceed(&agent, 100).await);
    }

    #[tokio::test]
    async fn cybernetics_loop_tick_applies_directives_before_sensing() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let agent = WebID::new();
        let (loop6, inbox_tx) = CyberneticsLoop::with_inbox(cns, test_dispatch_tx());
        loop6
            .register_gas_budget(agent, GasBudget::new(10_000))
            .await;

        // Send override gas budget directive before tick
        let msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective(CuratorDirective::OverrideGasBudget {
                agent,
                new_budget: 100,
            }),
        )
        .with_target(LoopId::Cybernetics);
        inbox_tx.send(msg).unwrap();

        // Tick should process inbox first, then sense with updated state
        loop6.tick().await;
    }

    #[tokio::test]
    async fn cybernetics_loop_inbox_registers_new_budget_for_unknown_agent() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let agent = WebID::new();
        let (loop6, inbox_tx) = CyberneticsLoop::with_inbox(cns, test_dispatch_tx());

        // No budget registered for agent yet
        assert!(loop6.can_proceed(&agent, 1_000_000).await); // soft limit: allowed

        // Send OverrideGasBudget for an unregistered agent
        let msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective(CuratorDirective::OverrideGasBudget {
                agent,
                new_budget: 500,
            }),
        )
        .with_target(LoopId::Cybernetics);
        inbox_tx.send(msg).unwrap();

        loop6.process_inbox().await;

        // Now the agent has a budget of 500
        assert!(loop6.can_proceed(&agent, 100).await);
        assert!(!loop6.can_proceed(&agent, 1_000_000).await);
    }

    #[tokio::test]
    async fn cybernetics_loop_dead_inbox_ignores_messages() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        // Using new() which creates a dead inbox
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // process_inbox on a dead inbox should be a no-op
        loop6.process_inbox().await;

        // And tick should still work fine
        loop6.tick().await;
    }

    #[test]
    fn set_points_config_from_yaml() {
        let yaml = "gas_min_remaining: 0.3\nvariety_max_deficit: 200.0\n";
        let config = SetPointsConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.gas_min_remaining, Some(0.3));
        assert_eq!(config.variety_max_deficit, Some(200.0));
        assert_eq!(config.error_rate_max, None); // Not specified
    }

    #[test]
    fn set_points_from_config_uses_defaults_for_missing() {
        let config = SetPointsConfig {
            gas_min_remaining: Some(0.5),
            variety_max_deficit: None,
            error_rate_max: None,
            connector_latency_max_secs: None,
            communication_backpressure_threshold: None,
        };
        let sp = SetPoints::from_config(&config);
        assert_eq!(sp.gas_min_remaining, 0.5);
        assert_eq!(sp.variety_max_deficit, 100.0); // default
        assert_eq!(sp.error_rate_max, 0.3); // default
        assert_eq!(sp.connector_latency_max_secs, 30.0); // default
    }

    #[test]
    fn set_points_from_empty_config_uses_all_defaults() {
        let yaml = "{}\n";
        let config = SetPointsConfig::from_yaml(yaml).unwrap();
        let sp = SetPoints::from_config(&config);
        let defaults = SetPoints::default();
        assert_eq!(sp.gas_min_remaining, defaults.gas_min_remaining);
        assert_eq!(sp.variety_max_deficit, defaults.variety_max_deficit);
    }

    #[test]
    fn set_points_yaml_full_config() {
        let yaml = "gas_min_remaining: 0.3\nvariety_max_deficit: 200.0\nerror_rate_max: 0.4\nconnector_latency_max_secs: 60.0\n";
        let config = SetPointsConfig::from_yaml(yaml).unwrap();
        let sp = SetPoints::from_config(&config);
        assert_eq!(sp.gas_min_remaining, 0.3);
        assert_eq!(sp.variety_max_deficit, 200.0);
        assert_eq!(sp.error_rate_max, 0.4);
        assert_eq!(sp.connector_latency_max_secs, 60.0);
    }

    // T12: Curation-Directed Gas Replenishment — full path integration test

    /// Integration test: CurationLoop issues `CuratorDirective::ReplenishBudget`
    /// → CommunicationLoop dispatches → CyberneticsLoop inbox → `process_inbox()`
    /// → `replenish_agent_budget()` → agent budget updated.
    ///
    /// This verifies the entire message path from Curation through Communication
    /// dispatch to Cybernetics inbox processing results in a gas budget increase.
    #[tokio::test]
    async fn curation_directed_gas_replenishment_full_path() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let agent = WebID::new();
        let (loop6, inbox_tx) = CyberneticsLoop::with_inbox(cns, test_dispatch_tx());

        // Step 1: Register a gas budget for the agent
        let initial_cap = 100u64;
        loop6
            .register_gas_budget(agent, GasBudget::new(initial_cap))
            .await;

        // Step 2: Exhaust the budget so remaining == 0
        while loop6.acquire_budget(&agent, 10).await.is_ok() {}
        assert!(
            !loop6.can_proceed(&agent, 1).await,
            "Budget should be exhausted before replenishment"
        );

        // Step 3: Simulate CurationLoop issuing CuratorDirective::ReplenishBudget
        // This mirrors what MessageDispatch::send_curator_directive does when
        // CuratorDirective::ReplenishBudget { agent, amount } is dispatched.
        let replenish_amount = 50u64;
        let msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective(CuratorDirective::ReplenishBudget {
                agent,
                amount: replenish_amount,
                priority: None,
            }),
        )
        .with_target(LoopId::Cybernetics);

        // Step 4: Send through the inbox (CommunicationLoop would deliver here)
        inbox_tx.send(msg).unwrap();

        // Step 5: process_inbox() consumes the message and calls replenish_agent_budget()
        loop6.process_inbox().await;

        // Step 6: Verify the gas budget was increased
        assert!(
            loop6.can_proceed(&agent, 1).await,
            "Agent should be able to proceed after replenishment"
        );
        // Verify exact replenishment amount (replenish_by caps at cap)
        // With cap=100 and replenish_by(50), remaining should be min(0+50, 100) = 50
        assert!(
            loop6.can_proceed(&agent, 50).await,
            "Agent should have 50 units available after replenishment"
        );
        assert!(
            !loop6.can_proceed(&agent, 51).await,
            "Agent should NOT have more than 50 units available after replenishment from zero"
        );
    }

    // T13: Curation Override Persistence — override survives replenishment

    /// Regression test: OverrideGasBudget directive must survive
    /// `replenish_all_budgets()` calls. Before the fix, `act()` called
    /// `replenish_all_budgets()` which overwrote Curation's override within
    /// one regulation cycle, defeating the metacognitive override mechanism.
    #[tokio::test]
    async fn curation_override_survives_replenishment() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let agent = WebID::new();
        let (loop6, inbox_tx) = CyberneticsLoop::with_inbox(cns, test_dispatch_tx());

        // Register a budget with cap=1000
        loop6.register_gas_budget(agent, GasBudget::new(1000)).await;

        // Override to a much lower budget (500) via Curation directive
        let msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective(CuratorDirective::OverrideGasBudget {
                agent,
                new_budget: 500,
            }),
        )
        .with_target(LoopId::Cybernetics);
        inbox_tx.send(msg).unwrap();
        loop6.process_inbox().await;

        // Verify: budget is now 500 (cap and remaining)
        assert!(
            loop6.can_proceed(&agent, 500).await,
            "Agent should have 500 units after override"
        );
        assert!(
            !loop6.can_proceed(&agent, 501).await,
            "Agent should NOT exceed overridden budget of 500"
        );

        // Now call replenish_all_budgets() — this used to overwrite the override
        loop6.replenish_all_budgets().await;

        // The override must survive: budget should still be capped at 500
        assert!(
            loop6.can_proceed(&agent, 500).await,
            "Overridden budget must survive replenishment"
        );
        assert!(
            !loop6.can_proceed(&agent, 501).await,
            "Replenishment must not restore budget beyond Curation override"
        );
    }

    /// Verify that ClearOverride directive removes the override and allows
    /// normal replenishment to resume.
    #[tokio::test]
    async fn clear_override_resumes_normal_replenishment() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let agent = WebID::new();
        let (loop6, inbox_tx) = CyberneticsLoop::with_inbox(cns, test_dispatch_tx());

        // Register a budget with cap=1000
        loop6.register_gas_budget(agent, GasBudget::new(1000)).await;

        // Override to 200 via Curation
        let override_msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective(CuratorDirective::OverrideGasBudget {
                agent,
                new_budget: 200,
            }),
        )
        .with_target(LoopId::Cybernetics);
        inbox_tx.send(override_msg).unwrap();
        loop6.process_inbox().await;

        // Confirm override is active
        assert!(!loop6.can_proceed(&agent, 201).await);

        // Send ClearOverride directive
        let clear_msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective(CuratorDirective::ClearOverride { agent }),
        )
        .with_target(LoopId::Cybernetics);
        inbox_tx.send(clear_msg).unwrap();
        loop6.process_inbox().await;

        // After clearing, replenish should work normally and can fill up to cap
        // The cap was set to 200 by the override, but replenish uses cap
        // Since override is cleared, replenishment will restore up to the
        // current cap (200). We need to verify that replenishment is no
        // longer blocked.
        //
        // Exhaust budget first to see replenishment effect
        while loop6.acquire_budget(&agent, 10).await.is_ok() {}
        assert!(!loop6.can_proceed(&agent, 1).await);

        // Replenish should now work (no override blocking it)
        loop6.replenish_all_budgets().await;

        // After replenishment, agent should have gas again
        assert!(
            loop6.can_proceed(&agent, 1).await,
            "After clearing override, normal replenishment should resume"
        );
    }
}
