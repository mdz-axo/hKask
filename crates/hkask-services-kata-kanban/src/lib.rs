//! hKask Kata-Kanban Workflow Service — Toyota Kata process engine with Kanban board mechanics.
//!
//! ## Design Principle
//!
//! > **Kata is the process. Kanban is the tool/board/framework for applying the kata process to work.**
//!
//! This crate unifies the previously separate `hkask-services-kata` and `hkask-services-kanban`
//! crates. See `docs/plans/kata-kanban-merge-plan.md` for the full merge rationale and
//! implementation plan.
//!
//! ## Module Structure
//!
//! - `kata/` — Kata process engine: coaching, improvement, starter, execution, history, metrics
//! - `kanban/` — Kanban board mechanics: boards, tasks, contracts, verification, de-jam, socratic inquiry
//! - `bridge.rs` — Kata↔Kanban integration executor (replaces the old kanban_impl/kata.rs duplication)

mod bridge;
pub mod kanban;
pub mod kata;

// Re-export the public API at crate root.
// Types intentionally pub(crate) are NOT re-exported here — they are accessed
// through KanbanService or KataEngine methods.
pub use kanban::{
    Board, ColumnDef, KanbanError, KanbanService, Priority, SpawnSpec, Task, TaskFilter, TaskSpec,
    TaskStatus, Verification, VerificationCriterion,
};
pub use kata::{KataEngine, KataError, KataManifest, KataResult};
