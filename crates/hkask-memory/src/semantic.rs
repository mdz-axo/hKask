//! Semantic memory pipeline

use crate::recall_dedup::{self, DedupResult};
use hkask_storage::{Embedding, EmbeddingError, EmbeddingStore, Triple, TripleError, TripleStore};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SemanticMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
}

/// Semantic memory — shared knowledge graph
pub struct SemanticMemory {
    triple_store: TripleStore,
    embedding_store: EmbeddingStore,
}

impl SemanticMemory {
    pub fn new(triple_store: TripleStore, embedding_store: EmbeddingStore) -> Self {
        Self {
            triple_store,
            embedding_store,
        }
    }

    /// Store a semantic triple (public by default)
    pub fn store(&self, triple: Triple) -> Result<(), SemanticMemoryError> {
        self.triple_store.insert(&triple)?;
        Ok(())
    }

    /// Query by entity
    pub fn query(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError> {
        Ok(self.triple_store.query_by_entity(entity)?)
    }

    /// Query by entity with deduplication (entity_attribute_value_hash strategy).
    ///
    /// Filters duplicate triples at recall time. Two triples are considered
    /// duplicates if they share the same entity, attribute, and canonical value —
    /// regardless of timestamps, confidence, or perspective metadata.
    pub fn query_deduped(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(recall_dedup::dedup_triples(triples))
    }

    /// Query by entity with deduplication and statistics.
    pub fn query_deduped_with_stats(
        &self,
        entity: &str,
    ) -> Result<DedupResult, SemanticMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(recall_dedup::dedup_triples_with_stats(triples))
    }

    /// Store embedding for semantic search
    pub fn store_embedding(&self, embedding: Embedding) -> Result<(), SemanticMemoryError> {
        self.embedding_store.insert(&embedding)?;
        Ok(())
    }

    /// Consolidate episodic memories into semantic knowledge.
    ///
    /// Takes episodic triples (with perspective) and creates semantic triples
    /// (without perspective). Deduplicates before storing to avoid redundant
    /// semantic entries from multiple episodic observations.
    ///
    /// Returns the number of new semantic triples stored.
    pub fn consolidate(&self, episodic_triples: Vec<Triple>) -> Result<usize, SemanticMemoryError> {
        let semantic: Vec<Triple> = episodic_triples
            .into_iter()
            .map(|t| {
                Triple::new(&t.entity, &t.attribute, t.value, t.owner_webid)
                    .with_confidence(t.confidence)
                    .with_visibility(t.visibility)
            })
            .collect();

        let deduped = recall_dedup::dedup_triples(semantic);
        let count = deduped.len();

        for triple in &deduped {
            self.triple_store.insert(triple)?;
        }

        Ok(count)
    }

    /// Recall semantic knowledge for an entity.
    ///
    /// Returns deduplicated semantic triples (no perspective).
    pub fn recall(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError> {
        self.query_deduped(entity)
    }
}
