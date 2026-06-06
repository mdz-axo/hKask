//! Unified API error type with Axum `IntoResponse` implementation.
//!
//! Route handlers return `Result<Json<T>, ApiError>` instead of hand-building
//! `(StatusCode, Json<ErrorResponse>)` tuples. Domain error types implement
//! `From<_X_> for ApiError` so that `?` propagation works directly in handlers.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use hkask_agents::{ConsentError, MemoryError};
use hkask_storage::GoalRepositoryError;

use crate::ErrorResponse;

// ---------------------------------------------------------------------------
// ApiError
// ---------------------------------------------------------------------------

/// Unified error type for all API route handlers.
///
/// Each variant maps to a specific HTTP status code and produces a
/// JSON body matching the existing `ErrorResponse` shape
/// (`{ error, code, details }`).
#[derive(Debug)]
pub enum ApiError {
    /// `404 Not Found` — the requested resource does not exist.
    NotFound { resource: String, id: String },
    /// `401 Unauthorized` — authentication failed or is missing.
    Unauthorized { reason: String },
    /// `403 Forbidden` — authenticated but not authorised for this action.
    Forbidden { reason: String },
    /// `400 Bad Request` — the request body or parameters are invalid.
    BadRequest { message: String },
    /// `409 Conflict` — the request conflicts with existing state.
    Conflict { message: String },
    /// `429 Too Many Requests` — rate-limited; includes retry hint.
    RateLimited {
        retry_after_secs: u64,
        message: String,
    },
    /// `500 Internal Server Error` — unexpected server-side failure.
    Internal { message: String },
}

// ---------------------------------------------------------------------------
// IntoResponse
// ---------------------------------------------------------------------------

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, code, details) = match self {
            ApiError::NotFound { resource, id } => (
                StatusCode::NOT_FOUND,
                "not_found".to_string(),
                "NOT_FOUND".to_string(),
                Some(serde_json::json!({
                    "message": format!("{resource} {id} not found")
                })),
            ),
            ApiError::Unauthorized { reason } => (
                StatusCode::UNAUTHORIZED,
                "unauthorized".to_string(),
                "UNAUTHORIZED".to_string(),
                Some(serde_json::json!({ "message": reason })),
            ),
            ApiError::Forbidden { reason } => (
                StatusCode::FORBIDDEN,
                "forbidden".to_string(),
                "FORBIDDEN".to_string(),
                Some(serde_json::json!({ "message": reason })),
            ),
            ApiError::BadRequest { message } => (
                StatusCode::BAD_REQUEST,
                "bad_request".to_string(),
                "BAD_REQUEST".to_string(),
                Some(serde_json::json!({ "message": message })),
            ),
            ApiError::Conflict { message } => (
                StatusCode::CONFLICT,
                "conflict".to_string(),
                "CONFLICT".to_string(),
                Some(serde_json::json!({ "message": message })),
            ),
            ApiError::RateLimited {
                retry_after_secs,
                message,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited".to_string(),
                "RATE_LIMITED".to_string(),
                Some(serde_json::json!({
                    "message": message,
                    "retry_after_secs": retry_after_secs,
                })),
            ),
            ApiError::Internal { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error".to_string(),
                "INTERNAL_ERROR".to_string(),
                Some(serde_json::json!({ "message": message })),
            ),
        };

        let body = ErrorResponse {
            error,
            code,
            details,
        };
        (status, Json(body)).into_response()
    }
}

// ---------------------------------------------------------------------------
// From impls — enable `?` propagation in handlers
// ---------------------------------------------------------------------------

/// Map goal repository errors to API errors.
///
/// Authority denials surface as 403; not-found as 404; invalid
/// transitions as 400; everything else as 500.
impl From<GoalRepositoryError> for ApiError {
    fn from(e: GoalRepositoryError) -> Self {
        use GoalRepositoryError as E;
        match &e {
            E::VisibilityDenied(_) => ApiError::Forbidden {
                reason: e.to_string(),
            },
            E::NotFound(id) => ApiError::NotFound {
                resource: "Goal".to_string(),
                id: id.clone(),
            },
            E::InvalidTransition(_) | E::MaxDepthExceeded(_) => ApiError::BadRequest {
                message: e.to_string(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

/// Map episodic memory errors to API errors.
///
/// Capability denials surface as 403; everything else as 500.
impl From<MemoryError> for ApiError {
    fn from(e: MemoryError) -> Self {
        match &e {
            MemoryError::CapabilityDenied(reason) => ApiError::Forbidden {
                reason: reason.clone(),
            },
            other => ApiError::Internal {
                message: other.to_string(),
            },
        }
    }
}

/// Map consent errors to API errors.
///
/// Not-found consent maps to 404; everything else to 500.
impl From<ConsentError> for ApiError {
    fn from(e: ConsentError) -> Self {
        use ConsentError as E;
        match &e {
            E::ConsentNotFound(id) => ApiError::NotFound {
                resource: "Consent".to_string(),
                id: id.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}
