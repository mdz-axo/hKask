//! GovernedInference — Energy-budget-gated, observability-emitting membrane for inference.
//!
//! Wraps an `InferencePort` and implements `InferencePort` itself. Before delegating
//! to the inner port, it checks:
//! 1. Budget (Cybernetics) — reserve gas based on estimated token cost
//! 2. Emits span (CNS) — cns.inference.invoked
//! 3. Delegates to inner inference port
//! 4. Settles energy cost (Cybernetics) — settle actual vs. reserved
//! 5. Emits outcome span (CNS) — cns.inference.completed
//!
//! This is the membrane where Cybernetics governs inference calls — the same
//! hold-settle pattern used by GovernedTool for tool invocations.
//!
//! # Token-based cost estimation
//! Inference cost is estimated from `max_tokens` in LLMParameters. The estimator
//! converts max_tokens to gas units (1 token ≈ 1 gas unit by default).

use crate::cybernetics_loop::CyberneticsLoop;
use crate::energy::EnergyCost;
use hkask_types::NuEventSink;
use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, Phase, Span, SpanKind, SpanNamespace};
use hkask_types::ports::{InferenceError, InferencePort, InferenceResult};
use hkask_types::template::LLMParameters;
use hkask_rsolidity as rs;

use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Estimates inference gas cost from LLMParameters.
///
/// Default: 1 token ≈ 1 gas unit. The `max_tokens` parameter is the primary
/// cost driver — it bounds the maximum possible output.
fn estimate_inference_cost(params: &LLMParameters) -> u64 {
    params.max_tokens.max(1) as u64
}

/// GovernedInference — the membrane through which all inference calls pass.
///
/// Wraps an `InferencePort` and enforces energy budgets and CNS observability.
/// Implements `InferencePort` itself — the membrane IS an InferencePort.
///
/// Hold-settle pattern: gas is reserved before invocation, then settled
/// after with actual cost. If actual cost < reserved, the difference is
/// refunded to the budget.
pub struct GovernedInference {
    inner: Arc<dyn InferencePort>,
    cybernetics: Arc<RwLock<CyberneticsLoop>>,
    event_sink: Arc<dyn NuEventSink>,
    agent: WebID,
}

impl GovernedInference {
    /// Create a new GovernedInference membrane wrapping an inner InferencePort.
    /// Create a new governed inference wrapper.
    ///
    /// REQ: P9-cns-gov-inf-new
    /// expect: "The system creates a governed inference membrane that gates LLM calls behind energy budgets" [P9]
    /// [P9] Motivating: Homeostatic Self-Regulation — inference governance enables cybernetic control
    /// \[P4\] Constraining: Clear Boundaries — membrane wraps inner InferencePort at OCAP boundary
    /// \[P12\] Constraining: Affirmative Consent — agent identity is required for attribution
    /// pre:  inference is valid, cns is valid
    /// post: returns GovernedInference
    #[rs::contract(id = "P9-cns-gov-inf-new", principle = "P9")]
    pub fn new(
        inner: Arc<dyn InferencePort>,
        cybernetics: Arc<RwLock<CyberneticsLoop>>,
        event_sink: Arc<dyn NuEventSink>,
        agent: WebID,
    ) -> Self {
        Self {
            inner,
            cybernetics,
            event_sink,
            agent,
        }
    }

    /// Builder: change the agent for this membrane.
    #[must_use = "builder methods must be chained or assigned"]
    /// Set the agent WebID for attribution.
    ///
    /// REQ: P12-cns-gov-inf-with-agent
    /// expect: "I can bind an agent identity to the inference membrane for attribution" [P12]
    /// [P12] Motivating: Affirmative Consent — agent identity is the consent anchor
    /// \[P4\] Constraining: Clear Boundaries — OCAP gate enforces boundary per inference call
    /// @must_use because builder methods must be chained or assigned
    /// post: returns Self with agent set (builder pattern)
    #[rs::contract(id = "P12-cns-gov-inf-with-agent", principle = "P12")]
    pub fn with_agent(mut self, agent: WebID) -> Self {
        self.agent = agent;
        self
    }
}

impl InferencePort for GovernedInference {
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        // Delegate to generate_with_model with no model override.
        // The budget check happens in generate_with_model.
        self.generate_with_model(prompt, parameters, None)
    }

    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        let estimated_cost = EnergyCost(estimate_inference_cost(parameters));
        let model_name = model_override.unwrap_or("default").to_string();
        let agent = self.agent;
        let cybernetics = Arc::clone(&self.cybernetics);
        let event_sink = Arc::clone(&self.event_sink);
        let inner = Arc::clone(&self.inner);

        // Capture owned copies for the async block
        let prompt_owned = prompt.to_string();
        let params_owned = parameters.clone();
        let model_owned = model_override.map(|s| s.to_string());

        Box::pin(async move {
            // Step 1: Check and reserve energy budget
            let loop6 = cybernetics.read().await;
            if !loop6.can_proceed(&agent, estimated_cost).await {
                // Emit cns.gas.depleted span
                let depleted_span = Span::from_kind(SpanKind::GasDepleted);
                let depleted_event = NuEvent::new(
                    agent,
                    depleted_span,
                    Phase::Sense,
                    serde_json::json!({
                        "operation": "inference",
                        "model": model_name,
                        "estimated_cost": estimated_cost.0,
                    }),
                    0,
                );
                let _ = event_sink.persist(&depleted_event);

                debug!(
                    target: "cns.inference",
                    agent = ?agent,
                    model = %model_name,
                    estimated_cost = estimated_cost.0,
                    "Inference rejected — energy budget exceeded"
                );
                return Err(InferenceError::Generation(
                    "Energy budget exceeded for inference call".into(),
                ));
            }

            if let Err(e) = loop6.reserve_gas(&agent, estimated_cost).await {
                warn!(
                    target: "cns.inference",
                    agent = ?agent,
                    model = %model_name,
                    error = %e,
                    "Failed to reserve gas for inference"
                );
                return Err(InferenceError::Generation(format!(
                    "Gas reservation failed: {e}"
                )));
            }
            drop(loop6);

            // Emit cns.gas.reserved span
            let reserved_span = Span::from_kind(SpanKind::GasReserved);
            let reserved_event = NuEvent::new(
                agent,
                reserved_span,
                Phase::Act,
                serde_json::json!({
                    "server": crate::composite_energy_estimator::CompositeEnergyEstimator::INFERENCE_SERVER,
                    "operation": "inference",
                    "model": model_name,
                    "estimated_cost": estimated_cost.0,
                }),
                0,
            );
            let _ = event_sink.persist(&reserved_event);

            // Emit cns.inference.invoked span
            let invoked_span = Span::new(SpanNamespace::from(CnsSpan::Inference), "invoked");
            let invoked_event = NuEvent::new(
                agent,
                invoked_span,
                Phase::Sense,
                serde_json::json!({
                    "model": model_name,
                    "estimated_cost": estimated_cost.0,
                    "max_tokens": params_owned.max_tokens,
                    "settled": false,
                }),
                0,
            );
            let _ = event_sink.persist(&invoked_event);

            // Step 2: Delegate to inner inference port
            info!(
                target: "cns.inference",
                agent = ?agent,
                model = %model_name,
                estimated_cost = estimated_cost.0,
                "Delegating inference call (gas reserved)"
            );
            let result = inner
                .generate_with_model(&prompt_owned, &params_owned, model_owned.as_deref())
                .await;

            // Step 3: Settle energy cost
            let actual_cost = match &result {
                Ok(_) => estimated_cost.0,
                Err(_) => estimated_cost.0 / 2,
            };
            let loop6 = cybernetics.read().await;
            if let Err(e) = loop6
                .settle_gas(&agent, estimated_cost, EnergyCost(actual_cost))
                .await
            {
                warn!(
                    target: "cns.inference",
                    agent = ?agent,
                    model = %model_name,
                    error = %e,
                    reserved = estimated_cost.0,
                    actual = actual_cost,
                    "Failed to settle gas after inference"
                );
            } else {
                info!(
                    target: "cns.inference",
                    agent = ?agent,
                    model = %model_name,
                    reserved = estimated_cost.0,
                    actual = actual_cost,
                    refunded = estimated_cost.0.saturating_sub(actual_cost),
                    "Gas settled after inference"
                );
            }
            drop(loop6);

            // Emit cns.gas.settled span
            let settled_span = Span::from_kind(SpanKind::GasSettled);
            let settled_event = NuEvent::new(
                agent,
                settled_span,
                Phase::Act,
                serde_json::json!({
                    "server": crate::composite_energy_estimator::CompositeEnergyEstimator::INFERENCE_SERVER,
                    "operation": "inference",
                    "model": model_name,
                    "reserved": estimated_cost.0,
                    "actual": actual_cost,
                    "refunded": estimated_cost.0.saturating_sub(actual_cost),
                }),
                0,
            );
            let _ = event_sink.persist(&settled_event);

            // Step 4: Emit outcome span
            let (outcome_phase, outcome_obs) = match &result {
                Ok(response) => (
                    Phase::Act,
                    serde_json::json!({
                        "model": model_name,
                        "estimated_cost": estimated_cost.0,
                        "actual_cost": actual_cost,
                        "status": "success",
                        "tokens_used": response.usage.total_tokens,
                        "settled": true,
                    }),
                ),
                Err(e) => (
                    Phase::Act,
                    serde_json::json!({
                        "model": model_name,
                        "estimated_cost": estimated_cost.0,
                        "actual_cost": actual_cost,
                        "status": "failure",
                        "error": e.to_string(),
                        "settled": true,
                    }),
                ),
            };
            let completed_span = Span::new(SpanNamespace::from(CnsSpan::Inference), "completed");
            let completed_event =
                NuEvent::new(agent, completed_span, outcome_phase, outcome_obs, 0)
                    .with_parent(invoked_event.id);
            let _ = event_sink.persist(&completed_event);

            result
        })
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P9-cns-gov-inf-est-cost-max-tokens
    #[test]
    fn estimate_inference_cost_uses_max_tokens() {
        let params = LLMParameters {
            max_tokens: 2048,
            ..LLMParameters::default()
        };
        assert_eq!(estimate_inference_cost(&params), 2048);
    }

    // REQ: P9-cns-gov-inf-est-cost-floors-at-one
    #[test]
    fn estimate_inference_cost_floors_at_one() {
        let params = LLMParameters {
            max_tokens: 0,
            ..LLMParameters::default()
        };
        assert_eq!(estimate_inference_cost(&params), 1);
    }
}
