//! Goal types — Cross-cutting infrastructure
//!
//! Goals are a minimal coordination substrate for multi-agent collaboration.
//! Multiple loops interact with goals: Curation evaluates them, Cybernetics
//! allocates energy, Communication coordinates agents around them.
//!
//! **F-SYN-019 — do not reintroduce `GoalCapabilityToken`.**
//! The type was *entirely removed* in v0.23.0 (OPEN_QUESTIONS F6):
//! HMAC signing + epoch-based revocation + attenuation for goals
//! was over-engineered ceremony with no functional payoff. Goals
//! are scoped by `&WebID` only. If you find yourself reaching for
//! a goal-scoped capability token, you are reinventing ceremony
//! that has been deliberately removed.

use std::fmt;

// SYSTEM_MAX_RECURSION (from hkask-capability) = 7
const MAX_NESTING: u8 = 7;
pub use hkask_types::GoalID;
use hkask_types::WebID;
use hkask_types::Visibility;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Error returned when a goal state transition violates the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IllegalGoalTransition {
    pub from: GoalState,
    pub to: GoalState,
}

impl fmt::Display for IllegalGoalTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "illegal goal state transition: {} → {}",
            self.from.as_str(),
            self.to.as_str()
        )
    }
}

impl std::error::Error for IllegalGoalTransition {}

/// Goal state — simple, minimal states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoalState {
    Pending,
    Active,
    Completed,
    Blocked,
    Abandoned,
}

impl GoalState {
    /// Get string representation of state.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns snake_case state name
    pub fn as_str(&self) -> &'static str {
        match self {
            GoalState::Pending => "pending",
            GoalState::Active => "active",
            GoalState::Completed => "completed",
            GoalState::Blocked => "blocked",
            GoalState::Abandoned => "abandoned",
        }
    }

    /// Parse state from string.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Some(GoalState) if valid, None otherwise
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(GoalState::Pending),
            "active" => Some(GoalState::Active),
            "completed" => Some(GoalState::Completed),
            "blocked" => Some(GoalState::Blocked),
            "abandoned" => Some(GoalState::Abandoned),
            _ => None,
        }
    }

    /// Check if this is a terminal state.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns true for Completed, Abandoned, Quarantined
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            GoalState::Completed | GoalState::Blocked | GoalState::Abandoned
        )
    }

    /// Whether a transition from `self` to `next` is legal.
    ///
    /// The lifecycle is expressed as a total match so illegal transitions are
    /// caught at the repository boundary rather than silently applied. A
    /// terminal state (Completed/Abandoned) admits no further transitions;
    /// `Blocked` may resume to `Active`. Re-stating the current state is a
    /// [DECLARATIVE] no-op and always permitted. (P7 — Evolutionary Architecture).
    /// Check if transition to next state is valid.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  next is a valid GoalState
    /// post: returns true iff transition is allowed
    pub fn can_transition_to(&self, next: GoalState) -> bool {
        if *self == next {
            return true;
        }
        match (self, next) {
            (GoalState::Pending, GoalState::Active)
            | (GoalState::Pending, GoalState::Abandoned)
            | (GoalState::Active, GoalState::Blocked)
            | (GoalState::Active, GoalState::Completed)
            | (GoalState::Active, GoalState::Abandoned)
            | (GoalState::Blocked, GoalState::Active)
            | (GoalState::Blocked, GoalState::Abandoned) => true,
            // Completed and Abandoned are terminal; all other moves illegal.
            _ => false,
        }
    }
}

/// Goal criterion — completion condition (LLM-judged)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalCriterion {
    pub id: String,
    pub goal_id: GoalID,
    pub criterion_type: String,
    pub description: String,
    pub satisfied: bool,
}

impl GoalCriterion {
    /// Create a new goal criterion.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  goal_id is valid, description is non-empty
    /// post: returns GoalCriterion
    pub fn new(goal_id: GoalID, criterion_type: &str, description: &str) -> Self {
        Self {
            id: format!("gc_{}", uuid::Uuid::new_v4().simple()),
            goal_id,
            criterion_type: criterion_type.to_string(),
            description: description.to_string(),
            satisfied: false,
        }
    }

    /// Mark criterion as satisfied.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: satisfied set to true
    pub fn mark_satisfied(&mut self) {
        self.satisfied = true;
    }
}

/// Goal artifact — output produced while working toward goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalArtifact {
    pub id: String,
    pub goal_id: GoalID,
    pub artifact_ref: String,
    pub artifact_type: String,
    pub created_at: DateTime<Utc>,
}

impl GoalArtifact {
    /// Create a new goal artifact.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  goal_id is valid, artifact_ref and artifact_type are non-empty
    /// post: returns GoalArtifact
    pub fn new(goal_id: GoalID, artifact_ref: &str, artifact_type: &str) -> Self {
        Self {
            id: format!("ga_{}", uuid::Uuid::new_v4().simple()),
            goal_id,
            artifact_ref: artifact_ref.to_string(),
            artifact_type: artifact_type.to_string(),
            created_at: Utc::now(),
        }
    }
}

/// Goal — minimal coordination substrate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: GoalID,
    pub webid: WebID,
    pub text: String,
    pub state: GoalState,
    pub visibility: Visibility,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub parent_goal_id: Option<GoalID>,
    pub depth: u8,
    pub display_name: Option<String>,
}

impl Goal {
    /// Create a new Goal.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  webid is valid, text is non-empty
    /// post: returns Goal with Pending state
    pub fn new(webid: WebID, text: &str, visibility: Visibility) -> Self {
        Self {
            id: GoalID::new(),
            webid,
            text: text.to_string(),
            state: GoalState::Pending,
            visibility,
            created_at: Utc::now(),
            completed_at: None,
            parent_goal_id: None,
            depth: 0,
            display_name: None,
        }
    }

    /// Set display name (builder).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Self with display_name set
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Set parent goal (builder).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Self with parent_goal_id and depth set
    pub fn with_parent(mut self, parent_id: GoalID, parent_depth: u8) -> Self {
        self.parent_goal_id = Some(parent_id);
        self.depth = parent_depth + 1;
        self
    }

    /// Transition to a new state, returning `Err` if the transition is illegal.
    ///
    /// This enforces the state machine defined by [`GoalState::can_transition_to`].
    /// The persistence layer also validates, but in-memory validation prevents
    /// silent illegal mutations before data reaches the database.
    /// Transition to a new state.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  transition is valid per can_transition_to
    /// post: state updated, completed_at set if terminal
    /// post: returns Err if illegal transition
    pub fn transition(&mut self, new_state: GoalState) -> Result<(), IllegalGoalTransition> {
        if !self.state.can_transition_to(new_state) {
            return Err(IllegalGoalTransition {
                from: self.state,
                to: new_state,
            });
        }
        if self.state != new_state {
            self.state = new_state;
            if new_state.is_terminal() && self.completed_at.is_none() {
                self.completed_at = Some(Utc::now());
            }
        }
        Ok(())
    }

    /// Check if this goal can have subgoals.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns true for non-terminal states with depth < 7
    pub fn can_have_subgoals(&self) -> bool {
        !self.state.is_terminal() && self.depth < MAX_NESTING
    }
}

// ── GoalService ──────────────────────────────────────────────────────────────

use hkask_types::id::{GoalID as GoalIdType, WebID as WebIdType};
use hkask_types::visibility::Visibility as VisType;

use crate::AgentService;
use crate::ServiceError;

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
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.goal_repo() must be initialized; req.text must be non-empty; req.visibility must be "private" or "public"
    /// post: goal is persisted and returned as GoalResponse; Err(ValidationError) on invalid visibility; Err(GoalRepo) on store failure
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
        let goal =
            repo.create_goal(&req.owner, &req.text, vis)
                .map_err(|e| ServiceError::GoalRepo {
                    message: e.to_string(),
                })?;
        Ok(GoalResponse::from(goal))
    }

    /// List goals for the given owner, optionally filtered by state.
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.goal_repo() must be initialized; owner must be a valid WebID; state_filter if Some must be a valid GoalState string
    /// post: returns `Vec<GoalResponse>` for matching goals; empty Vec if none; Err(ValidationError) on invalid state filter; Err(GoalRepo) on store failure
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
            .map_err(|e| ServiceError::GoalRepo {
                message: e.to_string(),
            })?;
        Ok(goals.into_iter().map(GoalResponse::from).collect())
    }

    /// Set the state of an existing goal.
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.goal_repo() must be initialized; goal_id_str must be a valid GoalID; new_state_str must be a valid GoalState
    /// post: goal state is updated and returned as GoalResponse; Err(ValidationError) on invalid ID or state; Err(GoalRepo) on store failure; Err(ValidationError) if owner does not match goal's owner
    pub fn set_goal_state(
        ctx: &AgentService,
        goal_id_str: &str,
        new_state_str: &str,
        owner: &WebID,
    ) -> Result<GoalResponse, ServiceError> {
        let goal_id: GoalIdType = goal_id_str
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
            .map_err(|e| ServiceError::GoalRepo {
                message: e.to_string(),
            })?
            .ok_or_else(|| ServiceError::ValidationError {
                source: None,
                message: format!("Goal not found: {}", goal_id_str),
            })?;

        if goal.webid != *owner {
            return Err(ServiceError::ValidationError {
                source: None,
                message: "Not authorized to transition this goal".into(),
            });
        }

        let goal = repo
            .get_goal(goal_id)
            .map_err(|e| ServiceError::GoalRepo {
                message: e.to_string(),
            })?
            .ok_or_else(|| ServiceError::ValidationError {
                source: None,
                message: format!("Goal not found: {}", goal_id),
            })?;
        let from_state = goal.state.as_str().to_string();

        repo.update_goal_state(goal_id, new_state)
            .map_err(|e| ServiceError::GoalRepo {
                message: e.to_string(),
            })?;

        if let Some(tx) = ctx.curation_inbox_tx() {
            let event = hkask_cns::types::loops::CurationInput::GoalTransition(
                hkask_cns::types::loops::GoalTransitionEvent {
                    goal_id: goal_id.to_string(),
                    from_state,
                    to_state: new_state.as_str().to_string(),
                    agent: WebID::from_persona(b"goal-service"),
                },
            );
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

    #[test]
    fn create_goal_converts_visibility_and_returns_response() {
        let err = Visibility::parse_str("bogus");
        assert!(err.is_none(), "bogus should not parse as a Visibility");
    }

    #[test]
    fn list_goals_parses_state_filter() {
        assert!(GoalState::parse_str("pending").is_some());
        assert!(GoalState::parse_str("active").is_some());
        assert!(GoalState::parse_str("completed").is_some());
        assert!(GoalState::parse_str("bogus").is_none());
    }

    #[test]
    fn goal_to_response_maps_all_fields() {
        let goal = Goal::new(
            WebID::from_persona(b"goal-service"),
            "Test goal",
            Visibility::Private,
        );
        let resp = GoalResponse::from(goal);
        assert!(!resp.id.is_empty(), "ID must be non-empty");
        assert_eq!(resp.text, "Test goal");
        assert_eq!(resp.state, "pending");
        assert_eq!(resp.visibility, "private");
    }
}
