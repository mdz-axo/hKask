//! Goal coordination routes.
//!
//! HTTP surface for the goal substrate, mirroring `kask goal` (CLI) and the
//! `goal` MCP tools for MCP ≡ CLI ≡ API equivalence (REQ-IFC-001).
//! Authority is co-located with effect: the caller's WebID is passed directly
//! to the goal service, and denials are observed via the repository's CNS
//! telemetry sink.

use axum::extract::Extension;
use axum::{Json, extract::Path, extract::Query, extract::State, routing::Router};
use hkask_services::{GoalContext, GoalService};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

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
) -> Result<Json<GoalResponse>, ApiError> {
    let goal_ctx = GoalContext::from(&*state.service_context);
    let visibility_str = req.visibility.as_deref().unwrap_or("private");

    let goal = GoalService::create_goal(&goal_ctx, &auth.webid, &req.text, visibility_str)
        .map_err(ApiError::from)?;

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
) -> Result<Json<GoalListResponse>, ApiError> {
    let goal_ctx = GoalContext::from(&*state.service_context);
    let state_filter = params.get("state").map(|s| s.as_str());

    let goals =
        GoalService::list_goals(&goal_ctx, &auth.webid, state_filter).map_err(ApiError::from)?;

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
) -> Result<Json<GoalResponse>, ApiError> {
    let goal_ctx = GoalContext::from(&*state.service_context);

    GoalService::set_goal_state(&goal_ctx, &id, &req.state).map_err(ApiError::from)?;

    // Parse goal ID back for the response (already validated by service)
    let goal_id = GoalService::parse_goal_id(&id).map_err(ApiError::from)?;

    Ok(Json(GoalResponse {
        id: goal_id.to_string(),
        text: String::new(),
        state: req.state.clone(),
        visibility: String::new(),
    }))
}
