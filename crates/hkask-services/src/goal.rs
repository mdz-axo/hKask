//! Goal coordination service — create, list, and transition goals.
//!
//! `GoalService` centralizes goal ID parsing, visibility parsing, state
//! parsing, and error normalization across CLI and API surfaces. Both surfaces
//! construct a `GoalContext` from `ServiceContext` and delegate to this service.
//!
//! # Depth test
//!
//! Deleting this module would cause GoalID/GoalState/Visibility parsing and
//! error normalization to reappear across 6 call sites (3 API + 3 CLI), with
//! 11 additional repo methods awaiting surface wiring. Passes deletion test.
//!
//! # Design decisions
//!
//! - **Constraint: Guideline** — Auth/capability checks stay in the API surface.
//!   The service layer does not enforce who can create goals; WebID scoping
//!   is the repository's concern.
//! - **Constraint: Guideline** — Goal state machine validation stays in the
//!   repository (`can_transition_to`). The service parses and delegates;
//!   `GoalRepositoryError::InvalidTransition` maps to `ServiceError::GoalRepo`.
//! - **String inputs** — Service methods take string inputs (goal_id, visibility,
//!   state) and parse them internally. This keeps the interface deep: callers
//!   don't need to know the enum types. `WebID` is an exception because it
//!   comes from different sources per surface (AuthContext vs persona).

use std::sync::Arc;

use hkask_storage::SqliteGoalRepository;
use hkask_types::goal::{Goal, GoalState};
use hkask_types::id::GoalID;
use hkask_types::id::WebID;
use hkask_types::visibility::Visibility;

use crate::ServiceError;

/// Lightweight context for `GoalService` calls.
///
/// Contains only the goal repository needed for goal operations. Surfaces
/// construct this from `ServiceContext`:
/// ```ignore
/// let goal_ctx = GoalContext::from(&state.service_context);
/// ```
pub struct GoalContext {
    /// Goal repository for CRUD operations.
    pub goal_repo: Arc<SqliteGoalRepository>,
}

impl GoalContext {
    /// Construct from individual parts.
    pub fn from_parts(goal_repo: Arc<SqliteGoalRepository>) -> Self {
        Self { goal_repo }
    }
}

impl From<&crate::ServiceContext> for GoalContext {
    fn from(ctx: &crate::ServiceContext) -> Self {
        Self {
            goal_repo: ctx.goal_repo.clone(),
        }
    }
}

/// Goal coordination service — create, list, and transition goals.
///
/// Use `GoalService::create_goal()` etc. to delegate goal operations through
/// the service layer. Surfaces construct a `GoalContext` from their own state
/// and call service methods.
pub struct GoalService;

impl GoalService {
    /// Parse a goal ID string into a `GoalID`, normalizing UUID validation.
    ///
    /// Both CLI and API previously duplicated `id.parse::<GoalID>()` with
    /// different error messages. This helper centralizes the parsing and
    /// returns a consistent `ServiceError::ValidationError` for invalid IDs.
    ///
    /// # REQ: svc-goal-001 — parse_goal_id validates UUID format
    pub fn parse_goal_id(id: &str) -> Result<GoalID, ServiceError> {
        id.parse::<GoalID>()
            .map_err(|e| ServiceError::ValidationError(format!("Invalid goal ID: {}", e)))
    }

    /// Parse a visibility string into a `Visibility` enum.
    ///
    /// Accepts lowercase and mixed-case forms (private, shared, public).
    ///
    /// # REQ: svc-goal-002 — parse_visibility accepts valid visibility strings
    pub fn parse_visibility(vis: &str) -> Result<Visibility, ServiceError> {
        Visibility::parse_str(vis).ok_or_else(|| {
            ServiceError::ValidationError(format!(
                "Invalid visibility '{}': expected private | shared | public",
                vis
            ))
        })
    }

    /// Parse a goal state string into a `GoalState` enum.
    ///
    /// Accepts lowercase and mixed-case forms.
    ///
    /// # REQ: svc-goal-003 — parse_goal_state accepts valid state strings
    pub fn parse_goal_state(state: &str) -> Result<GoalState, ServiceError> {
        GoalState::parse_str(state).ok_or_else(|| {
            ServiceError::ValidationError(format!(
                "Invalid goal state '{}': expected pending | active | completed | blocked | abandoned",
                state
            ))
        })
    }

    /// Create a goal owned by the given WebID.
    ///
    /// Parses the visibility string and delegates to the goal repository.
    /// Returns the created `Goal` with all fields populated by the repository.
    ///
    /// # REQ: svc-goal-004 — create_goal creates a goal with parsed visibility
    pub fn create_goal(
        ctx: &GoalContext,
        webid: &WebID,
        text: &str,
        visibility: &str,
    ) -> Result<Goal, ServiceError> {
        let vis = Self::parse_visibility(visibility)?;
        ctx.goal_repo
            .create_goal(webid, text, vis)
            .map_err(ServiceError::GoalRepo)
    }

    /// List goals for a WebID, optionally filtered by state.
    ///
    /// Parses the optional state filter string and delegates to the repository.
    ///
    /// # REQ: svc-goal-005 — list_goals returns goals with optional state filter
    pub fn list_goals(
        ctx: &GoalContext,
        webid: &WebID,
        state_filter: Option<&str>,
    ) -> Result<Vec<Goal>, ServiceError> {
        let filter = match state_filter {
            Some(s) => Some(Self::parse_goal_state(s)?),
            None => None,
        };
        ctx.goal_repo
            .list_goals(webid, filter)
            .map_err(ServiceError::GoalRepo)
    }

    /// Transition a goal to a new state (legal transitions only).
    ///
    /// Parses the goal ID and state strings. The repository validates the
    /// state machine (`can_transition_to`); illegal transitions return
    /// `ServiceError::GoalRepo(GoalRepositoryError::InvalidTransition)`.
    ///
    /// # REQ: svc-goal-006 — set_goal_state transitions goal state with parsed goal ID
    pub fn set_goal_state(
        ctx: &GoalContext,
        goal_id: &str,
        state: &str,
    ) -> Result<(), ServiceError> {
        let id = Self::parse_goal_id(goal_id)?;
        let new_state = Self::parse_goal_state(state)?;
        ctx.goal_repo
            .update_goal_state(id, new_state)
            .map_err(ServiceError::GoalRepo)
    }
}

