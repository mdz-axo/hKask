//! API error type with Axum IntoResponse — maps to HTTP status codes per variant.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use hkask_rsolidity as rs;
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

// ── Domain error → ServiceErrorResponse bridges ───────────────────────
//
// The `?` operator needs direct `From<E> for ServiceErrorResponse`.
// These impls bridge domain errors that route handlers propagate with `?`.
// They delegate to `ServiceError`'s `#[from]` wrappers.

impl From<hkask_agents::a2a::A2AError> for ServiceErrorResponse {
    fn from(e: hkask_agents::a2a::A2AError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::A2A {
            message: e.to_string(),
        })
    }
}

impl From<hkask_storage::EscalationError> for ServiceErrorResponse {
    fn from(e: hkask_storage::EscalationError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::Escalation {
            message: e.to_string(),
        })
    }
}

impl From<uuid::Error> for ServiceErrorResponse {
    fn from(e: uuid::Error) -> Self {
        let msg = e.to_string();
        ServiceErrorResponse(hkask_services::ServiceError::InvalidWebID {
            source: Some(e),
            message: msg,
        })
    }
}

impl From<hkask_storage::AgentRegistryError> for ServiceErrorResponse {
    fn from(e: hkask_storage::AgentRegistryError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })
    }
}

impl From<hkask_agents::pod::AgentPodError> for ServiceErrorResponse {
    fn from(e: hkask_agents::pod::AgentPodError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::Pod {
            message: e.to_string(),
        })
    }
}

impl From<hkask_types::ports::RegistryError> for ServiceErrorResponse {
    fn from(e: hkask_types::ports::RegistryError) -> Self {
        ServiceErrorResponse(hkask_services::ServiceError::Registry {
            message: e.to_string(),
        })
    }
}

impl From<hkask_services_backup::BackupError> for ServiceErrorResponse {
    fn from(e: hkask_services_backup::BackupError) -> Self {
        let msg = e.to_string();
        ServiceErrorResponse(hkask_services::ServiceError::Backup {
            source: Some(Box::new(e)),
            message: msg,
        })
    }
}

// ── Service layer adapter ────────────────────────────────────────────

impl From<hkask_services::ServiceError> for ApiError {
    fn from(e: hkask_services::ServiceError) -> Self {
        use hkask_services::ServiceError as SE;
        match e {
            SE::EscalationNotFound { message: id, .. } => ApiError::NotFound {
                resource: "escalation".into(),
                id,
            },
            SE::AgentNotFound { message: name, .. } => ApiError::NotFound {
                resource: "agent".into(),
                id: name,
            },
            SE::UserNotFound { message: name, .. } => ApiError::NotFound {
                resource: "user".into(),
                id: name,
            },
            SE::PodNotFound { message: id, .. } => ApiError::NotFound {
                resource: "pod".into(),
                id,
            },
            SE::LoginFailed { .. } => ApiError::Unauthorized {
                reason: "Invalid credentials".into(),
            },
            // TODO: Restore error kind discrimination after ServiceError gets error kind fields.
            // Currently all domain errors are flattened to { message: String }. This means
            // NotFound/Forbidden/Conflict distinctions within A2A, Escalation, AgentRegistry,
            // GoalRepo, Triple, ConsentStore, UserStore, Consent, Pod, and Template errors
            // all collapse to Internal.
            SE::A2A { message } => ApiError::Forbidden { reason: message },
            SE::SovereigntyStore { message } => ApiError::BadRequest { message },
            SE::InvalidAgentType { message: msg, .. } => ApiError::BadRequest { message: msg },
            SE::InvalidPassphrase { message: msg, .. } => ApiError::BadRequest {
                message: format!("Invalid passphrase: {}", msg),
            },
            SE::ValidationError { message: msg, .. } => ApiError::BadRequest { message: msg },
            SE::AgentRegistrationFailed { message: msg, .. } => ApiError::Conflict { message: msg },
            SE::Escalation { message } => ApiError::Internal { message },
            SE::AgentRegistryStore { message } => ApiError::Internal { message },
            SE::GoalRepo { message } => ApiError::Internal { message },
            SE::Triple { message } => ApiError::Internal { message },
            SE::ConsentStore { message } => ApiError::Internal { message },
            SE::UserStore { message } => ApiError::Internal { message },
            SE::Consent { message } => ApiError::Internal { message },
            SE::Spec { message } => ApiError::Internal { message },
            SE::Pod { message } => ApiError::Internal { message },
            SE::Template { message } => ApiError::Internal { message },
            SE::Keystore { message: msg, .. } => ApiError::ServiceUnavailable { reason: msg },
            SE::Infra(err) => ApiError::Internal {
                message: err.to_string(),
            },
            SE::Backup { message: msg, .. } => ApiError::Internal {
                message: format!("Backup failed: {}", msg),
            },
            other => ApiError::Internal {
                message: other.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    // contract: api-error-001
    #[test]
    fn apierror_maps_to_correct_status_codes() {
        let (status, _) = ApiError::NotFound {
            resource: "agent".into(),
            id: "test".into(),
        }
        .status_and_message();
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, _) = ApiError::Unauthorized {
            reason: "bad token".into(),
        }
        .status_and_message();
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, _) = ApiError::Forbidden {
            reason: "no access".into(),
        }
        .status_and_message();
        assert_eq!(status, StatusCode::FORBIDDEN);

        let (status, _) = ApiError::BadRequest {
            message: "invalid".into(),
        }
        .status_and_message();
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let (status, _) = ApiError::Internal {
            message: "boom".into(),
        }
        .status_and_message();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    // contract: api-error-002
    #[test]
    fn apierror_display_is_readable() {
        let err = ApiError::NotFound {
            resource: "pod".into(),
            id: "p1".into(),
        };
        assert_eq!(err.to_string(), "pod not found: p1");

        let err = ApiError::Unauthorized {
            reason: "expired".into(),
        };
        assert_eq!(err.to_string(), "Unauthorized: expired");

        let err = ApiError::BadRequest {
            message: "missing field".into(),
        };
        assert_eq!(err.to_string(), "Bad request: missing field");
    }

    // contract: api-error-003
    #[test]
    fn apierror_into_response_produces_correct_status() {
        let err = ApiError::NotFound {
            resource: "agent".into(),
            id: "alice".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let err = ApiError::Internal {
            message: "boom".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
