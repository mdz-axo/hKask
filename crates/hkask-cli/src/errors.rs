//! Structured error types for hkask-cli commands
//!
//! Composes from domain crate errors where possible.
//! Shallow string-wrappers are cut — each command module uses
//! the domain error type directly or a local enum with `#[from]` composition.

use thiserror::Error;

/// Errors that can occur during agent operations
///
/// P3.5: most variants are now `#[from]`-style wrappers around typed upstream
/// errors (`AcpError`, `AgentRegistryError`, `RegistryError`,
/// `RegistryLoaderError`, `uuid::Error`). The remaining `String` variants are
/// sentinels for *user-facing* input errors (unknown agent name, unknown
/// agent kind) — those don't come from a typed upstream source.
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Invalid agent type: {0}")]
    InvalidType(String),

    /// User-visible registration/unregistration failure. The `Display` impl
    /// carries the upstream error; the `source()` chain keeps the typed
    /// original for programmatic matching via the `From` impls below.
    #[error("Agent registration failed: {0}")]
    RegistrationFailed(String),

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

    /// Upstream infrastructure failure (DB, IO, serialization, etc.).
    /// P3.5: replaces stringified error mapping at the call site.
    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// P8 invariant: `From<hkask_agents::acp::AcpError>` produces a
    /// `RegistrationFailed` whose rendered message includes the upstream
    /// cause, so a CLI user sees the actual `AcpError` text rather than
    /// "Agent registration failed: " with no body.
    #[test]
    fn from_acp_error_includes_cause_in_display() {
        let acp_err =
            hkask_agents::acp::AcpError::AgentAlreadyRegistered(hkask_types::WebID::new());
        let agent_err: AgentError = acp_err.into();
        let rendered = agent_err.to_string();
        assert!(
            rendered.contains("Agent registration failed"),
            "rendered message should include the variant prefix: {rendered}"
        );
        assert!(
            rendered.contains("already registered") || rendered.contains("already"),
            "rendered message should include the upstream cause text: {rendered}"
        );
    }

    /// P8 invariant: `From<uuid::Error>` produces a `RegistrationFailed` with
    /// the "Invalid WebID" prefix so a user can distinguish bad-UUID input
    /// from other registration failures.
    #[test]
    fn from_uuid_error_includes_invalid_webid_prefix() {
        let uuid_err = uuid::Uuid::parse_str("not-a-uuid").unwrap_err();
        let agent_err: AgentError = uuid_err.into();
        assert!(matches!(agent_err, AgentError::RegistrationFailed(_)));
        let rendered = agent_err.to_string();
        assert!(
            rendered.contains("Invalid WebID"),
            "rendered message should include the 'Invalid WebID' prefix: {rendered}"
        );
    }

    /// P8 invariant: the `#[from]` wrappers (`Registry`, `RegistryLoader`)
    /// are `#[error(transparent)]` so the underlying error is rendered
    /// directly, not wrapped in an extra "AgentError" prefix.
    #[test]
    fn registry_error_is_transparent() {
        let inner = hkask_types::InfrastructureError::Database("test".to_string());
        let inner_rendered = inner.to_string();
        let reg_err: RegistryError = inner.into();
        let agent_err: AgentError = reg_err.into();
        assert_eq!(agent_err.to_string(), inner_rendered);
    }
}
