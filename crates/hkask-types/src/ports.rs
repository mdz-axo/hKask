//! Hexagonal port traits — Infrastructure abstractions
//
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations. Per the Authority DAG,
//! domain crates depend on these port traits (not on each other).

use crate::cns::{CircuitState, CnsHealth};
use crate::error::GitError;
use crate::template::LLMParameters;
use crate::template::TemplateCrate;
use serde::{Deserialize, Serialize};

// =============================================================================
// Circuit Breaker Port — Cybernetics membrane
// =============================================================================

/// Circuit Breaker Port — Hexagonal boundary for circuit breaking
///
/// Circuit breaking is a Cybernetics regulation mechanism: it enforces
/// homeostatic control over external service calls by preventing cascading
/// failures. This port allows the Inference loop to use circuit breaking
/// without depending on the Cybernetics (hkask-cns) crate.
///
/// Implementations:
/// - `CircuitBreaker` — Production implementation (in hkask-cns)
pub trait CircuitBreakerPort: Send + Sync {
    fn allow_request(&self) -> bool;
    fn record_success(&self);
    fn record_failure(&self);
    fn state(&self) -> CircuitState;
}

// =============================================================================
// CNS Port — Cybernetics observability membrane
// =============================================================================

/// CNS Port — Hexagonal boundary for CNS observability
///
/// Provides read/write access to CNS health and variety data.
/// This port allows crates to observe and increment CNS state without
/// depending on the Cybernetics (hkask-cns) crate.
///
/// Implementations:
/// - `CnsRuntime` — Production implementation (in hkask-cns)
#[allow(async_fn_in_trait)]
pub trait CnsPort: Send + Sync {
    async fn health(&self) -> CnsHealth;
    async fn variety(&self) -> Vec<(String, u64)>;
    async fn increment_variety(&self, domain: &str, state_name: &str);
}

// =============================================================================
// Git CAS Port — Template crate loading
// =============================================================================

/// Git CAS Port — Hexagonal boundary for template crate loading
///
/// Implementations:
/// - `GitCasAdapter` — Production adapter using gix (in hkask-agents)
pub trait GitCASPort: Send + Sync {
    /// Load a template crate from the content-addressable store
    fn load_template_crate(&self, crate_name: &str) -> Result<TemplateCrate, GitError>;

    /// Resolve the current SHA for a crate
    fn resolve_sha(&self, crate_name: &str) -> Result<String, GitError>;
}

// =============================================================================
// Inference Port — LLM backends membrane
// =============================================================================

/// Inference error types
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Model error: {0}")]
    Model(String),
    #[error("Generation error: {0}")]
    Generation(String),
    #[error("JSON error: {0}")]
    Json(String),
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Token probability from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenProbability {
    pub token: String,
    pub prob: f64,
    pub top_k: Vec<TokenProb>,
}

/// Top-k token probability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenProb {
    pub token: String,
    pub prob: f64,
}

/// Inference result from LLM backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub text: String,
    pub model: String,
    pub usage: InferenceUsage,
    pub finish_reason: String,
    pub token_probabilities: Option<Vec<TokenProbability>>,
}

/// Inference Port — Hexagonal boundary for LLM backends
///
/// Trait for LLM invocations. `OkapiInference` is the default implementation.
/// Defined in hkask-types so domain crates (e.g. hkask-agents) can use it
/// without depending on hkask-templates.
///
/// Implementations:
/// - `OkapiInference` — Production implementation (in hkask-templates)
#[async_trait::async_trait]
pub trait InferencePort: Send + Sync {
    /// Generate text with parameters
    async fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError>;

    /// Generate text with an optional model override.
    /// The `model_override` parameter specifies a model ID to use
    /// instead of the default. Implementations should fall back to
    /// `generate()` when `None` is passed.
    async fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        _model_override: Option<&str>,
    ) -> Result<InferenceResult, InferenceError> {
        self.generate(prompt, parameters).await
    }

    /// Generate multiple outputs for template selection
    async fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> Result<Vec<InferenceResult>, InferenceError> {
        use futures_util::future::join_all;
        let futures: Vec<_> = (0..n).map(|_| self.generate(prompt, parameters)).collect();
        let results = join_all(futures).await;
        results.into_iter().collect()
    }
}
