//! Inference Loop — prompt → context → model → response → parse → act (Loop 1)
//!
//! Owns its gas budget reservation and tracks the active inference model.
//! Monitors circuit breaker state, gas consumption, and model availability.
//! Lives in `hkask-agents` because domain loops (Inference, Episodic, Semantic,
//! Communication, Curation) are domain logic — they belong with the agents crate.
//! Governance is applied externally via `GovernedTool` (in `hkask-cns`) before
//! the port is passed to this loop.

use hkask_types::WebID;
use hkask_types::loops::dispatch::{LoopMessage, LoopPayload};
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal,
};
use hkask_types::ports::{CircuitBreakerPort, InferencePort};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Gas budget set-point: when gas remaining drops below this ratio,
/// the loop self-throttles via `AdjustGasBudget`.
const GAS_SET_POINT: f64 = 0.2;

/// Inference Loop — owns gas budget and model selection state.
///
/// Wraps an `InferencePort` and optional `CircuitBreakerPort` to provide
/// loop-level observability. Owns its own gas budget reservation (separate
/// from Cybernetics' global tracking) and tracks the active inference model.
///
/// When the circuit breaker is open or gas is depleted, the loop produces
/// `Throttle`/`AdjustGasBudget` actions targeting itself (self-throttle).
/// When the model is unavailable, it produces `Calibrate` to signal that
/// model selection is needed.
pub struct InferenceLoop<I: InferencePort = Arc<dyn InferencePort>> {
    inference: I,
    circuit_breaker: Option<Arc<dyn CircuitBreakerPort>>,
    /// Gas remaining in this loop's own budget (simple atomic counter,
    /// updated by external callers after each inference call).
    gas_remaining: Arc<AtomicU64>,
    /// Gas capacity — the budget cap set at allocation time.
    gas_cap: u64,
    /// Currently active inference model (None = not yet selected / unavailable).
    current_model: Option<String>,
    /// Dispatch channel for routing LoopActions through Communication.
    ///
    /// When set, `act()` converts each `LoopAction` to a `LoopMessage` and sends
    /// it through this channel. The Communication Loop receives and delivers to
    /// the target loop's inbox.
    dispatch_tx: Option<tokio::sync::mpsc::UnboundedSender<LoopMessage>>,
}

impl<I: InferencePort + 'static> InferenceLoop<I> {
    /// Create a new Inference Loop wrapping an inference port.
    pub fn new(inference: I) -> Self {
        Self {
            inference,
            circuit_breaker: None,
            gas_remaining: Arc::new(AtomicU64::new(0)),
            gas_cap: 0,
            current_model: None,
            dispatch_tx: None,
        }
    }

    /// Create an Inference Loop with a circuit breaker.
    pub(crate) fn with_circuit_breaker(
        inference: I,
        circuit_breaker: Arc<dyn CircuitBreakerPort>,
    ) -> Self {
        Self {
            inference,
            circuit_breaker: Some(circuit_breaker),
            gas_remaining: Arc::new(AtomicU64::new(0)),
            gas_cap: 0,
            current_model: None,
            dispatch_tx: None,
        }
    }

    /// Set the gas budget for this loop.
    ///
    /// `cap` is the total gas allocation; `remaining` is the current balance.
    /// Both are stored so that `sense()` can emit the gas-remaining ratio.
    pub fn with_gas_budget(mut self, cap: u64, remaining: u64) -> Self {
        self.gas_cap = cap;
        self.gas_remaining = Arc::new(AtomicU64::new(remaining));
        self
    }

    /// Set the active inference model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.current_model = Some(model.into());
        self
    }

    /// Set the dispatch channel for routing LoopActions through Communication.
    ///
    /// When set, `act()` converts each `LoopAction` to a `LoopMessage` and sends
    /// it through this channel. The Communication Loop receives and delivers to
    /// the target loop's inbox.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_dispatch(mut self, tx: tokio::sync::mpsc::UnboundedSender<LoopMessage>) -> Self {
        self.dispatch_tx = Some(tx);
        self
    }

    /// Access the underlying inference port.
    pub(crate) fn inference(&self) -> &I {
        &self.inference
    }

    /// Get the current gas remaining value (read-only sense signal).
    pub fn gas_remaining(&self) -> u64 {
        self.gas_remaining.load(Ordering::Relaxed)
    }

    /// Read-only accessor for the L1 domain metric.
    ///
    /// Returns `(remaining, cap)` — the loop's token budget state as a
    /// sense signal. The L6 budget (CyberneticsLoop's GasBudget) is the
    /// authoritative regulator; this counter is a read-only mirror.
    pub fn token_usage(&self) -> (u64, u64) {
        (self.gas_remaining.load(Ordering::Relaxed), self.gas_cap)
    }

    /// Consume gas from this loop's budget. Returns the old remaining value.
    ///
    /// `pub(crate)` — external callers should use `sync_gas_state()` to
    /// mirror the authoritative L6 budget, not incrementally adjust this counter.
    pub(crate) fn consume_gas(&self, amount: u64) -> u64 {
        self.gas_remaining.fetch_sub(amount, Ordering::Relaxed)
    }

    /// Replenish gas in this loop's budget. Returns the old remaining value.
    ///
    /// `pub(crate)` — external callers should use `sync_gas_state()` to
    /// mirror the authoritative L6 budget, not incrementally adjust this counter.
    pub(crate) fn replenish_gas(&self, amount: u64) -> u64 {
        self.gas_remaining.fetch_add(amount, Ordering::Relaxed)
    }

    /// Sync this loop's gas counter from the authoritative L6 budget.
    ///
    /// Call after CyberneticsLoop gas operations (reserve, settle, replenish)
    /// to keep the L1 sense signal (`inference_gas_remaining`) in sync with
    /// the L6 regulatory budget. This is the ONLY way external code should
    /// update InferenceLoop's gas counter.
    pub fn sync_gas_state(&self, remaining: u64, _cap: u64) {
        self.gas_remaining.store(remaining, Ordering::Relaxed);
        // Note: gas_cap is not atomic; it's only set at construction time
        // and should not change during a session. If sync provides a different
        // cap, we accept it as the loop is sharing state with CyberneticsLoop.
    }

    /// Get the gas budget cap.
    pub fn gas_cap(&self) -> u64 {
        self.gas_cap
    }

    /// Get the current model name, if set.
    pub(crate) fn current_model(&self) -> Option<&str> {
        self.current_model.as_deref()
    }

    /// Set the current model (e.g., after a model selection event).
    pub(crate) fn set_model(&mut self, model: impl Into<String>) {
        self.current_model = Some(model.into());
    }

    /// Clear the current model (e.g., model became unavailable).
    pub(crate) fn clear_model(&mut self) {
        self.current_model = None;
    }

    // ====================================================================
    // Explicit 4-stage cycle: sense → compare → compute → act
    // ====================================================================

    /// **Sense stage** (sense → compare → compute → act):
    /// Read token budget remaining via `token_usage()`, check circuit breaker
    /// state, and verify model availability. Produces afferent signals for
    /// gas remaining ratio, circuit breaker state, and model availability.
    pub async fn sense(&self) -> Vec<Signal> {
        <Self as HkaskLoop>::sense(self).await
    }

    /// **Compare stage** (sense → compare → compute → act):
    /// Check if remaining gas is below the set-point ratio (0.2), whether
    /// the circuit breaker is open, or whether the model is unavailable.
    /// Detects deviations from healthy operating set-points.
    pub async fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        <Self as HkaskLoop>::compare(self, signals).await
    }

    /// **Compute stage** (sense → compare → compute → act):
    /// Determine model selection / gas allocation based on deviations.
    /// Circuit breaker open or inference unavailable → Throttle. Gas below
    /// set-point → AdjustGasBudget (self-throttle). Model unavailable →
    /// Calibrate (signal model selection needed).
    pub async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        <Self as HkaskLoop>::compute(self, deviations).await
    }

    /// **Act stage** (sense → compare → compute → act):
    /// Execute inference call if budget allows. Logs all regulatory actions
    /// with structured spans. Gas self-throttle and model unavailability
    /// are logged at warn level.
    pub async fn act(&self, actions: &[LoopAction]) {
        <Self as HkaskLoop>::act(self, actions).await
    }
}

#[async_trait::async_trait]
impl<I: InferencePort + 'static> HkaskLoop for InferenceLoop<I> {
    fn id(&self) -> LoopId {
        LoopId::Inference
    }

    /// Sense: read circuit breaker state, inference availability, gas budget, and model state.
    ///
    /// Produces signals for:
    /// - `circuit_breaker_state` — 0.0=closed, 1.0=open, 0.5=half-open (set_point 0.0)
    /// - `inference_available` — 1.0 if circuit breaker allows, 0.0 if not (set_point 1.0)
    /// - `inference_gas_remaining` — ratio of gas remaining in loop's own budget (set_point 0.2)
    /// - `inference_model_available` — 1.0 if model is set, 0.0 if not (set_point 1.0)
    async fn sense(&self) -> Vec<Signal> {
        let (cb_state, available) = match &self.circuit_breaker {
            Some(cb) => {
                let state_value = match cb.state() {
                    hkask_types::CircuitState::Closed => 0.0,
                    hkask_types::CircuitState::Open => 1.0,
                    hkask_types::CircuitState::HalfOpen => 0.5,
                };
                let available = if cb.allow_request() { 1.0 } else { 0.0 };
                (state_value, available)
            }
            None => {
                // No circuit breaker means inference is always available
                (0.0, 1.0)
            }
        };

        let gas_ratio = if self.gas_cap > 0 {
            self.gas_remaining.load(Ordering::Relaxed) as f64 / self.gas_cap as f64
        } else {
            // No budget allocated — report full to avoid spurious throttling
            1.0
        };

        let model_available = if self.current_model.is_some() {
            1.0
        } else {
            0.0
        };

        vec![
            Signal::new(LoopId::Inference, "circuit_breaker_state", cb_state, 0.0),
            Signal::new(LoopId::Inference, "inference_available", available, 1.0),
            Signal::new(
                LoopId::Inference,
                "inference_gas_remaining",
                gas_ratio,
                GAS_SET_POINT,
            ),
            Signal::new(
                LoopId::Inference,
                "inference_model_available",
                model_available,
                1.0,
            ),
        ]
    }

    /// Compute: produce regulatory actions for detected deviations.
    ///
    /// Handles:
    /// - Circuit breaker open → `Throttle`
    /// - Inference unavailable → `Throttle`
    /// - Gas below set-point → `AdjustGasBudget` (self-throttle)
    /// - Model unavailable → `Calibrate` (signal model selection needed)
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            match dev.signal.metric.as_str() {
                "circuit_breaker_state" if dev.direction == DeviationDirection::AboveSetPoint => {
                    actions.push(LoopAction::new(
                        LoopId::Inference,
                        ActionType::Throttle,
                        serde_json::json!({
                            "reason": "circuit_breaker_open",
                            "state": dev.signal.value,
                        }),
                    ));
                }
                "inference_available" if dev.direction == DeviationDirection::BelowSetPoint => {
                    actions.push(LoopAction::new(
                        LoopId::Inference,
                        ActionType::Throttle,
                        serde_json::json!({
                            "reason": "inference_unavailable",
                            "available": dev.signal.value,
                        }),
                    ));
                }
                "inference_gas_remaining" if dev.direction == DeviationDirection::BelowSetPoint => {
                    actions.push(LoopAction::new(
                        LoopId::Inference,
                        ActionType::AdjustGasBudget,
                        serde_json::json!({
                            "reason": "gas_below_set_point",
                            "remaining_ratio": dev.signal.value,
                            "set_point": dev.signal.set_point,
                        }),
                    ));
                }
                "inference_model_available"
                    if dev.direction == DeviationDirection::BelowSetPoint =>
                {
                    actions.push(LoopAction::new(
                        LoopId::Inference,
                        ActionType::Calibrate,
                        serde_json::json!({
                            "reason": "model_unavailable",
                        }),
                    ));
                }
                _ => {}
            }
        }

        actions
    }

    /// Act: execute regulatory actions.
    ///
    /// Logs all actions with structured spans. Gas self-throttle and model
    /// unavailability are logged at warn level to surface budget depletion
    /// and model selection needs.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            match action.action_type {
                ActionType::AdjustGasBudget => {
                    tracing::warn!(
                        target: "cns.inference",
                        action_type = ?action.action_type,
                        target_loop = %action.target,
                        parameters = %action.parameters,
                        "Inference Loop self-throttle: gas budget below set-point"
                    );
                }
                ActionType::Calibrate => {
                    tracing::warn!(
                        target: "cns.inference",
                        action_type = ?action.action_type,
                        target_loop = %action.target,
                        parameters = %action.parameters,
                        "Inference Loop calibrate: model selection needed"
                    );
                }
                _ => {
                    tracing::info!(
                        target: "cns.inference",
                        action_type = ?action.action_type,
                        target_loop = %action.target,
                        "Inference Loop regulatory action"
                    );
                }
            }
        }

        // Route LoopActions through Communication Loop via dispatch channel
        if let Some(ref tx) = self.dispatch_tx {
            for action in actions {
                let payload = LoopPayload::CyberneticsRegulation {
                    regulation_type: match action.action_type {
                        ActionType::Throttle => "throttle",
                        ActionType::Calibrate => "calibrate",
                        ActionType::AdjustGasBudget => "adjust_gas_budget",
                        _ => continue, // Skip non-routable actions
                    }
                    .to_string(),
                    target: WebID::new(),
                    parameters: action.parameters.clone(),
                };
                let msg = LoopMessage::new(action.priority, LoopId::Inference, payload)
                    .with_target(action.target);
                if let Err(e) = tx.send(msg) {
                    tracing::warn!(
                        target: "cns.inference",
                        error = %e,
                        "Failed to dispatch Inference LoopAction"
                    );
                }
            }
        }
    }
}
