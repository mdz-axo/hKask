//! Inference Loop — prompt → context → model → response → parse → act (Loop 1)
//!
//! Monitors circuit breaker state and inference availability.
//! Lives in the CNS crate because loop orchestration (sense → compare → compute → act)
//! is a Cybernetics concern — domain crates provide port implementations,
//! the CNS provides loop governance.

use hkask_types::WebID;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal,
};
use hkask_types::ports::{CircuitBreakerPort, InferencePort};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cybernetics_loop::CyberneticsLoop;
use crate::governed_inference::GovernedInference;

/// Inference Loop — monitors circuit breaker and inference availability.
///
/// Wraps an `InferencePort` and optional `CircuitBreakerPort` to provide
/// loop-level observability. When the circuit breaker is open, the loop
/// produces `Throttle` actions targeting itself (self-throttle).
pub struct InferenceLoop {
    inference: Arc<dyn InferencePort>,
    circuit_breaker: Option<Arc<dyn CircuitBreakerPort>>,
}

impl InferenceLoop {
    /// Create a new Inference Loop wrapping an inference port.
    pub fn new(inference: Arc<dyn InferencePort>) -> Self {
        Self {
            inference,
            circuit_breaker: None,
        }
    }

    /// Create an Inference Loop with a circuit breaker.
    pub fn with_circuit_breaker(
        inference: Arc<dyn InferencePort>,
        circuit_breaker: Arc<dyn CircuitBreakerPort>,
    ) -> Self {
        Self {
            inference,
            circuit_breaker: Some(circuit_breaker),
        }
    }

    /// Create an Inference Loop governed by Cybernetics.
    ///
    /// This wraps the inference port with energy budget enforcement
    /// before creating the loop. The returned loop uses the governed
    /// port, so every `generate()` call passes through budget checks.
    pub fn governed(
        inference: Arc<dyn InferencePort>,
        cybernetics: Arc<RwLock<CyberneticsLoop>>,
        agent: WebID,
    ) -> Self {
        let governed = Arc::new(GovernedInference::new(inference, cybernetics, agent));
        Self {
            inference: governed,
            circuit_breaker: None,
        }
    }

    /// Create a governed Inference Loop with a circuit breaker.
    pub fn governed_with_circuit_breaker(
        inference: Arc<dyn InferencePort>,
        cybernetics: Arc<RwLock<CyberneticsLoop>>,
        agent: WebID,
        circuit_breaker: Arc<dyn CircuitBreakerPort>,
    ) -> Self {
        let governed = Arc::new(GovernedInference::new(inference, cybernetics, agent));
        Self {
            inference: governed,
            circuit_breaker: Some(circuit_breaker),
        }
    }

    /// Access the underlying inference port.
    pub fn inference(&self) -> &Arc<dyn InferencePort> {
        &self.inference
    }
}

#[async_trait::async_trait]
impl HkaskLoop for InferenceLoop {
    fn id(&self) -> LoopId {
        LoopId::Inference
    }

    /// Sense: read circuit breaker state and inference availability.
    ///
    /// Produces signals for:
    /// - `circuit_breaker_state` — 0.0=closed, 1.0=open, 0.5=half-open (set_point 0.0)
    /// - `inference_available` — 1.0 if circuit breaker allows, 0.0 if not (set_point 1.0)
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

        vec![
            Signal::new(LoopId::Inference, "circuit_breaker_state", cb_state, 0.0),
            Signal::new(LoopId::Inference, "inference_available", available, 1.0),
        ]
    }

    /// Compute: if circuit breaker is open, produce self-throttle action.
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
                _ => {}
            }
        }

        actions
    }

    /// Act: log regulatory actions.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            tracing::info!(
                target: "cns.inference",
                action_type = ?action.action_type,
                target_loop = %action.target,
                "Inference Loop regulatory action"
            );
        }
    }
}
