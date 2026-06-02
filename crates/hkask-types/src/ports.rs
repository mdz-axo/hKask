//! Hexagonal port traits — Infrastructure abstractions
//
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations. Per the Authority DAG,
//! domain crates depend on these port traits (not on each other).

use crate::cns::{CircuitState, CnsHealth};
use crate::error::GitError;
use crate::lexicon::TemplateType;
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

// =============================================================================
// Registry Index Port — Template registry membrane
// =============================================================================

/// Registry entry for template discovery
///
/// Moved from `hkask-templates` to `hkask-types` so that downstream crates
/// (e.g. `hkask-ensemble`) can perform R4 capability intersection checks
/// without depending on the curation layer.
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub id: String,
    pub template_type: TemplateType,
    pub lexicon_terms: Vec<String>,
    pub description: String,
    pub source_path: String,
    /// Required capabilities for this template (R4: Capability Intersection)
    pub required_capabilities: Vec<String>,
}

/// Error type for registry index operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistryError {
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Registry error: {0}")]
    Other(String),
}

/// Registry index port — template registry lookups
///
/// Moved from `hkask-templates` to `hkask-types` so that downstream crates
/// can depend on the abstraction without depending on `hkask-templates`.
///
/// Implementations:
/// - `Registry` — In-memory filesystem-based registry (in hkask-templates)
/// - `SqliteRegistry` — SQLite-backed registry (in hkask-templates)
pub trait RegistryIndex {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry>;

    fn list_with_capabilities(&self, capabilities: &[String]) -> Vec<RegistryEntry> {
        self.list(None)
            .into_iter()
            .filter(|e| {
                e.required_capabilities.is_empty()
                    || e.required_capabilities
                        .iter()
                        .all(|c| capabilities.contains(c))
            })
            .collect()
    }

    fn get(&self, id: &str) -> Result<RegistryEntry, RegistryError>;
}

// =============================================================================
// Session Store Port — Standing session persistence membrane
// =============================================================================

/// Error type for session store operations
///
/// Self-contained error type that does not depend on `hkask-storage`,
/// allowing the port to live in `hkask-types` without pulling in storage deps.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionStoreError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session is sealed: {0}")]
    Sealed(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Persistent record for a standing session
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub session_id: String,
    pub config_yaml: String,
    pub created_at: String,
    pub last_active: String,
}

/// Persistent record for a session message
#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub id: i64,
    pub session_id: String,
    pub from_webid: String,
    pub content: String,
    pub timestamp: String,
    pub template_id: Option<String>,
}

/// Session store port — hexagonal boundary for standing session persistence
///
/// Moved from `hkask-agents` to `hkask-types` so that `hkask-ensemble`
/// can depend on the abstraction without violating the Authority DAG.
///
/// Implementations:
/// - `StandingSessionStoreAdapter` — Production adapter via SQLite (in hkask-agents)
pub trait StandingSessionPort: Send + Sync {
    fn save_session(&self, session: &SessionRecord) -> Result<(), SessionStoreError>;

    fn get_session(&self, session_id: &str) -> Result<SessionRecord, SessionStoreError>;

    fn save_message(&self, message: &MessageRecord) -> Result<i64, SessionStoreError>;

    fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRecord>, SessionStoreError>;

    fn update_last_active(&self, session_id: &str) -> Result<(), SessionStoreError>;
}
