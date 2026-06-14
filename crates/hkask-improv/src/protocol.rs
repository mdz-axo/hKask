//! Improv protocol — trait, input/output types.
//!
//! The `ImprovProtocol` trait defines the single contract that every improv mode
//! must fulfill: accept a contribution and produce a response. Mode-specific
//! operations (`cycle` for freestyling, `resolve` for riffing) live on their
//! respective types, not on the trait — each thing does one thing.
//!
//! `Contribution` is the input type (a turn in conversation).
//! `ImprovResponse` is the unified output enum covering all five modes.

use crate::ConversationContext;
use crate::plussing::PlussedResponse;
use crate::riffing::RiffReturn;
use hkask_types::id::WebID;
use std::time::Duration;

/// A single turn in a conversation — the atomic unit of improv interaction.
///
/// Owned by the mode that processes it. The `source` field identifies
/// which agent produced this contribution.
#[derive(Debug, Clone)]
pub struct Contribution {
    /// The agent that produced this contribution.
    pub source: WebID,
    /// The text content of the contribution.
    pub content: String,
    /// Position in the conversation sequence (0-based).
    pub turn_index: usize,
}

/// Unified response type covering all five improv modes plus cascade errors.
///
/// Each variant carries mode-specific output data. Callers match on the
/// variant to determine what kind of response was produced.
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

    /// Error during cascade execution (e.g., recursion limit exceeded).
    Error { message: String },
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
            ImprovResponse::Error { message } => format!("[improv error] {}", message),
        }
    }
}

/// The improv protocol — one trait, one method.
///
/// Every improv mode implements this trait. The single method `respond()`
/// accepts a contribution and produces a mode-appropriate response.
/// Mode-specific operations (`FreestyleSession::cycle()`, `riffing::resolve()`)
/// live on their respective types — each thing does one thing.
pub trait ImprovProtocol {
    /// Accept a contribution and produce a response according to the mode.
    fn respond(&self, prior: &Contribution, context: &ConversationContext) -> ImprovResponse;
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: Contribution carries source, content, and turn_index
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

    // REQ: ImprovResponse variants are constructable
    #[test]
    fn improv_response_variants_constructable() {
        // Plussed
        let pr = PlussedResponse {
            selected_seeds: vec![],
            build: "build".to_string(),
        };
        let r = ImprovResponse::Plussed(pr);
        assert!(matches!(r, ImprovResponse::Plussed(_)));

        // Extended
        let r = ImprovResponse::Extended {
            accepted_base: "base".to_string(),
            extension: "ext".to_string(),
        };
        assert!(matches!(r, ImprovResponse::Extended { .. }));

        // Constrained
        let r = ImprovResponse::Constrained {
            accepted_base: "base".to_string(),
            constraint: "limit".to_string(),
        };
        assert!(matches!(r, ImprovResponse::Constrained { .. }));

        // FreestyleTurn
        let r = ImprovResponse::FreestyleTurn {
            content: "rapid".to_string(),
            time_remaining: Duration::from_secs(60),
        };
        assert!(matches!(r, ImprovResponse::FreestyleTurn { .. }));

        // Riff
        let r = ImprovResponse::Riff {
            tangent: "tangent".to_string(),
            return_policy: RiffReturn::ReturnToGroup,
        };
        assert!(matches!(r, ImprovResponse::Riff { .. }));
    }
}
