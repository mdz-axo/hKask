//! hKask Goal Primitive — Types for goal representation and lifecycle
//!
//! **hLexicon Integration:**
//! - WordAct: `GoalCommitment` (commissive acts: pledge, commit, undertake, promise)
//! - FlowDef: `GoalFlow` (decomposition: sequence, parallel, choice)
//! - KnowAct: Monitoring via `GoalVerifier` (orient, ground, monitor, evaluate, regulate)
//!
//! **Design Principles:**
//! - Goals are first-class entities with formal lifecycle
//! - Completion is externally verified (never self-reported)
//! - OCAP-gated delegation with capability attenuation

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::id::SessionId;
use crate::visibility::Visibility;

/// Unique identifier for a goal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GoalId(pub Uuid);

impl GoalId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for GoalId {
    fn default() -> Self {
        Self::new()
    }
}

/// WordAct commissive acts — goal commitment semantics
///
/// Based on speech act theory (Searle 1969), commissive acts commit the speaker
/// to a future course of action. The strength of commitment varies:
/// - Pledge: Weak intention ("I intend to...")
/// - Commit: Medium commitment ("I will...")
/// - Undertake: Strong acceptance ("I accept responsibility for...")
/// - Promise: Strongest guarantee ("I guarantee...")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalCommitment {
    Pledge,
    #[default]
    Commit,
    Undertake,
    Promise,
}

impl std::fmt::Display for GoalCommitment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalCommitment::Pledge => write!(f, "pledge"),
            GoalCommitment::Commit => write!(f, "commit"),
            GoalCommitment::Undertake => write!(f, "undertake"),
            GoalCommitment::Promise => write!(f, "promise"),
        }
    }
}

/// Goal lifecycle state
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalState {
    #[default]
    Active,
    Paused { reason: String },
    Done { reason: String },
    Cleared,
    Blocked { reason: String },
}

/// FlowDef decomposition patterns for goal structure
///
/// Based on workflow patterns (van der Aalst 2003), goals can be decomposed into:
/// - Sequence: Linear ordering of subgoals
/// - Parallel: Concurrent execution branches
/// - Choice: Conditional branching based on criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "flow_type", rename_all = "snake_case")]
pub enum GoalFlow {
    Sequence { steps: Vec<SubgoalSpec> },
    Parallel { branches: Vec<SubgoalSpec> },
    Choice { branches: Vec<(String, SubgoalSpec)> },
}

/// Subgoal specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgoalSpec {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_estimate: Option<String>,
}

/// Completion criterion types for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompletionCriterion {
    /// Command execution with exit code check
    Command {
        command: String,
        expected_exit_code: i32,
    },
    /// State inspection with pattern match
    State {
        check: String,
        expected_pattern: String,
    },
    /// Semantic evaluation via LLM
    Semantic {
        evaluator: String,
        criteria: String,
    },
}

/// Goal specification (creation input)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSpec {
    pub owner_webid: String,
    pub session_id: SessionId,
    pub goal_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_ref: Option<String>,
    #[serde(default)]
    pub commitment_level: GoalCommitment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<GoalFlow>,
    #[serde(default)]
    pub completion_criteria: Vec<CompletionCriterion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_budget: Option<u64>,
    #[serde(default)]
    pub visibility: Visibility,
}

/// Goal entity (full representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: GoalId,
    pub session_id: SessionId,
    pub owner_webid: String,
    pub goal_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_ref: Option<String>,
    pub state: GoalState,
    pub commitment_level: GoalCommitment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<GoalFlow>,
    #[serde(default)]
    pub completion_criteria: Vec<CompletionCriterion>,
    #[serde(default)]
    pub subgoals: Vec<Subgoal>,
    #[serde(default)]
    pub turns_used: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_budget: Option<u64>,
    #[serde(default)]
    pub energy_used: u64,
    #[serde(default)]
    pub max_turns: u32,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub visibility: Visibility,
}

impl Goal {
    /// Create a new goal from specification
    pub fn new(spec: GoalSpec) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: GoalId::new(),
            session_id: spec.session_id,
            owner_webid: spec.owner_webid,
            goal_text: spec.goal_text,
            template_ref: spec.template_ref,
            state: GoalState::Active,
            commitment_level: spec.commitment_level,
            flow: spec.flow,
            completion_criteria: spec.completion_criteria,
            subgoals: Vec::new(),
            turns_used: 0,
            energy_budget: spec.energy_budget,
            energy_used: 0,
            max_turns: spec.max_turns.unwrap_or(20),
            created_at: now,
            last_turn_at: None,
            completed_at: None,
            visibility: spec.visibility,
        }
    }
    
    /// Estimate goal complexity for variety counter
    pub fn estimate_complexity(&self) -> usize {
        let base = 1; // Goal itself
        let criteria = self.completion_criteria.len();
        let subgoals = self.subgoals.len();
        let flow_complexity = self.flow.as_ref().map_or(0, |f| match f {
            GoalFlow::Sequence { steps } => steps.len(),
            GoalFlow::Parallel { branches } => branches.len() * 2,
            GoalFlow::Choice { branches } => branches.len() * 3,
        });
        
        base + criteria + subgoals + flow_complexity
    }
}

/// Subgoal (user-added criteria)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subgoal {
    pub ordinal: u32,
    pub text: String,
    #[serde(default)]
    pub satisfied: bool,
}

/// Goal outcome (execution result)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome_type", rename_all = "snake_case")]
pub enum GoalOutcome {
    Success {
        summary: String,
        #[serde(default)]
        artifacts: Vec<String>,
    },
    Failure {
        reason: String,
        #[serde(default)]
        recoverable: bool,
    },
    Partial {
        summary: String,
        #[serde(default)]
        completed_criteria: Vec<usize>,
        #[serde(default)]
        failed_criteria: Vec<usize>,
    },
}

/// Verification verdict from GoalVerifier
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    Done {
        reason: String,
        #[serde(default = "default_confidence")]
        confidence: f64,
    },
    Continue {
        reason: String,
    },
    Blocked {
        reason: String,
        #[serde(default)]
        needs_human: bool,
    },
}

fn default_confidence() -> f64 {
    1.0
}
