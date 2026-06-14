//! Goal coordination routes — delegates to GoalService.

use axum::extract::Extension;
use axum::{Json, extract::Path, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use crate::ApiState;
use crate::ApiError;
use crate::middleware::AuthContext;
pub fn goal_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_goals))
        .routes(routes!(create_goal))
        .routes(routes!(set_goal_state))
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateGoalRequest {
    pub text: String,
    pub visibility: Option<String>,
pub struct SetGoalStateRequest {
    pub state: String,
pub struct GoalResponse {
    pub id: String,
    pub visibility: String,
impl From<hkask_services::GoalResponse> for GoalResponse {
    fn from(g: hkask_services::GoalResponse) -> Self {
        Self {
            id: g.id,
            text: g.text,
            state: g.state,
            visibility: g.visibility,
        }
    }
pub struct GoalListResponse {
    pub goals: Vec<GoalResponse>,
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
) -> Result<Json<GoalResponse>, ApiError> {
    let svc_req = hkask_services::CreateGoalRequest {
        text: req.text,
        visibility: req.visibility.unwrap_or_else(|| "private".into()),
        owner: auth.webid,
    };
    let goal = hkask_services::GoalService::create_goal(&state.agent_service, svc_req)
        ?;
    Ok(Json(goal.into()))
/// List all goals for the authenticated agent, optionally filtered by state.
    get, path = "/api/goals", tag = "goals",
    params(("state" = Option<String>, Query, description = "Optional state filter")),
        (status = 200, description = "Goals listed", body = GoalListResponse),
        (status = 400, description = "Invalid state filter"),
pub(crate) async fn list_goals(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<GoalListResponse>, ApiError> {
    let state_filter = params.get("state").map(|s| s.as_str());
    let goals =
        hkask_services::GoalService::list_goals(&state.agent_service, &auth.webid, state_filter)
            ?;
    Ok(Json(GoalListResponse {
        goals: goals.into_iter().map(|g| g.into()).collect(),
    }))
/// Transition a goal to a new state (legal transitions only).
    post, path = "/api/goals/{id}/state", tag = "goals",
    params(("id" = String, Path, description = "Goal ID")),
    request_body = SetGoalStateRequest,
        (status = 200, description = "Goal state changed"),
        (status = 400, description = "Invalid or illegal transition"),
        (status = 404, description = "Goal not found"),
pub(crate) async fn set_goal_state(
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<SetGoalStateRequest>,
    let goal = hkask_services::GoalService::set_goal_state(&state.agent_service, &id, &req.state)
