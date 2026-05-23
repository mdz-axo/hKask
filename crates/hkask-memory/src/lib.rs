//! hKask Memory — Semantic and episodic memory pipelines
//!
//! **Three-Layer DRY System:**
//! - Layer 1: Memory recall dedup (`recall_dedup`) — entity-attribute-value hash
//! - Layer 2: Session message dedup (`hkask-ensemble/src/chat_dedup.rs`)
//! - Layer 3: Prompt assembly dedup (`hkask-templates/src/context_assembly.rs`)

pub mod bayesian;
pub mod episodic;
pub mod goal_memory;
pub mod recall_dedup;
pub mod semantic;

pub use bayesian::BayesianOps;
pub use episodic::EpisodicMemory;
pub use goal_memory::{
    GoalEpisodicMemory, GoalMemory, GoalMemoryPort, GoalSemanticMemory, MemoryError,
};
pub use recall_dedup::{DedupResult, dedup_triples, dedup_triples_with_stats, eav_hash};
pub use semantic::SemanticMemory;
