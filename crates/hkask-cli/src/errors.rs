//! Structured error types for hkask-cli commands
//!
//! Composes from domain crate errors where possible.
//! Shallow string-wrappers are cut — each command module uses
//! the domain error type directly or a local enum with `#[from]` composition.

use std::sync::PoisonError;
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

    /// Upstream registry init/load failure.
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
///
/// P3.5: most variants are now `#[from]`-style wrappers around typed upstream
/// errors (`StandingSessionError`). The remaining `String` variants are
/// sentinels for *user-facing* input errors (e.g. missing config file).
#[derive(Debug, Error)]
pub enum EnsembleError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Upstream standing-session bootstrap failure.
    /// P3.5: replaces `.map_err(|e| SessionCreationFailed(e.to_string()))` calls.
    #[error(transparent)]
    Standing(#[from] hkask_ensemble::StandingSessionError),
}

/// Errors that can occur during curator operations
///
/// P3.5: most variants are now `#[from]`-style wrappers around typed upstream
/// errors (`EscalationError`, `MetacognitionError`, `RegistryError`). The
/// remaining `String` variants are sentinels for *user-facing* input errors
/// (e.g. unknown escalation id).
#[derive(Debug, Error)]
pub enum CuratorError {
    #[error("Escalation not found: {0}")]
    EscalationNotFound(String),

    /// Upstream registry / database failure (DB open, IO, schema).
    /// P3.5: replaces the `.map_err(|e| DatabaseError(e.to_string()))` calls
    /// in `commands/curator.rs` and `commands/config.rs::open_registry_db`.
    #[error(transparent)]
    Registry(#[from] RegistryError),

    /// Upstream escalation-queue failure (`EscalationQueue::new`,
    /// `list_pending`, `resolve`, `dismiss`).
    /// P3.5: replaces the `.map_err(|e| DatabaseError/EscalationNotFound/...
    /// (e.to_string())` calls in `commands/curator.rs`.
    #[error(transparent)]
    Escalation(#[from] hkask_agents::EscalationError),

    /// Upstream metacognition-loop failure.
    /// P3.5: replaces the `.map_err(|e| MetacognitionFailed(e.to_string()))`
    /// call in `commands/curator.rs::curator_metacognition`.
    #[error(transparent)]
    Metacognition(#[from] hkask_agents::curator_agent::metacognition::MetacognitionError),

    /// Upstream service-layer failure (ServiceContext::build, config resolution).
    /// Manual `From` impl exists below (maps ServiceError variants to CuratorError).
    #[error("Service error: {0}")]
    Service(hkask_services::ServiceError),
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
///
/// P3.5: String-payload variants for upstream typed errors have been replaced
/// with `#[from]`-style transparent wrappers (`Store`, `Infra`). The remaining
/// `String` variants are sentinels for *user-facing* input errors:
/// - `NotFound` — replicant not found by name
/// - `SessionNotFound` — session not found by ID
/// - `LoginFailed` — deliberately opaque login failure (no source leak)
/// - `InvalidPassphrase` — passphrase validation failure (from `RegistrationError`)
/// - `ValidationError` — registration field validation failure (from `RegistrationError`)
///
/// `RegistrationFailed(String)` and `DatabaseError(String)` are replaced by the
/// typed `Store(#[from] UserStoreError)` and `Infra(#[from] InfrastructureError)`
/// wrappers, which render the upstream error directly without double-wrapping.
#[derive(Debug, Error)]
pub enum UserError {
    // ── Sentinels for user-facing input errors ──────────────────────────────
    /// Replicant identity not found by name.
    #[error("User not found: {0}")]
    NotFound(String),

    /// Session not found by ID.
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Login failure — deliberately opaque to prevent information leakage.
    /// The source `UserStoreError` is dropped; the message is always
    /// "Invalid credentials" regardless of the underlying cause.
    #[error("Login failed: {0}")]
    LoginFailed(String),

    /// Passphrase validation failure from `validate_passphrase`.
    /// Context-dependent: same `RegistrationError` type maps here for passphrase
    /// errors and to `ValidationError` for field-validation errors, so we
    /// can't use `#[from]` — the call site decides the variant.
    #[error("Invalid passphrase: {0}")]
    InvalidPassphrase(String),

    /// Registration field validation failure from `validate_registration`.
    /// Context-dependent: same `RegistrationError` type maps here for field
    /// errors and to `InvalidPassphrase` for passphrase errors.
    #[error("Validation error: {0}")]
    ValidationError(String),

    // ── Typed upstream wrappers — #[from] composition ─────────────────────
    /// Upstream store failure (registration, lookup, session management).
    /// P3.5: replaces `RegistrationFailed(String)` and `DatabaseError(String)`.
    /// The upstream `UserStoreError` already carries domain semantics
    /// (`NotFound`, `ReplicantNameTaken`, `InvalidCredentials`, `Encryption`).
    #[error(transparent)]
    Store(#[from] hkask_storage::UserStoreError),

    /// Upstream infrastructure failure (lock poisoning, DB, IO).
    /// P3.5: replaces `DatabaseError(format!("Lock poisoned: {}", e))`.
    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
}

/// Direct conversion for lock poisoning — avoids the two-step
/// `PoisonError → InfrastructureError → UserError` chain that `?` cannot
/// perform in a single step.
impl<T> From<PoisonError<T>> for UserError {
    fn from(_: PoisonError<T>) -> Self {
        UserError::Infra(hkask_types::InfrastructureError::LockPoisoned)
    }
}

// ── Service layer adapters ──────────────────────────────────────────────
// These `From<ServiceError>` impls allow CLI command functions that call
// service operations to propagate errors with `?`. As service operations
// are extracted, these adapters ensure seamless error conversion.

impl From<hkask_services::ServiceError> for CuratorError {
    fn from(e: hkask_services::ServiceError) -> Self {
        use hkask_services::ServiceError as SE;
        match e {
            SE::EscalationNotFound(id) => CuratorError::EscalationNotFound(id),
            SE::Escalation(err) => CuratorError::Escalation(err),
            SE::Metacognition(err) => CuratorError::Metacognition(err),
            SE::Infra(err) => CuratorError::Registry(RegistryError::Infra(err)),
            SE::Storage(err) => {
                CuratorError::Registry(RegistryError::DatabaseError(err.to_string()))
            }
            other => CuratorError::Registry(RegistryError::DatabaseError(other.to_string())),
        }
    }
}

impl From<hkask_services::ServiceError> for AgentError {
    fn from(e: hkask_services::ServiceError) -> Self {
        use hkask_services::ServiceError as SE;
        match e {
            SE::AgentNotFound(name) => AgentError::NotFound(name),
            SE::InvalidAgentType(t) => AgentError::InvalidType(t),
            SE::AgentRegistrationFailed(msg) => AgentError::RegistrationFailed(msg),
            SE::Acp(err) => AgentError::RegistrationFailed(err.to_string()),
            SE::AgentRegistry(err) => AgentError::RegistryLoader(err),
            SE::AgentRegistryStore(err) => AgentError::RegistrationFailed(err.to_string()),
            SE::Infra(err) => AgentError::Registry(RegistryError::Infra(err)),
            other => AgentError::RegistrationFailed(other.to_string()),
        }
    }
}

impl From<hkask_services::ServiceError> for EnsembleError {
    fn from(e: hkask_services::ServiceError) -> Self {
        use hkask_services::ServiceError as SE;
        match e {
            SE::SessionNotFound(id) => EnsembleError::SessionNotFound(id),
            SE::StandingSession(err) => EnsembleError::Standing(err),
            other => EnsembleError::Standing(hkask_ensemble::StandingSessionError::Bootstrap(
                other.to_string(),
            )),
        }
    }
}

impl From<hkask_services::ServiceError> for UserError {
    fn from(e: hkask_services::ServiceError) -> Self {
        use hkask_services::ServiceError as SE;
        match e {
            SE::UserNotFound(id) => UserError::NotFound(id),
            SE::LoginFailed(msg) => UserError::LoginFailed(msg),
            SE::InvalidPassphrase(msg) => UserError::InvalidPassphrase(msg),
            SE::ValidationError(msg) => UserError::ValidationError(msg),
            SE::UserStore(err) => UserError::Store(err),
            SE::Infra(err) => UserError::Infra(err),
            other => UserError::Infra(hkask_types::InfrastructureError::Database(
                other.to_string(),
            )),
        }
    }
}
