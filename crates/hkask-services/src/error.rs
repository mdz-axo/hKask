//! Unified domain error hierarchy for hKask service operations.
//!
//! `ServiceError` composes from all domain crate errors. Surface layers
//! (CLI, API) adapt `ServiceError` into their own presentation types:
//!
//! - CLI: `impl From<ServiceError> for CuratorError`, `AgentError`, etc.
//! - API: `impl From<ServiceError> for ApiError` (maps to HTTP status codes)
//!
//! MCP servers continue using `anyhow` for isolated process errors and do
//! NOT depend on this crate.

use thiserror::Error;

/// Unified domain error for all service operations.
///
/// This replaces the 7 CLI error enums and the API `ApiError` with a single
/// canonical type. Surface adapters translate `ServiceError` into presentation
/// format (terminal output, HTTP response).
#[derive(Debug, Error)]
pub enum ServiceError {
    // ── Curator domain ──────────────────────────────────────────────────
    #[error("Escalation not found: {0}")]
    EscalationNotFound(String),

    #[error(transparent)]
    Escalation(#[from] hkask_agents::EscalationError),

    #[error(transparent)]
    Metacognition(#[from] hkask_agents::curator_agent::metacognition::MetacognitionError),

    // ── Agent domain ────────────────────────────────────────────────────
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error(transparent)]
    AgentRegistry(#[from] hkask_agents::registry_loader::RegistryLoaderError),

    #[error(transparent)]
    Acp(#[from] hkask_agents::acp::AcpError),

    // ── Storage domain ───────────────────────────────────────────────────
    #[error(transparent)]
    Storage(#[from] hkask_storage::DatabaseError),

    #[error(transparent)]
    Registry(#[from] hkask_storage::AgentRegistryError),

    #[error(transparent)]
    GoalRepo(#[from] hkask_storage::goals::GoalRepositoryError),

    // ── CNS domain ──────────────────────────────────────────────────────
    #[error("CNS operation failed: {0}")]
    Cns(String),

    // ── Inference domain ────────────────────────────────────────────────
    #[error("Inference failed: {0}")]
    Inference(String),

    // ── Infrastructure ──────────────────────────────────────────────────
    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
}
