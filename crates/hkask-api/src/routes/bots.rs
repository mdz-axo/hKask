//! Bot capability management routes

use axum::{Json, extract::Path, extract::State, http::StatusCode};
use utoipa_axum::{router::OpenApiRouter, routes};
use crate::{ApiState, GrantCapabilityRequest};
/// Create bots router
pub fn bots_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_capabilities))
        .routes(routes!(grant_capability))
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
)]
pub(crate) async fn list_capabilities(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
) -> Json<Vec<String>> {
    Json(vec![])
/// Grant capability to bot
    post,
    path = "/api/bots/{id}/grant",
    request_body = GrantCapabilityRequest,
        (status = 200, description = "Capability granted"),
        (status = 400, description = "Invalid request"),
pub(crate) async fn grant_capability(
    Json(_req): Json<GrantCapabilityRequest>,
) -> StatusCode {
    StatusCode::OK
