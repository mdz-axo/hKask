//! Goal coordination routes — delegates to GoalService.

use axum::extract::Extension;
use axum::{Json, extract::Path, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiState;
use crate::error::ServiceErrorResponse;
use crate::middleware::AuthContext;

/// REQ: API-003
/// pre:  none
/// post: returns OpenApiRouter<ApiState> with goal routes registered
pub fn goal_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_goals))
        .routes(routes!(create_goal))
        .routes(routes!(set_goal_state))
}

/// Create goal request — P4 OCAP-gated goal creation.
///
/// Visibility defaults to "private" when omitted. See P11 (Digital Public/Private
/// Sphere) for the visibility taxonomy.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateGoalRequest {
    /// Goal description text
    pub text: String,
    /// Visibility: "private" or "shared" (defaults to "private")
    pub visibility: Option<String>,
}

/// Set goal state request — state machine transition.
///
/// Legal states: "active", "completed", "abandoned".
/// Legal transitions: active→completed, active→abandoned.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SetGoalStateRequest {
    /// Target state: "active", "completed", or "abandoned"
    pub state: String,
}

/// Goal response — reflects current goal state and visibility (P4, P11).
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GoalResponse {
    /// Unique goal identifier
    pub id: String,
    /// Goal description text
    pub text: String,
    /// Current state: "active", "completed", or "abandoned"
    pub state: String,
    /// Visibility: "private" or "shared"
    pub visibility: String,
}

impl From<hkask_services::GoalResponse> for GoalResponse {
    fn from(g: hkask_services::GoalResponse) -> Self {
        Self {
            id: g.id,
            text: g.text,
            state: g.state,
            visibility: g.visibility,
        }
    }
}

/// Goal list response.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GoalListResponse {
    /// Goals for the authenticated agent
    pub goals: Vec<GoalResponse>,
}

/// Create a new goal for the authenticated agent.
#[utoipa::path(
    post, path = "/api/goals", tag = "goals",
    request_body = CreateGoalRequest,
    responses(
        (status = 200, description = "Goal created", body = GoalResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Authority denied"),
    ),
)]
pub(crate) async fn create_goal(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateGoalRequest>,
) -> Result<Json<GoalResponse>, ServiceErrorResponse> {
    let svc_req = hkask_services::CreateGoalRequest {
        text: req.text,
        visibility: req.visibility.unwrap_or_else(|| "private".into()),
        owner: auth.webid,
    };
    let goal = hkask_services::GoalService::create_goal(&state.agent_service, svc_req)?;
    Ok(Json(goal.into()))
}

/// List all goals for the authenticated agent, optionally filtered by state.
#[utoipa::path(
    get, path = "/api/goals", tag = "goals",
    params(("state" = Option<String>, Query, description = "Optional state filter")),
    responses(
        (status = 200, description = "Goals listed", body = GoalListResponse),
        (status = 400, description = "Invalid state filter"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Authority denied"),
    ),
)]
pub(crate) async fn list_goals(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<GoalListResponse>, ServiceErrorResponse> {
    let state_filter = params.get("state").map(|s| s.as_str());
    let goals =
        hkask_services::GoalService::list_goals(&state.agent_service, &auth.webid, state_filter)?;
    Ok(Json(GoalListResponse {
        goals: goals.into_iter().map(|g| g.into()).collect(),
    }))
}

/// Transition a goal to a new state (legal transitions only).
#[utoipa::path(
    post, path = "/api/goals/{id}/state", tag = "goals",
    params(("id" = String, Path, description = "Goal ID")),
    request_body = SetGoalStateRequest,
    responses(
        (status = 200, description = "Goal state changed"),
        (status = 400, description = "Invalid or illegal transition"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Authority denied"),
        (status = 404, description = "Goal not found"),
    ),
)]
pub(crate) async fn set_goal_state(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<SetGoalStateRequest>,
) -> Result<Json<GoalResponse>, ServiceErrorResponse> {
    let goal = hkask_services::GoalService::set_goal_state(&state.agent_service, &id, &req.state, &auth.webid)?;
    Ok(Json(goal.into()))
}
