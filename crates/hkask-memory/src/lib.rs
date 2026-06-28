//! hKask Memory — Semantic and episodic memory pipelines
//!
//! **Two-Layer DRY System:**
//! - Layer 1: Memory recall dedup (`recall_dedup`) — entity-attribute-value hash
//! - Layer 2: Prompt assembly dedup (`hkask-templates/src/context_assembly.rs`)

pub(crate) mod bayesian; // Loop 2b (semantic confidence combination)
pub mod consolidation; // Episodic → Semantic bridge
pub mod consolidation_ops;
pub mod consolidation_service;
pub mod episodic; // Loop 2a
pub mod episodic_loop;
pub mod ranking;
pub mod recall_dedup;
pub mod salience;
pub mod semantic; // Loop 2b
pub mod semantic_loop;

pub use consolidation::ConsolidationBridge;
pub use consolidation_ops::*;
pub use consolidation_service::ConsolidationService;
pub use episodic::{EpisodicMemory, EpisodicMemoryError};
pub use episodic_loop::EpisodicLoop;
pub use semantic::{SemanticMemory, SemanticMemoryError};
pub use semantic_loop::SemanticLoop;
