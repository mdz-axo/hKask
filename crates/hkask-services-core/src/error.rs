//! Unified domain error hierarchy for hKask service operations.
//! # REQ: P8 (Semantic Grounding) — every error variant is a distinct semantic state.
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
//! - Domain errors are boxed via `Box<dyn Error + Send + Sync>`. This keeps
//!   `hkask-services-core` lightweight — domain crates implement `From` impls
//!   in their own code, not here.
//! - Surface types (`Json<T>`, HTTP status codes, `println!` formatting)
//!   NEVER appear in `ServiceError` — those belong in surface adapters.
//! - `ServiceError` does NOT depend on surface types (CLI errors, API errors).
//!   Dependency direction: surface → service → domain. Never the reverse.

use hkask_types::InfrastructureError;
use thiserror::Error;

/// Unified domain error for all service operations.
#[derive(Debug, Error)]
pub enum ServiceError {
    // ── Curator domain ──────────────────────────────────────────────────
    #[error("Escalation not found: {message}")]
    EscalationNotFound { message: String },
    #[error("Escalation error: {message}")]
    Escalation { message: String },

    #[error("Metacognition error: {message}")]
    Metacognition { message: String },

    // ── Agent / A2A domain ───────────────────────────────────────────────
    #[error("Agent not found: {message}")]
    AgentNotFound { message: String },
    #[error("Invalid agent type: {message}")]
    InvalidAgentType { message: String },
    #[error("Agent registration failed: {message}")]
    AgentRegistrationFailed { message: String },
    #[error("A2A error: {message}")]
    A2A { message: String },
    #[error("Agent registry error: {message}")]
    AgentRegistry { message: String },
    #[error("Agent registry store error: {message}")]
    AgentRegistryStore { message: String },
    #[error("Consent error: {message}")]
    Consent { message: String },

    // ── Storage domain ──────────────────────────────────────────────────
    #[error("Storage error: {message}")]
    Storage { message: String },
    #[error("Registry error: {message}")]
    Registry { message: String },
    #[error("Template error: {message}")]
    Template { message: String },
    #[error("Goal repo error: {message}")]
    GoalRepo { message: String },
    #[error("Triple error: {message}")]
    Triple { message: String },
    #[error("User store error: {message}")]
    UserStore { message: String },
    #[error("Consent store error: {message}")]
    ConsentStore { message: String },
    #[error("Sovereignty store error: {message}")]
    SovereigntyStore { message: String },
    #[error("Spec error: {message}")]
    Spec { message: String },

    // ── Memory domain ────────────────────────────────────────────────────
    #[error("Episodic memory error: {message}")]
    EpisodicMemory { message: String },
    #[error("Semantic memory error: {message}")]
    SemanticMemory { message: String },
    #[error("Consolidation failed: {message}")]
    Consolidation { message: String },

    // ── CNS domain ──────────────────────────────────────────────────────
    #[error("CNS operation failed: {message}")]
    Cns { message: String },
    #[error("Keystore resolution failed: {message}")]
    Keystore {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("Energy budget error: {message}")]
    Gas { message: String },

    // ── Pod domain ────────────────────────────────────────────────────
    #[error("Pod not found: {message}")]
    PodNotFound { message: String },
    #[error("Pod error: {message}")]
    Pod { message: String },

    // ── Inference domain ────────────────────────────────────────────────
    #[error("Inference error: {message}")]
    InferencePort { message: String, retryable: bool },
    #[error("Embedding error: {message}")]
    Embedding { message: String, retryable: bool },

    // ── User domain ─────────────────────────────────────────────────────
    #[error("User not found: {message}")]
    UserNotFound { message: String },
    #[error("Login failed: {message}")]
    LoginFailed { message: String },
    #[error("Invalid passphrase: {message}")]
    InvalidPassphrase { message: String },
    #[error("Validation error: {message}")]
    ValidationError { message: String },
    #[error("Invalid WebID: {message}")]
    InvalidWebID { message: String },

    // ── Infrastructure ──────────────────────────────────────────────────
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
    #[error("Registry initialization failed: {message}")]
    RegistryInitFailed { message: String },
    #[error("Registry load failed: {message}")]
    RegistryLoadFailed { message: String },

    // ── Archival domain ──────────────────────────────────────────────────
    #[error("Archival failed: {message}")]
    Archival { message: String },

    // ── Embedding pipeline domain ─────────────────────────────────────────
    #[error("Embed failed: {message}")]
    Embed { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    // ── Style composition domain ────────────────────────────────────────
    #[error("Compose failed: {message}")]
    Compose { message: String },

    // ── Skill domain ────────────────────────────────────────────────────────
    #[error("Skill failed: {message}")]
    Skill { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    // ── Verification domain ─────────────────────────────────────────────────
    #[error("Verification failed: {message}")]
    Verification { message: String },

    // ── Wallet domain ───────────────────────────────────────────────────
    #[error("Wallet error: {message}")]
    Wallet { message: String },
    #[error("Consent denied for wallet operation: {message}")]
    ConsentDenied { message: String },

    // ── Backup domain ──────────────────────────────────────────────────
    #[error("Backup failed: {message}")]
    Backup { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    // ── Rate limiting ──────────────────────────────────────────────────────
    #[error("{message}")]
    RateLimited { message: String },

    // ── Configuration / setup ───────────────────────────────────────────
    #[error("Config error: {message}")]
    Config { message: String },

    // ── Matrix / communication ──────────────────────────────────────────
    #[error("Matrix error: {message}")]
    Matrix { message: String },

    // ── MCP tool errors ─────────────────────────────────────────────────
    #[error("{kind}: {message} (server={server}, tool={tool})")]
    McpTool {
        kind: hkask_types::McpErrorKind,
        server: String,
        tool: String,
        message: String,
    },
}

// ── From impls for std / uuid types (no domain deps) ──────────────────

impl From<uuid::Error> for ServiceError {
    fn from(e: uuid::Error) -> Self {
        ServiceError::InvalidWebID {
            message: e.to_string(),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for ServiceError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        ServiceError::Infra(InfrastructureError::LockPoisoned)
    }
}

// ── Retryability ───────────────────────────────────────────────────────

impl ServiceError {
    pub fn is_retryable(&self) -> bool {
        match self {
            ServiceError::InferencePort { retryable, .. } => *retryable,
            ServiceError::Embedding { retryable, .. } => *retryable,
            ServiceError::Infra(e) => matches!(e, InfrastructureError::Io(_)),
            ServiceError::RateLimited { .. } => true,
            ServiceError::Matrix { .. } => true,
            ServiceError::Config { .. } => true,
            ServiceError::Keystore { .. } => true,
            ServiceError::McpTool { kind, .. } => kind.is_retryable(),
            _ => false,
        }
    }

    pub fn message_key(&self) -> &'static str {
        match self {
            ServiceError::EscalationNotFound { .. } => "error.escalation.not_found",
            ServiceError::Escalation { .. } => "error.escalation",
            ServiceError::Metacognition { .. } => "error.metacognition",
            ServiceError::AgentNotFound { .. } => "error.agent.not_found",
            ServiceError::InvalidAgentType { .. } => "error.agent.invalid_type",
            ServiceError::AgentRegistrationFailed { .. } => "error.agent.registration",
            ServiceError::A2A { .. } => "error.a2a",
            ServiceError::AgentRegistry { .. } => "error.agent.registry",
            ServiceError::AgentRegistryStore { .. } => "error.agent.registry_store",
            ServiceError::Consent { .. } => "error.agent.consent",
            ServiceError::Storage { .. } => "error.storage",
            ServiceError::Registry { .. } => "error.registry",
            ServiceError::Template { .. } => "error.template",
            ServiceError::GoalRepo { .. } => "error.goal_repo",
            ServiceError::Triple { .. } => "error.triple",
            ServiceError::UserStore { .. } => "error.user_store",
            ServiceError::ConsentStore { .. } => "error.consent_store",
            ServiceError::SovereigntyStore { .. } => "error.sovereignty_store",
            ServiceError::Spec { .. } => "error.spec",
            ServiceError::EpisodicMemory { .. } => "error.episodic_memory",
            ServiceError::SemanticMemory { .. } => "error.semantic_memory",
            ServiceError::Consolidation { .. } => "error.consolidation",
            ServiceError::Cns { .. } => "error.cns",
            ServiceError::Keystore { .. } => "error.keystore",
            ServiceError::Gas { .. } => "error.gas",
            ServiceError::PodNotFound { .. } => "error.pod.not_found",
            ServiceError::Pod { .. } => "error.pod",
            ServiceError::InferencePort { .. } => "error.inference",
            ServiceError::Embedding { .. } => "error.embedding",
            ServiceError::UserNotFound { .. } => "error.user.not_found",
            ServiceError::LoginFailed { .. } => "error.login",
            ServiceError::InvalidPassphrase { .. } => "error.passphrase",
            ServiceError::ValidationError { .. } => "error.validation",
            ServiceError::InvalidWebID { .. } => "error.webid",
            ServiceError::Infra(_) => "error.infra",
            ServiceError::RegistryInitFailed { .. } => "error.registry.init",
            ServiceError::RegistryLoadFailed { .. } => "error.registry.load",
            ServiceError::Archival { .. } => "error.archival",
            ServiceError::Embed { .. } => "error.embed",
            ServiceError::Compose { .. } => "error.compose",
            ServiceError::Skill { .. } => "error.skill",
            ServiceError::Verification { .. } => "error.verification",
            ServiceError::Wallet { .. } => "error.wallet",
            ServiceError::ConsentDenied { .. } => "error.consent_denied",
            ServiceError::Backup { .. } => "error.backup",
            ServiceError::RateLimited { .. } => "error.rate_limited",
            ServiceError::Config { .. } => "error.config",
            ServiceError::Matrix { .. } => "error.matrix",
            ServiceError::McpTool { .. } => "error.mcp_tool",
        }
    }

    pub fn nu_event(&self) -> (&'static str, &'static str, String) {
        let key = self.message_key();
        let msg = match self {
            ServiceError::EscalationNotFound { message, .. }
            | ServiceError::Escalation { message, .. }
            | ServiceError::Metacognition { message, .. }
            | ServiceError::AgentNotFound { message, .. }
            | ServiceError::InvalidAgentType { message, .. }
            | ServiceError::AgentRegistrationFailed { message, .. }
            | ServiceError::A2A { message, .. }
            | ServiceError::AgentRegistry { message, .. }
            | ServiceError::AgentRegistryStore { message, .. }
            | ServiceError::Consent { message, .. }
            | ServiceError::Storage { message, .. }
            | ServiceError::Registry { message, .. }
            | ServiceError::Template { message, .. }
            | ServiceError::GoalRepo { message, .. }
            | ServiceError::Triple { message, .. }
            | ServiceError::UserStore { message, .. }
            | ServiceError::ConsentStore { message, .. }
            | ServiceError::SovereigntyStore { message, .. }
            | ServiceError::Spec { message, .. }
            | ServiceError::EpisodicMemory { message, .. }
            | ServiceError::SemanticMemory { message, .. }
            | ServiceError::Consolidation { message, .. }
            | ServiceError::Cns { message, .. }
            | ServiceError::Keystore { message, .. }
            | ServiceError::Gas { message, .. }
            | ServiceError::PodNotFound { message, .. }
            | ServiceError::Pod { message, .. }
            | ServiceError::InferencePort { message, .. }
            | ServiceError::Embedding { message, .. }
            | ServiceError::UserNotFound { message, .. }
            | ServiceError::LoginFailed { message, .. }
            | ServiceError::InvalidPassphrase { message, .. }
            | ServiceError::ValidationError { message, .. }
            | ServiceError::InvalidWebID { message, .. }
            | ServiceError::RegistryInitFailed { message, .. }
            | ServiceError::RegistryLoadFailed { message, .. }
            | ServiceError::Archival { message, .. }
            | ServiceError::Embed { message, .. }
            | ServiceError::Compose { message, .. }
            | ServiceError::Skill { message, .. }
            | ServiceError::Verification { message, .. }
            | ServiceError::Wallet { message, .. }
            | ServiceError::ConsentDenied { message, .. }
            | ServiceError::Backup { message, .. }
            | ServiceError::RateLimited { message, .. }
            | ServiceError::Config { message, .. }
            | ServiceError::Matrix { message, .. } => message.clone(),
            ServiceError::Infra(e) => return ("cns.cybernetics", key, e.to_string()),
            ServiceError::McpTool { message, .. } => message.clone(),
        };
        ("cns.cybernetics", key, msg)
    }
}
