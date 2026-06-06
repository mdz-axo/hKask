//! Semantic memory pipeline
//!
//! Provides the following subloops:
//! - **Storage budget** (6e): Per-entity storage limit with deletion of
//!   lowest-confidence triples when budget is exceeded.
//! - **Similarity-augmented recall**: KNN search over embeddings to find
//!   semantically related triples, enabling context assembly that goes
//!   beyond exact entity matches.
//! - **Corpus centroid**: Mean embedding vector for style cluster validation.
//! - **Prefix purge**: Idempotent re-ingest by deleting embeddings matching a prefix.

use crate::recall_dedup;
use hkask_storage::{
    EmbeddingError, EmbeddingStore, SimilarityResult, Triple, TripleError, TripleStore,
};
use hkask_types::Visibility;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SemanticMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
    #[error("Invalid visibility for semantic store: {0}")]
    InvalidVisibility(String),
    #[error("No embeddings found for centroid: {0}")]
    NoEmbeddingsForCentroid(String),
    #[error(
        "Semantic memory requires no perspective (use consolidation bridge for episodic→semantic promotion)"
    )]
    HasPerspective,
}

/// Result of computing a style centroid
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CentroidResult {
    /// The centroid vector (arithmetic mean of matching embeddings)
    pub centroid: Vec<f32>,
    /// Number of passages used to compute the centroid
    pub passage_count: usize,
    /// Whether the centroid was stored under `store_as`
    pub stored: bool,
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
    embedding: Arc<EmbeddingStore>,
}

impl SemanticMemory {
    pub fn new(triple_store: TripleStore, embedding_store: EmbeddingStore) -> Self {
        Self {
            triple_store,
            embedding: Arc::new(embedding_store),
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
            .filter(|t| t.access.visibility == Visibility::Shared)
            .collect();
        Ok(recall_dedup::dedup_triples(filtered))
    }

    pub fn store(&self, triple: Triple) -> Result<(), SemanticMemoryError> {
        if triple.access.visibility != Visibility::Shared {
            return Err(SemanticMemoryError::InvalidVisibility(format!(
                "Semantic memory requires Shared visibility, got {:?}",
                triple.access.visibility
            )));
        }
        if triple.access.perspective.is_some() {
            return Err(SemanticMemoryError::HasPerspective);
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
        Ok(self.triple_store.count_semantic_by_entity(entity)?)
    }

    // Embedding operations (Loop 2b) — similarity-augmented recall

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

    /// Retrieve all entity_refs matching a prefix.
    ///
    /// Uses SQL LIKE query instead of zero-vector KNN scan.
    /// Returns entity_refs for prefix-based operations (centroid, purge).
    fn entity_refs_by_prefix(&self, prefix: &str) -> Result<Vec<String>, SemanticMemoryError> {
        Ok(self.embedding.query_by_prefix(prefix)?)
    }

    // Corpus operations (Loop 2b) — centroid + purge for style embeddings

    /// Compute the centroid (mean embedding vector) for embeddings matching a prefix.
    ///
    /// Only includes embeddings whose `entity_ref` starts with `prefix` but does NOT
    /// start with `exclude_prefix` and does NOT equal `exclude_ref`. This filters out
    /// meta-entries (style rules, centroids) that are not prose exemplars.
    ///
    /// The centroid is the arithmetic mean of all matching vectors, used for
    /// style cluster validation: generated prose should fall within a cosine
    /// distance threshold of this centroid.
    ///
    /// If `store_as` is provided, the centroid is also stored as an embedding
    /// under that entity_ref, enabling one-step compute+store.
    pub fn compute_centroid(
        &self,
        prefix: &str,
        exclude_prefix: &str,
        exclude_ref: &str,
        dim: usize,
        store_as: Option<&str>,
        model: Option<&str>,
    ) -> Result<CentroidResult, SemanticMemoryError> {
        let matching_refs: Vec<String> = self
            .entity_refs_by_prefix(prefix)?
            .into_iter()
            .filter(|r| !r.starts_with(exclude_prefix) && r != exclude_ref)
            .collect();

        if matching_refs.is_empty() {
            return Err(SemanticMemoryError::NoEmbeddingsForCentroid(
                prefix.to_string(),
            ));
        }

        // Fetch each embedding and compute mean
        let mut centroid = vec![0.0f32; dim];
        let mut count = 0usize;
        for entity_ref in &matching_refs {
            if let Ok(emb) = self.embedding.get(entity_ref) {
                for (i, v) in emb.vector.iter().enumerate() {
                    centroid[i] += v;
                }
                count += 1;
            }
        }

        if count == 0 {
            return Err(SemanticMemoryError::NoEmbeddingsForCentroid(
                prefix.to_string(),
            ));
        }

        let n = count as f32;
        for v in centroid.iter_mut() {
            *v /= n;
        }

        let stored = if let Some(ref_to_store) = store_as {
            if let Some(m) = model {
                let _id = self.embedding.store(ref_to_store, &centroid, m)?;
                true
            } else {
                false
            }
        } else {
            false
        };

        tracing::info!(
            target: "cns.semantic",
            prefix = %prefix,
            passage_count = count,
            stored = stored,
            "Centroid computed"
        );

        Ok(CentroidResult {
            centroid,
            passage_count: count,
            stored,
        })
    }

    /// Purge all embeddings whose `entity_ref` starts with `prefix`.
    ///
    /// Uses SQL prefix query to find candidates, then deletes.
    /// Returns the number of embeddings deleted.
    ///
    /// Used for idempotent re-ingest: purge an author's existing embeddings
    /// before re-downloading and re-embedding their corpus.
    pub fn purge_by_prefix(&self, prefix: &str) -> Result<usize, SemanticMemoryError> {
        let to_delete = self.entity_refs_by_prefix(prefix)?;

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

    /// Chunk text into passages for embedding.
    ///
    /// Splits on paragraph boundaries (double newlines), then applies
    /// min/max word count constraints. Long paragraphs are split at
    /// sentence boundaries. Short paragraphs are concatenated until
    /// min_words is reached.
    ///
    /// Returns (entity_ref, text) pairs with entity_ref formatted as
    /// `{entity_ref_prefix}:{chunk_index}`.
    pub fn chunk_text(
        text: &str,
        entity_ref_prefix: &str,
        min_words: usize,
        max_words: usize,
        sentence_boundary: &str,
    ) -> Vec<(String, String)> {
        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();

        let mut passages = Vec::new();
        let mut buffer = String::new();
        let mut buffer_words = 0;
        let mut chunk_index = 0;
        let boundary_bytes: Vec<u8> = sentence_boundary.bytes().collect();

        for paragraph in &paragraphs {
            let word_count = paragraph.split_whitespace().count();

            if buffer_words + word_count > max_words && buffer_words >= min_words {
                let entity_ref = format!("{}:{}", entity_ref_prefix, chunk_index);
                passages.push((entity_ref, buffer.trim().to_string()));
                chunk_index += 1;
                buffer.clear();
                buffer_words = 0;
            }

            if word_count > max_words {
                if !buffer.is_empty() && buffer_words >= min_words {
                    let entity_ref = format!("{}:{}", entity_ref_prefix, chunk_index);
                    passages.push((entity_ref, buffer.trim().to_string()));
                    chunk_index += 1;
                    buffer.clear();
                    buffer_words = 0;
                }

                let words: Vec<&str> = paragraph.split_whitespace().collect();
                let mut current = Vec::new();

                for word in &words {
                    current.push(*word);

                    if current.len() >= max_words {
                        let last = current.last().unwrap();
                        let ends_with_boundary = last
                            .chars()
                            .last()
                            .map(|c| boundary_bytes.contains(&(c as u8)))
                            .unwrap_or(false);

                        if ends_with_boundary || current.len() >= max_words * 2 {
                            let text = current.join(" ");
                            if current.len() >= min_words {
                                let entity_ref = format!("{}:{}", entity_ref_prefix, chunk_index);
                                passages.push((entity_ref, text));
                                chunk_index += 1;
                            } else if !buffer.is_empty() {
                                buffer.push(' ');
                                buffer.push_str(&text);
                                buffer_words += current.len();
                            } else {
                                let entity_ref = format!("{}:{}", entity_ref_prefix, chunk_index);
                                passages.push((entity_ref, text));
                                chunk_index += 1;
                            }
                            current = Vec::new();
                        }
                    }
                }

                if !current.is_empty() {
                    let text = current.join(" ");
                    if !buffer.is_empty() {
                        buffer.push(' ');
                        buffer.push_str(&text);
                        buffer_words += current.len();
                    } else {
                        let entity_ref = format!("{}:{}", entity_ref_prefix, chunk_index);
                        passages.push((entity_ref, text));
                        chunk_index += 1;
                    }
                }
            } else {
                if !buffer.is_empty() {
                    buffer.push(' ');
                }
                buffer.push_str(paragraph);
                buffer_words += word_count;
            }
        }

        if !buffer.is_empty() {
            let entity_ref = format!("{}:{}", entity_ref_prefix, chunk_index);
            passages.push((entity_ref, buffer.trim().to_string()));
        }

        passages
    }

    /// Strip Project Gutenberg headers and footers from text.
    ///
    /// Looks for the standard `*** START OF` / `*** END OF` markers.
    pub fn strip_gutenberg_headers(text: &str) -> String {
        let start_marker = "*** START OF";
        let end_marker = "*** END OF";

        let start = text
            .find(start_marker)
            .and_then(|i| text[i..].find('\n').map(|j| i + j + 1))
            .unwrap_or(0);

        let end = text.find(end_marker).unwrap_or(text.len());

        text[start..end].trim().to_string()
    }

    // Deletion (Loop 2b) — Cybernetics membrane operation

    // Note: The following four methods (delete_triple, lowest_confidence_triples,
    // low_confidence_count, low_confidence_triples) are `pub` rather than
    // `pub(crate)` because they are needed by:
    //   1. `ConsolidationService` (in this crate) for user-triggered cleanup
    //   2. `hkask-mcp-semantic` MCP server (external crate) for the
    //      `semantic_consolidate` tool
    //
    // This is safe because these are data operations, not authority operations.
    // Semantic triples are shared/public knowledge (visibility: Shared,
    // perspective: None) — deleting or querying them doesn't bypass the OCAP
    // membrane. The ConsolidationToken and GovernedTool membrane control who
    // can *trigger* the operations; these methods just execute them.

    /// Delete a semantic triple by ID (budget enforcement / consolidation cleanup).
    ///
    /// When the semantic storage budget is exceeded or consolidation cleanup
    /// targets low-confidence triples, they are deleted outright.
    pub fn delete_triple(&self, id: &hkask_storage::TripleID) -> Result<(), SemanticMemoryError> {
        tracing::info!(
            target: "cns.semantic",
            triple_id = %id,
            "Semantic triple deleted (budget enforcement)"
        );
        self.triple_store.delete_by_id(id)?;
        Ok(())
    }

    // Budget enforcement (Loop 2b) — Cybernetics membrane operation

    /// Identify the lowest-confidence semantic triples for budget enforcement.
    ///
    /// Returns up to `limit` triples with `perspective IS NULL`, ordered by
    /// confidence ascending then `valid_from` ascending (oldest first).
    pub fn lowest_confidence_triples(
        &self,
        limit: usize,
    ) -> Result<Vec<Triple>, SemanticMemoryError> {
        Ok(self.triple_store.query_semantic_lowest_confidence(limit)?)
    }

    /// Count semantic triples at or below a confidence threshold.
    ///
    /// Used by `SemanticLoop::sense()` and `ConsolidationService`
    /// for the consolidation trigger signal.
    pub fn low_confidence_count(&self, threshold: f64) -> Result<usize, SemanticMemoryError> {
        Ok(self
            .triple_store
            .count_semantic_below_confidence(threshold)?)
    }

    /// Query semantic triples at or below a confidence threshold.
    ///
    /// Returns up to `limit` triples with `confidence <= threshold`,
    /// ordered by confidence ascending then `valid_from` ascending.
    ///
    /// Used by `SemanticLoop::act()` and `ConsolidationService`
    /// for the consolidation trigger.
    pub fn low_confidence_triples(
        &self,
        threshold: f64,
        limit: usize,
    ) -> Result<Vec<Triple>, SemanticMemoryError> {
        Ok(self
            .triple_store
            .query_semantic_below_confidence(threshold, limit)?)
    }
}
