//! Hexagonal port traits — Infrastructure abstractions
//
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations. Per the Authority DAG,
//! domain crates depend on these port traits (not on each other).

use crate::cns::{CircuitState, CnsHealth};
use crate::error::GitError;
use crate::id::WebID;
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

    /// Create a snapshot (commit) of all staged changes in the repository.
    /// Returns the SHA of the new commit, or the current HEAD SHA if nothing to commit.
    fn commit(&self, message: &str) -> Result<String, GitError>;
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
/// The unified entry type for the registry. Covers all template types
/// (WordAct, KnowAct, FlowDef) and carries cascade metadata so that
/// the port trait doesn't lose information on conversion.
///
/// Moved from `hkask-templates` to `hkask-types` so that downstream crates
/// (e.g. `hkask-ensemble`) can perform R4 capability intersection checks
/// without depending on the curation layer.
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub id: String,
    pub template_type: TemplateType,
    /// Human-readable name for display
    pub name: String,
    pub lexicon_terms: Vec<String>,
    pub description: String,
    pub source_path: String,
    /// Required capabilities for this template (R4: Capability Intersection)
    pub required_capabilities: Vec<String>,
    /// Cascade depth for matroshka (nested template) recursion
    pub cascade_level: u32,
    /// Maximum nesting depth (matroshka limit)
    pub matroshka_limit: u32,
}

impl RegistryEntry {
    /// Validate the entry's internal consistency.
    ///
    /// Returns a list of validation warnings. An empty vec means the entry is valid.
    /// This does **not** reject the entry — it logs advisory warnings.
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

    /// Check whether this entry can still nest (matroshka recursion).
    ///
    /// Returns `true` when `cascade_level < matroshka_limit`, meaning the
    /// template may invoke another template.
    pub fn can_nest(&self) -> bool {
        self.cascade_level < self.matroshka_limit
    }
}

/// Skill — a named composition of templates
///
/// A Skill binds WordAct, KnowAct, and FlowDef templates together
/// into a coherent agent capability. The `cascade_order` defines
/// the execution sequence when the skill is invoked.
///
/// Specification templates are FlowDef manifests that define constraints;
/// they are referenced via `flow_def` rather than a separate field.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    pub id: String,
    pub domain: TemplateType,
    pub word_act: Option<String>,
    pub flow_def: Option<String>,
    pub know_act: Option<String>,
    /// Cascade order: template IDs executed in sequence
    pub cascade_order: Vec<String>,
    /// Skill polarity — cognitive role in a bundle composition
    pub polarity: Option<crate::bundle::SkillPolarity>,
    /// SHA-256 content hash of the skill manifest (for evolution tracking)
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

    /// Set the skill polarity (generative, evaluative, regulative, procedural)
    pub fn with_polarity(mut self, polarity: crate::bundle::SkillPolarity) -> Self {
        self.polarity = Some(polarity);
        self
    }

    /// Set the content hash (SHA-256 of the skill manifest)
    pub fn with_content_hash(mut self, hash: String) -> Self {
        self.content_hash = Some(hash);
        self
    }

    /// Compute and set the content hash from the skill's manifest data.
    /// Uses SHA-256 of the skill's key fields (id, domain, word_act, flow_def, know_act, cascade_order).
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

/// Error type for registry index operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistryError {
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Registry error: {0}")]
    Other(String),
}

/// Skill registry index — skill composition lookups
///
/// Implementations provide CRUD for skills, which compose templates
/// into coherent agent capabilities.
///
/// All read methods return owned `Skill` values. This makes the trait
/// compatible with both in-memory (HashMap-backed) and SQLite implementations:
/// the in-memory Registry clones from its HashMap, while SqliteRegistry
/// constructs owned values from database rows.
pub trait SkillRegistryIndex {
    /// Register a new skill
    fn register_skill(&mut self, skill: Skill);
    /// Retrieve a skill by ID
    fn get_skill(&self, id: &str) -> Option<Skill>;
    /// List all skills
    fn list_skills(&self) -> Vec<Skill>;
    /// List skills by domain
    fn skills_by_domain(&self, domain: TemplateType) -> Vec<Skill>;
    /// Find skills that reference a given template ID
    fn skills_referencing_template(&self, template_id: &str) -> Vec<Skill>;
    /// Remove a skill by ID
    fn remove_skill(&mut self, id: &str) -> Option<Skill>;
}

/// Bundle registry index — bundle manifest lookups
///
/// Implementations provide CRUD for bundle manifests, which compose
/// multiple skills into orchestrated process flows.
///
/// All read methods return owned `BundleManifest` values for compatibility
/// with both in-memory and SQLite-backed registries.
pub trait BundleRegistryIndex {
    /// Register a new bundle manifest
    fn register_bundle(&mut self, bundle: crate::BundleManifest);
    /// Retrieve a bundle manifest by ID
    fn get_bundle(&self, id: &str) -> Option<crate::BundleManifest>;
    /// List all bundle manifests
    fn list_bundles(&self) -> Vec<crate::BundleManifest>;
    /// Remove a bundle manifest by ID
    fn remove_bundle(&mut self, id: &str) -> Option<crate::BundleManifest>;
    /// Find a bundle that contains exactly the given set of skills
    fn find_bundle_by_skills(&self, skill_ids: &[String]) -> Option<crate::BundleManifest>;
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
/// - `StandingSessionStore` — Production implementation via SQLite (in hkask-storage)
pub trait StandingSessionPort: Send + Sync {
    fn save_session(&self, session: &SessionRecord) -> Result<(), SessionStoreError>;

    fn get_session(&self, session_id: &str) -> Result<SessionRecord, SessionStoreError>;

    fn save_message(&self, message: &MessageRecord) -> Result<i64, SessionStoreError>;

    fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRecord>, SessionStoreError>;

    fn update_last_active(&self, session_id: &str) -> Result<(), SessionStoreError>;
}

// =============================================================================
// Consolidation Port — Episodic → Semantic bridge membrane
// =============================================================================

use crate::capability::tokens::ConsolidationToken;

/// Result of a consolidation operation (mirrors ConsolidationResult from hkask-memory)
#[derive(Debug, Clone)]
pub struct ConsolidationOutcome {
    pub consolidated_count: usize,
    pub retracted_count: usize,
    pub failed_count: usize,
}

/// Consolidation Port — Hexagonal boundary for Episodic → Semantic consolidation
///
/// The ConsolidationBridge is a Curation-directed one-way operation:
/// episodic triples are stripped of perspective and seeded into semantic
/// memory. This port requires a ConsolidationToken proving that Cybernetics
/// (or Curator as Cybernetics' governor) authorized the operation.
///
/// Implementations:
/// - `ConsolidationBridge` — Production implementation (in hkask-memory)
pub trait ConsolidationPort: Send + Sync {
    /// Consolidate up to `limit` episodic triples for the given perspective.
    ///
    /// Requires a `ConsolidationToken` proving Cybernetics authority.
    /// The one-way bridge cannot be traversed without this token.
    fn consolidate(
        &self,
        token: &ConsolidationToken,
        perspective: &WebID,
        limit: usize,
    ) -> Result<ConsolidationOutcome, String>;

    /// Count episodic triples eligible for consolidation for the given perspective.
    fn consolidation_candidate_count(&self, perspective: &WebID) -> usize;
}

// =============================================================================
// CNS Observer Port — Bot observation membrane
// =============================================================================

use crate::event::SpanNamespace;
use crate::loops::LoopId;

/// Emitted when an agent's gas budget is approaching exhaustion.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepletionSignal {
    pub agent: WebID,
    pub remaining: u64,
    pub cap: u64,
    pub usage_ratio: f64,
}

/// Emitted when the system is applying backpressure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackpressureSignal {
    pub source: LoopId,
    pub reason: String,
    pub severity: f64,
}

/// A subscriber that receives CNS events matching its interest mask.
///
/// Bots subscribe to span namespaces, not individual events. A Bot
/// interested in tool invocations subscribes to `SpanNamespace::new("cns.tool")`.
/// The CNS delivers matching events asynchronously.
///
/// This keeps the observation path separate from the regulation path.
#[async_trait::async_trait]
pub trait CnsObserver: Send + Sync {
    /// Which span namespaces this observer is interested in.
    /// Only events matching these namespaces will be delivered.
    fn interest_mask(&self) -> Vec<SpanNamespace>;

    /// Called when a NuEvent matching the interest mask is emitted.
    async fn on_event(&self, event: &crate::event::NuEvent);

    /// Called when a depletion signal fires for this observer's agent.
    async fn on_depletion(&self, signal: &DepletionSignal);

    /// Called when a backpressure signal fires.
    async fn on_backpressure(&self, signal: &BackpressureSignal);
}

// =============================================================================
// Tool Port — Governance membrane for MCP tool invocation
// =============================================================================

use crate::capability::DelegationToken;

/// Error type for governed tool invocation
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

/// Tool Port — Hexagonal boundary for tool invocation.
///
/// This is the singular membrane through which all MCP tool invocations pass.
/// The `GovernedTool` in `hkask-cns` wraps a `dyn ToolPort` and implements
/// `ToolPort` itself — the membrane IS a ToolPort. Before delegating, it checks:
/// 1. Authority (OCAP) — CapabilityChecker::verify
/// 2. Budget (Cybernetics) — CyberneticsLoop::can_proceed / acquire_budget
/// 3. Emits span (CNS) — NuEventSink::emit
/// 4. Delegates to inner tool
/// 5. Accounts energy cost (Cybernetics)
/// 6. Emits outcome span (CNS)
///
/// Implementations:
/// - `McpDispatcher` — Production tool invocation (in hkask-mcp)
#[async_trait::async_trait]
pub trait ToolPort: Send + Sync {
    /// Invoke a tool by name with the given input and capability token.
    ///
    /// The token proves the agent is authorized for this tool invocation.
    /// Returns the tool's response or an error.
    async fn invoke(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Value,
        token: &DelegationToken,
    ) -> Result<serde_json::Value, ToolPortError>;

    /// Discover available tools.
    async fn discover_tools(&self) -> Vec<String>;

    /// Get metadata for a specific tool.
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}

/// Tool information metadata
///
/// Canonical definition lives in `hkask_types::ports::ToolInfo`.
/// Re-exported from `hkask-templates` for backward compatibility.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_id: String,
    pub required_capability: Option<String>,
}

// =============================================================================
// Embedding Port — Vector storage and similarity search membrane
// =============================================================================

/// Stored embedding record with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredEmbedding {
    /// Unique identifier for this embedding
    pub id: String,
    /// Reference to the entity (triple ID or arbitrary key)
    pub entity_ref: String,
    /// Raw embedding vector (f32)
    pub vector: Vec<f32>,
    /// Name of the model that produced this embedding
    pub model: String,
}

/// Result of a similarity search query
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimilarityResult {
    /// The matching embedding record
    pub embedding: StoredEmbedding,
    /// Distance metric (lower = more similar for L2/cosine)
    pub distance: f64,
}

/// Error type for embedding operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Embedding not found: {0}")]
    NotFound(String),
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Embedding Port — Hexagonal boundary for vector storage and similarity search
///
/// Provides storage and KNN similarity search over embedding vectors.
/// The sqlite-vec virtual table provides the KNN index; the `embeddings`
/// table stores metadata. This port abstracts over both.
///
/// Implementations:
/// - `EmbeddingStore` — Production implementation via sqlite-vec (in hkask-storage)
pub trait EmbeddingPort: Send + Sync {
    /// Store an embedding vector, indexed by entity reference.
    ///
    /// The vector is stored in both the `embeddings` metadata table and the
    /// `vec_embeddings` virtual table for KNN search.
    fn store(
        &self,
        entity_ref: &str,
        vector: &[f32],
        model: &str,
    ) -> Result<String, EmbeddingError>;

    /// Retrieve an embedding by its entity reference.
    fn get(&self, entity_ref: &str) -> Result<StoredEmbedding, EmbeddingError>;

    /// Search for the K nearest neighbors of a query vector.
    ///
    /// Returns results ordered by ascending distance (most similar first).
    fn search(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError>;

    /// Delete an embedding by entity reference.
    fn delete(&self, entity_ref: &str) -> Result<(), EmbeddingError>;

    /// Count total embeddings stored.
    fn count(&self) -> Result<usize, EmbeddingError>;
}

// =============================================================================
// Embedding Generation Port — Okapi embedding API membrane
// =============================================================================

/// Error type for embedding generation via Okapi.
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

/// Embedding Generation Port — Hexagonal boundary for embedding vector generation
///
/// Generates embedding vectors from text sentences via an external API
/// (Okapi's `/api/embed/sentences` endpoint). This port is distinct from
/// `EmbeddingPort` which handles storage and KNN search.
///
/// Implementations:
/// - `OkapiEmbedding` — Production implementation via Okapi HTTP API (in hkask-templates)
#[async_trait::async_trait]
pub trait EmbeddingGenerationPort: Send + Sync {
    /// Generate embedding vectors for a batch of sentences.
    ///
    /// Returns one vector per input sentence, in the same order.
    /// Vector dimension is determined by the model (e.g., 384 for qwen3-embedding:0.6b).
    async fn embed_sentences(
        &self,
        sentences: &[&str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingGenerationError>;

    /// Generate an embedding vector for a single sentence.
    ///
    /// Convenience wrapper around `embed_sentences` for single-input cases.
    async fn embed_sentence(&self, sentence: &str) -> Result<Vec<f32>, EmbeddingGenerationError> {
        let results = self.embed_sentences(&[sentence]).await?;
        results
            .into_iter()
            .next()
            .ok_or(EmbeddingGenerationError::EmptyResponse)
    }

    /// Get the embedding dimension for the current model.
    fn embedding_dim(&self) -> usize;
}
