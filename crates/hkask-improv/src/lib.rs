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
pub mod freestyling;
pub mod kata;
pub mod modes;
pub mod plussing;
pub mod protocol;
pub mod riffing;

pub use cascade::ImprovCascade;
pub use freestyling::FreestyleSession;
pub use modes::ImprovMode;
pub use protocol::{Contribution, ConversationContext, ImprovResponse};

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
    ) -> Result<ImprovResponse, cascade::ImprovError> {
        match mode {
            ImprovMode::Cascade(cascade) => cascade.execute(contribution, context),
            other => Ok(other.respond(contribution, context)),
        }
    }
}
