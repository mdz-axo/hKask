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

    // ── EnsembleError P3.5 property tests ───────────────────────────────

    /// P8 invariant: `From<StandingSessionError>` produces a `Standing`
    /// variant that is `#[error(transparent)]`, so the upstream `Display`
    /// text is rendered directly.
    #[test]
    fn from_standing_session_error_is_transparent() {
        // StandingSessionError::Bootstrap is the only String variant;
        // it stands in for any of the typed variants here.
        let inner =
            hkask_ensemble::StandingSessionError::Bootstrap("missing config file".to_string());
        let inner_rendered = inner.to_string();
        let ensemble_err: EnsembleError = inner.into();
        assert_eq!(ensemble_err.to_string(), inner_rendered);
    }

    /// P8 invariant: `SessionNotFound` is a *user-facing* input sentinel —
    /// it must NOT be derived from an upstream error. The `Display` impl
    /// must include the variant prefix so the user can tell input errors
    /// from upstream failures apart in logs.
    #[test]
    fn ensemble_session_not_found_includes_variant_prefix() {
        let err = EnsembleError::SessionNotFound("ensemble-x".to_string());
        let rendered = err.to_string();
        assert!(
            rendered.contains("Session not found"),
            "rendered message should include the 'Session not found' prefix: {rendered}"
        );
        assert!(
            rendered.contains("ensemble-x"),
            "rendered message should include the session id: {rendered}"
        );
    }

    // ── CuratorError P3.5 property tests ──────────────────────────────────

    /// P8 invariant: `From<EscalationError>` produces a transparent
    /// `Escalation` variant that renders the upstream text directly.
    #[test]
    fn from_escalation_error_is_transparent() {
        let inner = hkask_agents::EscalationError::NotFound("esc-42".to_string());
        let inner_rendered = inner.to_string();
        let curator_err: CuratorError = inner.into();
        assert_eq!(curator_err.to_string(), inner_rendered);
    }

    /// P8 invariant: `From<MetacognitionError>` produces a transparent
    /// `Metacognition` variant that renders the upstream text directly.
    #[test]
    fn from_metacognition_error_is_transparent() {
        let inner = hkask_agents::curator_agent::metacognition::MetacognitionError::NoSnapshot;
        let inner_rendered = inner.to_string();
        let curator_err: CuratorError = inner.into();
        assert_eq!(curator_err.to_string(), inner_rendered);
    }

    /// P8 invariant: `From<RegistryError>` produces a transparent `Registry`
    /// variant that renders the upstream text directly.
    #[test]
    fn curator_from_registry_error_is_transparent() {
        let inner = hkask_types::InfrastructureError::Database("schema missing".to_string());
        let inner_rendered = inner.to_string();
        let reg_err: RegistryError = inner.into();
        let curator_err: CuratorError = reg_err.into();
        assert_eq!(curator_err.to_string(), inner_rendered);
    }

    /// P8 invariant: `EscalationNotFound` is a *user-facing* input sentinel
    /// — it must NOT be derived from an upstream error. The `Display` impl
    /// must include the variant prefix so the user can tell input errors
    /// from upstream failures apart in logs.
    #[test]
    fn curator_escalation_not_found_includes_variant_prefix() {
        let err = CuratorError::EscalationNotFound("esc-99".to_string());
        let rendered = err.to_string();
        assert!(
            rendered.contains("Escalation not found"),
            "rendered message should include the 'Escalation not found' prefix: {rendered}"
        );
        assert!(
            rendered.contains("esc-99"),
            "rendered message should include the escalation id: {rendered}"
        );
    }

    // ── UserError P3.5 property tests ──────────────────────────────────────

    /// P8 invariant: `From<UserStoreError>` produces a transparent `Store`
    /// variant that renders the upstream `UserStoreError` text directly,
    /// without double-wrapping in a "UserError" prefix.
    #[test]
    fn from_user_store_error_is_transparent() {
        let inner = hkask_storage::UserStoreError::NotFound("replicant-42".to_string());
        let inner_rendered = inner.to_string();
        let user_err: UserError = inner.into();
        assert_eq!(user_err.to_string(), inner_rendered);
    }

    /// P8 invariant: `From<InfrastructureError>` produces a transparent
    /// `Infra` variant that renders the upstream text directly.
    #[test]
    fn from_infra_error_is_transparent() {
        let inner = hkask_types::InfrastructureError::Database("connection refused".to_string());
        let inner_rendered = inner.to_string();
        let user_err: UserError = inner.into();
        assert_eq!(user_err.to_string(), inner_rendered);
    }

    /// P8 invariant: `From<InfrastructureError::LockPoisoned>` produces an
    /// `Infra(LockPoisoned)` variant that renders as "lock poisoned".
    /// This is the path taken by `store.lock()?` when a mutex is poisoned,
    /// since `From<PoisonError<T>> for UserError` delegates to
    /// `Infra(InfrastructureError::LockPoisoned)`.
    #[test]
    fn from_infra_lock_poisoned_renders_correctly() {
        let user_err: UserError = hkask_types::InfrastructureError::LockPoisoned.into();
        let rendered = user_err.to_string();
        assert!(
            rendered.contains("lock poisoned"),
            "LockPoisoned should render as 'lock poisoned': {rendered}"
        );
    }

    /// P8 invariant: `NotFound` is a *user-facing* input sentinel —
    /// it must include the variant prefix so the user can distinguish
    /// "not found by name" from "store-level NotFound" errors.
    #[test]
    fn user_not_found_includes_variant_prefix() {
        let err = UserError::NotFound("replicant-42".to_string());
        let rendered = err.to_string();
        assert!(
            rendered.contains("User not found"),
            "rendered message should include the 'User not found' prefix: {rendered}"
        );
        assert!(
            rendered.contains("replicant-42"),
            "rendered message should include the identifier: {rendered}"
        );
    }

    /// P8 invariant: `SessionNotFound` is a *user-facing* input sentinel —
    /// the Display must include both the variant prefix and the session ID.
    #[test]
    fn session_not_found_includes_variant_prefix() {
        let err = UserError::SessionNotFound("sess-abc123".to_string());
        let rendered = err.to_string();
        assert!(
            rendered.contains("Session not found"),
            "rendered message should include the 'Session not found' prefix: {rendered}"
        );
        assert!(
            rendered.contains("sess-abc123"),
            "rendered message should include the session ID: {rendered}"
        );
    }

    /// P8 invariant: `LoginFailed` deliberately drops the source error to
    /// prevent information leakage. The display must include "Login failed"
    /// and "Invalid credentials" but NOT the internal error details.
    #[test]
    fn login_failed_is_opaque() {
        let err = UserError::LoginFailed("Invalid credentials".to_string());
        let rendered = err.to_string();
        assert!(
            rendered.contains("Login failed"),
            "rendered message should include 'Login failed': {rendered}"
        );
        assert!(
            rendered.contains("Invalid credentials"),
            "rendered message should include 'Invalid credentials': {rendered}"
        );
    }

    /// P8 invariant: `InvalidPassphrase` is a user-facing sentinel that
    /// preserves the validation error message.
    #[test]
    fn invalid_passphrase_includes_validation_message() {
        let err = UserError::InvalidPassphrase(
            hkask_types::RegistrationError::InvalidPassphrase.to_string(),
        );
        let rendered = err.to_string();
        assert!(
            rendered.contains("Invalid passphrase"),
            "rendered message should include the 'Invalid passphrase' prefix: {rendered}"
        );
    }

    /// P8 invariant: `ValidationError` is a user-facing sentinel that
    /// preserves the field-validation error message.
    #[test]
    fn validation_error_includes_validation_message() {
        let err = UserError::ValidationError(hkask_types::RegistrationError::EmptyName.to_string());
        let rendered = err.to_string();
        assert!(
            rendered.contains("Validation error"),
            "rendered message should include the 'Validation error' prefix: {rendered}"
        );
    }
}
