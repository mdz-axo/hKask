//! Semantic memory pipeline
//!
//! Provides the following subloops:
//! - **Confidence promotion** (6d): Bayesian seeding when consolidating from episodic
//!   (confidence seeding at 0.5 baseline) to promote confidence.
//! - **Storage budget** (6e): Per-entity storage limit with retraction candidate
//!   identification for lowest-confidence triples.
//! - **Similarity-augmented recall**: KNN search over embeddings to find
//!   semantically related triples, enabling context assembly that goes
//!   beyond exact entity matches.

use crate::bayesian;
use crate::recall_dedup;
use hkask_storage::{EmbeddingStore, Triple, TripleError, TripleStore};
use hkask_types::Visibility;
use hkask_types::ports::{EmbeddingError, EmbeddingPort, SimilarityResult};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SemanticMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
    #[error("Triple not found for retraction: {entity}/{attribute}")]
    TripleNotFound { entity: String, attribute: String },
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
    #[error("Invalid visibility for semantic store: {0}")]
    InvalidVisibility(String),
}

/// Semantic memory — shared knowledge graph
///
/// Provides the following subloops:
/// - **Confidence promotion** (6d): Bayesian seeding when consolidating from episodic
///   (confidence seeding at 0.5 baseline) to promote confidence.
/// - **Storage budget** (6e): Per-entity storage limit with retraction candidate
///   identification for lowest-confidence triples.
/// - **Similarity-augmented recall**: KNN search over embeddings to find
///   semantically related triples, enabling context assembly that goes
///   beyond exact entity matches.
pub struct SemanticMemory {
    triple_store: TripleStore,
    embedding: Arc<dyn EmbeddingPort>,
}

impl SemanticMemory {
    pub fn new(triple_store: TripleStore, embedding_store: EmbeddingStore) -> Self {
        Self {
            triple_store,
            embedding: Arc::new(embedding_store),
        }
    }

    /// Create with a pre-wired embedding port (for testing or custom backends).
    pub fn with_embedding_port(
        triple_store: TripleStore,
        embedding: Arc<dyn EmbeddingPort>,
    ) -> Self {
        Self {
            triple_store,
            embedding,
        }
    }

    /// Query by entity with deduplication (entity_attribute_value_hash strategy).
    ///
    /// Filters duplicate triples at recall time. Two triples are considered
    /// duplicates if they share the same entity, attribute, and canonical value —
    /// regardless of timestamps, confidence, or perspective metadata.
    pub fn query_deduped(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let filtered: Vec<Triple> = triples
            .into_iter()
            .filter(|t| t.visibility == Visibility::Shared)
            .collect();
        Ok(recall_dedup::dedup_triples(filtered))
    }

    pub fn store(&self, triple: Triple) -> Result<(), SemanticMemoryError> {
        if triple.visibility != Visibility::Shared {
            return Err(SemanticMemoryError::InvalidVisibility(format!(
                "Semantic memory requires Shared visibility, got {:?}",
                triple.visibility
            )));
        }
        if triple.perspective.is_some() {
            return Err(SemanticMemoryError::InvalidVisibility(
                "Semantic memory requires no perspective (use consolidation bridge for episodic→semantic promotion)".to_string()
            ));
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

    // ========================================================================
    // Embedding operations (Loop 2b) — similarity-augmented recall
    // ========================================================================

    /// Store an embedding vector for a semantic triple.
    ///
    /// The embedding is indexed by the triple's ID (`entity_ref`), enabling
    /// similarity search to find semantically related triples.
    pub fn store_embedding(
        &self,
        entity_ref: &str,
        vector: &[f32],
        model: &str,
    ) -> Result<String, SemanticMemoryError> {
        Ok(self.embedding.store(entity_ref, vector, model)?)
    }

    /// Search for semantically similar embeddings.
    ///
    /// Returns KNN results ordered by ascending distance (most similar first).
    /// Use this for context assembly that goes beyond exact entity matches —
    /// given a query embedding, find triples that are semantically close even
    /// if their entity keys differ.
    pub fn search_similar(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<SimilarityResult>, SemanticMemoryError> {
        Ok(self.embedding.search(query_vector, limit)?)
    }

    /// Count stored embeddings.
    pub fn embedding_count(&self) -> Result<usize, SemanticMemoryError> {
        Ok(self.embedding.count()?)
    }

    // ========================================================================
    // Retraction (Loop 2b) — Cybernetics membrane operation
    // ========================================================================

    /// Retract a semantic triple by reducing its confidence (not deleting).
    ///
    /// Semantic triples are shared knowledge, so retraction reduces confidence
    /// rather than removing entirely. Uses `bayesian::retract()` for the
    /// confidence reduction.
    ///
    /// **Membrane-sealed:** Only callable from within this crate.
    pub(crate) fn retract_triple(
        &self,
        entity: &str,
        attribute: &str,
        retraction_confidence: f64,
    ) -> Result<f64, SemanticMemoryError> {
        let triples = self
            .triple_store
            .query_by_entity_attribute(entity, attribute)?;
        // Semantic triples have perspective = None
        let triple = triples
            .into_iter()
            .find(|t| t.perspective.is_none())
            .ok_or_else(|| SemanticMemoryError::TripleNotFound {
                entity: entity.to_string(),
                attribute: attribute.to_string(),
            })?;

        let retracted = bayesian::retract(triple.confidence, retraction_confidence);
        tracing::info!(
            target: "cns.semantic",
            entity = %entity,
            attribute = %attribute,
            original_confidence = triple.confidence,
            retracted_confidence = retracted,
            "Semantic confidence retracted"
        );
        self.triple_store
            .update(&triple.id, triple.value.clone(), retracted)?;

        Ok(retracted)
    }

    // ========================================================================
    // Budget enforcement (Loop 2b) — Cybernetics membrane operation
    // ========================================================================

    /// Identify the lowest-confidence semantic triples for budget enforcement.
    ///
    /// Returns up to `limit` triples with `perspective IS NULL`, ordered by
    /// confidence ascending then `valid_from` ascending (oldest first).
    ///
    /// **Membrane-sealed:** Only callable from within this crate.
    pub(crate) fn lowest_confidence_triples(
        &self,
        limit: usize,
    ) -> Result<Vec<Triple>, SemanticMemoryError> {
        Ok(self.triple_store.query_semantic_lowest_confidence(limit)?)
    }
}
