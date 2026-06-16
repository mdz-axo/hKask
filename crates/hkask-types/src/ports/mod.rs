//! Hexagonal port traits — Infrastructure abstractions
//
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations. Per the Authority DAG,
//! domain crates depend on these port traits (not on each other).

pub mod git_cas;

use crate::cns::CircuitState;
use crate::event::SpanNamespace;
use crate::id::WebID;
use crate::lexicon::TemplateType;
use crate::loops::LoopId;
use crate::template::LLMParameters;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
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

/// Confidence = avg(prob) × (1 - sqrt(variance)). Higher avg + lower variance = higher confidence.
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

/// Structured tool call from a model response (OpenAI/Anthropic/Gemini native function calling).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredToolCall {
    pub server: String,
    pub tool: String,
    pub args: serde_json::Value,
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
    /// Populated when `finish_reason == "tool_calls"`. For models without native function calling,
    /// always empty — `parse_tool_calls()` in `tool_augmented` handles `<<tool:...>>` fallback.
    #[serde(default)]
    pub tool_calls: Vec<StructuredToolCall>,
}

/// LLM invocation boundary. Uses `Pin<Box<dyn Future>>` (not `async_trait`) for object-safety.
/// Impls: `InferenceRouter` (hkask-inference), `Arc<dyn InferencePort>` (blanket).
pub trait InferencePort: Send + Sync {
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>>;

    /// Falls back to `generate()` when `model_override` is `None`.
    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        _model_override: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.generate(prompt, parameters)
    }

    fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<InferenceResult>, InferenceError>> + Send + '_>>
    {
        use futures_util::future::join_all;
        let futures: Vec<_> = (0..n).map(|_| self.generate(prompt, parameters)).collect();
        Box::pin(async move {
            let results = join_all(futures).await;
            results.into_iter().collect()
        })
    }

    /// Stream inference chunks. Default: yields single chunk from `generate()`. Override for SSE/streaming backends.
    fn generate_stream(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        let future = self.generate(prompt, parameters);
        Box::pin(futures_util::stream::once(async move {
            Ok(InferenceStreamChunk::from(future.await?))
        }))
    }

    /// Stream with optional model override. Falls back to `generate_stream()` when `model_override` is `None`.
    fn generate_stream_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        if model_override.is_some() {
            let future = self.generate_with_model(prompt, parameters, model_override);
            Box::pin(futures_util::stream::once(async move {
                Ok(InferenceStreamChunk::from(future.await?))
            }))
        } else {
            self.generate_stream(prompt, parameters)
        }
    }

    /// Vision inference — send base64-encoded images to a multimodal model.
    /// Default: falls back to `generate_with_model()` (text-only). Override for vision-capable backends.
    fn generate_vision(
        &self,
        prompt: &str,
        _images: &[String],
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.generate_with_model(prompt, parameters, model_override)
    }
}

/// A single chunk of streaming inference output. Final chunk has `finish_reason` + `usage`.
#[derive(Debug, Clone)]
pub struct InferenceStreamChunk {
    pub text_delta: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub usage: Option<InferenceUsage>,
    pub tool_calls: Vec<StructuredToolCall>,
}

impl From<InferenceResult> for InferenceStreamChunk {
    fn from(r: InferenceResult) -> Self {
        Self {
            text_delta: r.text,
            model: r.model,
            finish_reason: Some(r.finish_reason),
            usage: Some(r.usage),
            tool_calls: r.tool_calls,
        }
    }
}

/// Blanket impl — enables `InferenceLoop<Arc<dyn InferencePort>>` default type param.
/// Vtable dispatch only at construction; hot path uses static dispatch.
impl InferencePort for Arc<dyn InferencePort> {
    fn generate(
        &self,
        p: &str,
        pa: &LLMParameters,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate(p, pa)
    }
    fn generate_with_model(
        &self,
        p: &str,
        pa: &LLMParameters,
        m: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate_with_model(p, pa, m)
    }
    fn generate_n(
        &self,
        p: &str,
        pa: &LLMParameters,
        n: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<InferenceResult>, InferenceError>> + Send + '_>>
    {
        self.as_ref().generate_n(p, pa, n)
    }
    fn generate_stream(
        &self,
        p: &str,
        pa: &LLMParameters,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        self.as_ref().generate_stream(p, pa)
    }
    fn generate_stream_with_model(
        &self,
        p: &str,
        pa: &LLMParameters,
        m: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        self.as_ref().generate_stream_with_model(p, pa, m)
    }
    fn generate_vision(
        &self,
        p: &str,
        imgs: &[String],
        pa: &LLMParameters,
        m: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate_vision(p, imgs, pa, m)
    }
}

/// Unified registry entry covering all template types with cascade metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        if self.cascade_level >= self.matroshka_limit {
            warnings.push(format!(
                "entry '{}' cascade_level ({}) >= matroshka_limit ({}) — nesting exhausted",
                self.id, self.cascade_level, self.matroshka_limit
            ));
        }
        warnings
    }
    pub fn can_nest(&self) -> bool {
        self.cascade_level < self.matroshka_limit
    }
}

/// Two-zone model: Private (`.agents/skills/` source), Public (`skills/` build artifact).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum SkillZone {
    #[default]
    Private,
    Public,
}

impl SkillZone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
        }
    }
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "private" | "Private" => Some(Self::Private),
            "public" | "Public" => Some(Self::Public),
            _ => None,
        }
    }
    pub fn directory(&self) -> &'static str {
        match self {
            Self::Private => ".agents/skills",
            Self::Public => "skills",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    pub id: String,
    pub domain: TemplateType,
    pub word_act: Option<String>,
    pub flow_def: Option<String>,
    pub know_act: Option<String>,
    pub polarity: Option<crate::bundle::SkillPolarity>,
    pub content_hash: Option<String>,
    pub visibility: crate::visibility::Visibility,
    pub zone: SkillZone,
    /// Namespace (replicant handle) for collision-free public sharing.
    ///
    /// [DECLARATIVE] Always a user replicant name (e.g. "alice"), never a system agent. (P6 — Space for Replicants).
    /// System agents (bots) don't author or share skills — only human replicants do.
    ///
    /// In the public zone, skills are stored as `<namespace>--<id>/` directories.
    /// In the private zone, namespace is typically `None` (user-local, no collision).
    /// When set, `qualified_id()` returns `<namespace>--<id>`.
    pub namespace: Option<String>,
}

impl Skill {
    pub fn new(id: &str, domain: TemplateType) -> Self {
        Self {
            id: id.to_string(),
            domain,
            word_act: None,
            flow_def: None,
            know_act: None,
            polarity: None,
            content_hash: None,
            visibility: crate::visibility::Visibility::Private,
            zone: SkillZone::Private,
            namespace: None,
        }
    }

    /// Builders with `Option<String>` from `&str`.
    pub fn with_word_act(mut self, v: &str) -> Self {
        self.word_act = Some(v.to_string());
        self
    }
    pub fn with_flow_def(mut self, v: &str) -> Self {
        self.flow_def = Some(v.to_string());
        self
    }
    pub fn with_know_act(mut self, v: &str) -> Self {
        self.know_act = Some(v.to_string());
        self
    }
    pub fn with_polarity(mut self, v: crate::bundle::SkillPolarity) -> Self {
        self.polarity = Some(v);
        self
    }
    pub fn with_content_hash(mut self, v: String) -> Self {
        self.content_hash = Some(v);
        self
    }
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_visibility(mut self, v: crate::visibility::Visibility) -> Self {
        self.visibility = v;
        self
    }
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_zone(mut self, v: SkillZone) -> Self {
        self.zone = v;
        self
    }
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_namespace(mut self, v: impl Into<String>) -> Self {
        self.namespace = Some(v.into());
        self
    }

    /// Qualified ID: `<namespace>--<id>` if namespace set, else just `id`. Double-dash is unambiguous for filesystem dirs.
    pub fn qualified_id(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}--{}", ns, self.id),
            None => self.id.clone(),
        }
    }
    /// Parse `<namespace>--<id>` into `(namespace, id)`. Returns `None` if not a qualified ID.
    pub fn parse_qualified_id(qualified: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = qualified.splitn(2, "--").collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    /// Compute and set SHA-256 content hash from key fields.
    pub fn compute_content_hash(&mut self) {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.id.as_bytes());
        h.update(self.domain.as_str().as_bytes());
        h.update(self.visibility.as_str().as_bytes());
        h.update(self.zone.as_str().as_bytes());
        if let Some(ref v) = self.namespace {
            h.update(v.as_bytes());
        }
        if let Some(ref v) = self.word_act {
            h.update(v.as_bytes());
        }
        if let Some(ref v) = self.flow_def {
            h.update(v.as_bytes());
        }
        if let Some(ref v) = self.know_act {
            h.update(v.as_bytes());
        }
        self.content_hash = Some(hex::encode(h.finalize()));
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
    fn list_skills_by_visibility(&self, visibility: crate::visibility::Visibility) -> Vec<Skill>;
    fn skills_by_domain(&self, domain: TemplateType) -> Vec<Skill>;
    fn skills_referencing_template(&self, template_id: &str) -> Vec<Skill>;
    fn remove_skill(&mut self, id: &str) -> Option<Skill>;
    /// P2 (Affirmative Consent): default-deny access. Private context sees all skills. Public/Shared sees only Public or Shared.
    fn list_skills_visible_to(
        &self,
        caller_visibility: crate::visibility::Visibility,
    ) -> Vec<Skill> {
        match caller_visibility {
            crate::visibility::Visibility::Private => self.list_skills(),
            _ => {
                let mut result =
                    self.list_skills_by_visibility(crate::visibility::Visibility::Public);
                result
                    .extend(self.list_skills_by_visibility(crate::visibility::Visibility::Public));
                result
            }
        }
    }
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
    EnergyBudgetExceeded(String),
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

/// Canonical tool metadata for OCAP capability matching.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_id: String,
    /// The capability required to invoke this tool, derived from the server ID.
    /// Maps `hkask-mcp-<domain>` → `tool:<domain>:execute`.
    /// `None` for servers that don't follow the `hkask-mcp-` naming convention.
    pub required_capability: Option<String>,
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
