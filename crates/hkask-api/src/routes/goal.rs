//! Goal coordination routes.
//!
//! HTTP surface for the goal substrate, mirroring `kask goal` (CLI) and the
//! `goal` MCP tools for MCP ≡ CLI ≡ API equivalence (REQ-IFC-001).
//! Authority is co-located with effect: the caller's WebID is passed directly
//! to the goal repository, and denials are observed via the repository's CNS
//! telemetry sink.

use axum::extract::Extension;
use axum::{
    Json, extract::Path, extract::Query, extract::State, http::StatusCode, routing::Router,
};
use hkask_types::goal::GoalState;
use hkask_types::id::GoalID;
use hkask_types::visibility::Visibility;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::middleware::AuthContext;
use crate::{ApiState, ErrorResponse};

/// Create goal router
pub fn goal_router() -> Router<ApiState> {
    Router::new()
        .route(
            "/api/goals",
            axum::routing::get(list_goals).post(create_goal),
        )
        .route("/api/goals/:id/state", axum::routing::post(set_goal_state))
}

/// Request to create a goal.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateGoalRequest {
    /// Goal text.
    pub text: String,
    /// Visibility: private | shared | public. Defaults to private.
    pub visibility: Option<String>,
}

/// Request to transition a goal's state.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SetGoalStateRequest {
    /// Target state: pending | active | completed | blocked | abandoned.
    pub state: String,
}

/// A goal as returned over the API.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GoalResponse {
    pub id: String,
    pub text: String,
    pub state: String,
    pub visibility: String,
}

/// A list of goals.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GoalListResponse {
    pub goals: Vec<GoalResponse>,
}

fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_request".to_string(),
            code: "GOAL_BAD_REQUEST".to_string(),
            details: Some(serde_json::json!({ "message": message })),
        }),
    )
}

/// Map a goal repository error to an HTTP response. Authority denials surface
/// as 403 (and have already emitted CNS telemetry); not-found as 404.
fn repo_error(e: hkask_storage::GoalRepositoryError) -> (StatusCode, Json<ErrorResponse>) {
    use hkask_storage::GoalRepositoryError as E;
    let (status, code) = match &e {
        E::VisibilityDenied(_) => (StatusCode::FORBIDDEN, "GOAL_DENIED"),
        E::NotFound(_) => (StatusCode::NOT_FOUND, "GOAL_NOT_FOUND"),
        E::InvalidTransition(_) | E::MaxDepthExceeded(_) => {
            (StatusCode::BAD_REQUEST, "GOAL_BAD_REQUEST")
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "GOAL_ERROR"),
    };
    (
        status,
        Json(ErrorResponse {
            error: "goal_operation_failed".to_string(),
            code: code.to_string(),
            details: Some(serde_json::json!({ "message": e.to_string() })),
        }),
    )
}

/// Create a goal owned by the authenticated caller.
#[utoipa::path(
    post,
    path = "/api/goals",
    tag = "goals",
    request_body = CreateGoalRequest,
    responses(
        (status = 200, description = "Goal created", body = GoalResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Authority denied"),
    ),
)]
async fn create_goal(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateGoalRequest>,
) -> Result<Json<GoalResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visibility_str = req.visibility.as_deref().unwrap_or("private");
    let visibility = Visibility::parse_str(visibility_str)
        .ok_or_else(|| bad_request("visibility must be private | shared | public"))?;

    let goal = state
        .goal_repo
        .create_goal(&auth.webid, &req.text, visibility)
        .map_err(repo_error)?;

    Ok(Json(GoalResponse {
        id: goal.id.to_string(),
        text: goal.text,
        state: goal.state.as_str().to_string(),
        visibility: goal.visibility.as_str().to_string(),
    }))
}

/// List the authenticated caller's goals, optionally filtered by state.
#[utoipa::path(
    get,
    path = "/api/goals",
    tag = "goals",
    params(
        ("state" = Option<String>, Query, description = "Optional state filter"),
    ),
    responses(
        (status = 200, description = "Goals listed", body = GoalListResponse),
        (status = 400, description = "Invalid state filter"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Authority denied"),
    ),
)]
async fn list_goals(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<GoalListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let state_filter = match params.get("state") {
        Some(s) => {
            Some(GoalState::parse_str(s).ok_or_else(|| bad_request("invalid state filter"))?)
        }
        None => None,
    };

    let goals = state
        .goal_repo
        .list_goals(&auth.webid, state_filter)
        .map_err(repo_error)?;

    Ok(Json(GoalListResponse {
        goals: goals
            .into_iter()
            .map(|g| GoalResponse {
                id: g.id.to_string(),
                text: g.text,
                state: g.state.as_str().to_string(),
                visibility: g.visibility.as_str().to_string(),
            })
            .collect(),
    }))
}

/// Transition a goal to a new state (legal transitions only).
#[utoipa::path(
    post,
    path = "/api/goals/{id}/state",
    tag = "goals",
    params(
        ("id" = String, Path, description = "Goal ID"),
    ),
    request_body = SetGoalStateRequest,
    responses(
        (status = 200, description = "Goal state changed"),
        (status = 400, description = "Invalid or illegal transition"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Authority denied"),
        (status = 404, description = "Goal not found"),
    ),
)]
async fn set_goal_state(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<SetGoalStateRequest>,
) -> Result<Json<GoalResponse>, (StatusCode, Json<ErrorResponse>)> {
    let goal_id = id
        .parse::<GoalID>()
        .map_err(|_| bad_request("Invalid goal ID"))?;
    let new_state = GoalState::parse_str(&req.state).ok_or_else(|| {
        bad_request("state must be pending | active | completed | blocked | abandoned")
    })?;

    state
        .goal_repo
        .update_goal_state(goal_id, new_state)
        .map_err(repo_error)?;

    Ok(Json(GoalResponse {
        id: goal_id.to_string(),
        text: String::new(),
        state: new_state.as_str().to_string(),
        visibility: String::new(),
    }))
}
