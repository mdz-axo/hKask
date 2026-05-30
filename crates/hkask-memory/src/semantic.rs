//! Semantic memory pipeline

use crate::bayesian;
use crate::recall_dedup::{self, DedupResult};
use hkask_storage::{
    Embedding, EmbeddingError, EmbeddingStore, Triple, TripleError, TripleID, TripleStore,
};
use std::collections::HashSet;
use thiserror::Error;
use tracing;

/// Default per-entity storage budget for semantic memory (max triples per entity).
pub const DEFAULT_SEMANTIC_BUDGET: usize = 100_000;

/// Prior confidence for Bayesian seeding during consolidation.
///
/// When episodic memories are promoted to semantic knowledge, their
/// confidence is combined with this prior to ensure semantic knowledge
/// doesn't start from zero confidence.
const CONSOLIDATION_PRIOR: f64 = 0.5;

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
    /// (without perspective). Confidence is promoted using Bayesian seeding
    /// (Loop 6d): `bayesian::combine(episodic_conf, 0.5)` ensures semantic
    /// knowledge doesn't start from zero confidence.
    ///
    /// Deduplicates before storing to avoid redundant semantic entries from
    /// multiple episodic observations.
    ///
    /// Returns the number of new semantic triples stored.
    pub fn consolidate(&self, episodic_triples: Vec<Triple>) -> Result<usize, SemanticMemoryError> {
        let semantic: Vec<Triple> = episodic_triples
            .into_iter()
            .map(|t| {
                let promoted_confidence = bayesian::combine(t.confidence, CONSOLIDATION_PRIOR);
                Triple::new(&t.entity, &t.attribute, t.value, t.owner_webid)
                    .with_confidence(promoted_confidence)
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

    /// Check whether storing `count` new triples for `entity` would exceed
    /// the per-entity storage budget.
    ///
    /// Returns `Ok(())` if within budget, or `Err(BudgetExceeded)` if the
    /// budget would be exceeded. Emits a `cns.memory.budget` span on
    /// budget violation.
    pub fn check_budget(&self, entity: &str, count: usize) -> Result<(), SemanticMemoryError> {
        let current = self.triple_store.query_by_entity(entity)?.len();
        if current + count > self.storage_budget {
            tracing::warn!(
                target: "cns.memory.budget",
                entity = %entity,
                current = current,
                requested = count,
                budget = self.storage_budget,
                "Semantic storage budget would be exceeded"
            );
            return Err(SemanticMemoryError::BudgetExceeded {
                stored: current,
                budget: self.storage_budget,
            });
        }
        Ok(())
    }

    /// Get the current storage usage for an entity (number of triples).
    pub fn storage_usage(&self, entity: &str) -> Result<usize, SemanticMemoryError> {
        let count = self.triple_store.query_by_entity(entity)?.len();
        Ok(count)
    }

    /// Identify triples eligible for retraction (lowest-confidence) when
    /// the per-entity budget is exceeded (6e).
    ///
    /// Returns triples sorted by retraction priority:
    /// lowest-confidence first.
    pub fn retraction_candidates(
        &self,
        entity: &str,
        limit: usize,
    ) -> Result<Vec<Triple>, SemanticMemoryError> {
        let mut triples = self.triple_store.query_by_entity(entity)?;

        // Sort by confidence ascending — lowest-confidence first for retraction
        triples.sort_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        triples.truncate(limit);
        Ok(triples)
    }

    /// Get the configured storage budget.
    pub fn semantic_budget(&self) -> usize {
        self.storage_budget
    }

    /// Recall semantic knowledge for an entity.
    ///
    /// Returns deduplicated semantic triples (no perspective).
    pub fn recall(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError> {
        self.query_deduped(entity)
    }

    /// Recall semantic knowledge with Bayesian confidence combination.
    ///
    /// When multiple triples share the same `(entity, attribute)` key,
    /// their confidences are combined iteratively using `bayesian::combine()`.
    /// The resulting triple carries the combined confidence, the value from
    /// the highest-confidence source triple, and the most recent `valid_from`.
    /// Perspective is `None` (semantic).
    pub fn recall_combined(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(combine_triples_by_attribute(triples))
    }

    /// Recall with Bayesian combination and statistics.
    ///
    /// Returns combined triples alongside counts of originals and duplicates merged.
    pub fn recall_combined_with_stats(
        &self,
        entity: &str,
    ) -> Result<CombineResult, SemanticMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let original_count = triples.len();
        let combined = combine_triples_by_attribute(triples);
        let combined_count = combined.len();
        Ok(CombineResult {
            triples: combined,
            original_count,
            combined_count,
            duplicates_merged: original_count - combined_count,
        })
    }

    /// Query similar triples by combining embedding-based nearest-neighbor
    /// search with entity-based keyword results.
    ///
    /// 1. Search the embedding store for the `k` nearest neighbors of `embedding`.
    /// 2. For each match with a valid `entity_ref`, look up the associated triple.
    /// 3. Also query by `entity` for keyword-based results.
    /// 4. Merge both result sets, deduplicating by triple ID.
    /// 5. Sort: embedding matches first (ordered by similarity), then entity-based
    ///    results ordered by confidence descending.
    pub fn query_similar(
        &self,
        entity: &str,
        embedding: &[f32],
        k: usize,
    ) -> Result<Vec<Triple>, SemanticMemoryError> {
        tracing::debug!(entity = entity, k = k, "cns.memory.semantic.query_similar");

        // 1. Embedding-based nearest neighbors
        let embedding_results = self.embedding_store.search(embedding, k)?;

        // 2. Resolve embedding matches to triples, tracking IDs seen via similarity
        let mut seen_ids: HashSet<TripleID> = HashSet::new();
        let mut similar_triples: Vec<Triple> = Vec::new();

        for (emb, _distance) in embedding_results {
            let triple_id = match emb.entity_ref {
                Some(id) => id,
                None => continue, // skip embeddings without entity_ref
            };

            if seen_ids.contains(&triple_id) {
                continue;
            }

            if let Some(triple) = self.triple_store.get_by_id(&triple_id)? {
                seen_ids.insert(triple_id);
                similar_triples.push(triple);
            }
        }

        // 3. Entity-based keyword results
        let entity_triples = self.triple_store.query_by_entity(entity)?;

        // 4. Merge: add entity triples not already seen from similarity search
        let mut merged: Vec<Triple> = similar_triples.clone();
        for triple in entity_triples {
            if seen_ids.insert(triple.id) {
                merged.push(triple);
            }
        }

        // 5. Sort: similarity matches first (in distance order), then entity results by confidence desc
        merged.sort_by(|a, b| {
            let a_in_sim = seen_from_similarity(&a.id, &similar_triples);
            let b_in_sim = seen_from_similarity(&b.id, &similar_triples);

            match (a_in_sim, b_in_sim) {
                (true, true) | (false, false) => b
                    .confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
            }
        });

        Ok(merged)
    }

    /// Higher-level recall combining entity-based and similarity-based results,
    /// deduplicates them, and applies confidence combination.
    ///
    /// This is the primary entry point for semantic recall in Loop 6a.
    /// When the same fact is found both via similarity and entity lookup,
    /// the confidences are combined using Bayesian combination.
    pub fn recall_with_similarity(
        &self,
        entity: &str,
        embedding: &[f32],
        k: usize,
    ) -> Result<Vec<Triple>, SemanticMemoryError> {
        tracing::debug!(
            entity = entity,
            k = k,
            "cns.memory.semantic.recall_with_similarity"
        );

        // 1. Embedding-based nearest neighbors
        let embedding_results = self.embedding_store.search(embedding, k)?;

        // 2. Resolve embedding matches to triples
        let mut sim_triples: Vec<Triple> = Vec::new();
        for (emb, _distance) in embedding_results {
            let triple_id = match emb.entity_ref {
                Some(id) => id,
                None => continue,
            };
            if let Some(triple) = self.triple_store.get_by_id(&triple_id)? {
                sim_triples.push(triple);
            }
        }

        // 3. Entity-based keyword results
        let entity_triples = self.triple_store.query_by_entity(entity)?;

        // 4. Merge with confidence combination
        // Build a map from EAV hash to (triple, combined_confidence).
        // If a triple appears in both sets, combine confidences.
        let mut seen: std::collections::HashMap<[u8; 32], Triple> =
            std::collections::HashMap::new();
        let mut confidences: std::collections::HashMap<[u8; 32], f64> =
            std::collections::HashMap::new();

        // Process similarity results first
        for triple in sim_triples {
            let hash = recall_dedup::eav_hash(&triple);
            let _ = seen.entry(hash).or_insert_with(|| triple.clone());
            confidences
                .entry(hash)
                .and_modify(|c| *c = crate::bayesian::combine(*c, triple.confidence))
                .or_insert(triple.confidence);
        }

        // Process entity results — combine confidence if already seen
        for triple in entity_triples {
            let hash = recall_dedup::eav_hash(&triple);
            let _ = seen.entry(hash).or_insert_with(|| triple.clone());
            confidences
                .entry(hash)
                .and_modify(|c| *c = crate::bayesian::combine(*c, triple.confidence))
                .or_insert(triple.confidence);
        }

        // 5. Apply combined confidences and sort by confidence descending
        let mut merged: Vec<Triple> = seen
            .into_iter()
            .map(|(hash, mut triple)| {
                if let Some(combined) = confidences.get(&hash) {
                    triple.confidence = *combined;
                }
                triple
            })
            .collect();

        merged.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(merged)
    }
}

/// Check whether a triple ID was found via similarity search.
fn seen_from_similarity(id: &TripleID, similar: &[Triple]) -> bool {
    similar.iter().any(|t| t.id == *id)
}

/// Combine triples sharing the same `(entity, attribute)` key using Bayesian
/// confidence combination.
///
/// For each group of triples with matching entity+attribute:
/// - Confidences are combined iteratively via `bayesian::join()`.
/// - The value is taken from the highest-confidence triple in the group.
/// - `valid_from` uses the most recent timestamp.
/// - `perspective` is `None` (semantic).
fn combine_triples_by_attribute(triples: Vec<Triple>) -> Vec<Triple> {
    use std::collections::HashMap;

    // Group by (entity, attribute)
    let mut groups: HashMap<(String, String), Vec<Triple>> = HashMap::new();
    for triple in triples {
        let key = (triple.entity.clone(), triple.attribute.clone());
        groups.entry(key).or_default().push(triple);
    }

    let mut result = Vec::with_capacity(groups.len());
    for mut group in groups.into_values() {
        if group.len() == 1 {
            result.push(group.into_iter().next().unwrap());
            continue;
        }

        // Sort by confidence descending so first element is highest-confidence
        group.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Combine confidences iteratively via bayesian::join
        let combined_confidence =
            crate::bayesian::join(&group.iter().map(|t| t.confidence).collect::<Vec<f64>>());

        // Value from the highest-confidence triple
        let best = &group[0];

        // Most recent valid_from
        let most_recent = group.iter().max_by_key(|t| t.valid_from).unwrap();

        result.push(Triple {
            id: TripleID::new(),
            entity: best.entity.clone(),
            attribute: best.attribute.clone(),
            value: best.value.clone(),
            valid_from: most_recent.valid_from,
            valid_to: None,
            confidence: combined_confidence,
            perspective: None,
            visibility: best.visibility,
            owner_webid: best.owner_webid,
        });
    }
    result
}

/// Result of a confidence-combined recall operation with statistics.
#[derive(Debug)]
pub struct CombineResult {
    pub triples: Vec<Triple>,
    pub original_count: usize,
    pub combined_count: usize,
    pub duplicates_merged: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::{Database, Embedding, EmbeddingStore};
    use hkask_types::{TripleID, WebID};

    fn test_db() -> (TripleStore, EmbeddingStore) {
        let db = Database::in_memory().expect("in-memory db");
        let ts = TripleStore::new(db.conn_arc());
        let es = EmbeddingStore::new(db.conn_arc());
        (ts, es)
    }

    fn test_webid() -> WebID {
        WebID::new()
    }

    fn make_embedding(vector: Vec<f32>, entity_ref: Option<TripleID>) -> Embedding {
        let mut emb = Embedding::new(vector, "test-model");
        if let Some(id) = entity_ref {
            emb = emb.with_entity_ref(id);
        }
        emb
    }

    #[test]
    fn query_similar_returns_embedding_and_entity_results() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Store a triple and its embedding
        let triple =
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid).with_confidence(0.9);
        mem.store(triple.clone()).unwrap();

        let mut embedding_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        embedding_vec[0] = 1.0;
        let emb = make_embedding(embedding_vec, Some(triple.id));
        mem.store_embedding(emb).unwrap();

        // Store another triple for the same entity (no embedding)
        let triple2 =
            Triple::new("entity1", "attr2", serde_json::json!("val2"), wid).with_confidence(0.7);
        mem.store(triple2.clone()).unwrap();

        // Query similar with the same embedding vector
        let mut query_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        query_vec[0] = 1.0;
        let results = mem.query_similar("entity1", &query_vec, 5).unwrap();

        // Should return both triples (one from embedding match, one from entity match)
        assert_eq!(results.len(), 2);
        // Embedding-matched triple should come first
        assert_eq!(results[0].id, triple.id);
    }

    #[test]
    fn query_similar_skips_embeddings_without_entity_ref() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Store a triple
        let triple =
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid).with_confidence(0.9);
        mem.store(triple.clone()).unwrap();

        // Store an embedding WITHOUT entity_ref
        let mut embedding_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        embedding_vec[0] = 1.0;
        let emb = make_embedding(embedding_vec, None);
        mem.store_embedding(emb).unwrap();

        // Query similar — should still return the entity-based triple
        let mut query_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        query_vec[0] = 1.0;
        let results = mem.query_similar("entity1", &query_vec, 5).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "entity1");
    }

    #[test]
    fn query_similar_deduplicates_by_triple_id() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Store a triple
        let triple =
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid).with_confidence(0.9);
        mem.store(triple.clone()).unwrap();

        // Store an embedding pointing to the same triple
        let mut embedding_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        embedding_vec[0] = 1.0;
        let emb = make_embedding(embedding_vec, Some(triple.id));
        mem.store_embedding(emb).unwrap();

        // Query similar — same triple found via both paths
        let mut query_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        query_vec[0] = 1.0;
        let results = mem.query_similar("entity1", &query_vec, 5).unwrap();

        // Should be deduplicated — only 1 result
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn query_similar_empty_embedding_results() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Store a triple (no embeddings at all)
        let triple =
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid).with_confidence(0.8);
        mem.store(triple).unwrap();

        // Query similar with a zero vector — no embedding matches,
        // but should still return entity-based results
        let query_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        let results = mem.query_similar("entity1", &query_vec, 5).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attribute, "attr1");
    }

    #[test]
    fn recall_with_similarity_combines_confidences() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Store a triple with confidence 0.8
        let triple =
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid).with_confidence(0.8);
        mem.store(triple.clone()).unwrap();

        // Store an embedding linking to the same triple
        let mut embedding_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        embedding_vec[0] = 1.0;
        let emb = make_embedding(embedding_vec, Some(triple.id));
        mem.store_embedding(emb).unwrap();

        // Recall with similarity — same fact found via both paths
        // Bayesian combine(0.8, 0.8) ≈ 0.941
        let mut query_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        query_vec[0] = 1.0;
        let results = mem
            .recall_with_similarity("entity1", &query_vec, 5)
            .unwrap();

        assert_eq!(results.len(), 1);
        // Combined confidence should be higher than either single source
        let combined = crate::bayesian::combine(0.8, 0.8);
        assert!((results[0].confidence - combined).abs() < 0.01);
    }

    #[test]
    fn recall_with_similarity_deduplicates_eav() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Store two triples for same entity with different attributes
        let t1 =
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid).with_confidence(0.9);
        mem.store(t1.clone()).unwrap();

        let t2 =
            Triple::new("entity1", "attr2", serde_json::json!("val2"), wid).with_confidence(0.7);
        mem.store(t2.clone()).unwrap();

        // Store embedding for t1
        let mut embedding_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        embedding_vec[0] = 1.0;
        let emb = make_embedding(embedding_vec, Some(t1.id));
        mem.store_embedding(emb).unwrap();

        let mut query_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        query_vec[0] = 1.0;
        let results = mem
            .recall_with_similarity("entity1", &query_vec, 5)
            .unwrap();

        // Should return 2 distinct facts (different EAV)
        assert_eq!(results.len(), 2);
        // Higher-confidence result first (after combining, t1 is combined so even higher)
        assert!(results[0].confidence >= results[1].confidence);
    }

    #[test]
    fn query_similar_no_results_for_unknown_entity() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);

        let query_vec = vec![0.0f32; hkask_storage::database::embedding_dim()];
        let results = mem.query_similar("nonexistent", &query_vec, 5).unwrap();
        assert!(results.is_empty());
    }

    // ========================================================================
    // PR 6d: Confidence promotion in consolidation
    // ========================================================================

    #[test]
    fn consolidate_promotes_confidence_with_bayesian_seeding() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Episodic triple with high confidence → should remain ≈ the same
        let high_conf = Triple::new("ent1", "attr1", serde_json::json!("val1"), wid)
            .with_confidence(0.9)
            .with_visibility(hkask_types::Visibility::Shared);
        let count = mem.consolidate(vec![high_conf]).unwrap();
        assert_eq!(count, 1);

        let results = mem.query("ent1").unwrap();
        // combine(0.9, 0.5) ≈ 0.9 (high confidence barely changes with 0.5 prior)
        let promoted = crate::bayesian::combine(0.9, 0.5);
        assert!((results[0].confidence - promoted).abs() < 0.01);
    }

    #[test]
    fn consolidate_promotes_low_confidence_with_prior() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Episodic triple with low confidence → combine(0.3, 0.5) ≈ 0.3
        let low_conf = Triple::new("ent2", "attr2", serde_json::json!("val2"), wid)
            .with_confidence(0.3)
            .with_visibility(hkask_types::Visibility::Shared);
        mem.consolidate(vec![low_conf]).unwrap();

        let results = mem.query("ent2").unwrap();
        let promoted = crate::bayesian::combine(0.3, 0.5);
        assert!((results[0].confidence - promoted).abs() < 0.01);
    }

    // ========================================================================
    // PR 6e: Semantic storage budget
    // ========================================================================

    #[test]
    fn semantic_storage_budget_check() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es).with_storage_budget(2);
        let wid = test_webid();

        // Under budget should succeed
        assert!(mem.check_budget("entity1", 1).is_ok());
        assert!(mem.check_budget("entity1", 2).is_ok());

        // Store one triple
        let t = Triple::new("entity1", "attr1", serde_json::json!("val1"), wid);
        mem.store(t).unwrap();

        // Now budget check for 2 more should fail (1 stored + 2 requested > 2 budget)
        assert!(mem.check_budget("entity1", 2).is_err());
    }

    #[test]
    fn semantic_storage_budget_exceeded_returns_error() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es).with_storage_budget(1);
        let wid = test_webid();

        let t = Triple::new("entity1", "attr1", serde_json::json!("val1"), wid);
        mem.store(t).unwrap();

        let result = mem.check_budget("entity1", 1);
        assert!(result.is_err());
        match result.unwrap_err() {
            SemanticMemoryError::BudgetExceeded { stored, budget } => {
                assert_eq!(stored, 1);
                assert_eq!(budget, 1);
            }
            other => panic!("Expected BudgetExceeded, got {other:?}"),
        }
    }

    #[test]
    fn semantic_storage_usage() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        assert_eq!(mem.storage_usage("entity1").unwrap(), 0);

        mem.store(Triple::new("entity1", "a", serde_json::json!("v1"), wid))
            .unwrap();
        assert_eq!(mem.storage_usage("entity1").unwrap(), 1);

        mem.store(Triple::new("entity1", "b", serde_json::json!("v2"), wid))
            .unwrap();
        assert_eq!(mem.storage_usage("entity1").unwrap(), 2);
    }

    #[test]
    fn semantic_retraction_candidates_returns_lowest_confidence() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        mem.store(Triple::new("e1", "a", serde_json::json!("v1"), wid).with_confidence(0.9))
            .unwrap();
        mem.store(Triple::new("e1", "b", serde_json::json!("v2"), wid).with_confidence(0.3))
            .unwrap();
        mem.store(Triple::new("e1", "c", serde_json::json!("v3"), wid).with_confidence(0.7))
            .unwrap();

        let candidates = mem.retraction_candidates("e1", 2).unwrap();
        assert_eq!(candidates.len(), 2);
        // Lowest confidence first
        assert!((candidates[0].confidence - 0.3).abs() < 0.01);
        assert!((candidates[1].confidence - 0.7).abs() < 0.01);
    }

    #[test]
    fn semantic_budget_default_and_builder() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        assert_eq!(mem.semantic_budget(), DEFAULT_SEMANTIC_BUDGET);

        let (ts2, es2) = test_db();
        let mem_custom = SemanticMemory::new(ts2, es2).with_storage_budget(50);
        assert_eq!(mem_custom.semantic_budget(), 50);
    }

    // --- recall_combined tests ---

    #[test]
    fn recall_combined_single_triple_unchanged() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        mem.store(Triple::new("e1", "attr1", serde_json::json!("val1"), wid).with_confidence(0.8))
            .unwrap();

        let results = mem.recall_combined("e1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "e1");
        assert_eq!(results[0].attribute, "attr1");
        assert!((results[0].confidence - 0.8).abs() < 0.001);
    }

    #[test]
    fn recall_combined_merges_same_entity_attribute() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        // Two triples with same entity/attribute but different values and confidences
        mem.store(Triple::new("e1", "color", serde_json::json!("red"), wid).with_confidence(0.7))
            .unwrap();
        mem.store(Triple::new("e1", "color", serde_json::json!("blue"), wid).with_confidence(0.6))
            .unwrap();

        let results = mem.recall_combined("e1").unwrap();
        // Should combine into one triple
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attribute, "color");
        // Value from highest-confidence triple ("red" at 0.7)
        assert_eq!(results[0].value, serde_json::json!("red"));
        // Combined confidence: combine(0.7, 0.6)
        let expected = crate::bayesian::combine(0.7, 0.6);
        assert!((results[0].confidence - expected).abs() < 0.001);
        // Semantic: no perspective
        assert!(results[0].perspective.is_none());
    }

    #[test]
    fn recall_combined_preserves_different_attributes() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        mem.store(Triple::new("e1", "color", serde_json::json!("red"), wid).with_confidence(0.7))
            .unwrap();
        mem.store(Triple::new("e1", "size", serde_json::json!("large"), wid).with_confidence(0.8))
            .unwrap();

        let results = mem.recall_combined("e1").unwrap();
        // Different attributes — no combining needed
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn recall_combined_three_triples_iterative() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        mem.store(Triple::new("e1", "type", serde_json::json!("mammal"), wid).with_confidence(0.6))
            .unwrap();
        mem.store(Triple::new("e1", "type", serde_json::json!("animal"), wid).with_confidence(0.7))
            .unwrap();
        mem.store(
            Triple::new("e1", "type", serde_json::json!("vertebrate"), wid).with_confidence(0.5),
        )
        .unwrap();

        let results = mem.recall_combined("e1").unwrap();
        assert_eq!(results.len(), 1);
        // Value from highest-confidence triple ("animal" at 0.7)
        assert_eq!(results[0].value, serde_json::json!("animal"));
        // Combined confidence: join([0.6, 0.7, 0.5])
        let expected = crate::bayesian::join(&[0.6, 0.7, 0.5]);
        assert!((results[0].confidence - expected).abs() < 0.001);
    }

    #[test]
    fn recall_combined_with_stats_counts_correctly() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);
        let wid = test_webid();

        mem.store(Triple::new("e1", "color", serde_json::json!("red"), wid).with_confidence(0.7))
            .unwrap();
        mem.store(Triple::new("e1", "color", serde_json::json!("blue"), wid).with_confidence(0.6))
            .unwrap();
        mem.store(Triple::new("e1", "size", serde_json::json!("large"), wid).with_confidence(0.8))
            .unwrap();

        let stats = mem.recall_combined_with_stats("e1").unwrap();
        assert_eq!(stats.original_count, 3);
        assert_eq!(stats.combined_count, 2);
        assert_eq!(stats.duplicates_merged, 1);
    }

    #[test]
    fn recall_combined_empty_entity() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);

        let results = mem.recall_combined("nonexistent").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn recall_combined_with_stats_empty() {
        let (ts, es) = test_db();
        let mem = SemanticMemory::new(ts, es);

        let stats = mem.recall_combined_with_stats("nonexistent").unwrap();
        assert_eq!(stats.original_count, 0);
        assert_eq!(stats.combined_count, 0);
        assert_eq!(stats.duplicates_merged, 0);
        assert!(stats.triples.is_empty());
    }
}
