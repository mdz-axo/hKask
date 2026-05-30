//! Goal primitive — minimal coordination substrate for multi-agent collaboration
//!
//! Goals enable shared intention tracking across human, replicant, and bot agents.
//! Per Scott Page's "The Difference": high-performance groups require:
//! 1. Shared language (hLexicon provides this)
//! 2. Shared goals (this module provides this)
//!
//! Design principles:
//! - Minimal: text + criteria + state, nothing more
//! - Transient: goals aren't retained long-term; agent memory holds the experience
//! - LLM-judged: verification uses LLM to avoid Goodhart's law
//! - Cross-agent: works equally for human, replicant, bot
//! - Hierarchical: sub-goals decompose large goals (max depth = SYSTEM_MAX_RECURSION)

use crate::capability::SYSTEM_MAX_RECURSION;
pub use crate::id::GoalID;
use crate::id::WebID;
use crate::visibility::Visibility;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub fn as_str(&self) -> &'static str {
        match self {
            GoalState::Pending => "pending",
            GoalState::Active => "active",
            GoalState::Completed => "completed",
            GoalState::Blocked => "blocked",
            GoalState::Abandoned => "abandoned",
        }
    }

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
    /// no-op and always permitted.
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
    pub fn new(goal_id: GoalID, criterion_type: &str, description: &str) -> Self {
        Self {
            id: format!("gc_{}", uuid::Uuid::new_v4().simple()),
            goal_id,
            criterion_type: criterion_type.to_string(),
            description: description.to_string(),
            satisfied: false,
        }
    }

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
}

impl Goal {
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
        }
    }

    pub fn with_parent(mut self, parent_id: GoalID, parent_depth: u8) -> Self {
        self.parent_goal_id = Some(parent_id);
        self.depth = parent_depth + 1;
        self
    }

    pub fn transition(&mut self, new_state: GoalState) {
        if self.state != new_state {
            self.state = new_state;
            if new_state.is_terminal() && self.completed_at.is_none() {
                self.completed_at = Some(Utc::now());
            }
        }
    }

    pub fn activate(&mut self) {
        if self.state == GoalState::Pending {
            self.state = GoalState::Active;
        }
    }

    pub fn complete(&mut self) {
        self.transition(GoalState::Completed);
    }

    pub fn block(&mut self) {
        self.transition(GoalState::Blocked);
    }

    pub fn abandon(&mut self) {
        self.transition(GoalState::Abandoned);
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    pub fn can_have_subgoals(&self) -> bool {
        !self.is_terminal() && self.depth < SYSTEM_MAX_RECURSION
    }
}

/// Goal verification result — from LLM judge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalVerification {
    pub goal_id: GoalID,
    pub verdict: GoalVerdict,
    pub reason: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoalVerdict {
    Done,
    Continue,
    Blocked,
}

impl GoalVerification {
    pub fn new(goal_id: GoalID, verdict: GoalVerdict, reason: &str, confidence: f32) -> Self {
        Self {
            goal_id,
            verdict,
            reason: reason.to_string(),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

