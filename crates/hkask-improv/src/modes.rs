//! Improv mode enum — the five core improv techniques as an exhaustive enum.
//!
//! Each variant encodes its specific parameters in the type system.
//! No fallback variants — every mode is explicit.

use crate::ConversationContext;
use crate::cascade::ImprovCascade;
use crate::plussing;
use crate::protocol::{Contribution, ImprovResponse};
use crate::riffing::RiffReturn;
use std::time::Duration;

/// The five improv modes plus Cascade for recursive composition.
///
/// Exhaustive enum — no `Other` or `Custom` fallback. New modes require
/// a new variant and corresponding implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImprovMode {
    /// Plussing (Catmull): Extract agreeable components, silently discard remainder.
    /// Build constructively on selected seeds. Never explicitly negate.
    Plussing,

    /// Yes And: Accept the whole contribution, extend with a novel layer.
    /// Extension must be additive, not substitutive.
    YesAnd,

    /// Yes But: Accept the whole contribution, append a constraint or redirect.
    /// Constraint narrows, does not contradict.
    YesBut,

    /// Freestyling: Rapid collaborative short-response cycling.
    /// Time-bounded, no single owner.
    Freestyling {
        /// Maximum duration for the freestyle session.
        time_bound: Duration,
    },

    /// Riffing: Solo divergent exploration from a seed contribution.
    /// Returns to group context or spawns a new thread.
    Riffing {
        /// Policy for how the riff resolves back to the group.
        return_policy: RiffReturn,
    },

    /// Cascade: Recursive composition of sub-modes executed sequentially.
    /// Bounded by the matryoshka limit (7 total applications).
    /// Each step's output feeds into the next step as input.
    Cascade(ImprovCascade),
}

impl ImprovMode {
    /// Human-readable label for this mode.
    pub fn label(&self) -> &'static str {
        match self {
            ImprovMode::Plussing => "plussing",
            ImprovMode::YesAnd => "yes-and",
            ImprovMode::YesBut => "yes-but",
            ImprovMode::Freestyling { .. } => "freestyling",
            ImprovMode::Riffing { .. } => "riffing",
            ImprovMode::Cascade(_) => "cascade",
        }
    }

    /// Respond to a contribution according to this mode's protocol.
    ///
    /// Delegates to mode-specific logic. For `Cascade` mode, executes the
    /// full cascade. This is the single dispatch point — callers don't need
    /// to know which mode is active.
    pub fn respond(
        &self,
        contribution: &Contribution,
        context: &ConversationContext,
    ) -> ImprovResponse {
        match self {
            ImprovMode::Plussing => {
                let plussed = plussing::process(contribution);
                ImprovResponse::Plussed(plussed)
            }
            ImprovMode::YesAnd => {
                let extended = format!(
                    "Yes, and also: {} extends with a new dimension building on your point about '{}'.",
                    context.agent_id.redacted_display(),
                    truncate_for_display(&contribution.content, 80)
                );
                ImprovResponse::Extended {
                    accepted_base: contribution.content.clone(),
                    extension: extended,
                }
            }
            ImprovMode::YesBut => {
                let constrained = format!(
                    "Yes, but consider: while '{}' holds, we must also account for boundary conditions that narrow the scope.",
                    truncate_for_display(&contribution.content, 80)
                );
                ImprovResponse::Constrained {
                    accepted_base: contribution.content.clone(),
                    constraint: constrained,
                }
            }
            ImprovMode::Freestyling { time_bound } => {
                let rapid = format!(
                    "[freestyle turn {}] {}",
                    context.turn_count,
                    truncate_for_display(&contribution.content, 60)
                );
                ImprovResponse::FreestyleTurn {
                    content: rapid,
                    time_remaining: *time_bound,
                }
            }
            ImprovMode::Riffing { return_policy } => {
                let tangent = format!(
                    "[riff from seed '{}'] exploring divergent path...",
                    truncate_for_display(&contribution.content, 60)
                );
                ImprovResponse::Riff {
                    tangent,
                    return_policy: return_policy.clone(),
                }
            }
            ImprovMode::Cascade(cascade) => {
                // Execute the cascade. If it fails, return a Plussed response
                // with the error message as the build (graceful degradation).
                match cascade.execute(contribution, context) {
                    Ok(response) => response,
                    Err(e) => ImprovResponse::Plussed(plussing::PlussedResponse {
                        selected_seeds: vec![],
                        build: format!("[cascade error] {}", e),
                    }),
                }
            }
        }
    }
}

/// Truncate a string for display, appending "…" if truncated.
fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Contribution;
    use crate::riffing::RiffReturn;
    use hkask_types::id::WebID;
    use std::time::Duration;

    fn make_contribution(content: &str) -> Contribution {
        Contribution {
            source: WebID::new(),
            content: content.to_string(),
            turn_index: 0,
        }
    }

    fn make_context() -> ConversationContext {
        ConversationContext::new(WebID::new())
    }

    // REQ: IMPROV-MODES-001 — Plussing produces a PlussedResponse variant
    #[test]
    fn plussing_mode_produces_plussed_response() {
        let mode = ImprovMode::Plussing;
        let contrib = make_contribution("I think we should refactor the auth module");
        let ctx = make_context();
        let response = mode.respond(&contrib, &ctx);
        assert!(matches!(response, ImprovResponse::Plussed(_)));
    }

    // REQ: IMPROV-MODES-002 — YesAnd produces Extended variant with accepted base preserved
    #[test]
    fn yes_and_preserves_accepted_base() {
        let mode = ImprovMode::YesAnd;
        let contrib = make_contribution("The error handling needs improvement");
        let ctx = make_context();
        let response = mode.respond(&contrib, &ctx);
        match response {
            ImprovResponse::Extended {
                accepted_base,
                extension: _,
            } => {
                assert_eq!(accepted_base, "The error handling needs improvement");
            }
            other => panic!("Expected Extended, got {:?}", other),
        }
    }

    // REQ: IMPROV-MODES-003 — YesBut produces Constrained variant with accepted base preserved
    #[test]
    fn yes_but_preserves_accepted_base() {
        let mode = ImprovMode::YesBut;
        let contrib = make_contribution("We should add caching everywhere");
        let ctx = make_context();
        let response = mode.respond(&contrib, &ctx);
        match response {
            ImprovResponse::Constrained {
                accepted_base,
                constraint: _,
            } => {
                assert_eq!(accepted_base, "We should add caching everywhere");
            }
            other => panic!("Expected Constrained, got {:?}", other),
        }
    }

    // REQ: IMPROV-MODES-004 — Freestyling produces FreestyleTurn with time_remaining
    #[test]
    fn freestyling_includes_time_bound() {
        let bound = Duration::from_secs(300);
        let mode = ImprovMode::Freestyling { time_bound: bound };
        let contrib = make_contribution("What about using a message queue?");
        let ctx = make_context();
        let response = mode.respond(&contrib, &ctx);
        match response {
            ImprovResponse::FreestyleTurn {
                content: _,
                time_remaining,
            } => {
                assert_eq!(time_remaining, bound);
            }
            other => panic!("Expected FreestyleTurn, got {:?}", other),
        }
    }

    // REQ: IMPROV-MODES-005 — Riffing produces Riff variant with return_policy
    #[test]
    fn riffing_includes_return_policy() {
        let policy = RiffReturn::ReturnToGroup;
        let mode = ImprovMode::Riffing {
            return_policy: policy.clone(),
        };
        let contrib = make_contribution("Maybe we could use a different database");
        let ctx = make_context();
        let response = mode.respond(&contrib, &ctx);
        match response {
            ImprovResponse::Riff {
                tangent: _,
                return_policy,
            } => {
                assert_eq!(return_policy, RiffReturn::ReturnToGroup);
            }
            other => panic!("Expected Riff, got {:?}", other),
        }
    }

    // REQ: IMPROV-MODES-006 — Mode labels are stable and human-readable
    #[test]
    fn mode_labels_are_stable() {
        assert_eq!(ImprovMode::Plussing.label(), "plussing");
        assert_eq!(ImprovMode::YesAnd.label(), "yes-and");
        assert_eq!(ImprovMode::YesBut.label(), "yes-but");
        assert_eq!(
            ImprovMode::Freestyling {
                time_bound: Duration::from_secs(60)
            }
            .label(),
            "freestyling"
        );
        assert_eq!(
            ImprovMode::Riffing {
                return_policy: RiffReturn::ReturnToGroup
            }
            .label(),
            "riffing"
        );
    }
}
