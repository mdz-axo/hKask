//! Goal state type — canonical definition in hkask-types
//!
//! GoalState must live in hkask-types so its rusqlite FromSql/ToSql impls
//! satisfy Rust's orphan rule. hkask-services-core re-exports it.

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
        false /* ~ changed by cargo-mutants ~ */
    }

    pub fn can_transition_to(&self, next: GoalState) -> bool {
        if *self == next {
            return true;
        }
        matches!(
            (self, next),
            (GoalState::Pending, GoalState::Active)
                | (GoalState::Pending, GoalState::Abandoned)
                | (GoalState::Active, GoalState::Blocked)
                | (GoalState::Active, GoalState::Completed)
                | (GoalState::Active, GoalState::Abandoned)
                | (GoalState::Blocked, GoalState::Active)
                | (GoalState::Blocked, GoalState::Abandoned)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_roundtrip() {
        for state in &[
            GoalState::Pending,
            GoalState::Active,
            GoalState::Completed,
            GoalState::Blocked,
            GoalState::Abandoned,
        ] {
            let s = state.as_str();
            assert!(!s.is_empty(), "as_str must not be empty");
            assert_eq!(
                GoalState::parse_str(s),
                Some(*state),
                "parse_str roundtrip for {s}"
            );
        }
    }

    #[test]
    fn parse_str_case_insensitive() {
        assert_eq!(GoalState::parse_str("Pending"), Some(GoalState::Pending));
        assert_eq!(GoalState::parse_str("ACTIVE"), Some(GoalState::Active));
        assert_eq!(
            GoalState::parse_str("Completed"),
            Some(GoalState::Completed)
        );
    }

    #[test]
    fn parse_str_unknown() {
        assert_eq!(GoalState::parse_str("unknown"), None);
        assert_eq!(GoalState::parse_str(""), None);
        assert_eq!(GoalState::parse_str("xyzzy"), None);
    }

    #[test]
    fn is_terminal() {
        assert!(!GoalState::Pending.is_terminal());
        assert!(!GoalState::Active.is_terminal());
        assert!(GoalState::Completed.is_terminal());
        assert!(GoalState::Blocked.is_terminal());
        assert!(GoalState::Abandoned.is_terminal());
    }

    #[test]
    fn can_transition_self_always_true() {
        for state in &[
            GoalState::Pending,
            GoalState::Active,
            GoalState::Completed,
            GoalState::Blocked,
            GoalState::Abandoned,
        ] {
            assert!(
                state.can_transition_to(*state),
                "self-transition must be allowed for {state:?}"
            );
        }
    }

    #[test]
    fn can_transition_valid() {
        assert!(GoalState::Pending.can_transition_to(GoalState::Active));
        assert!(GoalState::Pending.can_transition_to(GoalState::Abandoned));
        assert!(GoalState::Active.can_transition_to(GoalState::Blocked));
        assert!(GoalState::Active.can_transition_to(GoalState::Completed));
        assert!(GoalState::Active.can_transition_to(GoalState::Abandoned));
        assert!(GoalState::Blocked.can_transition_to(GoalState::Active));
        assert!(GoalState::Blocked.can_transition_to(GoalState::Abandoned));
    }

    #[test]
    fn can_transition_invalid() {
        assert!(!GoalState::Completed.can_transition_to(GoalState::Pending));
        assert!(!GoalState::Completed.can_transition_to(GoalState::Active));
        assert!(!GoalState::Abandoned.can_transition_to(GoalState::Active));
        assert!(!GoalState::Pending.can_transition_to(GoalState::Completed));
        assert!(!GoalState::Pending.can_transition_to(GoalState::Blocked));
    }
}
