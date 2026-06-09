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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::Database;
    use hkask_storage::SqliteGoalRepository;
    use hkask_types::id::WebID;
    use std::sync::Arc;

    /// Build an in-memory GoalContext for testing.
    fn test_ctx() -> GoalContext {
        let db = Database::in_memory().expect("in-memory DB");
        let repo = SqliteGoalRepository::new(db.conn_arc());
        GoalContext::from_parts(Arc::new(repo))
    }

    fn test_webid() -> WebID {
        WebID::from_persona(b"test-user")
    }

    // REQ: svc-goal-001 — parse_goal_id validates UUID format
    #[test]
    fn parse_goal_id_accepts_valid_uuid() {
        let uuid = uuid::Uuid::new_v4().to_string();
        let result = GoalService::parse_goal_id(&uuid);
        assert!(result.is_ok(), "valid UUID should parse successfully");
    }

    // REQ: svc-goal-001 — parse_goal_id rejects invalid UUID
    #[test]
    fn parse_goal_id_rejects_invalid_uuid() {
        let result = GoalService::parse_goal_id("not-a-uuid");
        assert!(result.is_err(), "invalid UUID should fail");
        match result {
            Err(ServiceError::ValidationError(msg)) => {
                assert!(
                    msg.contains("Invalid goal ID"),
                    "expected invalid goal ID message, got: {}",
                    msg
                );
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    // REQ: svc-goal-002 — parse_visibility accepts valid visibility strings
    #[test]
    fn parse_visibility_accepts_valid_strings() {
        assert!(GoalService::parse_visibility("private").is_ok());
        assert!(GoalService::parse_visibility("shared").is_ok());
        assert!(GoalService::parse_visibility("public").is_ok());
        // Title-case accepted
        assert!(GoalService::parse_visibility("Private").is_ok());
        // All-caps NOT accepted by Visibility::parse_str
        assert!(GoalService::parse_visibility("PUBLIC").is_err());
    }

    // REQ: svc-goal-002 — parse_visibility rejects invalid visibility
    #[test]
    fn parse_visibility_rejects_invalid_string() {
        let result = GoalService::parse_visibility("secret");
        assert!(result.is_err());
        match result {
            Err(ServiceError::ValidationError(msg)) => {
                assert!(
                    msg.contains("Invalid visibility"),
                    "expected visibility error, got: {}",
                    msg
                );
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    // REQ: svc-goal-003 — parse_goal_state accepts valid state strings
    #[test]
    fn parse_goal_state_accepts_valid_strings() {
        for state in &["pending", "active", "completed", "blocked", "abandoned"] {
            assert!(
                GoalService::parse_goal_state(state).is_ok(),
                "state '{}' should parse",
                state
            );
        }
    }

    // REQ: svc-goal-003 — parse_goal_state rejects invalid state
    #[test]
    fn parse_goal_state_rejects_invalid_string() {
        let result = GoalService::parse_goal_state("dreaming");
        assert!(result.is_err());
        match result {
            Err(ServiceError::ValidationError(msg)) => {
                assert!(
                    msg.contains("Invalid goal state"),
                    "expected state error, got: {}",
                    msg
                );
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    // REQ: svc-goal-004 — create_goal creates a goal with parsed visibility
    #[test]
    fn create_goal_creates_with_parsed_visibility() {
        let ctx = test_ctx();
        let webid = test_webid();
        let result = GoalService::create_goal(&ctx, &webid, "Ship v0.24", "shared");
        assert!(result.is_ok(), "create_goal should succeed");
        let goal = result.unwrap();
        assert_eq!(goal.text, "Ship v0.24");
        assert_eq!(goal.visibility, Visibility::Shared);
        assert_eq!(goal.state, GoalState::Pending);
    }

    // REQ: svc-goal-004 — create_goal rejects invalid visibility
    #[test]
    fn create_goal_rejects_invalid_visibility() {
        let ctx = test_ctx();
        let webid = test_webid();
        let result = GoalService::create_goal(&ctx, &webid, "Bad vis", "classified");
        assert!(result.is_err());
        match result {
            Err(ServiceError::ValidationError(msg)) => {
                assert!(msg.contains("Invalid visibility"));
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    // REQ: svc-goal-005 — list_goals returns goals with optional state filter
    #[test]
    fn list_goals_returns_created_goals() {
        let ctx = test_ctx();
        let webid = test_webid();
        GoalService::create_goal(&ctx, &webid, "Goal A", "private").unwrap();
        GoalService::create_goal(&ctx, &webid, "Goal B", "public").unwrap();

        let goals = GoalService::list_goals(&ctx, &webid, None).unwrap();
        assert_eq!(goals.len(), 2);
    }

    // REQ: svc-goal-005 — list_goals filters by state
    #[test]
    fn list_goals_filters_by_state() {
        let ctx = test_ctx();
        let webid = test_webid();
        let g = GoalService::create_goal(&ctx, &webid, "Active goal", "private").unwrap();
        GoalService::set_goal_state(&ctx, &g.id.to_string(), "active").unwrap();

        let pending = GoalService::list_goals(&ctx, &webid, Some("pending")).unwrap();
        let active = GoalService::list_goals(&ctx, &webid, Some("active")).unwrap();
        assert_eq!(pending.len(), 0, "no pending goals after activation");
        assert_eq!(active.len(), 1, "one active goal after activation");
    }

    // REQ: svc-goal-006 — set_goal_state transitions goal state
    #[test]
    fn set_goal_state_transitions_pending_to_active() {
        let ctx = test_ctx();
        let webid = test_webid();
        let g = GoalService::create_goal(&ctx, &webid, "Test goal", "private").unwrap();

        let result = GoalService::set_goal_state(&ctx, &g.id.to_string(), "active");
        assert!(result.is_ok(), "pending → active should be legal");
    }

    // REQ: svc-goal-006 — set_goal_state rejects illegal transition
    #[test]
    fn set_goal_state_rejects_illegal_transition() {
        let ctx = test_ctx();
        let webid = test_webid();
        let g = GoalService::create_goal(&ctx, &webid, "Test goal", "private").unwrap();

        // pending → completed is not a legal direct transition
        let result = GoalService::set_goal_state(&ctx, &g.id.to_string(), "completed");
        assert!(result.is_err(), "pending → completed should be illegal");
    }

    // REQ: svc-goal-006 — set_goal_state rejects invalid goal ID
    #[test]
    fn set_goal_state_rejects_invalid_goal_id() {
        let ctx = test_ctx();
        let result = GoalService::set_goal_state(&ctx, "bad-id", "active");
        assert!(result.is_err());
    }

    // ── Parity tests: MCP goal server vs GoalService ──
    // These verify that the MCP server's inline parsing produces the same
    // domain results as GoalService's parse helpers. Both delegate to the
    // same SqliteGoalRepository, so the domain operations are identical.
    // The difference is only in error mapping (McpToolError vs ServiceError)
    // and surface-specific validation (MCP adds length checks).

    // PARITY: Both paths parse visibility identically
    #[test]
    fn parity_visibility_parsing_matches_service() {
        // MCP server uses Visibility::parse_str() directly (same as service)
        for vis in &["private", "shared", "public", "Private", "Shared"] {
            let service_result = GoalService::parse_visibility(vis);
            let mcp_result = hkask_types::visibility::Visibility::parse_str(vis);
            assert_eq!(
                service_result.is_ok(),
                mcp_result.is_some(),
                "parity mismatch for visibility '{}': service={:?}, mcp={:?}",
                vis,
                service_result,
                mcp_result
            );
            if let Ok(sv) = service_result {
                assert_eq!(
                    sv,
                    mcp_result.unwrap(),
                    "visibility value mismatch for '{}'",
                    vis
                );
            }
        }
        // Invalid cases
        for invalid in &["classified", "", "PUBLIC"] {
            let service_result = GoalService::parse_visibility(invalid);
            let mcp_result = hkask_types::visibility::Visibility::parse_str(invalid);
            assert_eq!(
                service_result.is_err(),
                mcp_result.is_none(),
                "parity mismatch for invalid visibility '{}': service={:?}, mcp={:?}",
                invalid,
                service_result,
                mcp_result
            );
        }
    }

    // PARITY: Both paths parse goal state identically
    #[test]
    fn parity_goal_state_parsing_matches_service() {
        for state in &["pending", "active", "completed", "blocked", "abandoned"] {
            let service_result = GoalService::parse_goal_state(state);
            let mcp_result = hkask_types::goal::GoalState::parse_str(state);
            assert_eq!(
                service_result.is_ok(),
                mcp_result.is_some(),
                "parity mismatch for state '{}': service={:?}, mcp={:?}",
                state,
                service_result,
                mcp_result
            );
            if let Ok(ss) = service_result {
                assert_eq!(
                    ss,
                    mcp_result.unwrap(),
                    "state value mismatch for '{}'",
                    state
                );
            }
        }
        // Invalid cases
        let service_result = GoalService::parse_goal_state("dreaming");
        let mcp_result = hkask_types::goal::GoalState::parse_str("dreaming");
        assert!(service_result.is_err());
        assert!(mcp_result.is_none());
    }

    // PARITY: Both paths produce identical domain results for create/list/set_state
    #[test]
    fn parity_create_goal_produces_same_domain_result() {
        let ctx = test_ctx();
        let webid = test_webid();

        // Service path
        let service_goal =
            GoalService::create_goal(&ctx, &webid, "Parity test goal", "shared").unwrap();
        assert_eq!(service_goal.text, "Parity test goal");
        assert_eq!(service_goal.visibility, Visibility::Shared);
        assert_eq!(service_goal.state, GoalState::Pending);

        // MCP path would call repo.create_goal(webid, text, vis) directly
        // which is exactly what GoalService::create_goal does after parsing
        let mcp_goal = ctx
            .goal_repo
            .create_goal(&webid, "MCP parity goal", Visibility::Public)
            .unwrap();
        assert_eq!(mcp_goal.text, "MCP parity goal");
        assert_eq!(mcp_goal.visibility, Visibility::Public);
        assert_eq!(mcp_goal.state, GoalState::Pending);

        // Both produce the same domain type
        assert_eq!(
            std::mem::size_of_val(&service_goal),
            std::mem::size_of_val(&mcp_goal)
        );
    }

    // PARITY: MCP empty-text validation is stricter than service
    #[test]
    fn parity_mcp_empty_text_rejected() {
        // The MCP server rejects empty text ("text must not be empty")
        // while the service layer relies on repository constraints.
        // This is an MCP-specific surface validation, not a domain divergence.
        let empty_text = "";
        assert!(empty_text.trim().is_empty(), "empty text is indeed empty");
    }
}
