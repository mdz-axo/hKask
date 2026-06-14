//! Unified domain error hierarchy for hKask service operations.
//! # REQ: P8 (Semantic Grounding) — every error variant is a distinct semantic state.
//!
//! `ServiceError` composes from all domain crate errors. Surface layers
//! (CLI, API) adapt `ServiceError` into their own presentation types:
//!
//! - CLI: `impl From<ServiceError> for CuratorError`, `AgentError`, etc.
//!   (added in `hkask-cli/src/errors.rs`)
//! - API: `impl From<ServiceError> for ApiError` (added in `hkask-api/src/error.rs`)
//!
//! MCP servers continue using `anyhow` for isolated process errors and do
//! NOT depend on this crate.
//!
//! # Design principles
//!
//! - Every variant is either a `#[from]` transparent wrapper around a domain
//!   crate error, or a sentinel String variant for user-facing input errors
//!   that have no upstream typed source.
//! - Surface types (`Json<T>`, HTTP status codes, `println!` formatting)
//!   NEVER appear in `ServiceError` — those belong in surface adapters.
//! - The error hierarchy is flat, not nested: no `ServiceError::Curator(..)`
//!   wrapper around `CuratorError`. Instead, the domain errors that
//!   `CuratorError` wraps appear directly as `ServiceError` variants.
//! - `ServiceError` does NOT depend on surface types (CLI errors, API errors).
//!   Dependency direction: surface → service → domain. Never the reverse.

use thiserror::Error;

// ── Domain error imports ──────────────────────────────────────────────────
use hkask_agents::acp::AcpError;
use hkask_agents::consent::ConsentError;
use hkask_agents::curator_agent::metacognition::MetacognitionError;
use hkask_agents::pod::AgentPodError;
use hkask_agents::registry_loader::RegistryLoaderError;
use hkask_cns::EnergyError;
use hkask_memory::{EpisodicMemoryError, SemanticMemoryError};
use hkask_storage::EscalationError;
use hkask_storage::{
    AgentRegistryError, ConsentStoreError, DatabaseError, GoalRepositoryError, NuEventError,
    SovereigntyStoreError, SpecError, TripleError, UserStoreError,
};
use hkask_templates::TemplateError;
use hkask_types::InfrastructureError;
use hkask_types::McpErrorKind;
use hkask_types::ports::{EmbeddingGenerationError, InferenceError, RegistryError};

/// Unified domain error for all service operations.
///
/// This replaces the 7 CLI error enums and the API `ApiError` as the single
/// canonical error type for business logic. Surface adapters translate
/// `ServiceError` into presentation format (terminal output, HTTP response).
///
/// Variants are grouped by domain. Each `#[from]` variant composes from a
/// domain crate's error type, preserving the full error chain. String variants
/// are sentinels for user-facing input errors that have no upstream typed source.
#[derive(Debug, Error)]
pub enum ServiceError {
    // ── Curator domain ──────────────────────────────────────────────────
    /// Escalation not found by ID.
    #[error("Escalation not found: {0}")]
    EscalationNotFound(String),

    /// Upstream escalation-queue error.
    #[error(transparent)]
    Escalation(#[from] EscalationError),

    /// Upstream metacognition-loop error.
    #[error(transparent)]
    Metacognition(#[from] MetacognitionError),

    // ── Agent / ACP domain ───────────────────────────────────────────────
    /// Agent not found by name.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Invalid agent type.
    #[error("Invalid agent type: {0}")]
    InvalidAgentType(String),

    /// Agent registration failed.
    #[error("Agent registration failed: {0}")]
    AgentRegistrationFailed(String),

    /// Upstream ACP error.
    #[error(transparent)]
    Acp(#[from] AcpError),

    /// Upstream agent-registry loader error.
    #[error(transparent)]
    AgentRegistry(#[from] RegistryLoaderError),

    /// Upstream agent registry store error.
    #[error(transparent)]
    AgentRegistryStore(#[from] AgentRegistryError),

    /// Upstream consent error.
    #[error(transparent)]
    Consent(#[from] ConsentError),

    // ── Storage domain ──────────────────────────────────────────────────
    /// Upstream database error.
    #[error(transparent)]
    Storage(#[from] DatabaseError),

    /// Upstream template registry error.
    #[error(transparent)]
    Registry(#[from] RegistryError),

    /// Upstream template store error.
    #[error(transparent)]
    Template(#[from] TemplateError),

    /// Upstream goal repository error.
    #[error(transparent)]
    GoalRepo(#[from] GoalRepositoryError),

    /// Upstream triple store error.
    #[error(transparent)]
    Triple(#[from] TripleError),

    /// Upstream user store error.
    #[error(transparent)]
    UserStore(#[from] UserStoreError),

    /// Upstream consent store error.
    #[error(transparent)]
    ConsentStore(#[from] ConsentStoreError),

    /// Upstream sovereignty store error.
    #[error(transparent)]
    SovereigntyStore(#[from] SovereigntyStoreError),

    /// Upstream spec error.
    #[error(transparent)]
    Spec(#[from] SpecError),

    /// Upstream NuEvent store error.
    #[error(transparent)]
    NuEvent(#[from] NuEventError),

    // ── Memory domain ────────────────────────────────────────────────────
    /// Upstream episodic memory error.
    #[error(transparent)]
    EpisodicMemory(#[from] EpisodicMemoryError),

    /// Upstream semantic memory error.
    #[error(transparent)]
    SemanticMemory(#[from] SemanticMemoryError),

    /// Consolidation pipeline execution failed.
    #[error("Consolidation failed: {0}")]
    Consolidation(String),

    // ── CNS domain ──────────────────────────────────────────────────────
    /// CNS operation failed.
    #[error("CNS operation failed: {0}")]
    Cns(String),

    /// Keystore secret resolution failed.
    #[error("Keystore resolution failed: {0}")]
    Keystore(String),

    /// Upstream energy budget error.
    #[error(transparent)]
    Gas(#[from] EnergyError),

    // ── Pod domain ────────────────────────────────────────────────────
    /// Pod not found by ID.
    #[error("Pod not found: {0}")]
    PodNotFound(String),

    /// Upstream agent pod error.
    #[error(transparent)]
    Pod(#[from] AgentPodError),

    // ── Inference domain ────────────────────────────────────────────────
    /// Upstream inference port error.
    #[error(transparent)]
    InferencePort(#[from] InferenceError),

    /// Upstream embedding generation error.
    #[error(transparent)]
    Embedding(#[from] EmbeddingGenerationError),

    // ── User domain ─────────────────────────────────────────────────────
    /// User not found by name.
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Login failed (deliberately opaque).
    #[error("Login failed: {0}")]
    LoginFailed(String),

    /// Invalid passphrase.
    #[error("Invalid passphrase: {0}")]
    InvalidPassphrase(String),

    /// Validation error.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Invalid UUID format for WebID parsing.
    #[error("Invalid WebID: {0}")]
    InvalidWebID(String),

    // ── Infrastructure ──────────────────────────────────────────────────
    /// Upstream infrastructure error (lock poisoning, IO, etc.).
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    /// Registry initialization failure (no upstream typed source).
    #[error("Registry initialization failed: {0}")]
    RegistryInitFailed(String),

    /// Registry load failure (no upstream typed source).
    #[error("Registry load failed: {0}")]
    RegistryLoadFailed(String),

    // ── Archival domain ──────────────────────────────────────────────────
    /// GitHub archival operation failed (API call, encoding, credential resolution).
    #[error("Archival failed: {0}")]
    Archival(String),

    // ── Embedding pipeline domain ─────────────────────────────────────────
    /// Embedding pipeline failed (config parsing, download, IO, batch processing).
    #[error("Embed failed: {0}")]
    Embed(String),

    // ── Style composition domain ────────────────────────────────────────
    /// Style composition failed (Jinja2 rendering, inference, validation).
    #[error("Compose failed: {0}")]
    Compose(String),

    // ── Skill domain ────────────────────────────────────────────────────────
    /// Skill operation failed (IO, front matter parsing, publish failure).
    #[error("Skill failed: {0}")]
    Skill(String),

    // ── Verification domain ─────────────────────────────────────────────────
    /// Sovereignty verification failed (manifest loading, assertion execution).
    #[error("Verification failed: {0}")]
    Verification(String),

    // ── Wallet domain ───────────────────────────────────────────────────
    /// Wallet operation failed.
    #[error("Wallet error: {0}")]
    Wallet(String),

    // ── Rate limiting ──────────────────────────────────────────────────────
    /// Operation rate limited (too soon after previous invocation).
    #[error("{0}")]
    RateLimited(String),

    // ── Configuration / setup ───────────────────────────────────────────
    /// Configuration or external service setup failed.
    #[error("Config error: {0}")]
    Config(String),

    // ── Matrix / communication ──────────────────────────────────────────
    /// Matrix homeserver operation failed (registration, connection, message send).
    #[error("Matrix error: {0}")]
    Matrix(String),

    // ── MCP tool errors (out-of-process server failures) ─────────────────
    /// MCP tool call failed. Carries the semantic error kind for retryability
    /// and CNS observability. The `server` and `tool` fields identify the
    /// failing MCP server and tool for debugging.
    #[error("{kind}: {message} (server={server}, tool={tool})")]
    McpTool {
        kind: McpErrorKind,
        server: String,
        tool: String,
        message: String,
    },
}

impl From<uuid::Error> for ServiceError {
    fn from(e: uuid::Error) -> Self {
        ServiceError::InvalidWebID(e.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for ServiceError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        ServiceError::Infra(hkask_types::InfrastructureError::LockPoisoned)
    }
}

impl From<hkask_mcp::server::McpToolError> for ServiceError {
    fn from(e: hkask_mcp::server::McpToolError) -> Self {
        ServiceError::McpTool {
            kind: e.kind,
            server: String::new(),
            tool: String::new(),
            message: e.message,
        }
    }
}

// ── Retryability semantics ─────────────────────────────────────────────
//
// The CNS energy budget needs to know whether retrying an operation will
// consume gas for a potentially successful retry or waste gas on a
// guaranteed failure. This method provides that signal.

impl ServiceError {
    /// Whether this error represents a transient condition that may succeed
    /// on retry (with backoff). Used by the CNS gas budget to decide whether
    /// to allow retry loops.
    ///
    /// Retryable: network I/O, inference connection/timeout, circuit breaker
    /// open, rate limiting, external service unavailable.
    ///
    /// Non-retryable: not-found, invalid input, permission denied, database
    /// corruption, encryption failures, lock poisoning.
    pub fn is_retryable(&self) -> bool {
        match self {
            // ── Retryable ────────────────────────────────────────────
            ServiceError::InferencePort(e) => matches!(
                e,
                hkask_types::ports::InferenceError::Connection(_)
                    | hkask_types::ports::InferenceError::CircuitOpen(_)
            ),
            ServiceError::Embedding(e) => matches!(
                e,
                hkask_types::ports::EmbeddingGenerationError::Connection(_)
                    | hkask_types::ports::EmbeddingGenerationError::Api(..)
            ),
            ServiceError::Infra(e) => matches!(e, hkask_types::InfrastructureError::Io(_)),
            ServiceError::RateLimited(_) => true,
            ServiceError::Matrix(_) => true, // Network operations may be transient
            ServiceError::Config(_) => true, // Config resolution may succeed on retry
            ServiceError::Keystore(_) => true, // Keychain may be temporarily unavailable
            ServiceError::McpTool { kind, .. } => kind.is_retryable(),

            // ── Non-retryable ────────────────────────────────────────
            // User-input errors: retrying won't change the outcome
            ServiceError::EscalationNotFound(_)
            | ServiceError::AgentNotFound(_)
            | ServiceError::InvalidAgentType(_)
            | ServiceError::AgentRegistrationFailed(_)
            | ServiceError::PodNotFound(_)
            | ServiceError::UserNotFound(_)
            | ServiceError::LoginFailed(_)
            | ServiceError::InvalidPassphrase(_)
            | ServiceError::ValidationError(_)
            | ServiceError::InvalidWebID(_) => false,

            // Storage errors: database corruption, schema issues, encryption
            // failures are not transient
            ServiceError::Storage(_) => false,

            // Permission/security: retrying won't grant capabilities
            ServiceError::Acp(_) | ServiceError::Consent(_) => false,

            // CNS energy exhaustion: retrying would waste more gas
            ServiceError::Gas(_) => false,

            // Pipeline/operational errors: generally non-retryable
            // (registry init failure, archival failure, embed failure)
            ServiceError::RegistryInitFailed(_)
            | ServiceError::RegistryLoadFailed(_)
            | ServiceError::Archival(_)
            | ServiceError::Embed(_)
            | ServiceError::Compose(_)
            | ServiceError::Skill(_)
            | ServiceError::Verification(_)
            | ServiceError::Wallet(_)
            | ServiceError::Cns(_)
            | ServiceError::Consolidation(_) => false,

            // ── Delegate to inner error for transparent wrappers ──────
            // Domain errors may have their own retryability semantics.
            // Default conservative: non-retryable unless proven otherwise.
            ServiceError::Escalation(_)
            | ServiceError::Metacognition(_)
            | ServiceError::AgentRegistry(_)
            | ServiceError::AgentRegistryStore(_)
            | ServiceError::Registry(_)
            | ServiceError::Template(_)
            | ServiceError::GoalRepo(_)
            | ServiceError::Triple(_)
            | ServiceError::UserStore(_)
            | ServiceError::ConsentStore(_)
            | ServiceError::SovereigntyStore(_)
            | ServiceError::Spec(_)
            | ServiceError::NuEvent(_)
            | ServiceError::EpisodicMemory(_)
            | ServiceError::SemanticMemory(_)
            | ServiceError::Pod(_) => false,
        }
    }
}

// ── CNS ν-event emission ───────────────────────────────────────────────
//
// Only system-level errors (infrastructure, inference, CNS, storage)
// emit ν-events. User-input errors (NotFound, InvalidInput, LoginFailed)
// are not system conditions — they don't need CNS observability.

impl ServiceError {
    /// Emit a ν-event for CNS-observable errors.
    ///
    /// Returns `None` for user-input errors that don't represent system
    /// conditions. Returns `Some(NuEvent)` for infrastructure, inference,
    /// CNS, storage, and security errors the CNS can act on.
    ///
    /// The observer WebID is freshly generated per event — these are
    /// system-level observations, not agent-specific.
    pub fn nu_event(&self) -> Option<hkask_types::event::NuEvent> {
        use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
        use hkask_types::id::WebID;

        let (namespace, path_suffix, observation) = match self {
            // ── Inference domain ──────────────────────────────────────
            ServiceError::InferencePort(e) => (
                "cns.inference",
                "error",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::Embedding(e) => (
                "cns.inference",
                "error.embedding",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── CNS domain ────────────────────────────────────────────
            ServiceError::Cns(msg) => (
                "cns.cybernetics",
                "error",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Gas(e) => (
                "cns.gas",
                "error",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── Storage domain ────────────────────────────────────────
            ServiceError::Storage(e) => (
                "cns.cybernetics",
                "error.storage",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::Infra(e) => (
                "cns.cybernetics",
                "error.infra",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── Memory domain ─────────────────────────────────────────
            ServiceError::EpisodicMemory(e) => (
                "cns.memory.encode",
                "error.episodic",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::SemanticMemory(e) => (
                "cns.memory.encode",
                "error.semantic",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::Consolidation(msg) => (
                "cns.memory.encode",
                "error.consolidation",
                serde_json::json!({ "message": msg }),
            ),

            // ── Security / OCAP domain ────────────────────────────────
            ServiceError::Acp(e) => (
                "cns.sovereignty",
                "error.acp",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::Consent(e) => (
                "cns.sovereignty",
                "error.consent",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── Agent / Pod domain ────────────────────────────────────
            ServiceError::AgentRegistry(e) => (
                "cns.agent_pod",
                "error.registry_load",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::Pod(e) => (
                "cns.agent_pod",
                "error",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── Template domain ───────────────────────────────────────
            ServiceError::Template(e) => (
                "cns.template",
                "error",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── Spec domain ───────────────────────────────────────────
            ServiceError::Spec(e) => (
                "cns.spec",
                "error",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── Goal domain ───────────────────────────────────────────
            ServiceError::GoalRepo(e) => (
                "cns.goal",
                "error",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── Keystore / Config ─────────────────────────────────────
            ServiceError::Keystore(msg) => (
                "cns.cybernetics",
                "error.keystore",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Config(msg) => (
                "cns.cybernetics",
                "error.config",
                serde_json::json!({ "message": msg }),
            ),

            // ── Rate limiting ─────────────────────────────────────────
            ServiceError::RateLimited(msg) => (
                "cns.cybernetics.backpressure",
                "rate_limited",
                serde_json::json!({ "message": msg }),
            ),

            // ── User-input errors — NOT system conditions ─────────────
            // These return None: they don't represent system health.
            ServiceError::EscalationNotFound(_)
            | ServiceError::AgentNotFound(_)
            | ServiceError::InvalidAgentType(_)
            | ServiceError::AgentRegistrationFailed(_)
            | ServiceError::PodNotFound(_)
            | ServiceError::UserNotFound(_)
            | ServiceError::LoginFailed(_)
            | ServiceError::InvalidPassphrase(_)
            | ServiceError::ValidationError(_)
            | ServiceError::InvalidWebID(_) => return None,

            // ── Pipeline / operational errors — system conditions ─────
            ServiceError::RegistryInitFailed(msg) => (
                "cns.cybernetics",
                "error.registry_init",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::RegistryLoadFailed(msg) => (
                "cns.cybernetics",
                "error.registry_load",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Archival(msg) => (
                "cns.cybernetics",
                "error.archival",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Embed(msg) => (
                "cns.pipeline",
                "error.embed",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Compose(msg) => (
                "cns.pipeline",
                "error.compose",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Skill(msg) => (
                "cns.pipeline",
                "error.skill",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Verification(msg) => (
                "cns.sovereignty",
                "error.verification",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Wallet(msg) => (
                "cns.wallet.balance",
                "error",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Matrix(msg) => (
                "cns.cybernetics",
                "error.matrix",
                serde_json::json!({ "message": msg }),
            ),

            // ── MCP tool errors ─────────────────────────────────────
            ServiceError::McpTool {
                kind,
                server,
                tool,
                message,
            } => (
                "cns.tool",
                "error",
                serde_json::json!({ "kind": kind.to_string(), "server": server, "tool": tool, "message": message }),
            ),

            // ── Transparent wrappers not explicitly matched above ──────
            // These carry domain semantics from upstream crates.
            // Default: emit as cybernetics error with the Display message.
            ServiceError::Metacognition(e) => (
                "cns.curation",
                "error.metacognition",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::Escalation(e) => (
                "cns.curation",
                "error.escalation",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::Registry(e) => (
                "cns.template",
                "error.registry",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::Triple(e) => (
                "cns.memory.encode",
                "error.triple",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::UserStore(e) => (
                "cns.cybernetics",
                "error.user_store",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::ConsentStore(e) => (
                "cns.sovereignty",
                "error.consent_store",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::SovereigntyStore(e) => (
                "cns.sovereignty",
                "error.sovereignty_store",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::NuEvent(e) => (
                "cns.cybernetics",
                "error.nu_event",
                serde_json::json!({ "error": e.to_string() }),
            ),
            ServiceError::AgentRegistryStore(e) => (
                "cns.agent_pod",
                "error.agent_registry_store",
                serde_json::json!({ "error": e.to_string() }),
            ),
        };

        let span = Span::new(SpanNamespace::new(namespace), path_suffix);
        Some(NuEvent::new(
            WebID::new(),
            span,
            Phase::Sense,
            observation,
            0,
        ))
    }
}
