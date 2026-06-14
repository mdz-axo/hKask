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
//! 2. `ImprovResponse` — unified output type
//! 3. `Contribution` — input type (a turn in conversation)
//! 4. `FreestyleSession` — session state for freestyling
//! 5. `ImprovCascade` — recursive mode composition with matryoshka limit
//! 6. `ImprovSkill` — facade applying a mode or cascade to a contribution
//! 7. `ImprovSkill::register_with_cns()` — CNS integration

pub mod cascade;
pub mod cns;
pub mod freestyling;
pub mod kata;
pub mod modes;
pub mod plussing;
pub mod protocol;
pub mod riffing;

pub use cascade::{ImprovCascade, ImprovError, MATRYOSHKA_LIMIT};
pub use cns::{ImprovCns, TracingImprovCns};
pub use freestyling::FreestyleSession;
pub use kata::{KataImprovResult, KataPhase};
pub use modes::ImprovMode;
pub use plussing::{AgreeableComponent, PlussedResponse};
pub use protocol::{Contribution, ImprovResponse};
pub use riffing::{RiffOutcome, RiffReturn};

use hkask_types::id::WebID;

/// Facade — apply an improv mode (or cascade) to a contribution.
pub struct ImprovSkill;

impl ImprovSkill {
    /// Apply an improv mode to a contribution.
    ///
    /// For `Cascade` mode, executes all steps in sequence, feeding each step's
    /// output as input to the next, bounded by the matryoshka limit.
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
    pub fn register_with_cns(cns: &mut dyn ImprovCns) {
        cns.register_improv_spans();
    }
}

/// Conversation context — agent, participants, turn count, recursion depth.
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub agent_id: WebID,
    pub participants: Vec<WebID>,
    pub turn_count: usize,
    /// Current recursion depth in the improv cascade (0 = top-level).
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

    /// Create a child context for one level deeper in the cascade.
    pub fn descend(&self) -> Self {
        Self {
            agent_id: self.agent_id,
            participants: self.participants.clone(),
            turn_count: self.turn_count,
            recursion_depth: self.recursion_depth.saturating_add(1),
        }
    }
}
