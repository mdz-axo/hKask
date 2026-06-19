//! Improv protocol types — input/output types for improv interaction.
//!
//! `Contribution` is the input type (a turn in conversation).
//! `ImprovResponse` is the unified output enum covering all five modes.

use crate::plussing::PlussedResponse;
use crate::riffing::RiffReturn;
use hkask_types::id::WebID;
use std::time::Duration;

/// Conversation context — agent, participants, turn count, recursion depth.
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub agent_id: WebID,
    pub participants: Vec<WebID>,
    pub turn_count: usize,
    pub recursion_depth: u8,
}

impl ConversationContext {
    pub fn new(agent_id: WebID) -> Self {
        Self {
            agent_id,
            participants: vec![agent_id],
            turn_count: 0,
            recursion_depth: 0,
        }
    }

    pub fn descend(&self) -> Self {
        Self {
            agent_id: self.agent_id,
            participants: self.participants.clone(),
            turn_count: self.turn_count,
            recursion_depth: self.recursion_depth.saturating_add(1),
        }
    }
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

    // contract: IMPROV-PROTOCOL-001
    // contract: IMPROV-PROTOCOL-002
    #[test]
    fn contribution_and_response_fields() {
        let source = WebID::new();
        let c = Contribution {
            source,
            content: "test content".to_string(),
            turn_index: 3,
        };
        assert_eq!(c.source, source);
        assert_eq!(c.content, "test content");
        assert_eq!(c.turn_index, 3);

        let pr = PlussedResponse {
            selected_seeds: vec![],
            build: "build".to_string(),
        };
        let response = ImprovResponse::Plussed(pr);
        assert_eq!(response.content_text(), "build");
    }
}
