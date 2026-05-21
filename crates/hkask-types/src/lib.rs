//! hKask Types — Foundation types for the hKask agent platform
//!
//! This crate provides:
//! - ID types (WebID, TemplateID, BotID, etc.)
//! - ν-event (cybernetic audit trail)
//! - hLexicon (canonical vocabulary)
//! - Visibility types (OCAP-enforced)
//! - Capability types (OCAP tokens)
//! - Template types (high-temperature templates, LLM parameters)
//! - Curation types (Curator, OCAP boundaries, curation decisions)
//! - CNS types (variety counters, algedonic alerts, kill zone detection)
//! - Sovereignty types (user sovereignty, acquisition resistance, kill-zone detection)

pub mod capability;
pub mod cns;
pub mod curation;
pub mod event;
pub mod id;
pub mod lexicon;
pub mod sovereignty;
pub mod template;
pub mod visibility;

pub use capability::*;
pub use cns::*;
pub use curation::*;
pub use event::*;
pub use id::*;
// Re-export lexicon types (includes TemplateType: Prompt, Process, Cognition)
pub use lexicon::{Domain, HLexicon, LexiconTerm, TemplateType};
// Re-export high-temp template types
pub use template::{
    HighTempTemplateType, LLMParameters, TemperatureRange, TemplateId, TemplateInvocation,
    TemplateOutcome,
};
pub use visibility::*;
// Re-export sovereignty types
pub use sovereignty::{
    AcquisitionResistance, DataCategory, DataSovereigntyBoundary, KillZoneDetector, SovereigntyId,
    UserSovereigntyState,
};
