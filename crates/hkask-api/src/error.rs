//! API error type with Axum IntoResponse implementation
//!
//! Replaces the hand-built `(StatusCode, Json(ErrorResponse{...}))` tuples
//! that were repeated identically across every route handler (Fowler C5).
//!
//! Each variant maps to an appropriate HTTP status code. The `IntoResponse`
//! impl converts these into the JSON format expected by API clients.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use hkask_storage::{
    AgentRegistryError, ConsentStoreError, GoalRepositoryError, NuEventError,
    SovereigntyStoreError, StandingSessionError, TripleError, UserStoreError,
};
use hkask_types::InfrastructureError;
use serde::Serialize;

/// Unified API error type.
///
/// Each variant maps to an appropriate HTTP status code and carries
/// a human-readable error message. The `IntoResponse` impl converts
/// these into the JSON format expected by clients.
#[derive(Debug)]
pub enum ApiError {
    /// The requested resource was not found (404)
    NotFound { resource: String, id: String },
    /// The request was unauthorized (401)
    Unauthorized { reason: String },
    /// The request was forbidden (403)
    Forbidden { reason: String },
    /// The request was malformed (400)
    BadRequest { message: String },
    /// A conflict occurred (409)
    Conflict { message: String },
    /// An internal server error occurred (500)
    Internal { message: String },
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound { resource, id } => write!(f, "{resource} not found: {id}"),
            ApiError::Unauthorized { reason } => write!(f, "Unauthorized: {reason}"),
            ApiError::Forbidden { reason } => write!(f, "Forbidden: {reason}"),
            ApiError::BadRequest { message } => write!(f, "Bad request: {message}"),
            ApiError::Conflict { message } => write!(f, "Conflict: {message}"),
            ApiError::Internal { message } => write!(f, "Internal error: {message}"),
        }
    }
}

impl std::error::Error for ApiError {}

/// JSON error response body — mirrors the existing `ErrorResponse` struct
/// for backward compatibility with API clients.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound { resource, id } => {
                (StatusCode::NOT_FOUND, format!("{resource} not found: {id}"))
            }
            ApiError::Unauthorized { reason } => (StatusCode::UNAUTHORIZED, reason),
            ApiError::Forbidden { reason } => (StatusCode::FORBIDDEN, reason),
            ApiError::BadRequest { message } => (StatusCode::BAD_REQUEST, message),
            ApiError::Conflict { message } => (StatusCode::CONFLICT, message),
            ApiError::Internal { message } => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

// ── Store error conversions ──────────────────────────────────────────

impl From<TripleError> for ApiError {
    fn from(e: TripleError) -> Self {
        match e {
            TripleError::NotFound => ApiError::NotFound {
                resource: "triple".into(),
                id: "unknown".into(),
            },
            TripleError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<GoalRepositoryError> for ApiError {
    fn from(e: GoalRepositoryError) -> Self {
        match &e {
            GoalRepositoryError::NotFound(id) => ApiError::NotFound {
                resource: "goal".into(),
                id: id.clone(),
            },
            GoalRepositoryError::VisibilityDenied(reason) => ApiError::Forbidden {
                reason: reason.clone(),
            },
            GoalRepositoryError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
            GoalRepositoryError::InvalidTransition(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            GoalRepositoryError::MaxDepthExceeded(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            GoalRepositoryError::Corrupt(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            GoalRepositoryError::QuarantineFailed(msg) => ApiError::Internal {
                message: msg.clone(),
            },
        }
    }
}

impl From<AgentRegistryError> for ApiError {
    fn from(e: AgentRegistryError) -> Self {
        match &e {
            AgentRegistryError::NotFound(name) => ApiError::NotFound {
                resource: "agent".into(),
                id: name.clone(),
            },
            AgentRegistryError::AlreadyRegistered(name) => ApiError::Conflict {
                message: format!("Agent already registered: {name}"),
            },
            AgentRegistryError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<NuEventError> for ApiError {
    fn from(e: NuEventError) -> Self {
        ApiError::Internal {
            message: e.to_string(),
        }
    }
}

impl From<ConsentStoreError> for ApiError {
    fn from(e: ConsentStoreError) -> Self {
        match &e {
            ConsentStoreError::NotFound(id) => ApiError::NotFound {
                resource: "consent".into(),
                id: id.clone(),
            },
            ConsentStoreError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<SovereigntyStoreError> for ApiError {
    fn from(e: SovereigntyStoreError) -> Self {
        match &e {
            SovereigntyStoreError::UuidParse(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            SovereigntyStoreError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<StandingSessionError> for ApiError {
    fn from(e: StandingSessionError) -> Self {
        match &e {
            StandingSessionError::NotFound(id) => ApiError::NotFound {
                resource: "session".into(),
                id: id.clone(),
            },
            StandingSessionError::Sealed(id) => ApiError::Forbidden {
                reason: format!("Session is sealed (key version mismatch): {id}"),
            },
            StandingSessionError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<UserStoreError> for ApiError {
    fn from(e: UserStoreError) -> Self {
        match &e {
            UserStoreError::NotFound(id) => ApiError::NotFound {
                resource: "user".into(),
                id: id.clone(),
            },
            UserStoreError::ReplicantNameTaken(name) => ApiError::Conflict {
                message: format!("Replicant name already registered: {name}"),
            },
            UserStoreError::InvalidCredentials => ApiError::Unauthorized {
                reason: "Invalid credentials".into(),
            },
            UserStoreError::Encryption(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            UserStoreError::Decryption(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            UserStoreError::KeyDerivation(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            UserStoreError::PasswordHash(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            UserStoreError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<InfrastructureError> for ApiError {
    fn from(e: InfrastructureError) -> Self {
        ApiError::Internal {
            message: e.to_string(),
        }
    }
}

// ── Agent error conversions ────────────────────────────────────────────

impl From<hkask_agents::ConsentError> for ApiError {
    fn from(e: hkask_agents::ConsentError) -> Self {
        match &e {
            hkask_agents::ConsentError::ConsentNotFound(id) => ApiError::NotFound {
                resource: "consent".into(),
                id: id.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_agents::EscalationError> for ApiError {
    fn from(e: hkask_agents::EscalationError) -> Self {
        match &e {
            hkask_agents::EscalationError::NotFound(id) => ApiError::NotFound {
                resource: "escalation".into(),
                id: id.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_types::GitError> for ApiError {
    fn from(e: hkask_types::GitError) -> Self {
        match &e {
            hkask_types::GitError::CrateNotFound(name) => ApiError::NotFound {
                resource: "template crate".into(),
                id: name.clone(),
            },
            hkask_types::GitError::Io(_) => ApiError::BadRequest {
                message: e.to_string(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_agents::AcpError> for ApiError {
    fn from(e: hkask_agents::AcpError) -> Self {
        match &e {
            hkask_agents::AcpError::AgentAlreadyRegistered(webid) => ApiError::Conflict {
                message: format!("Agent already registered: {}", webid),
            },
            hkask_agents::AcpError::AgentNotFound(webid) => ApiError::NotFound {
                resource: "agent".into(),
                id: webid.to_string(),
            },
            hkask_agents::AcpError::CapabilityDenied(webid, perm) => ApiError::Forbidden {
                reason: format!("Agent {} lacks permission: {}", webid, perm),
            },
            hkask_agents::AcpError::WildcardCapabilityNotAllowed => ApiError::BadRequest {
                message: "Wildcard capabilities are not allowed".into(),
            },
            hkask_agents::AcpError::MalformedCapability(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_templates::TemplateError> for ApiError {
    fn from(e: hkask_templates::TemplateError) -> Self {
        match &e {
            hkask_templates::TemplateError::NotFound(id) => ApiError::NotFound {
                resource: "template".into(),
                id: id.clone(),
            },
            hkask_templates::TemplateError::CapabilityDenied(msg) => ApiError::Forbidden {
                reason: msg.clone(),
            },
            hkask_templates::TemplateError::PathTraversal(_) => ApiError::BadRequest {
                message: e.to_string(),
            },
            hkask_templates::TemplateError::SandboxViolation(_) => ApiError::Forbidden {
                reason: e.to_string(),
            },
            hkask_templates::TemplateError::Validation(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_types::ports::RegistryError> for ApiError {
    fn from(e: hkask_types::ports::RegistryError) -> Self {
        match &e {
            hkask_types::ports::RegistryError::NotFound(id) => ApiError::NotFound {
                resource: "template".into(),
                id: id.clone(),
            },
            hkask_types::ports::RegistryError::Other(msg) => ApiError::Internal {
                message: msg.clone(),
            },
        }
    }
}
