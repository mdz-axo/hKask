//! Riffing — Solo divergent exploration from a seed contribution.
//!
//! Riffing takes a seed contribution and explores a tangent independently.
//! The riff may return to the group context with a synthesis, or spawn
//! a new thread for continued exploration.

use crate::protocol::Contribution;
use hkask_types::id::WebID;

/// Policy for how a riff resolves back to the group.
///
/// Enum, not boolean — makes the return contract explicit in the type system.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RiffReturn {
    /// The riff returns to the group context with a synthesis.
    #[default]
    ReturnToGroup,
    /// The riff spawns a new independent thread.
    SpawnThread,
    /// The riff returns after a fixed number of exploration steps.
    ReturnAfterSteps { max_steps: usize },
}

/// Outcome of resolving a riff tangent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiffOutcome {
    /// The riff returned to the group with a synthesis of findings.
    Returned {
        /// Synthesis of what was discovered during the riff.
        synthesis: String,
    },
    /// The riff spawned a new thread for continued exploration.
    Spawned {
        /// Identifier for the new thread.
        thread_id: WebID,
    },
}

/// Diverge from a seed contribution into a solo tangent.
///
/// Produces an initial tangent string that explores a dimension
/// of the seed contribution independently.
pub fn diverge(seed: &Contribution) -> String {
    format!(
        "[riff] Exploring a tangent from '{}': what if we consider this from a completely different perspective? Let's follow this thread independently and see where it leads.",
        truncate_for_riff(&seed.content, 100)
    )
}

/// Resolve a riff tangent according to the return policy.
///
/// - `ReturnToGroup`: produces a synthesis that bridges back to the group context.
/// - `SpawnThread`: creates a new thread identifier for continued exploration.
/// - `ReturnAfterSteps`: returns after the specified number of steps.
pub fn resolve(tangent: &Contribution, policy: &RiffReturn, steps_taken: usize) -> RiffOutcome {
    match policy {
        RiffReturn::ReturnToGroup => RiffOutcome::Returned {
            synthesis: format!(
                "[riff resolved] Returning from tangent exploration. Key insight from '{}': this perspective reveals alternative approaches worth integrating into the main discussion.",
                truncate_for_riff(&tangent.content, 80)
            ),
        },
        RiffReturn::SpawnThread => RiffOutcome::Spawned {
            thread_id: WebID::from_persona(
                format!("riff-thread:{}", truncate_for_riff(&tangent.content, 64)).as_bytes(),
            ),
        },
        RiffReturn::ReturnAfterSteps { max_steps } => {
            if steps_taken >= *max_steps {
                RiffOutcome::Returned {
                    synthesis: format!(
                        "[riff resolved after {} steps] Exploration complete. Synthesis from '{}'.",
                        steps_taken,
                        truncate_for_riff(&tangent.content, 80)
                    ),
                }
            } else {
                // Still exploring — signal continuation.
                RiffOutcome::Returned {
                    synthesis: format!(
                        "[riff step {}/{}] Continuing exploration of '{}'...",
                        steps_taken,
                        max_steps,
                        truncate_for_riff(&tangent.content, 80)
                    ),
                }
            }
        }
    }
}

fn truncate_for_riff(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_contribution(content: &str) -> Contribution {
        Contribution {
            source: WebID::new(),
            content: content.to_string(),
            turn_index: 0,
        }
    }

    // contract: IMPROV-RIFFING-001
    #[test]
    fn diverges_from_seed() {
        let seed = make_contribution("We should use a microservice architecture");
        let tangent = diverge(&seed);
        assert!(tangent.contains("[riff]"));
        assert!(tangent.contains("microservice"));
    }

    // contract: IMPROV-RIFFING-002
    #[test]
    fn resolves_return_to_group() {
        let tangent = make_contribution("Monoliths might actually be simpler for our scale");
        let outcome = resolve(&tangent, &RiffReturn::ReturnToGroup, 3);
        match outcome {
            RiffOutcome::Returned { synthesis } => {
                assert!(synthesis.contains("riff resolved"));
                assert!(synthesis.contains("Monoliths"));
            }
            other => panic!("Expected Returned, got {:?}", other),
        }
    }

    // contract: IMPROV-RIFFING-003
    #[test]
    fn spawns_new_thread() {
        let tangent = make_contribution("What about event sourcing?");
        let outcome = resolve(&tangent, &RiffReturn::SpawnThread, 1);
        match outcome {
            RiffOutcome::Spawned { thread_id: _ } => {
                // Thread ID should be a valid WebID.
            }
            other => panic!("Expected Spawned, got {:?}", other),
        }
    }

    // contract: IMPROV-RIFFING-004
    #[test]
    fn respects_return_after_steps() {
        let tangent = make_contribution("Exploring CQRS patterns");
        let policy = RiffReturn::ReturnAfterSteps { max_steps: 5 };

        // Before max steps: should still be exploring.
        let outcome = resolve(&tangent, &policy, 3);
        match outcome {
            RiffOutcome::Returned { synthesis } => {
                assert!(synthesis.contains("3/5"), "Should show step progress");
            }
            other => panic!("Expected Returned with progress, got {:?}", other),
        }

        // At max steps: should complete.
        let outcome = resolve(&tangent, &policy, 5);
        match outcome {
            RiffOutcome::Returned { synthesis } => {
                assert!(synthesis.contains("riff resolved after 5 steps"));
            }
            other => panic!("Expected Returned with completion, got {:?}", other),
        }
    }

    // contract: IMPROV-RIFFING-005
    #[test]
    fn default_return_is_return_to_group() {
        assert_eq!(RiffReturn::default(), RiffReturn::ReturnToGroup);
    }
}
