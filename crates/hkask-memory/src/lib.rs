//! hKask Memory — Semantic and episodic memory pipelines
//!
//! **Three-Layer DRY System:**
//! - Layer 1: Memory recall dedup (`recall_dedup`) — entity-attribute-value hash
//! - Layer 2: Session message dedup (`hkask-ensemble/src/chat_dedup.rs`)
//! - Layer 3: Prompt assembly dedup (`hkask-templates/src/context_assembly.rs`)

pub(crate) mod bayesian; // Loop 2b (semantic confidence combination)
pub mod consolidation; // Episodic → Semantic bridge
pub mod episodic; // Loop 2a
pub mod episodic_loop;
pub(crate) mod recall_dedup;
pub mod semantic; // Loop 2b
pub mod semantic_loop;

pub use consolidation::ConsolidationBridge;
pub use episodic::EpisodicMemory;
pub use episodic_loop::EpisodicLoop;
pub use semantic::SemanticMemory;
pub use semantic_loop::SemanticLoop;
