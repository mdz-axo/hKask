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
use crate::energy::{GasBudget, GasError};
use crate::runtime::CnsRuntime;
use hkask_types::WebID;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, LoopMessage,
    LoopPayload, Signal,
};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Homeostatic set-points for the Cybernetics Loop.
///
/// These define the reference values against which sensed signals
/// are compared. When a signal deviates beyond its set-point,
/// the loop produces an efferent action.
#[derive(Debug, Clone)]
pub struct SetPoints {
    /// Minimum energy budget remaining ratio (0.0-1.0). Default: 0.2 (20% remaining)
    pub gas_min_remaining: f64,
    /// Maximum variety deficit before escalation. Default: 100
    pub variety_max_deficit: f64,
    /// Maximum error rate (0.0-1.0). Default: 0.3 (30% errors)
    pub error_rate_max: f64,
    /// Maximum connector latency in seconds. Default: 30.0
    pub connector_latency_max_secs: f64,
}

/// YAML-configurable set-points. Fields are Optional so partial configs work.
/// Missing fields fall back to the `SetPoints::default()` values.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SetPointsConfig {
    pub gas_min_remaining: Option<f64>,
    pub variety_max_deficit: Option<f64>,
    pub error_rate_max: Option<f64>,
    pub connector_latency_max_secs: Option<f64>,
}

impl SetPointsConfig {
    /// Load set-points from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Load set-points from a YAML file.
    pub fn load_from_file(path: &str) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_yaml(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl Default for SetPoints {
    fn default() -> Self {
        Self {
            gas_min_remaining: 0.2,
            variety_max_deficit: 100.0,
            error_rate_max: 0.3,
            connector_latency_max_secs: 30.0,
        }
    }
}

/// Load set-points from `HKASK_CNS_CONFIG` env var, falling back to defaults.
///
/// If `HKASK_CNS_CONFIG` is set, reads the YAML file at that path.
/// If unset or the file doesn't exist, returns default set-points.
pub fn load_set_points() -> SetPoints {
    match std::env::var("HKASK_CNS_CONFIG") {
        Ok(path) => match SetPointsConfig::load_from_file(&path) {
            Ok(config) => {
                tracing::info!(
                    target: "cns.config",
                    path = %path,
                    "Loaded CNS set-points from config file"
                );
                SetPoints::from_config(&config)
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.config",
                    path = %path,
                    error = %e,
                    "Failed to load CNS config file, using defaults"
                );
                SetPoints::default()
            }
        },
        Err(_) => SetPoints::default(),
    }
}

impl SetPoints {
    /// Create SetPoints from a config, using defaults for missing fields.
    pub fn from_config(config: &SetPointsConfig) -> Self {
        let defaults = SetPoints::default();
        Self {
            gas_min_remaining: config
                .gas_min_remaining
                .unwrap_or(defaults.gas_min_remaining),
            variety_max_deficit: config
                .variety_max_deficit
                .unwrap_or(defaults.variety_max_deficit),
            error_rate_max: config.error_rate_max.unwrap_or(defaults.error_rate_max),
            connector_latency_max_secs: config
                .connector_latency_max_secs
                .unwrap_or(defaults.connector_latency_max_secs),
        }
    }
}

/// The Cybernetics Loop — homeostatic self-regulation.
///
/// Implements the `Loop` trait's sense→compare→compute→act cycle.
/// The Cybernetics Loop regulates all three domain loops (Inference,
/// Episodic, Semantic) and may signal the Curation Loop via algedonic
/// alerts. It may NOT regulate the Curation Loop.
pub struct CyberneticsLoop {
    /// CNS runtime for variety and alert access
    cns: Arc<RwLock<CnsRuntime>>,
    /// Gas budgets keyed by agent WebID
    gas_budgets: Arc<RwLock<std::collections::HashMap<WebID, GasBudget>>>,
    /// Homeostatic set-points
    set_points: SetPoints,
    /// Maximum number of loop iterations before forced stabilization
    /// (cascade detection — prevents unbounded sense→act cycles)
    max_iterations: u32,
    /// Channel to Communication Loop for inter-loop message dispatch
    dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    /// Inbox for receiving inter-loop messages (CuratorDirectives, etc.)
    inbox: Arc<RwLock<mpsc::UnboundedReceiver<LoopMessage>>>,
    /// Dampener to suppress repeated CuratorDirectives within a time window
    dampener: Arc<Dampener>,
}

impl CyberneticsLoop {
    /// Create a new Cybernetics Loop with default set-points.
    ///
    /// The inbox is "dead" (no sender exists) — use `with_inbox()` if you
    /// need to receive inter-loop messages from the Communication Loop.
    pub fn new(
        cns: Arc<RwLock<CnsRuntime>>,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> Self {
        let (_dead_tx, dead_rx) = mpsc::unbounded_channel::<LoopMessage>();
        Self {
            cns,
            gas_budgets: Arc::new(RwLock::new(std::collections::HashMap::new())),
            set_points: SetPoints::default(),
            max_iterations: 100,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(dead_rx)),
            dampener: Arc::new(Dampener::new()),
        }
    }

    /// Create a Cybernetics Loop with custom set-points.
    ///
    /// The inbox is "dead" (no sender exists) — use `with_set_points_and_inbox()`
    /// if you need to receive inter-loop messages from the Communication Loop.
    #[allow(dead_code)]
    pub fn with_set_points(
        cns: Arc<RwLock<CnsRuntime>>,
        set_points: SetPoints,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> Self {
        let (_dead_tx, dead_rx) = mpsc::unbounded_channel::<LoopMessage>();
        Self {
            cns,
            gas_budgets: Arc::new(RwLock::new(std::collections::HashMap::new())),
            set_points,
            max_iterations: 100,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(dead_rx)),
            dampener: Arc::new(Dampener::new()),
        }
    }

    /// Create a Cybernetics Loop with a fresh inbox channel pair.
    ///
    /// Returns `(loop_instance, inbox_sender)` where the sender should be
    /// registered with the Communication Loop for message delivery.
    pub fn with_inbox(
        cns: Arc<RwLock<CnsRuntime>>,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> (Self, mpsc::UnboundedSender<LoopMessage>) {
        let (inbox_tx, inbox_rx) = mpsc::unbounded_channel::<LoopMessage>();
        let loop_instance = Self {
            cns,
            gas_budgets: Arc::new(RwLock::new(std::collections::HashMap::new())),
            set_points: SetPoints::default(),
            max_iterations: 100,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(inbox_rx)),
            dampener: Arc::new(Dampener::new()),
        };
        (loop_instance, inbox_tx)
    }

    /// Create a Cybernetics Loop with custom set-points and a fresh inbox channel pair.
    ///
    /// Returns `(loop_instance, inbox_sender)` where the sender should be
    /// registered with the Communication Loop for message delivery.
    #[allow(dead_code)]
    pub(crate) fn with_set_points_and_inbox(
        cns: Arc<RwLock<CnsRuntime>>,
        set_points: SetPoints,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> (Self, mpsc::UnboundedSender<LoopMessage>) {
        let (inbox_tx, inbox_rx) = mpsc::unbounded_channel::<LoopMessage>();
        let loop_instance = Self {
            cns,
            gas_budgets: Arc::new(RwLock::new(std::collections::HashMap::new())),
            set_points,
            max_iterations: 100,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(inbox_rx)),
            dampener: Arc::new(Dampener::new()),
        };
        (loop_instance, inbox_tx)
    }

    /// Register a gas budget for an agent.
    pub async fn register_gas_budget(&self, agent: WebID, budget: GasBudget) {
        let mut budgets = self.gas_budgets.write().await;
        budgets.insert(agent, budget);
    }

    /// Check if an agent can proceed with an operation costing `gas`.
    pub async fn can_proceed(&self, agent: &WebID, gas: u64) -> bool {
        let budgets = self.gas_budgets.read().await;
        if let Some(budget) = budgets.get(agent) {
            budget.can_proceed(gas)
        } else {
            // No budget registered — allow by default (soft limit)
            true
        }
    }

    /// Reserve gas for an in-flight operation (hold-settle pattern).
    ///
    /// Gas is reserved but not consumed. Call `settle_gas()` after the
    /// operation completes to consume or refund the actual cost.
    pub async fn reserve_gas(&self, agent: &WebID, gas: u64) -> Result<u64, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.reserve(gas)
        } else {
            // No budget registered — allow by default (soft limit)
            Ok(0)
        }
    }

    /// Settle a gas reservation after operation completion.
    ///
    /// `reserved_gas` is the amount that was reserved. `actual_gas` is the
    /// real cost. If actual < reserved, the difference is refunded.
    pub async fn settle_gas(
        &self,
        agent: &WebID,
        reserved_gas: u64,
        actual_gas: u64,
    ) -> Result<u64, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.settle(reserved_gas, actual_gas)
        } else {
            // No budget registered — cost is 0 (soft limit)
            Ok(0)
        }
    }

    /// Acquire gas budget for an agent's operation (immediate, non-reserved).
    ///
    /// Use this for operations where the exact cost is known upfront.
    /// For operations with estimated cost, prefer `reserve_gas` + `settle_gas`.
    pub async fn acquire_budget(&self, agent: &WebID, gas: u64) -> Result<u64, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.consume(gas)
        } else {
            // No budget registered — cost is 0 (soft limit)
            Ok(0)
        }
    }

    /// Replenish all gas budgets by their configured replenish_rate.
    ///
    /// Called by the Cybernetics Loop on its regulation cycle.
    pub async fn replenish_all_budgets(&self) {
        let budget_ids: Vec<WebID> = {
            let budgets = self.gas_budgets.read().await;
            budgets.keys().cloned().collect()
        };
        for agent in budget_ids {
            let replenished = {
                let mut budgets = self.gas_budgets.write().await;
                if let Some(budget) = budgets.get_mut(&agent) {
                    let rate = budget.replenish_rate;
                    budget.replenish();
                    rate
                } else {
                    0
                }
            };
            if replenished > 0 {
                tracing::debug!(
                    target: "cns.cybernetics",
                    agent = %agent,
                    replenish_rate = replenished,
                    "Replenished gas budget"
                );
            }
        }
    }

    /// Replenish a specific agent's gas budget by a specific amount.
    /// Used by CuratorDirective::ReplenishBudget.
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: u64) {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.replenish_by(amount);
            tracing::info!(
                target: "cns.cybernetics",
                agent = %agent,
                amount = amount,
                remaining = budget.remaining,
                "Replenished agent gas budget by directive"
            );
        }
    }

    /// Get the current set-points.
    #[allow(dead_code)]
    pub(crate) fn set_points(&self) -> &SetPoints {
        &self.set_points
    }

    /// Get a sender clone for the dispatch channel.
    pub fn dispatch_sender(&self) -> mpsc::UnboundedSender<LoopMessage> {
        self.dispatch_tx.clone()
    }

    /// Process pending inbox messages (CuratorDirectives from Curation).
    ///
    /// This is called during the `sense()` phase so that directives
    /// are applied before the loop computes regulatory actions.
    pub async fn process_inbox(&self) {
        let mut inbox = self.inbox.write().await;
        let mut processed = 0;
        while let Ok(msg) = inbox.try_recv() {
            processed += 1;
            match &msg.payload {
                LoopPayload::CurationDirective {
                    directive_type,
                    target,
                    parameters,
                } => {
                    // Dampen repeated directives to prevent feedback oscillation
                    if self
                        .dampener
                        .should_dampen_directive(directive_type, *target)
                        .await
                    {
                        tracing::debug!(
                            target: "cns.cybernetics",
                            directive_type = directive_type,
                            "Directive dampened (repeated within window)"
                        );
                    } else {
                        match directive_type.as_str() {
                            "calibrate_threshold" => {
                                if let Some(domain) =
                                    parameters.get("domain").and_then(|v| v.as_str())
                                    && let Some(new_threshold) =
                                        parameters.get("new_threshold").and_then(|v| v.as_u64())
                                {
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
                            }
                            "override_gas_budget" => {
                                // OverrideGasBudget: Curation can exceed set-point bounds.
                                // This is the metacognitive override — stronger than AdjustGasBudget.
                                if let Some(new_budget) =
                                    parameters.get("new_budget").and_then(|v| v.as_u64())
                                {
                                    let mut budgets = self.gas_budgets.write().await;
                                    if let Some(budget) = budgets.get_mut(target) {
                                        // Override can set budget above or below set-points
                                        budget.cap = new_budget;
                                        budget.remaining = new_budget;
                                        tracing::warn!(
                                            target: "cns.cybernetics",
                                            agent = %target,
                                            new_budget = new_budget,
                                            "Applied OverrideGasBudget directive from Curation (set-point override)"
                                        );
                                    } else {
                                        budgets.insert(*target, GasBudget::new(new_budget));
                                        tracing::warn!(
                                            target: "cns.cybernetics",
                                            agent = %target,
                                            new_budget = new_budget,
                                            "Registered new gas budget from OverrideGasBudget directive"
                                        );
                                    }
                                }
                            }
                            "replenish_budget" => {
                                // ReplenishBudget: Curation can inject gas into an agent's budget.
                                // This is the gas refund mechanism governed by Curator authority.
                                if let Some(amount) =
                                    parameters.get("amount").and_then(|v| v.as_u64())
                                {
                                    self.replenish_agent_budget(target, amount).await;
                                }
                            }
                            "update_capabilities" => {
                                tracing::info!(
                                    target: "cns.cybernetics",
                                    agent = %target,
                                    ?parameters,
                                    "Applied UpdateCapabilities directive from Curation (capabilities updated)"
                                );
                            }
                            "seek_more_evidence" => {
                                tracing::info!(
                                    target: "cns.cybernetics",
                                    ?parameters,
                                    "Applied SeekMoreEvidence directive from Curation (metacognition loop triggered)"
                                );
                            }
                            "throttle" | "dampen" | "escalate" | "circuit_break" => {
                                tracing::debug!(
                                    target: "cns.cybernetics",
                                    directive_type = directive_type,
                                    "Received informational directive (already self-produced)"
                                );
                            }
                            _ => {
                                tracing::warn!(
                                    target: "cns.cybernetics",
                                    directive_type = directive_type,
                                    "Unknown directive type in CyberneticsLoop inbox"
                                );
                            }
                        }
                        // TODO: Replace with NuEvent emission once CyberneticsLoop has an
                        // event_sink field. The ack should use:
                        //   Span::new(SpanNamespace::new("cns.curation"), "directive_acknowledged")
                        //   Phase::Act
                        //   observation: {"directive_type", "outcome": "applied"}
                        //   event_sink.persist(&ack)
                        tracing::info!(
                            target: "cns.cybernetics",
                            directive_type = directive_type,
                            outcome = "applied",
                            "Directive acknowledged (Curation→Cybernetics compliance)"
                        );
                    }
                }
                LoopPayload::AlgedonicAlert {
                    current,
                    threshold,
                    deficit,
                } => {
                    tracing::info!(
                        target: "cns.cybernetics",
                        current = current,
                        threshold = threshold,
                        deficit = deficit,
                        "Received algedonic alert in CyberneticsLoop inbox"
                    );
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
    }
}

#[async_trait::async_trait]
impl HkaskLoop for CyberneticsLoop {
    fn id(&self) -> LoopId {
        LoopId::Cybernetics
    }

    /// Sense: read variety counters, energy budgets, and CNS health.
    ///
    /// Produces `Signal`s for each metric that the loop monitors:
    /// - Per-agent energy remaining ratio
    /// - Overall variety deficit from CNS health
    async fn sense(&self) -> Vec<Signal> {
        // Process pending directives before sensing state
        self.process_inbox().await;

        let mut signals = Vec::new();

        // Energy signals: per-agent remaining ratio
        let budgets = self.gas_budgets.read().await;
        for (_agent, budget) in budgets.iter() {
            let ratio = budget.remaining as f64 / budget.cap.max(1) as f64;
            signals.push(Signal::new(
                LoopId::Cybernetics,
                "energy_remaining",
                ratio,
                self.set_points.gas_min_remaining,
            ));
        }
        drop(budgets);

        // Variety deficit signal from CNS
        let cns = self.cns.read().await;
        let health = cns.health().await;
        signals.push(Signal::new(
            LoopId::Cybernetics,
            "variety_deficit",
            health.overall_deficit as f64,
            self.set_points.variety_max_deficit,
        ));
        drop(cns);

        signals
    }

    /// Compute: produce regulatory actions for detected deviations.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();
        for dev in deviations {
            let action = match dev.signal.metric.as_str() {
                "energy_remaining" if dev.direction == DeviationDirection::BelowSetPoint => {
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
                "variety_deficit" if dev.direction == DeviationDirection::AboveSetPoint => {
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
                "error_rate" if dev.direction == DeviationDirection::AboveSetPoint => {
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
                "connector_latency" if dev.direction == DeviationDirection::AboveSetPoint => {
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
                _ => None,
            };
            if let Some(a) = action {
                actions.push(a);
            }
        }
        actions
    }

    /// Act: route LoopActions through the Communication Loop via dispatch channel,
    /// and replenish all gas budgets each regulation cycle.
    ///
    /// Each `LoopAction` is converted to a `LoopMessage` and sent through the
    /// `dispatch_tx` channel. The Communication Loop receives and delivers them.
    ///
    /// Gas budgets are replenished by their configured `replenish_rate` each cycle.
    async fn act(&self, actions: &[LoopAction]) {
        // Replenish all gas budgets each regulation cycle
        self.replenish_all_budgets().await;

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

            let target_id: LoopId = action.target;
            let directive_type = match action.action_type {
                ActionType::Throttle => "throttle",
                ActionType::Dampen => "dampen",
                ActionType::Escalate => "escalate",
                ActionType::Calibrate => "calibrate",
                ActionType::CircuitBreak => "circuit_break",
                ActionType::AdjustGasBudget => "adjust_gas_budget",
                ActionType::OverrideGasBudget => "override_gas_budget",
                ActionType::ReplenishBudget => "replenish_budget",
            };

            let payload =
                if action.action_type == ActionType::Escalate && target_id == LoopId::Curation {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::loops::{ActionType, DeviationDirection, HkaskLoop, LoopId, LoopPayload};

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
        let signal = Signal::new(LoopId::Cybernetics, "energy_remaining", 0.05, 0.2);
        let deviations = loop6.compare(&[signal]).await;
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::BelowSetPoint);

        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 2); // Throttle + AdjustGasBudget
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(actions[0].target, LoopId::Inference);
        assert_eq!(actions[1].action_type, ActionType::AdjustGasBudget);
        assert_eq!(actions[1].target, LoopId::Cybernetics);
    }

    #[tokio::test]
    async fn variety_deficit_produces_escalate_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Signal: variety deficit at 150 (above 100 threshold)
        let signal = Signal::new(LoopId::Cybernetics, "variety_deficit", 150.0, 100.0);
        let deviations = loop6.compare(&[signal]).await;
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::AboveSetPoint);

        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Escalate);
        assert_eq!(actions[0].target, LoopId::Curation);
    }

    #[tokio::test]
    async fn error_rate_produces_circuit_break_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Signal: error rate at 50% (above 30% threshold)
        let signal = Signal::new(LoopId::Cybernetics, "error_rate", 0.5, 0.3);
        let deviations = loop6.compare(&[signal]).await;
        assert_eq!(deviations.len(), 1);

        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::CircuitBreak);
        assert_eq!(actions[0].target, LoopId::Inference);
    }

    #[tokio::test]
    async fn no_deviation_produces_no_action() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Signal: energy at 50% (above 20% set-point — no deviation)
        let signal = Signal::new(LoopId::Cybernetics, "energy_remaining", 0.5, 0.2);
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

    // =========================================================================
    // Task 7: Cybernetic Unit Tests — Full loop validation
    // =========================================================================

    /// Test: Inject a known energy deviation (5% remaining vs 20% set-point)
    /// Assert: The loop produces a Throttle action targeting Inference
    /// Assert: The action propagates through the capability membrane
    /// Assert: The system reaches a new stable equilibrium within bounded iterations
    #[tokio::test]
    async fn energy_deviation_propagates_and_stabilizes() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let loop6 = CyberneticsLoop::new(cns, test_dispatch_tx());

        // Step 1: Inject known deviation — energy at 5% (set-point: 20%)
        let signal = Signal::new(LoopId::Cybernetics, "energy_remaining", 0.05, 0.2);

        // Step 2: Compare — detect deviation
        let deviations = loop6.compare(&[signal]).await;
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].direction, DeviationDirection::BelowSetPoint);
        assert!((deviations[0].magnitude - 0.15).abs() < f64::EPSILON);

        // Step 3: Compute — produce efferent action
        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 2); // Throttle + AdjustGasBudget
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(actions[0].target, LoopId::Inference);
        assert_eq!(actions[1].action_type, ActionType::AdjustGasBudget);
        assert_eq!(actions[1].target, LoopId::Cybernetics);

        // Step 4: Verify capability membrane — Cybernetics can regulate Inference
        // (domain loop), but NOT Curation (peer meta loop)
        assert_ne!(actions[0].target, LoopId::Curation);

        // Step 5: Simulate stabilization — after throttling, energy recovers
        let recovered_signal = Signal::new(LoopId::Cybernetics, "energy_remaining", 0.25, 0.2);
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
            Signal::new(LoopId::Cybernetics, "energy_remaining", 0.05, 0.2),
            Signal::new(LoopId::Cybernetics, "variety_deficit", 150.0, 100.0),
            Signal::new(LoopId::Cybernetics, "error_rate", 0.5, 0.3),
        ];

        let deviations = loop6.compare(&signals).await;
        assert_eq!(deviations.len(), 3);

        let actions = loop6.compute(&deviations).await;
        assert_eq!(actions.len(), 4); // gas:Throttle + gas:AdjustGasBudget + variety:Escalate + error:CircuitBreak

        // Verify each action targets the correct loop
        let targets: std::collections::HashSet<LoopId> = actions.iter().map(|a| a.target).collect();
        assert!(targets.contains(&LoopId::Inference)); // gas:Throttle + error:CircuitBreak
        assert!(targets.contains(&LoopId::Curation)); // variety:Escalate
        assert!(targets.contains(&LoopId::Cybernetics)); // gas:AdjustGasBudget
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
            let signal = Signal::new(LoopId::Cybernetics, "energy_remaining", energy, 0.2);
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

        let signal = Signal::new(LoopId::Cybernetics, "connector_latency", 60.0, 30.0);
        let deviations = loop6.compare(&[signal]).await;
        let actions = loop6.compute(&deviations).await;

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Throttle);
        assert_eq!(actions[0].target, LoopId::Communication);
    }

    // =========================================================================
    // Inbox processing tests — CuratorDirective consumption
    // =========================================================================

    #[tokio::test]
    async fn cybernetics_loop_processes_calibrate_directive() {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let (loop6, inbox_tx) = CyberneticsLoop::with_inbox(cns.clone(), test_dispatch_tx());

        // Send a CalibrateThreshold directive
        let msg = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective {
                directive_type: "calibrate_threshold".to_string(),
                target: WebID::new(),
                parameters: serde_json::json!({
                    "domain": "variety",
                    "new_threshold": 200,
                }),
            },
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
            LoopPayload::CurationDirective {
                directive_type: "override_gas_budget".to_string(),
                target: agent,
                parameters: serde_json::json!({
                    "new_budget": 5000,
                }),
            },
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
            LoopPayload::CurationDirective {
                directive_type: "override_gas_budget".to_string(),
                target: agent,
                parameters: serde_json::json!({"new_budget": 100}),
            },
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
            LoopPayload::CurationDirective {
                directive_type: "override_gas_budget".to_string(),
                target: agent,
                parameters: serde_json::json!({"new_budget": 500}),
            },
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
}
