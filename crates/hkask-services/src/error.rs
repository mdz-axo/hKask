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
use hkask_agents::ensemble::StandingSessionError;
use hkask_agents::pod::AgentPodError;
use hkask_agents::registry_loader::RegistryLoaderError;
use hkask_cns::EnergyError;
use hkask_memory::{EpisodicMemoryError, SemanticMemoryError};
use hkask_storage::EscalationError;
use hkask_storage::{
    AgentRegistryError, ConsentStoreError, DatabaseError, GoalRepositoryError, NuEventError,
    SovereigntyStoreError, SpecError, StandingSessionError as StorageStandingSessionError,
    TripleError, UserStoreError,
};
use hkask_templates::TemplateError;
use hkask_types::InfrastructureError;
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

    /// Upstream standing-session store error (storage layer).
    #[error(transparent)]
    StandingSessionStore(#[from] StorageStandingSessionError),

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

    // ── Ensemble domain ─────────────────────────────────────────────────
    /// Ensemble session not found.
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Improv operation failed.
    #[error("Improv error: {0}")]
    Improv(String),

    /// Upstream standing-session error (ensemble layer).
    #[error(transparent)]
    StandingSession(#[from] StandingSessionError),

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

    // ── Rate limiting ──────────────────────────────────────────────────────
    /// Operation rate limited (too soon after previous invocation).
    #[error("{0}")]
    RateLimited(String),
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
