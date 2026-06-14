//! hKask Improv — Composable interaction grammar for agent communication
//!
//! Five improv modes (Plussing, Yes And, Yes But, Freestyling, Riffing) provide
//! a constructive-by-default interaction protocol for dual-presence chat, ensemble
//! sessions, and kata coaching loops.
//!
//! Modes compose recursively via `ImprovCascade` — sequences of mode applications
//! bounded by the matryoshka limit (7), mirroring the BundleManifest cascade depth
//! limit. Cascades can nest: a Cascade step can itself contain a Cascade, enabling
//! recursive composition within the depth bound.
//!
//! # Public API surface (7 items — deep-module discipline)
//!
//! 1. `ImprovMode` — the five improv modes + Cascade variant (exhaustive enum)
//! 2. `ImprovProtocol` — trait for processing contributions
//! 3. `ImprovResponse` — unified output type with recursion depth
//! 4. `Contribution` — input type with recursion depth
//! 5. `FreestyleSession` — session state for freestyling
//! 6. `ImprovSkill` — facade applying a mode or cascade to a contribution
//! 7. `ImprovSkill::register_with_cns()` — CNS integration

pub mod cascade;
pub mod cns;
pub mod freestyling;
pub mod modes;
pub mod plussing;
pub mod protocol;
pub mod riffing;

pub use cascade::{ImprovCascade, ImprovCascadeStep, ImprovError, MATRYOSHKA_LIMIT};
pub use cns::ImprovCns;
pub use freestyling::FreestyleSession;
pub use modes::ImprovMode;
pub use plussing::{AgreeableComponent, PlussedResponse};
pub use protocol::{Contribution, ImprovProtocol, ImprovResponse};
pub use riffing::{RiffOutcome, RiffReturn};

use hkask_types::id::WebID;

/// Facade — apply an improv mode (or cascade) to a contribution.
///
/// This is the single entry point for callers. For simple modes, it delegates
/// to the mode's `respond()`. For `Cascade` mode, it executes the full cascade
/// with recursion depth tracking.
pub struct ImprovSkill;

impl ImprovSkill {
    /// Apply an improv mode to a contribution.
    ///
    /// Returns an `ImprovResponse` appropriate to the mode. For `Cascade` mode,
    /// executes all steps in sequence, feeding each step's output as input to
    /// the next, bounded by the matryoshka limit.
    pub fn apply(
        mode: &ImprovMode,
        contribution: &Contribution,
        context: &ConversationContext,
    ) -> Result<ImprovResponse, ImprovError> {
        match mode {
            ImprovMode::Cascade(cascade) => cascade.execute(contribution, context),
            other => Ok(other.respond(contribution, context)),
        }
    }

    /// Register CNS spans for improv monitoring.
    ///
    /// Must be called once during CNS initialization. Registers:
    /// - `cns.improv.mode.active` — which mode is active
    /// - `cns.improv.plussing.ratio` — constructive ratio for Plussing
    /// - `cns.improv.freestyle.coherence` — freestyling coherence metric
    /// - `cns.improv.ensemble.coherence` — ensemble output quality
    /// - `cns.kata.improv.effectiveness` — kata improv effectiveness
    /// - `cns.improv.cascade.depth` — current cascade recursion depth
    pub fn register_with_cns(cns: &mut dyn ImprovCns) {
        cns.register_improv_spans();
    }
}

/// Conversation context — the surrounding state a mode operates within.
///
/// Carries participant info, session metadata, and the current recursion depth
/// for cascade tracking.
#[derive(Debug, Clone)]
pub struct ConversationContext {
    /// The agent applying the improv mode.
    pub agent_id: WebID,
    /// All participants in the conversation.
    pub participants: Vec<WebID>,
    /// Turn count so far.
    pub turn_count: usize,
    /// Optional session label (e.g., "architecture exploration").
    pub session_label: Option<String>,
    /// Current recursion depth in the improv cascade (0 = top-level).
    pub recursion_depth: u8,
}

impl ConversationContext {
    pub fn new(agent_id: WebID) -> Self {
        Self {
            agent_id,
            participants: vec![agent_id],
            turn_count: 0,
            session_label: None,
            recursion_depth: 0,
        }
    }

    pub fn with_participants(mut self, participants: Vec<WebID>) -> Self {
        self.participants = participants;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.session_label = Some(label.into());
        self
    }

    /// Create a child context for one level deeper in the cascade.
    pub fn descend(&self) -> Self {
        Self {
            agent_id: self.agent_id,
            participants: self.participants.clone(),
            turn_count: self.turn_count,
            session_label: self.session_label.clone(),
            recursion_depth: self.recursion_depth.saturating_add(1),
        }
    }
}
