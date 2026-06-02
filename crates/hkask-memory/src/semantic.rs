//! Semantic memory pipeline

use crate::recall_dedup;
use hkask_storage::{EmbeddingError, EmbeddingStore, Triple, TripleError, TripleStore};
use thiserror::Error;

/// Default per-entity storage budget for semantic memory (max triples per entity).
pub const DEFAULT_SEMANTIC_BUDGET: usize = 100_000;

#[derive(Error, Debug)]
pub enum SemanticMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
    #[error("Budget exceeded: stored {stored}, budget {budget}")]
    BudgetExceeded { stored: usize, budget: usize },
}

/// Semantic memory — shared knowledge graph
///
/// Provides the following subloops:
/// - **Confidence promotion** (6d): Bayesian seeding when consolidating from episodic,
///   using `bayesian::combine(episodic_conf, 0.5)` to promote confidence.
/// - **Storage budget** (6e): Per-entity storage limit with retraction candidate
///   identification for lowest-confidence triples.
pub struct SemanticMemory {
    triple_store: TripleStore,
    #[allow(dead_code)] // Will be used by SemanticLoop::tick() after loop migration
    embedding_store: EmbeddingStore,
    /// Per-entity storage budget (max triples per entity). Default: 100,000
    storage_budget: usize,
}

impl SemanticMemory {
    pub fn new(triple_store: TripleStore, embedding_store: EmbeddingStore) -> Self {
        Self {
            triple_store,
            embedding_store,
            storage_budget: DEFAULT_SEMANTIC_BUDGET,
        }
    }

    /// Set the per-entity storage budget (max triples per entity).
    pub fn with_storage_budget(mut self, budget: usize) -> Self {
        self.storage_budget = budget;
        self
    }

    // (store removed — zero external consumers)
    // (query removed — zero external consumers)

    /// Query by entity with deduplication (entity_attribute_value_hash strategy).
    ///
    /// Filters duplicate triples at recall time. Two triples are considered
    /// duplicates if they share the same entity, attribute, and canonical value —
    /// regardless of timestamps, confidence, or perspective metadata.
    pub fn query_deduped(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(recall_dedup::dedup_triples(triples))
    }

    // (query_deduped_with_stats removed — zero external consumers)
    // (store_embedding removed — zero external consumers)

    // (consolidate removed — zero external consumers)
    // (check_budget removed — zero external consumers)
    // (storage_usage removed — zero external consumers)
    // (retraction_candidates removed — zero external consumers)
    // (semantic_budget getter removed — zero external consumers)

    // (recall removed — zero external consumers)

    // (recall_combined removed — zero external consumers)
    // (recall_combined_with_stats removed — zero external consumers)
    // (query_similar removed — zero external consumers)
    // (recall_with_similarity removed — zero external consumers)
}
