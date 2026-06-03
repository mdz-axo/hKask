//! Semantic memory pipeline

use crate::recall_dedup;
use hkask_storage::{EmbeddingStore, Triple, TripleError, TripleStore};
use hkask_types::Visibility;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SemanticMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
}

/// Semantic memory — shared knowledge graph
///
/// Provides the following subloops:
/// - **Confidence promotion** (6d): Bayesian seeding when consolidating from episodic
///   (confidence seeding at 0.5 baseline) to promote confidence.
/// - **Storage budget** (6e): Per-entity storage limit with retraction candidate
///   identification for lowest-confidence triples.
pub struct SemanticMemory {
    triple_store: TripleStore,
    // Embedding store reserved for future SemanticLoop::tick() integration
    _embedding_store: EmbeddingStore,
}

impl SemanticMemory {
    pub fn new(triple_store: TripleStore, embedding_store: EmbeddingStore) -> Self {
        Self {
            triple_store,
            _embedding_store: embedding_store,
        }
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

    pub fn store(&self, mut triple: Triple) -> Result<(), SemanticMemoryError> {
        if triple.visibility != Visibility::Shared {
            tracing::warn!(
                target: "hkask.memory.semantic",
                visibility = ?triple.visibility,
                "Semantic store requires Shared visibility; overriding"
            );
            triple.visibility = Visibility::Shared;
        }
        if triple.perspective.is_some() {
            tracing::warn!(
                target: "hkask.memory.semantic",
                "Semantic store requires no perspective; clearing"
            );
            triple.perspective = None;
        }
        self.triple_store.insert(&triple)?;
        Ok(())
    }

    pub(crate) fn store_consolidated(&self, triple: Triple) -> Result<(), SemanticMemoryError> {
        self.triple_store.insert(&triple)?;
        Ok(())
    }

    pub fn triple_count(&self) -> Result<usize, SemanticMemoryError> {
        Ok(self.triple_store.count_semantic()?)
    }

    pub fn triple_count_for_entity(&self, entity: &str) -> Result<usize, SemanticMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(triples.len())
    }
}
