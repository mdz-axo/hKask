//! Improv protocol types — input/output types for improv interaction.
//!
//! `Contribution` is the input type (a turn in conversation).
//! `ImprovResponse` is the unified output enum covering all five modes.

use crate::ConversationContext;
use crate::plussing::PlussedResponse;
use crate::riffing::RiffReturn;
use hkask_types::id::WebID;
use std::time::Duration;

/// Protocol trait for improv modes — each mode implements respond().
pub trait ImprovProtocol {
    fn respond(&self, contribution: &Contribution, context: &ConversationContext)
    -> ImprovResponse;
}

/// A single turn in a conversation — the atomic unit of improv interaction.
#[derive(Debug, Clone)]
pub struct Contribution {
    /// The agent that produced this contribution.
    pub source: WebID,
    /// The text content of the contribution.
    pub content: String,
    /// Position in the conversation sequence (0-based).
    pub turn_index: usize,
}

/// Unified response type covering all five improv modes.
#[derive(Debug, Clone)]
pub enum ImprovResponse {
    /// Plussing output: selected agreeable components + constructive build.
    Plussed(PlussedResponse),

    /// Yes And output: accepted base + novel extension.
    Extended {
        accepted_base: String,
        extension: String,
    },

    /// Yes But output: accepted base + boundary condition.
    Constrained {
        accepted_base: String,
        constraint: String,
    },

    /// Freestyling output: a rapid turn within a time-bounded session.
    FreestyleTurn {
        content: String,
        time_remaining: Duration,
    },

    /// Riffing output: a divergent tangent with a return policy.
    Riff {
        tangent: String,
        return_policy: RiffReturn,
    },
}

impl ImprovResponse {
    /// Extract the primary text content from any response variant.
    ///
    /// Used by cascade execution to feed one step's output as the next step's input.
    pub fn content_text(&self) -> String {
        match self {
            ImprovResponse::Plussed(p) => p.build.clone(),
            ImprovResponse::Extended {
                accepted_base: _,
                extension,
            } => extension.clone(),
            ImprovResponse::Constrained {
                accepted_base: _,
                constraint,
            } => constraint.clone(),
            ImprovResponse::FreestyleTurn {
                content,
                time_remaining: _,
            } => content.clone(),
            ImprovResponse::Riff {
                tangent,
                return_policy: _,
            } => tangent.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: IMPROV-PROTOCOL-001 — Contribution carries source, content, and turn_index
    #[test]
    fn contribution_has_required_fields() {
        let source = WebID::new();
        let c = Contribution {
            source,
            content: "test content".to_string(),
            turn_index: 3,
        };
        assert_eq!(c.source, source);
        assert_eq!(c.content, "test content");
        assert_eq!(c.turn_index, 3);
    }

    // REQ: IMPROV-PROTOCOL-002 — ImprovResponse variants are constructable
    #[test]
    fn improv_response_variants_constructable() {
        let pr = PlussedResponse {
            selected_seeds: vec![],
            build: "build".to_string(),
        };
        assert!(matches!(
            ImprovResponse::Plussed(pr),
            ImprovResponse::Plussed(_)
        ));

        assert!(matches!(
            ImprovResponse::Extended {
                accepted_base: "base".to_string(),
                extension: "ext".to_string(),
            },
            ImprovResponse::Extended { .. }
        ));

        assert!(matches!(
            ImprovResponse::Constrained {
                accepted_base: "base".to_string(),
                constraint: "limit".to_string(),
            },
            ImprovResponse::Constrained { .. }
        ));

        assert!(matches!(
            ImprovResponse::FreestyleTurn {
                content: "rapid".to_string(),
                time_remaining: Duration::from_secs(60),
            },
            ImprovResponse::FreestyleTurn { .. }
        ));

        assert!(matches!(
            ImprovResponse::Riff {
                tangent: "tangent".to_string(),
                return_policy: RiffReturn::ReturnToGroup,
            },
            ImprovResponse::Riff { .. }
        ));
    }
}
