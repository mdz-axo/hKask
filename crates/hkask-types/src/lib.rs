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

pub mod capability;
pub mod cns;
pub mod curation;
pub mod event;
pub mod id;
pub mod lexicon;
pub mod template;
pub mod visibility;

pub use capability::*;
pub use cns::*;
pub use curation::*;
pub use event::*;
pub use id::*;
// Note: lexicon uses glob export but TemplateType conflict with template module
// Users should import lexicon::LexiconTerm explicitly if needed
pub use lexicon::{Domain, HLexicon, LexiconTerm};
pub use template::*;
pub use visibility::*;
