//! hKask Kanban Board Service — unjam detection and workflow management.
//!
//! Extracted from `hkask-services` to enable parallel compilation.

pub mod kanban;
mod kanban_impl;
pub mod socratic;

pub use kanban::{
    Board, CapabilityPackage, ColumnDef, Comment, ConditionResult, ConsentProof, ContractState,
    ContractVerification, GasEntry, KanbanPhase, Priority, SpawnSpec, Task, TaskContract,
    TaskFilter, TaskSpec, TaskStatus, Verification, VerificationCriterion,
};
pub use kanban_impl::{KanbanError, KanbanService, UnjamFix, UnjamItem};
