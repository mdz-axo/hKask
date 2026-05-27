//! Structured error types for hkask-cli commands
//!
//! This module provides typed errors for all CLI operations, replacing
//! the previous `Result<T, String>` pattern with proper error enums.

use thiserror::Error;

/// Errors that can occur during pod operations
#[derive(Debug, Error)]
pub enum PodError {
    #[error("Pod not found: {0}")]
    NotFound(String),

    #[error("Pod creation failed: {0}")]
    CreationFailed(String),

    #[error("Pod activation failed: {0}")]
    ActivationFailed(String),

    #[error("Pod deactivation failed: {0}")]
    DeactivationFailed(String),

    #[error("Invalid pod state: {0}")]
    InvalidState(String),

    #[error("Pod manager error: {0}")]
    ManagerError(String),
}

/// Errors that can occur during template operations
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Template registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Template search failed: {0}")]
    SearchFailed(String),

    #[error("Invalid template type: {0}")]
    InvalidType(String),

    #[error("Template mapping failed: {0}")]
    MappingFailed(String),
}

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

/// Errors that can occur during chat operations
#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Chat failed: {0}")]
    ChatFailed(String),

    #[error("Inference error: {0}")]
    InferenceError(String),

    #[error("Pod context error: {0}")]
    PodContextError(String),

    #[error("Russell adapter error: {0}")]
    RussellAdapterError(String),

    #[error("Russell session error: {0}")]
    RussellSessionError(String),
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

/// Top-level CLI error that wraps all command-specific errors
#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Pod(#[from] PodError),

    #[error(transparent)]
    Template(#[from] TemplateError),

    #[error(transparent)]
    Agent(#[from] AgentError),

    #[error(transparent)]
    Ensemble(#[from] EnsembleError),

    #[error(transparent)]
    Curator(#[from] CuratorError),

    #[error(transparent)]
    Chat(#[from] ChatError),

    #[error(transparent)]
    Registry(#[from] RegistryError),

    #[error(transparent)]
    User(#[from] UserError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<String> for CliError {
    fn from(s: String) -> Self {
        CliError::Config(s)
    }
}

impl From<&str> for CliError {
    fn from(s: &str) -> Self {
        CliError::Config(s.to_string())
    }
}
