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

use crate::capability::SYSTEM_MAX_RECURSION;
pub use crate::id::GoalID;
use crate::id::WebID;
use crate::visibility::Visibility;
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
    /// Check if transition to next state is valid.
    ///
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
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Set parent goal (builder).
    ///
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
    pub fn can_have_subgoals(&self) -> bool {
        !self.state.is_terminal() && self.depth < SYSTEM_MAX_RECURSION
    }
}
