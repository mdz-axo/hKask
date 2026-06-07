//! Goal types — Cross-cutting infrastructure
//!
//! Goals are a minimal coordination substrate for multi-agent collaboration.
//! Multiple loops interact with goals: Curation evaluates them, Cybernetics
//! allocates energy, Communication coordinates agents around them.

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
    pub display_name: Option<String>,
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
            display_name: None,
        }
    }

    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

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

    pub fn activate(&mut self) {
        // Pending → Active is always legal per can_transition_to
        let _ = self.transition(GoalState::Active);
    }

    pub fn complete(&mut self) {
        let _ = self.transition(GoalState::Completed);
    }

    pub fn block(&mut self) {
        let _ = self.transition(GoalState::Blocked);
    }

    pub fn abandon(&mut self) {
        let _ = self.transition(GoalState::Abandoned);
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    pub fn can_have_subgoals(&self) -> bool {
        !self.is_terminal() && self.depth < SYSTEM_MAX_RECURSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::WebID;
    use crate::visibility::Visibility;

    // ── GoalState variant count ──────────────────────────────────────────

    #[test]
    fn goal_state_has_exactly_five_variants() {
        // P8: GoalState has exactly 5 variants matching the lifecycle model.
        let variants = [
            GoalState::Pending,
            GoalState::Active,
            GoalState::Completed,
            GoalState::Blocked,
            GoalState::Abandoned,
        ];
        assert_eq!(variants.len(), 5);
        // Verify they are distinct.
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j], "duplicate variant");
            }
        }
    }

    // ── GoalState::as_str / parse_str roundtrip ────────────────────────

    #[test]
    fn goal_state_as_str_roundtrip() {
        // P8: every variant roundtrips through as_str() and parse_str().
        for variant in [
            GoalState::Pending,
            GoalState::Active,
            GoalState::Completed,
            GoalState::Blocked,
            GoalState::Abandoned,
        ] {
            let s = variant.as_str();
            assert_eq!(GoalState::parse_str(s), Some(variant));
        }
    }

    #[test]
    fn goal_state_parse_str_case_insensitive() {
        // P8: parse_str handles mixed case.
        assert_eq!(GoalState::parse_str("PENDING"), Some(GoalState::Pending));
        assert_eq!(GoalState::parse_str("active"), Some(GoalState::Active));
        assert_eq!(GoalState::parse_str("BLOCKED"), Some(GoalState::Blocked));
        assert_eq!(
            GoalState::parse_str("Completed"),
            Some(GoalState::Completed)
        );
        assert_eq!(
            GoalState::parse_str("aBaNdOnEd"),
            Some(GoalState::Abandoned)
        );
    }

    #[test]
    fn goal_state_parse_str_invalid_returns_none() {
        // P8: invalid string returns None.
        assert_eq!(GoalState::parse_str("unknown"), None);
        assert_eq!(GoalState::parse_str(""), None);
        assert_eq!(GoalState::parse_str("PENDINGX"), None);
    }

    // ── GoalState::is_terminal ──────────────────────────────────────────

    #[test]
    fn goal_state_is_terminal_for_completed_blocked_abandoned() {
        // P8: Completed, Blocked, Abandoned are terminal.
        assert!(GoalState::Completed.is_terminal());
        assert!(GoalState::Blocked.is_terminal());
        assert!(GoalState::Abandoned.is_terminal());
    }

    #[test]
    fn goal_state_is_not_terminal_for_pending_active() {
        // P8: Pending and Active are not terminal.
        assert!(!GoalState::Pending.is_terminal());
        assert!(!GoalState::Active.is_terminal());
    }

    // ── GoalState::can_transition_to ───────────────────────────────────

    #[test]
    fn goal_state_can_transition_to_pending_from_pending() {
        // P8: self-transitions are always allowed.
        assert!(GoalState::Pending.can_transition_to(GoalState::Pending));
        assert!(GoalState::Active.can_transition_to(GoalState::Active));
        assert!(GoalState::Completed.can_transition_to(GoalState::Completed));
        assert!(GoalState::Blocked.can_transition_to(GoalState::Blocked));
        assert!(GoalState::Abandoned.can_transition_to(GoalState::Abandoned));
    }

    #[test]
    fn goal_state_pending_transitions() {
        // P8: Pending → Active and Pending → Abandoned are legal;
        //     Pending → Completed/Blocked are illegal.
        assert!(GoalState::Pending.can_transition_to(GoalState::Active));
        assert!(GoalState::Pending.can_transition_to(GoalState::Abandoned));
        assert!(!GoalState::Pending.can_transition_to(GoalState::Completed));
        assert!(!GoalState::Pending.can_transition_to(GoalState::Blocked));
    }

    #[test]
    fn goal_state_active_transitions() {
        // P8: Active → Blocked/Completed/Abandoned are legal;
        //     Active → Pending is illegal.
        assert!(GoalState::Active.can_transition_to(GoalState::Blocked));
        assert!(GoalState::Active.can_transition_to(GoalState::Completed));
        assert!(GoalState::Active.can_transition_to(GoalState::Abandoned));
        assert!(!GoalState::Active.can_transition_to(GoalState::Pending));
    }

    #[test]
    fn goal_state_blocked_transitions() {
        // P8: Blocked → Active/Abandoned are legal;
        //     Blocked → Pending/Completed are illegal.
        assert!(GoalState::Blocked.can_transition_to(GoalState::Active));
        assert!(GoalState::Blocked.can_transition_to(GoalState::Abandoned));
        assert!(!GoalState::Blocked.can_transition_to(GoalState::Pending));
        assert!(!GoalState::Blocked.can_transition_to(GoalState::Completed));
    }

    #[test]
    fn goal_state_completed_is_terminal() {
        // P8: Completed → any non-Completed is illegal.
        assert!(!GoalState::Completed.can_transition_to(GoalState::Pending));
        assert!(!GoalState::Completed.can_transition_to(GoalState::Active));
        assert!(!GoalState::Completed.can_transition_to(GoalState::Blocked));
        assert!(!GoalState::Completed.can_transition_to(GoalState::Abandoned));
    }

    #[test]
    fn goal_state_abandoned_is_terminal() {
        // P8: Abandoned → any non-Abandoned is illegal.
        assert!(!GoalState::Abandoned.can_transition_to(GoalState::Pending));
        assert!(!GoalState::Abandoned.can_transition_to(GoalState::Active));
        assert!(!GoalState::Abandoned.can_transition_to(GoalState::Blocked));
        assert!(!GoalState::Abandoned.can_transition_to(GoalState::Completed));
    }

    // ── IllegalGoalTransition Display & Error ───────────────────────────

    #[test]
    fn illegal_goal_transition_display_format() {
        // P8: IllegalGoalTransition Display shows
        //    "illegal goal state transition: {from} → {to}".
        let err = IllegalGoalTransition {
            from: GoalState::Pending,
            to: GoalState::Completed,
        };
        assert_eq!(
            format!("{err}"),
            "illegal goal state transition: pending → completed"
        );
    }

    #[test]
    fn illegal_goal_transition_is_error() {
        // P8: IllegalGoalTransition implements std::error::Error.
        fn assert_error<E: std::error::Error>() {}
        assert_error::<IllegalGoalTransition>();
    }

    // ── Goal construction ──────────────────────────────────────────────

    fn make_goal() -> Goal {
        Goal::new(WebID::new(), "test goal", Visibility::Private)
    }

    #[test]
    fn goal_new_starts_as_pending() {
        // P8: new goals start in Pending state.
        let goal = make_goal();
        assert_eq!(goal.state, GoalState::Pending);
    }

    #[test]
    fn goal_new_has_no_parent() {
        // P8: new goals have no parent and depth 0.
        let goal = make_goal();
        assert!(goal.parent_goal_id.is_none());
        assert_eq!(goal.depth, 0);
    }

    // ── Goal::transition ───────────────────────────────────────────────

    #[test]
    fn goal_transition_pending_to_active_succeeds() {
        // P8: Pending → Active transition succeeds.
        let mut goal = make_goal();
        assert!(goal.transition(GoalState::Active).is_ok());
        assert_eq!(goal.state, GoalState::Active);
    }

    #[test]
    fn goal_transition_pending_to_completed_fails() {
        // P8: Pending → Completed transition fails.
        let mut goal = make_goal();
        let err = goal.transition(GoalState::Completed).unwrap_err();
        assert_eq!(err.from, GoalState::Pending);
        assert_eq!(err.to, GoalState::Completed);
    }

    #[test]
    fn goal_transition_terminal_state_fails() {
        // P8: terminal states reject all non-self transitions.
        for terminal in [GoalState::Completed, GoalState::Abandoned] {
            for target in [GoalState::Pending, GoalState::Active, GoalState::Blocked] {
                assert!(
                    !terminal.can_transition_to(target),
                    "{terminal:?} → {target:?} should be illegal"
                );
            }
        }
    }

    #[test]
    fn goal_transition_sets_completed_at() {
        // P8: transitioning to a terminal state sets completed_at.
        let mut goal = make_goal();
        assert!(goal.completed_at.is_none());
        goal.transition(GoalState::Active).unwrap();
        // Active is not terminal, so completed_at stays None.
        assert!(goal.completed_at.is_none());
        goal.transition(GoalState::Completed).unwrap();
        assert!(
            goal.completed_at.is_some(),
            "completed_at must be set on terminal transition"
        );
    }

    #[test]
    fn goal_transition_self_is_noop() {
        // P8: self-transition is a no-op that succeeds.
        let mut goal = make_goal();
        let state_before = goal.state;
        let completed_before = goal.completed_at;
        assert!(goal.transition(GoalState::Pending).is_ok());
        assert_eq!(goal.state, state_before);
        assert_eq!(goal.completed_at, completed_before);
    }

    // ── Goal::can_have_subgoals ────────────────────────────────────────

    #[test]
    fn goal_can_have_subgoals_when_active() {
        // P8: active non-terminal goals can have subgoals.
        let mut goal = make_goal();
        goal.activate();
        assert!(!goal.is_terminal());
        assert!(goal.can_have_subgoals());
    }

    #[test]
    fn goal_cannot_have_subgoals_when_terminal() {
        // P8: terminal goals cannot have subgoals.
        let mut goal = make_goal();
        goal.activate();
        goal.complete();
        assert!(goal.is_terminal());
        assert!(!goal.can_have_subgoals());
    }

    // F-SYN-011: depth-7 goals (the cap) cannot have subgoals.
    #[test]
    fn goal_at_depth_cap_cannot_have_subgoals() {
        use crate::capability::SYSTEM_MAX_RECURSION;
        // Build a goal whose depth equals the system cap by chaining
        // `with_parent` (each call returns a new goal with depth+1).
        let mut goal = make_goal();
        goal.activate();
        for _ in 0..SYSTEM_MAX_RECURSION {
            let new_depth = goal.depth + 1;
            let fresh_parent = crate::id::GoalID::new();
            // Use `std::mem::replace` to swap the moved goal with a
            // freshly-built one — we only care about the *depth*
            // invariant, not the parent link.
            goal =
                std::mem::replace(&mut goal, make_goal()).with_parent(fresh_parent, new_depth - 1);
        }
        assert_eq!(goal.depth, SYSTEM_MAX_RECURSION);
        // F-SYN-011: at the cap, can_have_subgoals is false.
        assert!(
            !goal.can_have_subgoals(),
            "a goal at depth {} (system cap) must not be able to have subgoals",
            goal.depth
        );
    }

    // F-SYN-011: depth 6 (just below the cap) can still have subgoals.
    #[test]
    fn goal_just_below_depth_cap_can_have_subgoals() {
        use crate::capability::SYSTEM_MAX_RECURSION;
        let mut goal = make_goal();
        goal.activate();
        for _ in 0..(SYSTEM_MAX_RECURSION - 1) {
            let new_depth = goal.depth + 1;
            let fresh_parent = crate::id::GoalID::new();
            goal =
                std::mem::replace(&mut goal, make_goal()).with_parent(fresh_parent, new_depth - 1);
        }
        assert_eq!(goal.depth, SYSTEM_MAX_RECURSION - 1);
        // F-SYN-011: at depth (cap - 1), can_have_subgoals is true.
        assert!(
            goal.can_have_subgoals(),
            "a goal at depth {} (just below cap {}) must be able to have subgoals",
            goal.depth,
            SYSTEM_MAX_RECURSION
        );
    }
}
