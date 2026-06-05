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
//! - **Corpus centroid**: Mean embedding vector for style cluster validation.
//! - **Prefix purge**: Idempotent re-ingest by deleting embeddings matching a prefix.

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
    #[error("No embeddings found for centroid: {0}")]
    NoEmbeddingsForCentroid(String),
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
    // Corpus operations (Loop 2b) — centroid + purge for style embeddings
    // ========================================================================

    /// Compute the centroid (mean embedding vector) for embeddings matching a prefix.
    ///
    /// Only includes embeddings whose `entity_ref` starts with `prefix` but does NOT
    /// start with `exclude_prefix` and does NOT equal `exclude_ref`. This filters out
    /// meta-entries (style rules, centroids) that are not prose exemplars.
    ///
    /// The centroid is the arithmetic mean of all matching vectors, used for
    /// style cluster validation: generated prose should fall within a cosine
    /// distance threshold of this centroid.
    pub fn compute_centroid(
        &self,
        prefix: &str,
        exclude_prefix: &str,
        exclude_ref: &str,
        dim: usize,
    ) -> Result<Vec<f32>, SemanticMemoryError> {
        let zero_vec = vec![0.0f32; dim];
        let results = self.embedding.search(&zero_vec, 10000)?;

        let matching: Vec<&hkask_types::ports::StoredEmbedding> = results
            .iter()
            .filter(|r| {
                let ref_str = &r.embedding.entity_ref;
                ref_str.starts_with(prefix)
                    && !ref_str.starts_with(exclude_prefix)
                    && ref_str != exclude_ref
            })
            .map(|r| &r.embedding)
            .collect();

        if matching.is_empty() {
            return Err(SemanticMemoryError::NoEmbeddingsForCentroid(
                prefix.to_string(),
            ));
        }

        let mut centroid = vec![0.0f32; dim];
        for emb in &matching {
            for (i, v) in emb.vector.iter().enumerate() {
                centroid[i] += v;
            }
        }
        let count = matching.len() as f32;
        for v in centroid.iter_mut() {
            *v /= count;
        }

        tracing::info!(
            target: "cns.semantic",
            prefix = %prefix,
            passage_count = matching.len(),
            "Centroid computed"
        );

        Ok(centroid)
    }

    /// Purge all embeddings whose `entity_ref` starts with `prefix`.
    ///
    /// Uses zero-vector KNN scan to find candidates, then filters by prefix
    /// and deletes. Returns the number of embeddings deleted.
    ///
    /// Used for idempotent re-ingest: purge an author's existing embeddings
    /// before re-downloading and re-embedding their corpus.
    pub fn purge_by_prefix(&self, prefix: &str, dim: usize) -> Result<usize, SemanticMemoryError> {
        let zero_vec = vec![0.0f32; dim];
        let results = self.embedding.search(&zero_vec, 10000)?;

        let to_delete: Vec<String> = results
            .iter()
            .filter(|r| r.embedding.entity_ref.starts_with(prefix))
            .map(|r| r.embedding.entity_ref.clone())
            .collect();

        let mut count = 0;
        for entity_ref in &to_delete {
            if self.embedding.delete(entity_ref).is_ok() {
                count += 1;
            }
        }

        tracing::info!(
            target: "cns.semantic",
            prefix = %prefix,
            purged = count,
            "Purged embeddings by prefix"
        );

        Ok(count)
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
