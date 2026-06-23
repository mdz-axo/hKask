//! Unified domain error hierarchy for hKask service operations.
//! # REQ: P8 (Semantic Grounding) — every error variant is a distinct semantic state.
//! expect: "Every service error variant represents a distinct semantic state"
//!
//! `ServiceError` composes from all domain crate errors. Surface layers
//! (CLI, API) use `ServiceError` directly — CLI commands return
//! `ServiceError`, API routes return `ServiceErrorResponse` (a newtype
//! implementing Axum's `IntoResponse`). No surface-specific error enums.
//!
//! - CLI: commands return `Result<_, ServiceError>`, rendered via `Display`
//! - API: routes return `Result<_, ServiceErrorResponse>`, mapped to HTTP
//!   status codes via `From<ServiceError> for ApiError`
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

use hkask_ports::{EmbeddingGenerationError, InferenceError};
use hkask_types::InfrastructureError;
use hkask_types::McpErrorKind;

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
    #[error("Escalation not found: {message}")]
    EscalationNotFound {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Upstream escalation-queue error.
    #[error("Escalation error: {message}")]
    Escalation { message: String },

    /// Upstream metacognition-loop error.
    #[error("Metacognition error: {message}")]
    Metacognition { message: String },

    // ── Agent / A2A domain ───────────────────────────────────────────────
    /// Agent not found by name.
    #[error("Agent not found: {message}")]
    AgentNotFound {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Invalid agent type.
    #[error("Invalid agent type: {message}")]
    InvalidAgentType {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Agent registration failed.
    #[error("Agent registration failed: {message}")]
    AgentRegistrationFailed {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Upstream A2A error.
    #[error("A2A error: {message}")]
    A2A { message: String },

    /// Upstream agent-registry loader error.
    #[error("Agent registry error: {message}")]
    AgentRegistry { message: String },

    /// Upstream agent registry store error.
    #[error("Agent registry store error: {message}")]
    AgentRegistryStore { message: String },

    /// Upstream consent error.
    #[error("Consent error: {message}")]
    Consent { message: String },

    // ── Storage domain ──────────────────────────────────────────────────
    /// Upstream database error.
    #[error("Storage error: {message}")]
    Storage { message: String },

    /// Upstream template registry error.
    #[error("Registry error: {message}")]
    Registry { message: String },

    /// Upstream template store error.
    #[error("Template error: {message}")]
    Template { message: String },

    /// Upstream goal repository error.
    #[error("Goal repo error: {message}")]
    GoalRepo { message: String },

    /// Upstream triple store error.
    #[error("Triple error: {message}")]
    Triple { message: String },

    /// Upstream user store error.
    #[error("User store error: {message}")]
    UserStore { message: String },

    /// Upstream consent store error.
    #[error("Consent store error: {message}")]
    ConsentStore { message: String },

    /// Upstream sovereignty store error.
    #[error("Sovereignty store error: {message}")]
    SovereigntyStore { message: String },

    /// Upstream spec error.
    #[error("Spec error: {message}")]
    Spec { message: String },

    // ── Memory domain ────────────────────────────────────────────────────
    /// Upstream episodic memory error.
    #[error("Episodic memory error: {message}")]
    EpisodicMemory { message: String },

    /// Upstream semantic memory error.
    #[error("Semantic memory error: {message}")]
    SemanticMemory { message: String },

    /// Consolidation pipeline execution failed.
    #[error("Consolidation failed: {message}")]
    Consolidation {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── CNS domain ──────────────────────────────────────────────────────
    /// CNS operation failed.
    #[error("CNS operation failed: {message}")]
    Cns {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Keystore secret resolution failed.
    #[error("Keystore resolution failed: {message}")]
    Keystore {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Upstream energy budget error.
    #[error("Energy budget error: {message}")]
    Gas { message: String },

    // ── Pod domain ────────────────────────────────────────────────────
    /// Pod not found by ID.
    #[error("Pod not found: {message}")]
    PodNotFound {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Upstream agent pod error.
    #[error("Pod error: {message}")]
    Pod { message: String },

    // ── Inference domain ────────────────────────────────────────────────
    /// Upstream inference port error.
    #[error("Inference error: {message}")]
    InferencePort { message: String, retryable: bool },

    /// Upstream embedding generation error.
    #[error("Embedding error: {message}")]
    Embedding { message: String, retryable: bool },

    // ── User domain ─────────────────────────────────────────────────────
    /// User not found by name.
    #[error("User not found: {message}")]
    UserNotFound {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Login failed (deliberately opaque).
    #[error("Login failed: {message}")]
    LoginFailed {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Invalid passphrase.
    #[error("Invalid passphrase: {message}")]
    InvalidPassphrase {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Validation error.
    #[error("Validation error: {message}")]
    ValidationError {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Invalid UUID format for WebID parsing.
    #[error("Invalid WebID: {message}")]
    InvalidWebID {
        #[source]
        source: Option<uuid::Error>,
        message: String,
    },

    // ── Infrastructure ──────────────────────────────────────────────────
    /// Upstream infrastructure error (lock poisoning, IO, etc.).
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    /// Registry initialization failure (no upstream typed source).
    #[error("Registry initialization failed: {message}")]
    RegistryInitFailed {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// Registry load failure (no upstream typed source).
    #[error("Registry load failed: {message}")]
    RegistryLoadFailed {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Archival domain ──────────────────────────────────────────────────
    /// GitHub archival operation failed (API call, encoding, credential resolution).
    #[error("Archival failed: {message}")]
    Archival {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Embedding pipeline domain ─────────────────────────────────────────
    /// Embedding pipeline failed (config parsing, download, IO, batch processing).
    #[error("Embed failed: {message}")]
    Embed {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Style composition domain ────────────────────────────────────────
    /// Style composition failed (Jinja2 rendering, inference, validation).
    #[error("Compose failed: {message}")]
    Compose {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Skill domain ────────────────────────────────────────────────────────
    /// Skill operation failed (IO, front matter parsing, publish failure).
    #[error("Skill failed: {message}")]
    Skill {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Verification domain ─────────────────────────────────────────────────
    /// Sovereignty verification failed (manifest loading, assertion execution).
    #[error("Verification failed: {message}")]
    Verification {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Wallet domain ───────────────────────────────────────────────────
    /// Wallet operation failed.
    #[error("Wallet error: {message}")]
    Wallet {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// P2 affirmative consent denied for wallet operation.
    /// Returned when the user has not granted consent for the requested
    /// wallet operation (e.g., withdrawal signing per MUST-4).
    #[error("Consent denied for wallet operation: {message}")]
    ConsentDenied { message: String },

    // ── Backup domain ──────────────────────────────────────────────────
    /// Backup operation failed (CAS, serialization, config, CNS).
    #[error("Backup failed: {message}")]
    Backup {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Rate limiting ──────────────────────────────────────────────────────
    /// Operation rate limited (too soon after previous invocation).
    #[error("{message}")]
    RateLimited {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Configuration / setup ───────────────────────────────────────────
    /// Configuration or external service setup failed.
    #[error("Config error: {message}")]
    Config {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    // ── Matrix / communication ──────────────────────────────────────────
    /// Matrix homeserver operation failed (registration, connection, message send).
    #[error("Matrix error: {message}")]
    Matrix {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

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

    // ── Federation ────────────────────────────────────────────────────
    /// Federation lifecycle operation failed.
    #[error("Federation error: {message}")]
    Federation { message: String },
}

// ── From impls ──────────────────────────────────────────────────────
//
// Domain crate error conversions use explicit ServiceError::Variant
// construction rather than blanket From impls, keeping hkask-services-core
// decoupled from domain crates.

impl From<InferenceError> for ServiceError {
    fn from(e: InferenceError) -> Self {
        let retryable = matches!(
            e,
            InferenceError::Connection(_) | InferenceError::CircuitOpen(_)
        );
        ServiceError::InferencePort {
            message: e.to_string(),
            retryable,
        }
    }
}
impl From<EmbeddingGenerationError> for ServiceError {
    fn from(e: EmbeddingGenerationError) -> Self {
        let retryable = matches!(
            e,
            EmbeddingGenerationError::Connection(_) | EmbeddingGenerationError::Api(..)
        );
        ServiceError::Embedding {
            message: e.to_string(),
            retryable,
        }
    }
}

impl From<uuid::Error> for ServiceError {
    fn from(e: uuid::Error) -> Self {
        let msg = e.to_string();
        ServiceError::InvalidWebID {
            source: Some(e),
            message: msg,
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for ServiceError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        ServiceError::Infra(hkask_types::InfrastructureError::LockPoisoned)
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
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be a valid ServiceError variant
    /// post: returns true for retryable errors (network, rate-limit, keystore); false for non-retryable (not-found, validation, permission)
    pub fn is_retryable(&self) -> bool {
        match self {
            // ── Retryable ────────────────────────────────────────────
            ServiceError::InferencePort { retryable, .. } => *retryable,
            ServiceError::Embedding { retryable, .. } => *retryable,
            ServiceError::Infra(e) => matches!(e, hkask_types::InfrastructureError::Io(_)),
            ServiceError::RateLimited { .. } => true,
            ServiceError::Matrix { .. } => true, // Network operations may be transient
            ServiceError::Config { .. } => true, // Config resolution may succeed on retry
            ServiceError::Keystore { .. } => true, // Keychain may be temporarily unavailable
            ServiceError::McpTool { kind, .. } => kind.is_retryable(),

            // ── Non-retryable ────────────────────────────────────────
            // User-input errors: retrying won't change the outcome
            ServiceError::EscalationNotFound { .. }
            | ServiceError::AgentNotFound { .. }
            | ServiceError::InvalidAgentType { .. }
            | ServiceError::AgentRegistrationFailed { .. }
            | ServiceError::PodNotFound { .. }
            | ServiceError::UserNotFound { .. }
            | ServiceError::LoginFailed { .. }
            | ServiceError::InvalidPassphrase { .. }
            | ServiceError::ValidationError { .. }
            | ServiceError::InvalidWebID { .. } => false,

            // Storage errors: database corruption, schema issues, encryption
            // failures are not transient
            ServiceError::Storage { .. } => false,

            // Permission/security: retrying won't grant capabilities
            ServiceError::A2A { .. } | ServiceError::Consent { .. } => false,

            // P2 consent denied: retrying won't grant consent
            ServiceError::ConsentDenied { .. } => false,

            // CNS energy exhaustion: retrying would waste more gas
            ServiceError::Gas { .. } => false,

            // Pipeline/operational errors: generally non-retryable
            // (registry init failure, archival failure, embed failure)
            ServiceError::RegistryInitFailed { .. }
            | ServiceError::RegistryLoadFailed { .. }
            | ServiceError::Archival { .. }
            | ServiceError::Embed { .. }
            | ServiceError::Compose { .. }
            | ServiceError::Skill { .. }
            | ServiceError::Verification { .. }
            | ServiceError::Wallet { .. }
            | ServiceError::Cns { .. }
            | ServiceError::Consolidation { .. }
            | ServiceError::Backup { .. } => false,

            // ── Delegate to inner error for transparent wrappers ──────
            // Domain errors may have their own retryability semantics.
            // Default conservative: non-retryable unless proven otherwise.
            ServiceError::Escalation { .. }
            | ServiceError::Metacognition { .. }
            | ServiceError::AgentRegistry { .. }
            | ServiceError::AgentRegistryStore { .. }
            | ServiceError::Registry { .. }
            | ServiceError::Template { .. }
            | ServiceError::GoalRepo { .. }
            | ServiceError::Triple { .. }
            | ServiceError::UserStore { .. }
            | ServiceError::ConsentStore { .. }
            | ServiceError::SovereigntyStore { .. }
            | ServiceError::Spec { .. }
            | ServiceError::EpisodicMemory { .. }
            | ServiceError::SemanticMemory { .. }
            | ServiceError::Pod { .. }
            | ServiceError::Federation { .. } => false,
        }
    }
}

// ── Internationalization (i18n) message keys ──────────────────────────
//
// Each variant carries a stable, language-independent key that surface
// adapters can use for translation lookup. The `#[error("...")]` strings
// are English fallbacks; `message_key()` returns the canonical key.

impl ServiceError {
    /// Returns a stable i18n key for this error variant.
    ///
    /// Surface adapters use this key for translation lookup instead of
    /// parsing `Display` strings. Keys follow the pattern
    /// `error.<domain>.<condition>`.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be a valid ServiceError variant
    /// post: returns &'static str i18n key (e.g., "error.curator.escalation_not_found")
    pub fn message_key(&self) -> &'static str {
        match self {
            // ── Curator domain ──────────────────────────────────────
            ServiceError::EscalationNotFound { .. } => "error.curator.escalation_not_found",
            ServiceError::Escalation { .. } => "error.curator.escalation",
            ServiceError::Metacognition { .. } => "error.curator.metacognition",

            // ── Agent / A2A domain ───────────────────────────────────
            ServiceError::AgentNotFound { .. } => "error.agent.not_found",
            ServiceError::InvalidAgentType { .. } => "error.agent.invalid_type",
            ServiceError::AgentRegistrationFailed { .. } => "error.agent.registration_failed",
            ServiceError::A2A { .. } => "error.agent.a2a",
            ServiceError::AgentRegistry { .. } => "error.agent.registry_load",
            ServiceError::AgentRegistryStore { .. } => "error.agent.registry_store",
            ServiceError::Consent { .. } => "error.agent.consent",

            // ── Storage domain ──────────────────────────────────────
            ServiceError::Storage { .. } => "error.storage.database",
            ServiceError::Registry { .. } => "error.storage.registry",
            ServiceError::Template { .. } => "error.storage.template",
            ServiceError::GoalRepo { .. } => "error.storage.goal_repo",
            ServiceError::Triple { .. } => "error.storage.triple",
            ServiceError::UserStore { .. } => "error.storage.user_store",
            ServiceError::ConsentStore { .. } => "error.storage.consent_store",
            ServiceError::SovereigntyStore { .. } => "error.storage.sovereignty_store",
            ServiceError::Spec { .. } => "error.storage.spec",

            // ── Memory domain ──────────────────────────────────────
            ServiceError::EpisodicMemory { .. } => "error.memory.episodic",
            ServiceError::SemanticMemory { .. } => "error.memory.semantic",
            ServiceError::Consolidation { .. } => "error.memory.consolidation",

            // ── CNS domain ──────────────────────────────────────────
            ServiceError::Cns { .. } => "error.cns.operation",
            ServiceError::Keystore { .. } => "error.cns.keystore",
            ServiceError::Gas { .. } => "error.cns.gas",

            // ── Pod domain ──────────────────────────────────────────
            ServiceError::PodNotFound { .. } => "error.pod.not_found",
            ServiceError::Pod { .. } => "error.pod.operation",

            // ── Inference domain ────────────────────────────────────
            ServiceError::InferencePort { .. } => "error.inference.port",
            ServiceError::Embedding { .. } => "error.inference.embedding",

            // ── User domain ─────────────────────────────────────────
            ServiceError::UserNotFound { .. } => "error.user.not_found",
            ServiceError::LoginFailed { .. } => "error.user.login_failed",
            ServiceError::InvalidPassphrase { .. } => "error.user.invalid_passphrase",
            ServiceError::ValidationError { .. } => "error.user.validation",
            ServiceError::InvalidWebID { .. } => "error.user.invalid_webid",

            // ── Infrastructure ──────────────────────────────────────
            ServiceError::Infra(_) => "error.infra",
            ServiceError::RegistryInitFailed { .. } => "error.infra.registry_init",
            ServiceError::RegistryLoadFailed { .. } => "error.infra.registry_load",

            // ── Pipeline / operational ──────────────────────────────
            ServiceError::Archival { .. } => "error.pipeline.archival",
            ServiceError::Embed { .. } => "error.pipeline.embed",
            ServiceError::Compose { .. } => "error.pipeline.compose",
            ServiceError::Skill { .. } => "error.pipeline.skill",
            ServiceError::Verification { .. } => "error.pipeline.verification",
            ServiceError::Wallet { .. } => "error.pipeline.wallet",
            ServiceError::ConsentDenied { .. } => "error.pipeline.wallet.consent_denied",

            // ── Backup domain ──────────────────────────────────────
            ServiceError::Backup { .. } => "error.backup",

            // ── Rate limiting / config / communication ──────────────
            ServiceError::RateLimited { .. } => "error.rate_limited",
            ServiceError::Config { .. } => "error.config",
            ServiceError::Matrix { .. } => "error.communication.matrix",

            // ── MCP tool errors ─────────────────────────────────────
            ServiceError::McpTool { .. } => "error.mcp.tool",

            // ── Federation ───────────────────────────────────────────
            ServiceError::Federation { .. } => "error.federation",
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
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be a valid ServiceError variant
    /// post: returns Some(NuEvent) for system-level errors (inference, CNS, storage, infra); None for user-input errors (not-found, validation)
    pub fn nu_event(&self) -> Option<hkask_types::event::NuEvent> {
        use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
        use hkask_types::id::WebID;

        let (namespace, path_suffix, observation) = match self {
            // ── Inference domain ──────────────────────────────────────
            ServiceError::InferencePort { message, .. } => (
                "cns.inference",
                "error",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::Embedding { message, .. } => (
                "cns.inference",
                "error.embedding",
                serde_json::json!({ "message": message }),
            ),

            // ── CNS domain ────────────────────────────────────────────
            ServiceError::Cns { message: msg, .. } => (
                "cns.cybernetics",
                "error",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Gas { message, .. } => (
                "cns.gas",
                "error",
                serde_json::json!({ "message": message }),
            ),

            // ── Storage domain ────────────────────────────────────────
            ServiceError::Storage { message, .. } => (
                "cns.cybernetics",
                "error.storage",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::Infra(e) => (
                "cns.cybernetics",
                "error.infra",
                serde_json::json!({ "error": e.to_string() }),
            ),

            // ── Memory domain ─────────────────────────────────────────
            ServiceError::EpisodicMemory { message, .. } => (
                "cns.memory.encode",
                "error.episodic",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::SemanticMemory { message, .. } => (
                "cns.memory.encode",
                "error.semantic",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::Consolidation { message: msg, .. } => (
                "cns.memory.encode",
                "error.consolidation",
                serde_json::json!({ "message": msg }),
            ),

            // ── Security / OCAP domain ────────────────────────────────
            ServiceError::A2A { message, .. } => (
                "cns.sovereignty",
                "error.a2a",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::Consent { message, .. } => (
                "cns.sovereignty",
                "error.consent",
                serde_json::json!({ "message": message }),
            ),

            // ── Agent / Pod domain ────────────────────────────────────
            ServiceError::AgentRegistry { message, .. } => (
                "cns.agent_pod",
                "error.registry_load",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::Pod { message, .. } => (
                "cns.agent_pod",
                "error",
                serde_json::json!({ "message": message }),
            ),

            // ── Template domain ───────────────────────────────────────
            ServiceError::Template { message, .. } => (
                "cns.template",
                "error",
                serde_json::json!({ "message": message }),
            ),

            // ── Spec domain ───────────────────────────────────────────
            ServiceError::Spec { message, .. } => (
                "cns.spec",
                "error",
                serde_json::json!({ "message": message }),
            ),

            // ── Goal domain ───────────────────────────────────────────
            ServiceError::GoalRepo { message, .. } => (
                "cns.goal",
                "error",
                serde_json::json!({ "message": message }),
            ),

            // ── Keystore / Config ─────────────────────────────────────
            ServiceError::Keystore { message: msg, .. } => (
                "cns.cybernetics",
                "error.keystore",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Config { message: msg, .. } => (
                "cns.cybernetics",
                "error.config",
                serde_json::json!({ "message": msg }),
            ),

            // ── Rate limiting ─────────────────────────────────────────
            ServiceError::RateLimited { message: msg, .. } => (
                "cns.cybernetics.backpressure",
                "rate_limited",
                serde_json::json!({ "message": msg }),
            ),

            // ── User-input errors — NOT system conditions ─────────────
            // These return None: they don't represent system health.
            ServiceError::EscalationNotFound { .. }
            | ServiceError::AgentNotFound { .. }
            | ServiceError::InvalidAgentType { .. }
            | ServiceError::AgentRegistrationFailed { .. }
            | ServiceError::PodNotFound { .. }
            | ServiceError::UserNotFound { .. }
            | ServiceError::LoginFailed { .. }
            | ServiceError::InvalidPassphrase { .. }
            | ServiceError::ValidationError { .. }
            | ServiceError::InvalidWebID { .. } => return None,

            // ── Pipeline / operational errors — system conditions ─────
            ServiceError::RegistryInitFailed { message: msg, .. } => (
                "cns.cybernetics",
                "error.registry_init",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::RegistryLoadFailed { message: msg, .. } => (
                "cns.cybernetics",
                "error.registry_load",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Archival { message: msg, .. } => (
                "cns.cybernetics",
                "error.archival",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Embed { message: msg, .. } => (
                "cns.pipeline",
                "error.embed",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Compose { message: msg, .. } => (
                "cns.pipeline",
                "error.compose",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Skill { message: msg, .. } => (
                "cns.pipeline",
                "error.skill",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Verification { message: msg, .. } => (
                "cns.sovereignty",
                "error.verification",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Wallet { message: msg, .. } => (
                "cns.wallet.balance",
                "error",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::ConsentDenied { message: msg } => (
                "cns.wallet.withdrawal",
                "error.consent_denied",
                serde_json::json!({ "message": msg }),
            ),
            ServiceError::Matrix { message: msg, .. } => (
                "cns.cybernetics",
                "error.matrix",
                serde_json::json!({ "message": msg }),
            ),

            // ── Backup domain ─────────────────────────────────────
            ServiceError::Backup { message: msg, .. } => (
                "cns.cybernetics",
                "error.backup",
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

            // ── Federation ───────────────────────────────────────────
            ServiceError::Federation { message } => (
                "cns.federation.sync",
                "error",
                serde_json::json!({ "message": message }),
            ),

            // ── Remaining transparent wrappers ──────────────────────
            // Each carries domain semantics from upstream crates.
            // Every variant has an explicit arm — this match is exhaustive.
            ServiceError::Metacognition { message, .. } => (
                "cns.curation",
                "error.metacognition",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::Escalation { message, .. } => (
                "cns.curation",
                "error.escalation",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::Registry { message, .. } => (
                "cns.template",
                "error.registry",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::Triple { message, .. } => (
                "cns.memory.encode",
                "error.triple",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::UserStore { message, .. } => (
                "cns.cybernetics",
                "error.user_store",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::ConsentStore { message, .. } => (
                "cns.sovereignty",
                "error.consent_store",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::SovereigntyStore { message, .. } => (
                "cns.sovereignty",
                "error.sovereignty_store",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::AgentRegistryStore { message, .. } => (
                "cns.agent_pod",
                "error.agent_registry_store",
                serde_json::json!({ "message": message }),
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
