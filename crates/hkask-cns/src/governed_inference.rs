//! GovernedInference — Cybernetics-enforced inference boundary
//!
//! Wraps an `InferencePort` and enforces energy budgets at the
//! inference boundary. A call to `generate()` that exceeds the
//! budget is rejected by Cybernetics, not by the caller.
//!
//! This is the membrane where Loop 6 (Cybernetics) governs Loop 1 (Inference).
//! Authority flows downward: Cybernetics → Inference.
//!
//! # Deprecation Notice
//!
//! **Deprecated since v0.22.0.** Use `GovernedTool` with an `InferencePort`-specific
//! `EnergyEstimator` instead. The `GovernedTool` membrane subsumes all functionality
//! of `GovernedInference` (OCAP authority, energy budget, NuEvent observability)
//! while being generic over any `ToolPort`, not just `InferencePort`.
//!
//! Migration path: Create an `InferenceEnergyEstimator` implementing `EnergyEstimator`
//! using the token estimation logic from this file, then wire `InferenceLoop` to
//! use `GovernedTool` with that estimator.

#![deprecated(since = "0.22.0", note = "Use GovernedTool with InferencePort instead")]

use crate::cybernetics_loop::CyberneticsLoop;
use hkask_types::WebID;
use hkask_types::ports::{InferenceError, InferencePort, InferenceResult};
use hkask_types::template::LLMParameters;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Tokens-per-character heuristic: roughly 4 characters per token for English text.
const CHARS_PER_TOKEN: usize = 4;

/// Estimate total tokens for a generation request.
///
/// Uses `prompt.len() / 4` for prompt tokens plus `parameters.max_tokens`
/// for the expected completion length.
fn estimate_tokens(prompt: &str, parameters: &LLMParameters) -> u64 {
    let prompt_tokens = prompt.len() as u64 / CHARS_PER_TOKEN as u64;
    let completion_tokens = parameters.max_tokens as u64;
    prompt_tokens + completion_tokens
}

/// GovernedInference — Cybernetics-enforced wrapper around `InferencePort`.
///
/// This struct sits at the membrane between Loop 6 (Cybernetics) and
/// Loop 1 (Inference). Every call to `generate()` or `generate_with_model()`
/// must pass through energy budget checks before reaching the underlying
/// inference backend.
///
/// Authority flows downward: Cybernetics governs Inference.
///
/// # Composition
///
/// ```ignore
/// use hkask_cns::{CyberneticsLoop, GovernedInference};
/// use hkask_agents::InferenceLoop;
///
/// let inference_port: Arc<dyn InferencePort> = Arc::new(OkapiInference::new(...));
/// let cybernetics_loop = Arc::new(RwLock::new(CyberneticsLoop::new(cns, dispatch_tx)));
///
/// // Wrap inference with energy budget enforcement
/// let governed: Arc<dyn InferencePort> = Arc::new(
///     GovernedInference::new(inference_port, cybernetics_loop, agent_webid)
/// );
///
/// // Pass governed port to InferenceLoop — it's just an InferencePort
/// let inference_loop = InferenceLoop::new(governed);
/// ```
pub struct GovernedInference {
    inference: Arc<dyn InferencePort>,
    cybernetics: Arc<RwLock<CyberneticsLoop>>,
    agent: WebID,
}

impl GovernedInference {
    /// Create a new GovernedInference wrapper.
    pub fn new(
        inference: Arc<dyn InferencePort>,
        cybernetics: Arc<RwLock<CyberneticsLoop>>,
        agent: WebID,
    ) -> Self {
        Self {
            inference,
            cybernetics,
            agent,
        }
    }

    /// Builder: change the agent for this wrapper.
    pub fn with_agent(mut self, agent: WebID) -> Self {
        self.agent = agent;
        self
    }
}

#[async_trait::async_trait]
impl InferencePort for GovernedInference {
    async fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        let estimated = estimate_tokens(prompt, parameters);

        // Step 1: Check if the agent can proceed within budget.
        let loop6 = self.cybernetics.read().await;
        if !loop6.can_proceed(&self.agent, estimated).await {
            debug!(
                target: "cns.inference",
                agent = ?self.agent,
                estimated_tokens = estimated,
                "Energy budget exceeded — generation rejected by Cybernetics"
            );
            return Err(InferenceError::Generation("energy budget exceeded".into()));
        }

        // Step 2: Acquire budget (atomic check-and-consume).
        if let Err(e) = loop6.acquire_budget(&self.agent, estimated).await {
            warn!(
                target: "cns.inference",
                agent = ?self.agent,
                estimated_tokens = estimated,
                error = %e,
                "Failed to acquire energy budget — generation rejected"
            );
            return Err(InferenceError::Generation(format!(
                "energy budget acquisition failed: {e}"
            )));
        }
        drop(loop6);

        // Step 3: Call the underlying inference port.
        debug!(
            target: "cns.inference",
            agent = ?self.agent,
            estimated_tokens = estimated,
            "Calling inference backend"
        );
        let result = self.inference.generate(prompt, parameters).await;

        // Steps 4 & 5: Return result; caller (or circuit breaker integration)
        // records success/failure at a higher level if needed.
        match result {
            Ok(ref inference_result) => {
                debug!(
                    target: "cns.inference",
                    agent = ?self.agent,
                    model = %inference_result.model,
                    total_tokens = inference_result.usage.total_tokens,
                    "Inference succeeded"
                );
            }
            Err(ref e) => {
                warn!(
                    target: "cns.inference",
                    agent = ?self.agent,
                    error = %e,
                    "Inference failed"
                );
            }
        }

        result
    }

    async fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Result<InferenceResult, InferenceError> {
        let estimated = estimate_tokens(prompt, parameters);

        // Step 1: Check if the agent can proceed within budget.
        let loop6 = self.cybernetics.read().await;
        if !loop6.can_proceed(&self.agent, estimated).await {
            debug!(
                target: "cns.inference",
                agent = ?self.agent,
                estimated_tokens = estimated,
                model = ?model_override,
                "Energy budget exceeded — generation_with_model rejected by Cybernetics"
            );
            return Err(InferenceError::Generation("energy budget exceeded".into()));
        }

        // Step 2: Acquire budget (atomic check-and-consume).
        if let Err(e) = loop6.acquire_budget(&self.agent, estimated).await {
            warn!(
                target: "cns.inference",
                agent = ?self.agent,
                estimated_tokens = estimated,
                model = ?model_override,
                error = %e,
                "Failed to acquire energy budget — generation_with_model rejected"
            );
            return Err(InferenceError::Generation(format!(
                "energy budget acquisition failed: {e}"
            )));
        }
        drop(loop6);

        // Step 3: Call the underlying inference port.
        debug!(
            target: "cns.inference",
            agent = ?self.agent,
            estimated_tokens = estimated,
            model = ?model_override,
            "Calling inference backend with model override"
        );
        let result = self
            .inference
            .generate_with_model(prompt, parameters, model_override)
            .await;

        match result {
            Ok(ref inference_result) => {
                debug!(
                    target: "cns.inference",
                    agent = ?self.agent,
                    model = %inference_result.model,
                    total_tokens = inference_result.usage.total_tokens,
                    "Inference with model override succeeded"
                );
            }
            Err(ref e) => {
                warn!(
                    target: "cns.inference",
                    agent = ?self.agent,
                    error = %e,
                    "Inference with model override failed"
                );
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy::GasBudget;
    use crate::runtime::CnsRuntime;
    use hkask_types::loops::LoopMessage;
    use hkask_types::ports::{InferenceUsage, TokenProb, TokenProbability};
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::mpsc;

    /// Mock inference port that returns a canned successful result.
    struct MockInferencePort {
        should_fail: AtomicBool,
    }

    impl MockInferencePort {
        fn new() -> Self {
            Self {
                should_fail: AtomicBool::new(false),
            }
        }

        fn failing() -> Self {
            Self {
                should_fail: AtomicBool::new(true),
            }
        }

        fn canned_result() -> InferenceResult {
            InferenceResult {
                text: "Hello, world!".into(),
                model: "test-model".into(),
                usage: InferenceUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
                finish_reason: "stop".into(),
                token_probabilities: Some(vec![TokenProbability {
                    token: "Hello".into(),
                    prob: 0.95,
                    top_k: vec![TokenProb {
                        token: "Hello".into(),
                        prob: 0.95,
                    }],
                }]),
            }
        }
    }

    #[async_trait::async_trait]
    impl InferencePort for MockInferencePort {
        async fn generate(
            &self,
            _prompt: &str,
            _parameters: &LLMParameters,
        ) -> Result<InferenceResult, InferenceError> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(InferenceError::Generation("mock inference failure".into()))
            } else {
                Ok(Self::canned_result())
            }
        }

        async fn generate_with_model(
            &self,
            _prompt: &str,
            _parameters: &LLMParameters,
            _model_override: Option<&str>,
        ) -> Result<InferenceResult, InferenceError> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(InferenceError::Generation("mock inference failure".into()))
            } else {
                Ok(Self::canned_result())
            }
        }
    }

    /// Helper: create a CyberneticsLoop wired for tests.
    fn test_cybernetics_loop() -> Arc<RwLock<CyberneticsLoop>> {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let (tx, _rx) = mpsc::unbounded_channel::<LoopMessage>();
        Arc::new(RwLock::new(CyberneticsLoop::new(cns, tx)))
    }

    /// Helper: default LLMParameters for tests.
    fn test_params() -> LLMParameters {
        LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 100,
            seed: None,
        }
    }

    #[tokio::test]
    async fn governed_inference_rejects_when_budget_exhausted() {
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        // Register a tiny budget: cap=10, cost_per_token=0.25 → 40 tokens max
        let budget = GasBudget::new(10);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inference = Arc::new(MockInferencePort::new());
        let governed = GovernedInference::new(inference, loop6, agent);

        // Prompt of 400 chars ≈ 100 prompt tokens + 100 max_tokens = 200 estimated tokens
        // Budget supports ~40 tokens → should be rejected
        let prompt = "a".repeat(400);
        let result = governed.generate(&prompt, &test_params()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            InferenceError::Generation(msg) => {
                assert!(
                    msg.contains("energy budget"),
                    "Expected energy budget error, got: {msg}"
                );
            }
            other => panic!("Expected InferenceError::Generation, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn governed_inference_allows_when_budget_sufficient() {
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        // Register a large budget: cap=100_000
        let budget = GasBudget::new(100_000);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inference = Arc::new(MockInferencePort::new());
        let governed = GovernedInference::new(inference, loop6, agent);

        // Prompt of 100 chars ≈ 25 prompt tokens + 100 max_tokens = 125 estimated
        // Budget supports ~400_000 tokens → should succeed
        let prompt = "Hello, this is a test prompt.".to_string();
        let result = governed.generate(&prompt, &test_params()).await;

        assert!(result.is_ok());
        let inference_result = result.unwrap();
        assert_eq!(inference_result.text, "Hello, world!");
        assert_eq!(inference_result.model, "test-model");
    }

    #[tokio::test]
    async fn governed_inference_deducts_energy_on_success() {
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        // Register budget with known cap
        let budget = GasBudget::new(10_000);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inference = Arc::new(MockInferencePort::new());
        let governed = GovernedInference::new(inference, loop6.clone(), agent);

        let prompt = "a".repeat(100); // ~25 prompt tokens + 100 max_tokens = 125 estimated
        let result = governed.generate(&prompt, &test_params()).await;
        assert!(result.is_ok());

        // After a successful call, the budget should have been deducted.
        // The call consumed ~125 tokens * 0.25 cost_per_token = ~31 energy units.
        // With 10_000 - 31 = ~9_969 remaining, which supports ~39_876 tokens.
        // A request for 40_000 tokens should now fail after the deduction,
        // proving that energy was consumed.
        let loop6_guard = loop6.read().await;
        assert!(!loop6_guard.can_proceed(&agent, 40_000).await);
        drop(loop6_guard);

        // And a smaller request should still succeed
        let loop6_guard = loop6.read().await;
        assert!(loop6_guard.can_proceed(&agent, 100).await);
    }

    #[tokio::test]
    async fn governed_inference_handles_inference_failure() {
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        // Register a large budget so budget checks pass
        let budget = GasBudget::new(100_000);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inference = Arc::new(MockInferencePort::failing());
        let governed = GovernedInference::new(inference, loop6, agent);

        let result = governed.generate("test prompt", &test_params()).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            InferenceError::Generation(msg) => {
                assert!(
                    msg.contains("mock inference failure"),
                    "Expected mock failure error, got: {msg}"
                );
            }
            other => panic!("Expected InferenceError::Generation, got: {other:?}"),
        }
    }
}
