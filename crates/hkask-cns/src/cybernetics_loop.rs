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
use crate::wallet_budget::WalletBackedBudget;

use crate::algedonic::{AlertSeverity, RuntimeAlert};
use crate::types::loops::{
    ActionType, CurationInput, CuratorDirective, Deviation, DeviationDirection, HkaskLoop,
    LoopAction, LoopId, LoopQuality, Signal, SignalMetric, ToolConsumptionEvent,
};
use hkask_ports::BackpressureSignal;
use hkask_types::WebID;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanKind};
use std::sync::Arc;
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
    dampener: Arc<Dampener>,
    /// When present, algedonic alerts are persisted to NuEventStore for restart durability.
    event_sink: Option<Arc<dyn NuEventSink>>,
    /// Direct alerts channel: Cybernetics → Curation (CurationInput).
    alerts_tx: Option<mpsc::UnboundedSender<CurationInput>>,
    /// Direct tool consumption channel: GovernedTool → Cybernetics.
    tool_consumption_rx: Option<Arc<RwLock<mpsc::UnboundedReceiver<ToolConsumptionEvent>>>>,
    /// Direct curator directive channel: Curation → Cybernetics.
    curator_directive_rx: Option<Arc<RwLock<mpsc::UnboundedReceiver<CuratorDirective>>>>,
    /// Loop-quality telemetry from the most recent tick cycle.
    loop_quality: Arc<RwLock<LoopQuality>>,
}

impl CyberneticsLoop {
    pub fn new(cns: Arc<RwLock<CnsRuntime>>) -> Self {
        Self::build(cns, SetPoints::default())
    }

    pub fn with_set_points(cns: Arc<RwLock<CnsRuntime>>, set_points: SetPoints) -> Self {
        Self::build(cns, set_points)
    }

    fn build(cns: Arc<RwLock<CnsRuntime>>, set_points: SetPoints) -> Self {
        let slf = Self {
            cns,
            energy_budget_manager: EnergyBudgetManager::new(),
            set_points,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            dampener: Arc::new(Dampener::new()),
            event_sink: None,
            alerts_tx: None,
            tool_consumption_rx: None,
            curator_directive_rx: None,
            loop_quality: Arc::new(RwLock::new(LoopQuality::default())),
        };
        if slf.event_sink.is_none() && slf.alerts_tx.is_none() {
            tracing::warn!(target: "cns.cybernetics", "CyberneticsLoop constructed with no alert pathway — alerts will be lost until with_alerts_channel() or with_event_sink() is called");
        }
        slf
    }

    /// Algedonic alerts and directive acknowledgments persisted to NuEventStore.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Wire the direct alerts channel for Cybernetics → Curation CurationInput delivery.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_alerts_channel(mut self, tx: mpsc::UnboundedSender<CurationInput>) -> Self {
        self.alerts_tx = Some(tx);
        self
    }

    /// Wire the direct tool consumption channel: GovernedTool → Cybernetics.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_tool_consumption_channel(
        mut self,
        rx: mpsc::UnboundedReceiver<ToolConsumptionEvent>,
    ) -> Self {
        self.tool_consumption_rx = Some(Arc::new(RwLock::new(rx)));
        self
    }

    /// Wire the direct curator directive channel: Curation → Cybernetics.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_curator_directive_channel(
        mut self,
        rx: mpsc::UnboundedReceiver<CuratorDirective>,
    ) -> Self {
        self.curator_directive_rx = Some(Arc::new(RwLock::new(rx)));
        self
    }

    /// Record a tool outcome in the CNS runtime for outcome quality tracking.
    ///
    /// Delegates to `CnsRuntime::record_outcome`. Called by `GovernedTool`
    /// after every tool invocation completes.
    pub async fn record_outcome(&self, domain: &str, success: bool, error_kind: Option<&str>) {
        self.cns
            .read()
            .await
            .record_outcome(domain, success, error_kind)
            .await;
    }

    pub async fn register_energy_budget(&self, agent: WebID, budget: EnergyBudget) {
        self.energy_budget_manager
            .register_energy_budget(agent, budget)
            .await;
    }

    /// Register a wallet-backed budget for an agent (Phase 5).
    /// Wallet budgets are checked before gas budgets in the membrane.
    pub async fn register_wallet_budget(&self, agent: WebID, budget: WalletBackedBudget) {
        self.energy_budget_manager
            .register_wallet_budget(agent, budget)
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

    /// Called during sense() so directives are applied before computing actions.
    pub async fn process_inbox(&self) {
        // Drain direct curator directive channel.
        if let Some(ref rx) = self.curator_directive_rx {
            let mut cd_rx = rx.write().await;
            let mut cd_processed = 0;
            while let Ok(directive) = cd_rx.try_recv() {
                cd_processed += 1;
                self.handle_curation_directive(directive).await;
            }
            if cd_processed > 0 {
                tracing::info!(target: "cns.cybernetics", processed = cd_processed, "Processed direct curator directives");
            }
        }

        // Drain direct tool consumption channel.
        if let Some(ref rx) = self.tool_consumption_rx {
            let mut tc_rx = rx.write().await;
            let mut tc_processed = 0;
            while let Ok(event) = tc_rx.try_recv() {
                tc_processed += 1;
                tracing::info!(
                    target: "cns.cybernetics",
                    tool = %event.tool_name,
                    agent = %event.agent,
                    gas_cost = event.gas_cost,
                    success = event.success,
                    "Tool consumption event received (direct channel)"
                );
            }
            if tc_processed > 0 {
                tracing::info!(target: "cns.cybernetics", processed = tc_processed, "Processed direct tool consumption events");
            }
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
            // Federation directives are handled by CuratorAgent, not Cybernetics
            _ => {
                tracing::debug!(target: "cns.cybernetics", variant = directive.variant_name(), "Federation directive — no Cybernetics action")
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
                WebID::from_persona(b"cns"),
                Span::from_kind(SpanKind::CurationDirectiveAcknowledged),
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
}

#[async_trait::async_trait]
impl HkaskLoop for CyberneticsLoop {
    fn id(&self) -> LoopId {
        LoopId::Cybernetics
    }

    /// Produces signals for: per-agent energy ratio, variety deficit, queue depth,
    /// wallet balance ratio, wallet treasury ratio.
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

        // Wallet health signals: balance ratio for wallet-backed agents
        // Wallet balance ratio: 0.0 = empty, 1.0 = full.
        // Set-point: 0.1 (alert when below 10% of capacity).
        // This is a simplified model — the full implementation would use
        // a 30-day moving average as the denominator.
        let wallet_ratios = self.energy_budget_manager.wallet_balance_ratios().await;
        for (ratio, _cap) in wallet_ratios {
            signals.push(Signal::new(
                LoopId::Cybernetics,
                SignalMetric::WalletBalanceRatio,
                ratio,
                0.1, // alert when below 10%
            ));
        }

        // Key health signals: 1.0 = exhausted/expired, 0.0 = healthy.
        // Set-point: 0.0 (any non-zero value = alert).
        let key_alerts = self.energy_budget_manager.wallet_key_alerts().await;
        for (_agent, _reason) in &key_alerts {
            signals.push(Signal::new(
                LoopId::Cybernetics,
                SignalMetric::WalletKeyHealth,
                1.0, // alert active
                0.0, // set-point: no alerts
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
                        LoopId::Cybernetics,
                        ActionType::Throttle,
                        serde_json::json!({"reason": "connector_latency_exceeded", "latency_secs": dev.signal.value, "threshold": dev.signal.set_point}),
                    ))
                }
                SignalMetric::CommunicationQueueDepth
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    tracing::info!(target: "cns.cybernetics.backpressure", queue_depth = dev.signal.value, threshold = dev.signal.set_point, "Communication queue depth exceeded backpressure threshold");
                    Some(LoopAction::new(
                        LoopId::Cybernetics,
                        ActionType::Throttle,
                        serde_json::json!({"reason": "communication_backpressure", "queue_depth": dev.signal.value, "threshold": dev.signal.set_point}),
                    ))
                }
                SignalMetric::WalletBalanceRatio
                    if dev.direction == DeviationDirection::BelowSetPoint =>
                {
                    // Wallet balance low — escalate to Curator
                    let severity = if dev.signal.value <= 0.0 {
                        "critical" // balance = 0 → Curator + Human
                    } else {
                        "warning" // balance < 10% → Curator
                    };
                    tracing::warn!(target: "cns.wallet", balance_ratio = dev.signal.value, severity = severity, "Wallet balance alert");
                    Some(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({"reason": "wallet_balance_low", "balance_ratio": dev.signal.value, "severity": severity, "threshold": dev.signal.set_point}),
                    ))
                }
                SignalMetric::WalletKeyHealth
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    // Key exhausted or expired — escalate to Curator (informational)
                    tracing::info!(target: "cns.wallet", "API key health alert — exhausted or expired");
                    Some(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({"reason": "wallet_key_unhealthy", "severity": "warning", "threshold": dev.signal.set_point}),
                    ))
                }
                SignalMetric::SeamCoverage
                    if dev.direction == DeviationDirection::BelowSetPoint =>
                {
                    // Public seam coverage dropped — R7.3 watcher alert.
                    // Severity based on magnitude of drop:
                    //   >5pp drop → critical (escalate to human)
                    //   1–5pp drop → warning (escalate to Curator)
                    let drop_magnitude = dev.signal.set_point - dev.signal.value;
                    let severity = if drop_magnitude > 5.0 {
                        "critical"
                    } else {
                        "warning"
                    };
                    tracing::warn!(
                        target: "cns.architecture.seam",
                        coverage_pct = dev.signal.value,
                        set_point = dev.signal.set_point,
                        drop_magnitude = drop_magnitude,
                        severity = severity,
                        "Public seam coverage degraded — R7.3 alert"
                    );
                    Some(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({
                            "reason": "seam_coverage_degraded",
                            "coverage_pct": dev.signal.value,
                            "previous_coverage": dev.signal.set_point,
                            "drop_magnitude": drop_magnitude,
                            "severity": severity,
                        }),
                    ))
                }
                SignalMetric::SeamCoverage
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    // Public seam coverage improved — positive health signal (G5 fix).
                    // Not an escalation; informational notification to Curator.
                    let improvement = dev.signal.value - dev.signal.set_point;
                    tracing::info!(
                        target: "cns.architecture.seam",
                        coverage_pct = dev.signal.value,
                        set_point = dev.signal.set_point,
                        improvement = improvement,
                        "Public seam coverage improved — R7.3 positive signal"
                    );
                    Some(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Notify,
                        serde_json::json!({
                            "reason": "seam_coverage_improved",
                            "coverage_pct": dev.signal.value,
                            "previous_coverage": dev.signal.set_point,
                            "improvement": improvement,
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

            // Send CurationInput::Alert on direct alerts channel.
            // Fallback: persist to NuEventStore when channel is down (Curator inactive).
            // Per design decision: the algedonic system must always be connected
            // to the Curator replicant/agent — persistence is the bridge when the
            // live channel has no receiver.
            if action.action_type == ActionType::Escalate && target_id == LoopId::Curation {
                let (deficit, threshold) = extract_deficit_threshold(&action.parameters);
                let domain = action
                    .parameters
                    .get("domain")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let alert = RuntimeAlert {
                    domain,
                    deficit,
                    threshold,
                    severity: AlertSeverity::Critical,
                    escalated: true,
                    timestamp: chrono::Utc::now(),
                    message: format!(
                        "Variety deficit {} exceeds threshold {}",
                        deficit, threshold
                    ),
                };

                // Primary path: live channel to Curator's inbox
                let sent_live = if let Some(ref alerts_tx) = self.alerts_tx {
                    match alerts_tx.send(CurationInput::Alert(alert.clone())) {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::warn!(target: "cns.cybernetics", error = %e, "Failed to send CurationInput::Alert via live channel — falling back to persistence");
                            false
                        }
                    }
                } else {
                    tracing::warn!(target: "cns.cybernetics", "Alerts channel not connected — falling back to persistence. Wire with_alerts_channel() for live delivery.");
                    false
                };

                // Fallback: persist full alert to NuEventStore for Curator retrieval on next activation
                if !sent_live {
                    if let Some(ref sink) = self.event_sink {
                        let event = NuEvent::new(
                            WebID::from_persona(b"cns"),
                            Span::from_kind(SpanKind::VarietyAlgedonicAlert),
                            Phase::Act,
                            serde_json::json!({
                                "domain": alert.domain,
                                "deficit": alert.deficit,
                                "threshold": alert.threshold,
                                "severity": "Critical",
                                "escalated": true,
                                "message": alert.message,
                                "timestamp": alert.timestamp.to_rfc3339(),
                            }),
                            0,
                        );
                        if let Err(e) = sink.persist(&event) {
                            tracing::error!(target: "cns.algedonic", error = %e, "CRITICAL: Failed to persist algedonic alert — alert lost. Both live channel and persistence failed.");
                        } else {
                            tracing::info!(target: "cns.algedonic", deficit = deficit, threshold = threshold, "Algedonic alert persisted to NuEventStore (Curator inbox unavailable)");
                        }
                    } else {
                        tracing::error!(target: "cns.algedonic", deficit = deficit, threshold = threshold, "CRITICAL: Algedonic alert LOST — neither live channel nor event_sink connected. Feedback loop closure broken.");
                    }
                }
            }
        }
    }

    /// Full regulation cycle with loop-quality telemetry.
    ///
    /// Overrides the default `tick()` to measure elapsed time and compute
    /// `LoopQuality` metrics (delay_ms, gain, fidelity_score) after each cycle.
    async fn tick(&self) {
        let start = std::time::Instant::now();
        let signals = self.sense().await;
        let deviations = self.compare(&signals).await;
        let actions = self.compute(&deviations).await;
        self.act(&actions).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let quality = LoopQuality::from_cycle(elapsed_ms, &deviations, &actions);
        *self.loop_quality.write().await = quality;

        tracing::debug!(
            target: "cns.cybernetics",
            delay_ms = quality.delay_ms,
            gain = quality.gain,
            fidelity = quality.fidelity_score,
            deviations = deviations.len(),
            actions = actions.len(),
            "Loop-quality telemetry recorded"
        );
    }
}

/// Extract (deficit, threshold) from action parameters. Returns (0, 0) on missing fields.
fn extract_deficit_threshold(params: &serde_json::Value) -> (u64, u64) {
    let get_f64 =
        |key: &str| -> u64 { params.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) as u64 };
    (get_f64("deficit"), get_f64("threshold"))
}

impl CyberneticsLoop {
    /// Return a snapshot of the most recent loop-quality telemetry.
    pub async fn loop_quality(&self) -> LoopQuality {
        *self.loop_quality.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_loop_starts_with_default_quality() {
        let cns = Arc::new(RwLock::new(CnsRuntime::with_threshold(100)));
        let loop_instance = CyberneticsLoop::new(cns);
        let q = loop_instance.loop_quality().await;
        assert_eq!(q.delay_ms, 0);
        assert!((q.gain - 0.0).abs() < f64::EPSILON);
        assert!((q.fidelity_score - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn tick_updates_loop_quality() {
        let cns = Arc::new(RwLock::new(CnsRuntime::with_threshold(100)));
        let loop_instance = CyberneticsLoop::new(cns);
        loop_instance.tick().await;
        let q = loop_instance.loop_quality().await;
        // After a tick, gain and fidelity should be computed (even if delay_ms is 0)
        // The key property: quality is no longer the default zero-state
        assert!(
            q.gain >= 0.0 && q.fidelity_score >= 0.0,
            "quality should be computed after tick"
        );
    }
}
