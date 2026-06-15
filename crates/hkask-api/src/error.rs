//! API error type with Axum IntoResponse — maps to HTTP status codes per variant.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use hkask_storage::{
    AgentRegistryError, ConsentStoreError, GoalRepositoryError, TripleError, UserStoreError,
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    NotFound { resource: String, id: String },
    Unauthorized { reason: String },
    Forbidden { reason: String },
    BadRequest { message: String },
    Conflict { message: String },
    ServiceUnavailable { reason: String },
    Internal { message: String },
}

impl ApiError {
    fn status_and_message(self) -> (StatusCode, String) {
        match self {
            ApiError::NotFound { resource, id } => {
                (StatusCode::NOT_FOUND, format!("{resource} not found: {id}"))
            }
            ApiError::Unauthorized { reason } => {
                (StatusCode::UNAUTHORIZED, format!("Unauthorized: {reason}"))
            }
            ApiError::Forbidden { reason } => {
                (StatusCode::FORBIDDEN, format!("Forbidden: {reason}"))
            }
            ApiError::BadRequest { message } => {
                (StatusCode::BAD_REQUEST, format!("Bad request: {message}"))
            }
            ApiError::Conflict { message } => {
                (StatusCode::CONFLICT, format!("Conflict: {message}"))
            }
            ApiError::ServiceUnavailable { reason } => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("Service unavailable: {reason}"),
            ),
            ApiError::Internal { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal error: {message}"),
            ),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound { resource, id } => write!(f, "{resource} not found: {id}"),
            ApiError::Unauthorized { reason } => write!(f, "Unauthorized: {reason}"),
            ApiError::Forbidden { reason } => write!(f, "Forbidden: {reason}"),
            ApiError::BadRequest { message } => write!(f, "Bad request: {message}"),
            ApiError::Conflict { message } => write!(f, "Conflict: {message}"),
            ApiError::ServiceUnavailable { reason } => write!(f, "Service unavailable: {reason}"),
            ApiError::Internal { message } => write!(f, "Internal error: {message}"),
        }
    }
}

impl std::error::Error for ApiError {}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = self.status_and_message();
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

// ── ServiceError newtype for Axum IntoResponse ───────────────────────
//
// Route handlers return `Result<Json<T>, ServiceErrorResponse>`.
// `ServiceError` cannot implement `IntoResponse` directly (orphan rule:
// both the trait and type are foreign). This newtype bridges the gap,
// delegating to `ApiError` for HTTP status code mapping.

/// Newtype wrapper that implements `IntoResponse` for `ServiceError`.
///
/// Route handlers return `Result<Json<T>, ServiceErrorResponse>`.
/// The `?` operator auto-converts `ServiceError` via the `From` impl.
#[derive(Debug)]
pub struct ServiceErrorResponse(pub hkask_services::ServiceError);

impl From<hkask_services::ServiceError> for ServiceErrorResponse {
    fn from(e: hkask_services::ServiceError) -> Self {
        ServiceErrorResponse(e)
    }
}

impl IntoResponse for ServiceErrorResponse {
    fn into_response(self) -> Response {
        ApiError::from(self.0).into_response()
    }
}

// ── Temporary bridge: allow existing ApiError constructions to be ──────
// returned from routes that now use ServiceErrorResponse. During migration,
// manual ApiError::BadRequest/etc. constructions still work. Once all
// manual constructions are converted to ServiceError variants, this impl
// can be deleted.

impl From<ApiError> for ServiceErrorResponse {
    fn from(e: ApiError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::Infra(
            hkask_types::InfrastructureError::Database(e.to_string()),
        ))
    }
}

// ── Domain error → ServiceErrorResponse bridges ───────────────────────
//
// The `?` operator needs direct `From<E> for ServiceErrorResponse`.
// These impls bridge domain errors that route handlers propagate with `?`.
// They delegate to `ServiceError`'s `#[from]` wrappers.

impl From<hkask_agents::acp::AcpError> for ServiceErrorResponse {
    fn from(e: hkask_agents::acp::AcpError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::Acp(e))
    }
}

impl From<hkask_storage::EscalationError> for ServiceErrorResponse {
    fn from(e: hkask_storage::EscalationError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::Escalation(e))
    }
}

impl From<uuid::Error> for ServiceErrorResponse {
    fn from(e: uuid::Error) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::InvalidWebID(e.to_string()))
    }
}

impl From<hkask_storage::AgentRegistryError> for ServiceErrorResponse {
    fn from(e: hkask_storage::AgentRegistryError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::AgentRegistryStore(e))
    }
}

impl From<hkask_agents::pod::AgentPodError> for ServiceErrorResponse {
    fn from(e: hkask_agents::pod::AgentPodError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::Pod(e))
    }
}

impl From<hkask_types::ports::RegistryError> for ServiceErrorResponse {
    fn from(e: hkask_types::ports::RegistryError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::Registry(e))
    }
}

// ── Service layer adapter ────────────────────────────────────────────

impl From<hkask_services::ServiceError> for ApiError {
    fn from(e: hkask_services::ServiceError) -> Self {
        use hkask_services::ServiceError as SE;
        match e {
            SE::EscalationNotFound(id) => ApiError::NotFound {
                resource: "escalation".into(),
                id,
            },
            SE::AgentNotFound(name) => ApiError::NotFound {
                resource: "agent".into(),
                id: name,
            },
            SE::UserNotFound(name) => ApiError::NotFound {
                resource: "user".into(),
                id: name,
            },
            SE::PodNotFound(id) => ApiError::NotFound {
                resource: "pod".into(),
                id,
            },
            SE::LoginFailed(_) => ApiError::Unauthorized {
                reason: "Invalid credentials".into(),
            },
            SE::Acp(hkask_agents::acp::AcpError::CapabilityDenied(webid, perm)) => {
                ApiError::Forbidden {
                    reason: format!("Agent {} lacks permission: {}", webid, perm),
                }
            }
            SE::Acp(hkask_agents::acp::AcpError::AgentNotFound(webid)) => ApiError::NotFound {
                resource: "agent".into(),
                id: webid.to_string(),
            },
            SE::Acp(hkask_agents::acp::AcpError::AgentAlreadyRegistered(webid)) => {
                ApiError::Conflict {
                    message: format!("Agent already registered: {}", webid),
                }
            }
            SE::Acp(_) => ApiError::Forbidden {
                reason: "Capability denied".into(),
            },
            SE::SovereigntyStore(hkask_storage::SovereigntyStoreError::UuidParse(msg)) => {
                ApiError::BadRequest { message: msg }
            }
            SE::InvalidAgentType(msg) => ApiError::BadRequest { message: msg },
            SE::InvalidPassphrase(msg) => ApiError::BadRequest {
                message: format!("Invalid passphrase: {}", msg),
            },
            SE::ValidationError(msg) => ApiError::BadRequest { message: msg },
            SE::AgentRegistrationFailed(msg) => ApiError::Conflict { message: msg },
            SE::Escalation(hkask_storage::EscalationError::NotFound(id)) => ApiError::NotFound {
                resource: "escalation".into(),
                id,
            },
            SE::Escalation(_) => ApiError::Internal {
                message: e.to_string(),
            },
            SE::AgentRegistryStore(err) => match err {
                AgentRegistryError::NotFound(name) => ApiError::NotFound {
                    resource: "agent".into(),
                    id: name,
                },
                AgentRegistryError::AlreadyRegistered(name) => ApiError::Conflict {
                    message: format!("Agent already registered: {name}"),
                },
                _ => ApiError::Internal {
                    message: err.to_string(),
                },
            },
            SE::GoalRepo(err) => match err {
                GoalRepositoryError::NotFound(id) => ApiError::NotFound {
                    resource: "goal".into(),
                    id,
                },
                GoalRepositoryError::VisibilityDenied(reason) => ApiError::Forbidden { reason },
                GoalRepositoryError::InvalidTransition(msg) => {
                    ApiError::BadRequest { message: msg }
                }
                GoalRepositoryError::MaxDepthExceeded(msg) => ApiError::BadRequest { message: msg },
                _ => ApiError::Internal {
                    message: err.to_string(),
                },
            },
            SE::Triple(err) => match err {
                TripleError::NotFound => ApiError::NotFound {
                    resource: "triple".into(),
                    id: "unknown".into(),
                },
                _ => ApiError::Internal {
                    message: err.to_string(),
                },
            },
            SE::ConsentStore(err) => match err {
                ConsentStoreError::NotFound(id) => ApiError::NotFound {
                    resource: "consent".into(),
                    id,
                },
                _ => ApiError::Internal {
                    message: err.to_string(),
                },
            },
            SE::UserStore(err) => match err {
                UserStoreError::NotFound(id) => ApiError::NotFound {
                    resource: "user".into(),
                    id,
                },
                UserStoreError::ReplicantNameTaken(name) => ApiError::Conflict {
                    message: format!("Replicant name already registered: {name}"),
                },
                UserStoreError::InvalidCredentials => ApiError::Unauthorized {
                    reason: "Invalid credentials".into(),
                },
                UserStoreError::PassphraseExpired(days) => ApiError::Unauthorized {
                    reason: format!("Passphrase expired {} days ago — must change", days),
                },
                _ => ApiError::Internal {
                    message: err.to_string(),
                },
            },
            SE::Consent(err) => match err {
                hkask_agents::ConsentError::ConsentNotFound(id) => ApiError::NotFound {
                    resource: "consent".into(),
                    id,
                },
                _ => ApiError::Internal {
                    message: err.to_string(),
                },
            },
            SE::Spec(err) => ApiError::Internal {
                message: err.to_string(),
            },
            SE::Pod(hkask_agents::pod::AgentPodError::PodNotFound(_)) => ApiError::NotFound {
                resource: "pod".into(),
                id: e.to_string(),
            },
            SE::Pod(hkask_agents::pod::AgentPodError::PersonaParseError(msg)) => {
                ApiError::BadRequest {
                    message: format!("Invalid persona: {}", msg),
                }
            }
            SE::Pod(hkask_agents::pod::AgentPodError::InvalidStateTransition(from, to)) => {
                ApiError::Conflict {
                    message: format!("Invalid pod state transition: {} -> {}", from, to),
                }
            }
            SE::Pod(_) => ApiError::Internal {
                message: e.to_string(),
            },
            SE::Template(err) => match err {
                hkask_templates::TemplateError::NotFound(id) => ApiError::NotFound {
                    resource: "template".into(),
                    id,
                },
                hkask_templates::TemplateError::CapabilityDenied(msg) => {
                    ApiError::Forbidden { reason: msg }
                }
                hkask_templates::TemplateError::PathTraversal(_) => ApiError::BadRequest {
                    message: err.to_string(),
                },
                hkask_templates::TemplateError::SandboxViolation(_) => ApiError::Forbidden {
                    reason: err.to_string(),
                },
                hkask_templates::TemplateError::Validation(msg) => {
                    ApiError::BadRequest { message: msg }
                }
                _ => ApiError::Internal {
                    message: err.to_string(),
                },
            },
            SE::NuEvent(err) => ApiError::Internal {
                message: err.to_string(),
            },
            SE::Keystore(msg) => ApiError::ServiceUnavailable { reason: msg },
            SE::Infra(err) => ApiError::Internal {
                message: err.to_string(),
            },
            SE::Backup(msg) => ApiError::Internal {
                message: format!("Backup failed: {}", msg),
            },
            other => ApiError::Internal {
                message: other.to_string(),
            },
        }
    }
}
