//! hKask Kata-Kanban Workflow Service — Toyota Kata process engine with Kanban board mechanics.
//!
//! ## Design Principle
//!
//! > **Kata is the process. Kanban is the tool/board/framework for applying the kata process to work.**
//!
//! This crate unifies the previously separate `hkask-services-kata` and `hkask-services-kanban`
//! crates into a single workflow service where PDCA phases map directly to Kanban task statuses.
//!
//! ## Module Structure
//!
//! - `kata/` — Kata process engine: coaching, improvement, starter, execution, history, metrics
//! - `kanban/` — Kanban board mechanics: boards, tasks, contracts, verification, de-jam, socratic inquiry
//! - `bridge.rs` — Kata↔Kanban integration executor (replaces the old kanban_impl/kata.rs duplication)

pub mod bridge;
pub mod kanban;
pub mod kata;

// Re-export the public API at crate root.
pub use kanban::{
    Board, ColumnDef, KanbanError, KanbanService, Priority, SpawnSpec, Task, TaskFilter, TaskSpec,
    TaskStatus, UnjamFix, UnjamItem, Verification, VerificationCriterion, socratic,
};
pub use kata::{
    ImprovementDirection, ImprovementSignal, KataEngine, KataError, KataHistory, KataManifest,
    KataResult, KataState, KataStep, PracticeEntry, StepExperience,
};
