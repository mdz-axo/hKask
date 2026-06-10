//! Goal coordination routes — call goal repo directly.

use axum::extract::Extension;
use axum::{Json, extract::Path, extract::Query, extract::State, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;
use hkask_types::id::WebID;
use hkask_types::loops::{CurationInput, GoalTransitionEvent};

pub fn goal_router() -> Router<ApiState> {
    Router::new()
        .route(
            "/api/goals",
            axum::routing::get(list_goals).post(create_goal),
        )
        .route("/api/goals/:id/state", axum::routing::post(set_goal_state))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateGoalRequest {
    pub text: String,
    pub visibility: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SetGoalStateRequest {
    pub state: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GoalResponse {
    pub id: String,
    pub text: String,
    pub state: String,
    pub visibility: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GoalListResponse {
    pub goals: Vec<GoalResponse>,
}

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
async fn create_goal(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateGoalRequest>,
) -> Result<Json<GoalResponse>, ApiError> {
    let repo = &state.agent_service.goal_repo;
    let vis_str = req.visibility.as_deref().unwrap_or("private");
    let vis = hkask_types::visibility::Visibility::parse_str(vis_str).ok_or_else(|| {
        ApiError::BadRequest {
            message: format!("Invalid visibility '{vis_str}'"),
        }
    })?;
    let goal = repo
        .create_goal(&auth.webid, &req.text, vis)
        .map_err(ApiError::from)?;
    Ok(Json(GoalResponse {
        id: goal.id.to_string(),
        text: goal.text,
        state: goal.state.as_str().to_string(),
        visibility: goal.visibility.as_str().to_string(),
    }))
}

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
async fn list_goals(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<GoalListResponse>, ApiError> {
    let repo = &state.agent_service.goal_repo;
    let filter = match params.get("state") {
        Some(s) => Some(hkask_types::goal::GoalState::parse_str(s).ok_or_else(|| {
            ApiError::BadRequest {
                message: format!("Invalid goal state '{s}'"),
            }
        })?),
        None => None,
    };
    let goals = repo
        .list_goals(&auth.webid, filter)
        .map_err(ApiError::from)?;
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
async fn set_goal_state(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<SetGoalStateRequest>,
) -> Result<Json<GoalResponse>, ApiError> {
    let repo = &state.agent_service.goal_repo;
    let goal_id: hkask_types::id::GoalID = id.parse().map_err(|e| ApiError::BadRequest {
        message: format!("Invalid goal ID '{id}': {e}"),
    })?;
    let new_state = hkask_types::goal::GoalState::parse_str(&req.state).ok_or_else(|| {
        ApiError::BadRequest {
            message: format!("Invalid goal state '{}'", req.state),
        }
    })?;
    let from_state = repo
        .get_goal(goal_id)
        .map_err(ApiError::from)?
        .map(|g| g.state.as_str().to_string())
        .unwrap_or_default();
    repo.update_goal_state(goal_id, new_state)
        .map_err(ApiError::from)?;

    // Notify Curation of the goal transition.
    if let Some(ref tx) = state.agent_service.curation_inbox_tx {
        let event = CurationInput::GoalTransition(GoalTransitionEvent {
            goal_id: goal_id.to_string(),
            from_state,
            to_state: new_state.as_str().to_string(),
            agent: WebID::new(),
        });
        let _ = tx.send(event);
    }

    Ok(Json(GoalResponse {
        id: goal_id.to_string(),
        text: String::new(),
        state: req.state.clone(),
        visibility: String::new(),
    }))
}
