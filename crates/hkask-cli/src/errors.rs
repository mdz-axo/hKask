//! Structured error types for hkask-cli commands
//!
//! Composes from domain crate errors where possible.
//! Shallow string-wrappers are cut — each command module uses
//! the domain error type directly or a local enum with `#[from]` composition.

use thiserror::Error;

/// Errors that can occur during agent operations
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Agent registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Agent unregistration failed: {0}")]
    UnregistrationFailed(String),

    #[error("Invalid agent type: {0}")]
    InvalidType(String),

    #[error("Capability error: {0}")]
    CapabilityError(String),

    /// Upstream registry init/load failure (from `commands::config::init_registry`).
    /// P3.5: replaces the `.map_err(|e| CapabilityError(e.to_string()))` calls.
    #[error(transparent)]
    Registry(#[from] RegistryError),

    /// Upstream agent-registry loader failure (boot, load_and_register).
    /// P3.5: replaces the `.map_err(|e| CapabilityError(e.to_string()))` calls.
    #[error(transparent)]
    RegistryLoader(#[from] hkask_agents::registry_loader::RegistryLoaderError),
}

// P3.5: `From<...>` impls for the upstream error sources that `commands::agent.rs`
// propagates with `?`. These were previously stringified via
// `.map_err(|e| AgentError::CapabilityError(e.to_string()))`.
impl From<hkask_agents::acp::AcpError> for AgentError {
    fn from(e: hkask_agents::acp::AcpError) -> Self {
        AgentError::RegistrationFailed(e.to_string())
    }
}

impl From<hkask_storage::AgentRegistryError> for AgentError {
    fn from(e: hkask_storage::AgentRegistryError) -> Self {
        AgentError::RegistrationFailed(e.to_string())
    }
}

impl From<uuid::Error> for AgentError {
    fn from(e: uuid::Error) -> Self {
        AgentError::RegistrationFailed(format!("Invalid WebID: {e}"))
    }
}

/// Errors that can occur during ensemble operations
#[derive(Debug, Error)]
pub enum EnsembleError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session creation failed: {0}")]
    SessionCreationFailed(String),

    #[error("Message send failed: {0}")]
    MessageSendFailed(String),

    #[error("Deliberation failed: {0}")]
    DeliberationFailed(String),

    #[error("Invalid session state: {0}")]
    InvalidState(String),
}

/// Errors that can occur during curator operations
#[derive(Debug, Error)]
pub enum CuratorError {
    #[error("Escalation not found: {0}")]
    EscalationNotFound(String),

    #[error("Escalation resolution failed: {0}")]
    EscalationResolutionFailed(String),

    #[error("Metacognition failed: {0}")]
    MetacognitionFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Errors that can occur during registry operations
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Registry initialization failed: {0}")]
    InitFailed(String),

    #[error("Registry load failed: {0}")]
    LoadFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Schema error: {0}")]
    SchemaError(String),
}

/// Errors that can occur during user operations
#[derive(Debug, Error)]
pub enum UserError {
    #[error("User not found: {0}")]
    NotFound(String),

    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Login failed: {0}")]
    LoginFailed(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid passphrase: {0}")]
    InvalidPassphrase(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
