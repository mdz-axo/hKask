//! Bot capability management routes

use axum::{Json, extract::Path, extract::State, http::StatusCode, routing::Router};

use crate::{ApiState, GrantCapabilityRequest};

/// Create bots router
pub fn bots_router() -> Router<ApiState> {
    Router::new()
        .route(
            "/api/bots/:id/capabilities",
            axum::routing::get(list_capabilities),
        )
        .route("/api/bots/:id/grant", axum::routing::post(grant_capability))
}

/// List bot capabilities
#[utoipa::path(
    get,
    path = "/api/bots/{id}/capabilities",
    tag = "bots",
    params(
        ("id" = String, Path, description = "Bot WebID"),
    ),
    responses(
        (status = 200, description = "List of capabilities", body = Vec<String>),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn list_capabilities(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
) -> Json<Vec<String>> {
    Json(vec![])
}

/// Grant capability to bot
#[utoipa::path(
    post,
    path = "/api/bots/{id}/grant",
    tag = "bots",
    params(
        ("id" = String, Path, description = "Bot WebID"),
    ),
    request_body = GrantCapabilityRequest,
    responses(
        (status = 200, description = "Capability granted"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn grant_capability(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
    Json(_req): Json<GrantCapabilityRequest>,
) -> StatusCode {
    StatusCode::OK
}
