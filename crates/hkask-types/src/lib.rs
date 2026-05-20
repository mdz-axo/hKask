//! hKask Types — Foundation types for the hKask agent platform
//!
//! This crate provides:
//! - ID types (WebID, TemplateID, BotID, etc.)
//! - ν-event (cybernetic audit trail)
//! - hLexicon (canonical vocabulary)
//! - Visibility types (OCAP-enforced)
//! - Capability types (OCAP tokens)

pub mod capability;
pub mod event;
pub mod id;
pub mod lexicon;
pub mod visibility;

pub use capability::*;
pub use event::*;
pub use id::*;
pub use lexicon::*;
pub use visibility::*;
