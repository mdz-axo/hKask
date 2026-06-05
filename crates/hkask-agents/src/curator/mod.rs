//! Curator — pure regulatory code (Loop 5)
//!
//! observe → evaluate → compose → regulate
//!
//! The Curation Loop is the ONLY loop that can override Cybernetics.
//! It observes system state and intervenes when Cybernetics
//! can't self-stabilize (e.g., alert cascade).
//!
//! # Curation / Agent Separation (Task 6)
//!
//! This module contains ONLY the pure regulatory code:
//! - `CurationLoop` — sense/compute/act, no persona, no chat, no memory
//! - `CuratorContext` — capability-disciplined runtime references
//! - `CurationConfidenceGate` — ARL confidence gate (IP-3)
//!
//! Persona concerns (metacognition, bot orchestration, spec curation,
//! human-facing reporting) moved to `crate::curator_agent`.

pub mod context;
pub mod curation_gate;
pub mod curation_loop;

pub use context::CuratorContext;
pub use curation_gate::{ConfidenceDecision, CurationConfidenceGate, CurationPort};
pub use curation_loop::CurationLoop;
