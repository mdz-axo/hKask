//! hKask Kanban Board Service — unjam detection and workflow management.
//!
//! Extracted from `hkask-services` to enable parallel compilation.

mod kanban_impl;

pub use kanban_impl::{KanbanError, KanbanService, UnjamFix, UnjamItem};
