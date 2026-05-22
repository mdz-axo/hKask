//! hKask CNS — Goal Verifier
//!
//! Cybernetic verification of goal completion using CNS ν-events.
//! Implements KnowAct monitoring: orient, ground, monitor, evaluate, regulate.

use hkask_types::{AlgedonicAlert, Span};
use hkask_types::goal::{Goal, GoalOutcome, Verdict};
use serde_json::Value;

use crate::SpanEmitter;

/// Goal verifier trait (hexagonal port)
pub trait GoalVerifierPort {
    type Error;
    
    /// Verify goal completion based on outcome
    fn verify(&self, goal: &Goal, outcome: &GoalOutcome) -> Result<Verdict, Self::Error>;
    
    /// Check variety deficit and emit algedonic alert if needed
    fn check_variety(&self, goal: &Goal) -> Result<(), AlgedonicAlert>;
}

/// CNS-based goal verifier implementation
///
/// **KnowAct Integration:**
/// - `orient`: Direct attention toward goal state
/// - `ground`: Anchor verification in observable data
/// - `monitor`: Track progress via CNS spans
/// - `evaluate`: Assess completion criteria
/// - `regulate`: Trigger algedonic alert on variety deficit
pub struct CNSGoalVerifier {
    span_emitter: SpanEmitter,
}

impl CNSGoalVerifier {
    pub fn new() -> Self {
        Self {
            span_emitter: SpanEmitter::default(),
        }
    }
    
    /// Emit CNS span for goal verification
    fn emit_verify_span(&self, _goal: &Goal, verdict: &Verdict) {
        let span = Span::goal("verify");
        let outcome = match verdict {
            Verdict::Done { reason, .. } => format!("done: {}", reason),
            Verdict::Continue { reason } => format!("continue: {}", reason),
            Verdict::Blocked { reason, needs_human } => {
                if *needs_human {
                    format!("blocked (human needed): {}", reason)
                } else {
                    format!("blocked: {}", reason)
                }
            }
        };
        
        self.span_emitter.emit(span, Value::String(outcome));
    }
    
    /// Estimate goal complexity for variety counter
    fn estimate_complexity(goal: &Goal) -> usize {
        let base = 1;
        let criteria = goal.completion_criteria.len();
        let subgoals = goal.subgoals.len();
        let flow_complexity = goal.flow.as_ref().map_or(0, |f| match f {
            hkask_types::goal::GoalFlow::Sequence { steps } => steps.len(),
            hkask_types::goal::GoalFlow::Parallel { branches } => branches.len() * 2,
            hkask_types::goal::GoalFlow::Choice { branches } => branches.len() * 3,
        });
        
        base + criteria + subgoals + flow_complexity
    }
}

impl Default for CNSGoalVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl GoalVerifierPort for CNSGoalVerifier {
    type Error = VerifierError;
    
    fn verify(&self, goal: &Goal, outcome: &GoalOutcome) -> Result<Verdict, Self::Error> {
        let verdict = match outcome {
            GoalOutcome::Success { summary, .. } => Verdict::Done {
                reason: summary.clone(),
                confidence: 1.0,
            },
            GoalOutcome::Failure { reason, recoverable } => {
                if *recoverable {
                    Verdict::Continue {
                        reason: format!("Recoverable: {}", reason),
                    }
                } else {
                    Verdict::Blocked {
                        reason: reason.clone(),
                        needs_human: true,
                    }
                }
            }
            GoalOutcome::Partial {
                summary,
                completed_criteria,
                failed_criteria,
            } => {
                if failed_criteria.is_empty() && !completed_criteria.is_empty() {
                    Verdict::Done {
                        reason: format!("All criteria met: {}", summary),
                        confidence: 0.9,
                    }
                } else if completed_criteria.is_empty() {
                    Verdict::Blocked {
                        reason: format!("No criteria met: {}", summary),
                        needs_human: false,
                    }
                } else {
                    Verdict::Continue {
                        reason: format!(
                            "{}/{} criteria complete",
                            completed_criteria.len(),
                            completed_criteria.len() + failed_criteria.len()
                        ),
                    }
                }
            }
        };
        
        self.emit_verify_span(goal, &verdict);
        
        Ok(verdict)
    }
    
    fn check_variety(&self, goal: &Goal) -> Result<(), AlgedonicAlert> {
        let environmental_states = Self::estimate_complexity(goal) as u64;
        let internal_states: u64 = 100;
        
        let deficit = environmental_states.saturating_sub(internal_states);
        
        if deficit > 100 {
            let span = Span::goal("variety_deficit");
            let outcome = format!(
                "Variety deficit: {} (environmental: {}, internal: {})",
                deficit, environmental_states, internal_states
            );
            self.span_emitter.emit(span, Value::String(outcome));
            
            Err(AlgedonicAlert::new(
                internal_states,
                100,
                hkask_types::CnsSpan::Goal,
            ))
        } else {
            Ok(())
        }
    }
}

/// Goal verifier errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum VerifierError {
    #[error("verification failed: {0}")]
    VerificationFailed(String),
    #[error("CNS error: {0}")]
    CnsError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::goal::{Goal, GoalCommitment, GoalId, GoalState};
    use hkask_types::id::SessionID;
    use hkask_types::visibility::Visibility;
    
    fn create_test_goal() -> Goal {
        Goal {
            id: GoalId::new(),
            session_id: SessionID::new(),
            owner_webid: "test".to_string(),
            goal_text: "Test goal".to_string(),
            template_ref: None,
            state: GoalState::Active,
            commitment_level: GoalCommitment::Commit,
            flow: None,
            completion_criteria: Vec::new(),
            subgoals: Vec::new(),
            turns_used: 0,
            energy_budget: None,
            energy_used: 0,
            max_turns: 20,
            created_at: 0,
            last_turn_at: None,
            completed_at: None,
            visibility: Visibility::Private,
        }
    }
    
    #[test]
    fn test_verify_success() {
        let verifier = CNSGoalVerifier::new();
        let goal = create_test_goal();
        let outcome = GoalOutcome::Success {
            summary: "Goal completed".to_string(),
            artifacts: Vec::new(),
        };
        
        let verdict = verifier.verify(&goal, &outcome).unwrap();
        
        match verdict {
            Verdict::Done { reason, confidence } => {
                assert_eq!(reason, "Goal completed");
                assert_eq!(confidence, 1.0);
            }
            _ => panic!("Expected Done verdict"),
        }
    }
    
    #[test]
    fn test_verify_failure_recoverable() {
        let verifier = CNSGoalVerifier::new();
        let goal = create_test_goal();
        let outcome = GoalOutcome::Failure {
            reason: "Temporary error".to_string(),
            recoverable: true,
        };
        
        let verdict = verifier.verify(&goal, &outcome).unwrap();
        
        match verdict {
            Verdict::Continue { .. } => (),
            _ => panic!("Expected Continue verdict"),
        }
    }
    
    #[test]
    fn test_variety_check_within_threshold() {
        let verifier = CNSGoalVerifier::new();
        let goal = create_test_goal();
        
        let result = verifier.check_variety(&goal);
        assert!(result.is_ok());
    }
}
