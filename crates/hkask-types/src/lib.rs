//! hKask Types — Foundation types for the hKask agent platform
//!
//! This crate provides:
//! - ID types (WebID, TemplateID, BotID, GoalID, SpecId, etc.)
//! - ν-event (cybernetic audit trail)
//! - hLexicon (canonical vocabulary)
//! - Visibility types (OCAP-enforced)
//! - Capability types (OCAP tokens)
//! - Template types (high-temperature templates, LLM parameters)
//! - Curation types (Curator, OCAP boundaries, curation decisions)
//! - CNS types (variety counters, algedonic alerts, kill zone detection)
//! - Sovereignty types (user sovereignty, acquisition resistance, kill-zone detection)
//! - Goal types (minimal coordination substrate for multi-agent collaboration)
//! - Spec types (DDMVSS domain types, completeness predicates, curation integration)

pub mod capability;
pub mod cns;
pub mod curation;
pub mod error;
pub mod event;
pub mod goal;
pub mod goal_capability;
pub mod id;
pub mod lexicon;
pub mod secret;
pub mod sovereignty;
pub mod spec;
pub mod template;
pub mod text;
pub mod visibility;

pub use capability::*;
pub use cns::*;
pub use curation::*;
pub use error::{ArchivalResult, AuthorizationError, GitArchivalError};
pub use event::*;
pub use goal::*;
pub use goal_capability::*;
pub use id::*;
pub use lexicon::{Domain, HLexicon, LexiconTerm, TemplateType};
pub use secret::SecretRef;
pub use spec::{
    CompletenessCheck, Criterion, DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurationRecord,
    SpecCurator, SpecError, SpecId, SpecObserver, SpecSigner, SpecStore,
};
pub use template::{
    HighTempTemplateType, LLMParameters, TemperatureRange, TemplateId, TemplateInvocation,
    TemplateOutcome,
};
pub use visibility::*;
pub use sovereignty::{
    AcquisitionResistance, DataCategory, DataSovereigntyBoundary, KillZoneDetector, SovereigntyId,
    UserSovereigntyState,
};
pub use text::{blake3_hash, estimate_tokens};
