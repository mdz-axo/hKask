//! Hexagonal port traits — Infrastructure abstractions
//
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations. Per the Authority DAG,
//! domain crates depend on these port traits (not on each other).

pub mod git_cas;

use crate::cns::CircuitState;
use crate::id::WebID;
use crate::lexicon::TemplateType;
use crate::template::LLMParameters;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Circuit breaker boundary for the Cybernetics membrane.
///
/// Allows the Inference loop to use circuit breaking without depending on hkask-cns.
/// Impl: `CircuitBreaker` (in hkask-cns)
pub trait CircuitBreakerPort: Send + Sync {
    fn allow_request(&self) -> bool;
    fn record_success(&self);
    fn record_failure(&self);
    fn state(&self) -> CircuitState;
}

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
    #[error("Circuit open: {0}")]
    CircuitOpen(String),
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

/// Compute confidence score from token probabilities.
///
/// Formula: avg(prob) × (1 - sqrt(variance))
///
/// Higher average probability with lower variance yields higher confidence.
/// Lives in hkask-types alongside `TokenProbability` so all crates can use
/// it without creating sideways dependencies in the Authority DAG.
pub fn compute_confidence(probs: &[TokenProbability]) -> f64 {
    if probs.is_empty() {
        return 0.0;
    }

    let avg_prob: f64 = probs.iter().map(|p| p.prob).sum::<f64>() / probs.len() as f64;

    let variance: f64 = probs
        .iter()
        .map(|p| (p.prob - avg_prob).powi(2))
        .sum::<f64>()
        / probs.len() as f64;

    avg_prob * (1.0 - variance.sqrt())
}

/// Structured tool call from a model response.
///
/// When a model supports native function calling (OpenAI, Anthropic, Gemini),
/// it returns structured tool call data rather than embedded text directives.
/// This type captures that structured data so the system can route it
/// through the Communication Loop without fragile text parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredToolCall {
    /// The MCP server ID (e.g., "hkask-mcp-inference")
    pub server: String,
    /// The tool name (e.g., "inference_generate")
    pub tool: String,
    /// The JSON arguments for the tool call
    pub args: serde_json::Value,
    /// Optional call ID from the model (for multi-turn tool use)
    pub call_id: Option<String>,
}

/// Inference result from LLM backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub text: String,
    pub model: String,
    pub usage: InferenceUsage,
    pub finish_reason: String,
    pub token_probabilities: Option<Vec<TokenProbability>>,
    /// Structured tool calls from models that support native function calling.
    ///
    /// When `finish_reason == "tool_calls"`, this vector contains the parsed
    /// tool call data. When `finish_reason == "stop"`, this is empty and
    /// the `text` field contains the response.
    ///
    /// For models that don't support native function calling, this is always
    /// empty — the fallback `parse_tool_calls()` function in `tool_augmented`
    /// handles `<<tool:...>>` text directives instead.
    #[serde(default)]
    pub tool_calls: Vec<StructuredToolCall>,
}

/// LLM invocation boundary.
///
/// Uses `Pin<Box<dyn Future>>` return types (not `async_trait`) to keep
/// the trait object-safe and make boxing visible.
/// Impls: `OkapiInference` (hkask-templates), `Arc<dyn InferencePort>` (backward compat)
pub trait InferencePort: Send + Sync {
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    >;

    /// Falls back to `generate()` when `model_override` is `None`.
    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        _model_override: Option<&str>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        self.generate(prompt, parameters)
    }

    fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<InferenceResult>, InferenceError>>
                + Send
                + '_,
        >,
    > {
        use futures_util::future::join_all;
        let futures: Vec<_> = (0..n).map(|_| self.generate(prompt, parameters)).collect();
        Box::pin(async move {
            let results = join_all(futures).await;
            results.into_iter().collect()
        })
    }
}

/// Blanket impl — enables `InferenceLoop<Arc<dyn InferencePort>>` default type param.
/// Vtable dispatch only at construction; hot path uses static dispatch.
impl InferencePort for Arc<dyn InferencePort> {
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        (**self).generate(prompt, parameters)
    }

    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        (**self).generate_with_model(prompt, parameters, model_override)
    }

    fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<InferenceResult>, InferenceError>>
                + Send
                + '_,
        >,
    > {
        (**self).generate_n(prompt, parameters, n)
    }
}

/// Unified registry entry covering all template types with cascade metadata.
/// Moved to hkask-types so downstream crates can do R4 capability checks
/// without depending on hkask-templates.
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub id: String,
    pub template_type: TemplateType,
    pub name: String,
    pub lexicon_terms: Vec<String>,
    pub description: String,
    pub source_path: String,
    pub required_capabilities: Vec<String>,
    pub cascade_level: u32,
    pub matroshka_limit: u32,
}

impl RegistryEntry {
    /// Returns validation warnings. Empty vec = valid. Does not reject the entry.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.id.is_empty() {
            warnings.push("entry id is empty".into());
        }
        if self.source_path.is_empty() {
            warnings.push(format!("entry '{}' has empty source_path", self.id));
        }
        if self.name.is_empty() {
            warnings.push(format!("entry '{}' has empty name", self.id));
        }

        // Matroshka enforcement: cascade_level must be < matroshka_limit
        // for the template to be invocable. If equal, nesting is exhausted.
        if self.cascade_level >= self.matroshka_limit {
            warnings.push(format!(
                "entry '{}' cascade_level ({}) >= matroshka_limit ({}) — nesting exhausted",
                self.id, self.cascade_level, self.matroshka_limit
            ));
        }

        warnings
    }

    /// `true` when `cascade_level < matroshka_limit`.
    pub fn can_nest(&self) -> bool {
        self.cascade_level < self.matroshka_limit
    }
}

/// Named composition of WordAct, KnowAct, and FlowDef templates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    pub id: String,
    pub domain: TemplateType,
    pub word_act: Option<String>,
    pub flow_def: Option<String>,
    pub know_act: Option<String>,
    pub cascade_order: Vec<String>,
    pub polarity: Option<crate::bundle::SkillPolarity>,
    pub content_hash: Option<String>,
}

impl Skill {
    pub fn new(id: &str, domain: TemplateType) -> Self {
        Self {
            id: id.to_string(),
            domain,
            word_act: None,
            flow_def: None,
            know_act: None,
            cascade_order: vec![],
            polarity: None,
            content_hash: None,
        }
    }

    pub fn with_word_act(mut self, template_id: &str) -> Self {
        self.word_act = Some(template_id.to_string());
        self
    }

    pub fn with_flow_def(mut self, template_id: &str) -> Self {
        self.flow_def = Some(template_id.to_string());
        self
    }

    pub fn with_know_act(mut self, template_id: &str) -> Self {
        self.know_act = Some(template_id.to_string());
        self
    }

    pub fn with_cascade_order(mut self, order: Vec<String>) -> Self {
        self.cascade_order = order;
        self
    }

    pub fn with_polarity(mut self, polarity: crate::bundle::SkillPolarity) -> Self {
        self.polarity = Some(polarity);
        self
    }

    pub fn with_content_hash(mut self, hash: String) -> Self {
        self.content_hash = Some(hash);
        self
    }

    /// Compute and set SHA-256 content hash from key fields.
    pub fn compute_content_hash(&mut self) {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.domain.as_str().as_bytes());
        if let Some(ref wa) = self.word_act {
            hasher.update(wa.as_bytes());
        }
        if let Some(ref fd) = self.flow_def {
            hasher.update(fd.as_bytes());
        }
        if let Some(ref ka) = self.know_act {
            hasher.update(ka.as_bytes());
        }
        for tmpl in &self.cascade_order {
            hasher.update(tmpl.as_bytes());
        }
        let result = hasher.finalize();
        self.content_hash = Some(hex::encode(result));
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistryError {
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Registry error: {0}")]
    Other(String),
}

/// CRUD for skills. Read methods return owned `Skill` for HashMap/SQLite compat.
pub trait SkillRegistryIndex {
    fn register_skill(&mut self, skill: Skill);
    fn get_skill(&self, id: &str) -> Option<Skill>;
    fn list_skills(&self) -> Vec<Skill>;
    fn skills_by_domain(&self, domain: TemplateType) -> Vec<Skill>;
    fn skills_referencing_template(&self, template_id: &str) -> Vec<Skill>;
    fn remove_skill(&mut self, id: &str) -> Option<Skill>;
}

/// CRUD for bundle manifests. Read methods return owned values for HashMap/SQLite compat.
pub trait BundleRegistryIndex {
    fn register_bundle(&mut self, bundle: crate::BundleManifest);
    fn get_bundle(&self, id: &str) -> Option<crate::BundleManifest>;
    fn list_bundles(&self) -> Vec<crate::BundleManifest>;
    fn remove_bundle(&mut self, id: &str) -> Option<crate::BundleManifest>;
    fn find_bundle_by_skills(&self, skill_ids: &[String]) -> Option<crate::BundleManifest>;
}

/// Template registry lookups. Moved to hkask-types for Authority DAG.
/// Impls: `Registry` (in-memory, hkask-templates), `SqliteRegistry` (hkask-templates)
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

/// Self-contained error type — no dependency on hkask-storage.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionStoreError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session is sealed: {0}")]
    Sealed(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub session_id: String,
    pub config_yaml: String,
    pub created_at: String,
    pub last_active: String,
}

#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub id: i64,
    pub session_id: String,
    pub from_webid: String,
    pub content: String,
    pub timestamp: String,
    pub template_id: Option<String>,
}

/// Parameters for consolidation. All fields except `limit` optional.
#[derive(Debug, Clone)]
pub struct ConsolidationRequest {
    pub limit: usize,
    pub confidence_floor: Option<f64>,
    pub max_semantic_triples: Option<usize>,
}

impl Default for ConsolidationRequest {
    fn default() -> Self {
        Self {
            limit: 100,
            confidence_floor: None,
            max_semantic_triples: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConsolidationOutcome {
    pub consolidated_count: usize,
    pub deleted_count: usize,
    pub failed_count: usize,
}

use crate::event::SpanNamespace;
use crate::loops::LoopId;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepletionSignal {
    pub agent: WebID,
    pub remaining: u64,
    pub cap: u64,
    pub usage_ratio: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackpressureSignal {
    pub source: LoopId,
    pub reason: String,
    pub severity: f64,
}

/// Subscribes to CNS events by span namespace.
#[async_trait::async_trait]
pub trait CnsObserver: Send + Sync {
    fn interest_mask(&self) -> Vec<SpanNamespace>;

    async fn on_event(&self, event: &crate::event::NuEvent);

    async fn on_depletion(&self, signal: &DepletionSignal);

    async fn on_backpressure(&self, signal: &BackpressureSignal);
}

use crate::capability::DelegationToken;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolPortError {
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Gas budget exceeded: {0}")]
    GasBudgetExceeded(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Tool invocation failed: {0}")]
    InvocationFailed(String),
}

/// Governance membrane for MCP tool invocation.
/// GovernedTool checks: OCAP authority → budget → emit span → delegate → account cost → emit outcome.
/// Impl: `McpDispatcher` (hkask-mcp)
pub trait ToolPort: Send + Sync {
    /// Token proves agent authorization for this invocation.
    fn invoke(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Value,
        token: &DelegationToken,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, ToolPortError>> + Send;

    fn discover_tools(&self) -> impl std::future::Future<Output = Vec<String>> + Send;

    fn get_tool_info(
        &self,
        tool_name: &str,
    ) -> impl std::future::Future<Output = Option<ToolInfo>> + Send;
}

/// Canonical definition. Re-exported from hkask-templates for backward compat.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_id: String,
    pub required_capability: Option<String>, // TODO: Populate from AgentDefinition.capabilities or server config
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum EmbeddingGenerationError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("API error: status {0}: {1}")]
    Api(u16, String),
    #[error("JSON parse error: {0}")]
    Json(String),
    #[error("Empty response from embedding model")]
    EmptyResponse,
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}
