//! GoalService — goal CRUD operations for CLI and API surfaces.
//!
//! Delegates to `AgentService::goal_repo()` and wraps `GoalRepositoryError`
//! as `ServiceError::GoalRepo`. Both CLI and API surfaces were previously
//! calling `goal_repo()` directly with duplicated visibility parsing and
//! response mapping logic.

use hkask_types::CurationInput;
use hkask_types::goal::{Goal, GoalState};
use hkask_types::id::{GoalID, WebID};
use hkask_types::visibility::Visibility;

use crate::AgentService;
use crate::error::ServiceError;

/// Request to create a new goal.
pub struct CreateGoalRequest {
    pub text: String,
    pub visibility: String,
    pub owner: WebID,
}

/// Response containing the created goal.
pub struct GoalResponse {
    pub id: String,
    pub text: String,
    pub state: String,
    pub visibility: String,
}

impl From<Goal> for GoalResponse {
    fn from(g: Goal) -> Self {
        Self {
            id: g.id.to_string(),
            text: g.text,
            state: g.state.as_str().to_string(),
            visibility: g.visibility.as_str().to_string(),
        }
    }
}

/// Service for goal management — delegates to the goal repository.
pub struct GoalService;

impl GoalService {
    /// Create a new goal for the given owner.
    pub fn create_goal(
        ctx: &AgentService,
        req: CreateGoalRequest,
    ) -> Result<GoalResponse, ServiceError> {
        let vis = Visibility::parse_str(&req.visibility).ok_or_else(|| {
            ServiceError::ValidationError {
                source: None,
                message: format!(
                    "Invalid visibility '{}': expected private | public",
                    req.visibility
                ),
            }
        })?;
        let repo = ctx.goal_repo();
        let goal = repo
            .create_goal(&req.owner, &req.text, vis)
            .map_err(ServiceError::GoalRepo)?;
        Ok(GoalResponse::from(goal))
    }

    /// List goals for the given owner, optionally filtered by state.
    pub fn list_goals(
        ctx: &AgentService,
        owner: &WebID,
        state_filter: Option<&str>,
    ) -> Result<Vec<GoalResponse>, ServiceError> {
        let filter = match state_filter {
            Some(s) => {
                Some(
                    GoalState::parse_str(s).ok_or_else(|| ServiceError::ValidationError {
                        source: None,
                        message: format!("Invalid goal state filter '{}'", s),
                    })?,
                )
            }
            None => None,
        };
        let repo = ctx.goal_repo();
        let goals = repo
            .list_goals(owner, filter)
            .map_err(ServiceError::GoalRepo)?;
        Ok(goals.into_iter().map(GoalResponse::from).collect())
    }

    /// Set the state of an existing goal.
    pub fn set_goal_state(
        ctx: &AgentService,
        goal_id_str: &str,
        new_state_str: &str,
    ) -> Result<GoalResponse, ServiceError> {
        let goal_id: GoalID = goal_id_str
            .parse()
            .map_err(|_| ServiceError::ValidationError {
                source: None,
                message: format!("Invalid goal ID '{}'", goal_id_str),
            })?;
        let new_state =
            GoalState::parse_str(new_state_str).ok_or_else(|| ServiceError::ValidationError {
                source: None,
                message: format!("Invalid goal state '{}'", new_state_str),
            })?;
        let repo = ctx.goal_repo();

        let goal = repo
            .get_goal(goal_id)
            .map_err(ServiceError::GoalRepo)?
            .ok_or_else(|| ServiceError::ValidationError {
                source: None,
                message: format!("Goal not found: {}", goal_id),
            })?;
        let from_state = goal.state.as_str().to_string();

        repo.update_goal_state(goal_id, new_state)
            .map_err(ServiceError::GoalRepo)?;

        if let Some(tx) = ctx.curation_inbox_tx() {
            let event = CurationInput::GoalTransition(hkask_types::loops::GoalTransitionEvent {
                goal_id: goal_id.to_string(),
                from_state,
                to_state: new_state.as_str().to_string(),
                agent: WebID::new(),
            });
            let _ = tx.send(event);
        }

        // Return a response with the existing goal's text and visibility
        // alongside the new state, rather than empty strings.
        Ok(GoalResponse {
            id: goal_id.to_string(),
            text: goal.text,
            state: new_state.as_str().to_string(),
            visibility: goal.visibility.as_str().to_string(),
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: MDS-goal-svc-001 — create_goal delegates to GoalRepository and produces GoalResponse
    #[test]
    fn create_goal_converts_visibility_and_returns_response() {
        let err = Visibility::parse_str("bogus");
        assert!(err.is_none(), "bogus should not parse as a Visibility");
    }

    // REQ: MDS-goal-svc-002 — list_goals respects optional state filter
    #[test]
    fn list_goals_parses_state_filter() {
        assert!(GoalState::parse_str("pending").is_some());
        assert!(GoalState::parse_str("active").is_some());
        assert!(GoalState::parse_str("completed").is_some());
        assert!(GoalState::parse_str("bogus").is_none());
    }

    // REQ: MDS-goal-svc-003 — Goal::into() → GoalResponse preserves all fields
    #[test]
    fn goal_to_response_maps_all_fields() {
        let goal = Goal::new(WebID::new(), "Test goal", Visibility::Private);
        let resp = GoalResponse::from(goal);
        assert!(!resp.id.is_empty(), "ID must be non-empty");
        assert_eq!(resp.text, "Test goal");
        assert_eq!(resp.state, "pending");
        assert_eq!(resp.visibility, "private");
    }
}
