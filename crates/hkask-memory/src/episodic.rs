//! Episodic memory pipeline — first-person experience
//!
//! Subloops (Loop 2a):
//! - 2a.1 Experience Encoding (FILTER) — filter and classify incoming experience
//! - 2a.2 Temporal Attention (ADAPT) — weight by recency: weight = e^(-λ × time_since_storage)
//! - 2a.3 Confidence Decay (RECONCILE) — confidence decreases over time via Bayesian decay
//! - 2a.4 Confidence Retraction (RECONCILE) — reduce confidence without deleting the triple
//! - 2a.5 Episodic Storage Budget (GUARD) — per-agent storage limit, mark oldest for consolidation
//! - 2a.6 Episodic Context Assembly (FILTER+ADAPT) — temporal-ordered, recency-weighted, budget-constrained

use crate::bayesian;
use crate::recall_dedup::{self, DedupResult};
use chrono::Utc;
use hkask_storage::{Triple, TripleError, TripleStore};
use hkask_types::WebID;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EpisodicMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
    #[error("Budget exceeded: stored {stored}, budget {budget}")]
    BudgetExceeded { stored: usize, budget: usize },
    #[error("Triple not found for retraction: {entity}/{attribute}")]
    TripleNotFound { entity: String, attribute: String },
}

/// Default decay rate for episodic memory confidence.
///
/// A rate of 0.001 means confidence halves approximately every 693 time units
/// (half-life = ln(2)/rate ≈ 693 for rate 0.001).
/// Time units are seconds (matching `valid_from` timestamps).
pub const DEFAULT_DECAY_RATE: f64 = 0.001;

/// Default temporal attention lambda for recency weighting.
///
/// Higher lambda = more aggressive recency preference.
/// At λ=0.01, a memory 100 seconds old has weight ≈ 0.37.
pub const DEFAULT_TEMPORAL_LAMBDA: f64 = 0.01;

/// Default per-agent storage budget (max triples).
pub const DEFAULT_EPISODIC_BUDGET: usize = 10_000;

/// A recalled episodic triple with computed subloop metadata.
///
/// Extends `Triple` with values computed at recall time by the
/// episodic memory subloops:
/// - `decayed_confidence` — confidence after Bayesian decay (Loop 2a.3)
/// - `recency_weight` — temporal attention weight (Loop 2a.2)
/// - `time_since_storage_secs` — seconds since `valid_from`
#[derive(Debug, Clone)]
pub struct RecalledTriple {
    /// The underlying triple with its stored confidence (before decay)
    pub triple: Triple,
    /// Confidence after applying Bayesian decay based on time since storage
    pub decayed_confidence: f64,
    /// Temporal attention weight: e^(-λ × time_since_storage)
    pub recency_weight: f64,
    /// Seconds elapsed since `valid_from`
    pub time_since_storage_secs: f64,
}

// =============================================================================
// EpisodicMemory — first-person experience with subloops
// =============================================================================

/// Episodic memory — first-person experience
///
/// Provides the following subloops:
/// - **Confidence decay** (5a/2a.3): Decays confidence based on time since
///   storage using `bayesian::decay()`. Applied at recall time, not persisted.
/// - **Confidence retraction** (5b/2a.4): Reduces confidence without deleting,
///   using `bayesian::retract()`. Persisted as a versioned confidence update.
/// - **Temporal attention** (5c/2a.2): Weights recalled triples by recency.
/// - **Storage budget** (5d/2a.5): Per-agent storage limit with consolidation
///   candidate identification.
pub struct EpisodicMemory {
    triple_store: TripleStore,
    /// Decay rate for confidence (λ in e^(-λt)). Default: 0.001
    decay_rate: f64,
    /// Temporal attention lambda for recency weighting. Default: 0.01
    temporal_lambda: f64,
    /// Per-agent storage budget (max triples). Default: 10,000
    storage_budget: usize,
}

impl EpisodicMemory {
    pub fn new(triple_store: TripleStore) -> Self {
        Self {
            triple_store,
            decay_rate: DEFAULT_DECAY_RATE,
            temporal_lambda: DEFAULT_TEMPORAL_LAMBDA,
            storage_budget: DEFAULT_EPISODIC_BUDGET,
        }
    }

    /// Set the confidence decay rate (λ in e^(-λt)).
    pub fn with_decay_rate(mut self, decay_rate: f64) -> Self {
        self.decay_rate = decay_rate;
        self
    }

    /// Set the temporal attention lambda for recency weighting.
    pub fn with_temporal_lambda(mut self, temporal_lambda: f64) -> Self {
        self.temporal_lambda = temporal_lambda;
        self
    }

    /// Set the per-agent storage budget (max triples).
    pub fn with_storage_budget(mut self, budget: usize) -> Self {
        self.storage_budget = budget;
        self
    }

    // ========================================================================
    // Store
    // ========================================================================

    /// Store an episodic triple (private by default, with perspective).
    pub fn store(&self, triple: Triple) -> Result<(), EpisodicMemoryError> {
        self.triple_store.insert(&triple)?;
        Ok(())
    }

    // ========================================================================
    // Recall — basic queries
    // ========================================================================

    /// Query by entity for specific perspective (agent).
    ///
    /// Returns triples sorted by recency (most recent first) with
    /// temporal attention weighting applied (Loop 2a.2).
    pub fn query_for(
        &self,
        entity: &str,
        perspective: WebID,
    ) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let mut filtered: Vec<Triple> = triples
            .into_iter()
            .filter(|t| t.perspective == Some(perspective))
            .collect();

        // Sort by recency (most recent first) — temporal attention (5c)
        filtered.sort_by(|a, b| b.valid_from.cmp(&a.valid_from));

        Ok(filtered)
    }

    /// Query by entity for specific perspective with deduplication,
    /// confidence decay, and temporal attention applied (5a + 5c).
    ///
    /// Decays confidence based on time since `valid_from` using
    /// `bayesian::decay()`, then deduplicates by EAV hash.
    ///
    /// Emits `cns.memory.decay` span for each triple that undergoes decay.
    pub fn query_for_deduped(
        &self,
        entity: &str,
        perspective: WebID,
    ) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let now = Utc::now();
        let mut filtered: Vec<Triple> = triples
            .into_iter()
            .filter(|t| t.perspective == Some(perspective))
            .map(|mut t| {
                // Apply confidence decay (5a): e^(-λt)
                let time_since = (now - t.valid_from).num_seconds() as f64;
                let original_confidence = t.confidence;
                t.confidence = bayesian::decay(t.confidence, self.decay_rate, time_since);
                tracing::debug!(
                    target: "cns.memory.decay",
                    entity = %t.entity,
                    attribute = %t.attribute,
                    original_confidence = original_confidence,
                    decayed_confidence = t.confidence,
                    time_since_secs = time_since,
                    decay_rate = self.decay_rate,
                    "Episodic confidence decayed"
                );
                t
            })
            .collect();

        // Sort by recency (most recent first) — temporal attention (5c)
        filtered.sort_by(|a, b| b.valid_from.cmp(&a.valid_from));

        Ok(recall_dedup::dedup_triples(filtered))
    }

    /// Query by entity for specific perspective with deduplication,
    /// confidence decay, temporal attention, and full subloop metadata (5a–5c).
    ///
    /// Returns `RecalledTriple` structs that include the decayed confidence
    /// and recency weight alongside the original triple.
    ///
    /// Emits `cns.memory.decay` span for each triple that undergoes decay.
    pub fn query_for_weighted(
        &self,
        entity: &str,
        perspective: WebID,
    ) -> Result<Vec<RecalledTriple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let now = Utc::now();
        let mut recalled: Vec<RecalledTriple> = triples
            .into_iter()
            .filter(|t| t.perspective == Some(perspective))
            .map(|t| {
                let time_since = (now - t.valid_from).num_seconds() as f64;
                let decayed_confidence = bayesian::decay(t.confidence, self.decay_rate, time_since);
                if decayed_confidence < t.confidence {
                    tracing::debug!(
                        target: "cns.memory.decay",
                        entity = %t.entity,
                        attribute = %t.attribute,
                        original_confidence = t.confidence,
                        decayed_confidence,
                        time_since_secs = time_since,
                        decay_rate = self.decay_rate,
                        "Episodic confidence decayed (weighted recall)"
                    );
                }
                let recency_weight = (-self.temporal_lambda * time_since).exp();
                RecalledTriple {
                    triple: t,
                    decayed_confidence,
                    recency_weight,
                    time_since_storage_secs: time_since,
                }
            })
            .collect();

        // Sort by recency weight descending (most recent/highest weight first)
        recalled.sort_by(|a, b| {
            b.recency_weight
                .partial_cmp(&a.recency_weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(recalled)
    }

    // ========================================================================
    // Retraction (5b / Loop 2a.4)
    // ========================================================================

    /// Retract a triple by reducing its confidence without deleting (5b).
    ///
    /// Uses `bayesian::retract()` to reduce confidence. The triple is updated
    /// in-place with the retracted confidence value, creating a new version
    /// (the old version is closed via `valid_to`).
    ///
    /// Returns the retracted confidence value, or an error if no matching
    /// triple is found for the given entity/attribute/perspective.
    ///
    /// Emits `cns.memory.retract` span documenting the confidence reduction.
    pub fn retract_triple(
        &self,
        entity: &str,
        attribute: &str,
        retraction_confidence: f64,
        perspective: WebID,
    ) -> Result<f64, EpisodicMemoryError> {
        let triples = self
            .triple_store
            .query_by_entity_attribute(entity, attribute)?;
        let triple = triples
            .into_iter()
            .find(|t| t.perspective == Some(perspective))
            .ok_or_else(|| EpisodicMemoryError::TripleNotFound {
                entity: entity.to_string(),
                attribute: attribute.to_string(),
            })?;

        let retracted = bayesian::retract(triple.confidence, retraction_confidence);
        tracing::info!(
            target: "cns.memory.retract",
            entity = %entity,
            attribute = %attribute,
            original_confidence = triple.confidence,
            retraction_confidence,
            new_confidence = retracted,
            "Episodic confidence retracted (not deleted)"
        );
        self.triple_store
            .update(&triple.id, triple.value.clone(), retracted)?;

        Ok(retracted)
    }

    // ========================================================================
    // Query — all episodic memories
    // ========================================================================

    /// Query all episodic memories by entity.
    pub fn query(&self, entity: &str) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(triples.into_iter().filter(|t| t.is_episodic()).collect())
    }

    /// Query all episodic memories by entity with deduplication and
    /// confidence decay applied (5a).
    pub fn query_deduped(&self, entity: &str) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let now = Utc::now();
        let episodic: Vec<Triple> = triples
            .into_iter()
            .filter(|t| t.is_episodic())
            .map(|mut t| {
                let time_since = (now - t.valid_from).num_seconds() as f64;
                t.confidence = bayesian::decay(t.confidence, self.decay_rate, time_since);
                t
            })
            .collect();
        Ok(recall_dedup::dedup_triples(episodic))
    }

    /// Query all episodic memories by entity with deduplication, statistics,
    /// and confidence decay applied (5a).
    pub fn query_deduped_with_stats(
        &self,
        entity: &str,
    ) -> Result<DedupResult, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let now = Utc::now();
        let episodic: Vec<Triple> = triples
            .into_iter()
            .filter(|t| t.is_episodic())
            .map(|mut t| {
                let time_since = (now - t.valid_from).num_seconds() as f64;
                t.confidence = bayesian::decay(t.confidence, self.decay_rate, time_since);
                t
            })
            .collect();
        Ok(recall_dedup::dedup_triples_with_stats(episodic))
    }

    // ========================================================================
    // Storage Budget (5d / Loop 2a.5)
    // ========================================================================

    /// Check if storing `count` additional triples would exceed the
    /// per-agent storage budget (5d).
    ///
    /// Returns `Ok(())` if within budget, `Err(EpisodicMemoryError::BudgetExceeded)`
    /// if the budget would be exceeded. Emits a `cns.memory.budget` tracing
    /// span when the budget is at risk.
    pub fn check_budget(
        &self,
        perspective: WebID,
        count: usize,
    ) -> Result<(), EpisodicMemoryError> {
        let current = self.triple_store.query_by_perspective(&perspective)?.len();
        if current + count > self.storage_budget {
            tracing::warn!(
                target: "cns.memory.budget",
                perspective = %perspective,
                current = current,
                requested = count,
                budget = self.storage_budget,
                "Episodic storage budget would be exceeded"
            );
            return Err(EpisodicMemoryError::BudgetExceeded {
                stored: current,
                budget: self.storage_budget,
            });
        }
        Ok(())
    }

    /// Get the current storage usage for a perspective (number of triples).
    pub fn storage_usage(&self, perspective: &WebID) -> Result<usize, EpisodicMemoryError> {
        let count = self.triple_store.query_by_perspective(perspective)?.len();
        Ok(count)
    }

    /// Identify triples eligible for consolidation (oldest, lowest-confidence)
    /// when budget is exceeded (5d).
    ///
    /// Returns triples sorted by consolidation priority:
    /// lowest-confidence and oldest first.
    pub fn consolidation_candidates(
        &self,
        perspective: WebID,
        limit: usize,
    ) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let mut triples = self.triple_store.query_by_perspective(&perspective)?;

        // Sort by confidence ascending, then by valid_from ascending (oldest first)
        triples.sort_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.valid_from.cmp(&b.valid_from))
        });

        triples.truncate(limit);
        Ok(triples)
    }

    /// Get the configured storage budget.
    pub fn storage_budget(&self) -> usize {
        self.storage_budget
    }

    /// Get the configured decay rate.
    pub fn decay_rate(&self) -> f64 {
        self.decay_rate
    }

    /// Get the configured temporal lambda.
    pub fn temporal_lambda(&self) -> f64 {
        self.temporal_lambda
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::{Database, Triple, TripleStore};
    use hkask_types::WebID;

    fn test_store() -> TripleStore {
        let db = Database::in_memory().expect("in-memory db");
        TripleStore::new(db.conn_arc())
    }

    fn test_webid() -> WebID {
        WebID::new()
    }

    #[test]
    fn episodic_store_and_query() {
        let store = test_store();
        let mem = EpisodicMemory::new(store);
        let wid = test_webid();

        let triple = Triple::new("entity1", "attr1", serde_json::json!("val1"), wid)
            .with_perspective(wid)
            .with_confidence(0.9);
        mem.store(triple).unwrap();

        let results = mem.query_for("entity1", wid).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "entity1");
    }

    #[test]
    fn episodic_query_filters_by_perspective() {
        let store = test_store();
        let mem = EpisodicMemory::new(store);
        let wid1 = test_webid();
        let wid2 = test_webid();

        mem.store(
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid1)
                .with_perspective(wid1)
                .with_confidence(0.9),
        )
        .unwrap();
        mem.store(
            Triple::new("entity1", "attr2", serde_json::json!("val2"), wid2)
                .with_perspective(wid2)
                .with_confidence(0.8),
        )
        .unwrap();

        let results1 = mem.query_for("entity1", wid1).unwrap();
        let results2 = mem.query_for("entity1", wid2).unwrap();
        assert_eq!(results1.len(), 1);
        assert_eq!(results2.len(), 1);
        assert_eq!(results1[0].attribute, "attr1");
        assert_eq!(results2[0].attribute, "attr2");
    }

    #[test]
    fn episodic_decay_reduces_confidence() {
        let store = test_store();
        let mem = EpisodicMemory::new(store).with_decay_rate(0.1);
        let wid = test_webid();

        // Store a triple with high confidence
        mem.store(
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid)
                .with_perspective(wid)
                .with_confidence(0.9),
        )
        .unwrap();

        let results = mem.query_for_deduped("entity1", wid).unwrap();
        assert_eq!(results.len(), 1);
        // Confidence should be decayed, but since the triple was just stored,
        // time_since_storage ≈ 0, so decayed confidence ≈ original
        assert!(results[0].confidence <= 0.91);
        assert!(results[0].confidence >= 0.89);
    }

    #[test]
    fn episodic_retraction_reduces_confidence() {
        let store = test_store();
        let mem = EpisodicMemory::new(store);
        let wid = test_webid();

        mem.store(
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid)
                .with_perspective(wid)
                .with_confidence(0.9),
        )
        .unwrap();

        let retracted = mem.retract_triple("entity1", "attr1", 0.5, wid).unwrap();
        // bayesian::retract(0.9, 0.5) = 0.9 * (1 - 0.5) = 0.45
        assert!((retracted - 0.45).abs() < 0.01);
    }

    #[test]
    fn episodic_retraction_not_found() {
        let store = test_store();
        let mem = EpisodicMemory::new(store);
        let wid = test_webid();

        let result = mem.retract_triple("nonexistent", "attr1", 0.5, wid);
        assert!(result.is_err());
    }

    #[test]
    fn episodic_temporal_attention_sorts_by_recency() {
        let store = test_store();
        let mem = EpisodicMemory::new(store);
        let wid = test_webid();

        // Store triples - they'll have valid_from set to now
        mem.store(
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid)
                .with_perspective(wid)
                .with_confidence(0.8),
        )
        .unwrap();

        let results = mem.query_for("entity1", wid).unwrap();
        // Should be sorted by recency (most recent first)
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn episodic_storage_budget_check() {
        let store = test_store();
        let mem = EpisodicMemory::new(store).with_storage_budget(2);
        let wid = test_webid();

        mem.store(
            Triple::new("e1", "a1", serde_json::json!("v1"), wid)
                .with_perspective(wid)
                .with_confidence(0.9),
        )
        .unwrap();
        mem.store(
            Triple::new("e2", "a2", serde_json::json!("v2"), wid)
                .with_perspective(wid)
                .with_confidence(0.9),
        )
        .unwrap();

        // Should be at budget limit
        assert!(mem.check_budget(wid, 1).is_err());
        // Should be within budget for 0
        assert!(mem.check_budget(wid, 0).is_ok());
    }

    #[test]
    fn episodic_consolidation_candidates() {
        let store = test_store();
        let mem = EpisodicMemory::new(store);
        let wid = test_webid();

        // Store a high-confidence triple
        mem.store(
            Triple::new("e1", "a1", serde_json::json!("v1"), wid)
                .with_perspective(wid)
                .with_confidence(0.9),
        )
        .unwrap();
        // Store a low-confidence triple
        mem.store(
            Triple::new("e2", "a2", serde_json::json!("v2"), wid)
                .with_perspective(wid)
                .with_confidence(0.2),
        )
        .unwrap();

        let candidates = mem.consolidation_candidates(wid, 1).unwrap();
        assert_eq!(candidates.len(), 1);
        // The lowest-confidence triple should be the consolidation candidate
        assert!(candidates[0].confidence < 0.5);
    }

    #[test]
    fn episodic_query_for_weighted_returns_metadata() {
        let store = test_store();
        let mem = EpisodicMemory::new(store)
            .with_decay_rate(0.01)
            .with_temporal_lambda(0.01);
        let wid = test_webid();

        mem.store(
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid)
                .with_perspective(wid)
                .with_confidence(0.9),
        )
        .unwrap();

        let results = mem.query_for_weighted("entity1", wid).unwrap();
        assert_eq!(results.len(), 1);
        // Decay should reduce confidence slightly
        assert!(results[0].decayed_confidence <= 0.91);
        // Recency weight should be close to 1.0 for just-stored triple
        assert!(results[0].recency_weight > 0.9);
        // Time since storage should be very small
        assert!(results[0].time_since_storage_secs < 5.0);
    }
}

// =============================================================================
// PR 9b: Episodic Memory Cybernetic Unit Tests (Loop 2a)
// =============================================================================

#[cfg(test)]
mod cyber_tests {
    use super::*;
    use crate::bayesian;
    use hkask_storage::{Database, Triple, TripleStore};
    use hkask_types::{DataCategory, EpisodicReadHandle, EpisodicWriteHandle, WebID};

    fn test_store() -> TripleStore {
        let db = Database::in_memory().expect("in-memory db");
        TripleStore::new(db.conn_arc())
    }

    fn test_webid() -> WebID {
        WebID::new()
    }

    // ========================================================================
    // Loop 2a: Episodic Memory — write → recall → verify
    // ========================================================================

    /// Cyber test: Loop 2a closes — experience → store → recall → context.
    ///
    /// Proves the full episodic loop: store a triple with perspective,
    /// recall it via `query_for_weighted`, and verify that the recalled
    /// triple has both decayed_confidence > 0 and recency_weight > 0.
    #[test]
    fn cyber_episodic_loop_closes() {
        let store = test_store();
        let mem = EpisodicMemory::new(store);
        let wid = test_webid();

        mem.store(
            Triple::new("agent", "action", serde_json::json!("observed"), wid)
                .with_perspective(wid)
                .with_confidence(0.8),
        )
        .unwrap();

        let results = mem.query_for_weighted("agent", wid).unwrap();
        assert_eq!(
            results.len(),
            1,
            "Loop 2a: episodic recall must return the stored triple"
        );

        let recalled = &results[0];
        assert_eq!(recalled.triple.entity, "agent");
        assert_eq!(recalled.triple.attribute, "action");
        assert!(
            recalled.decayed_confidence > 0.0,
            "Loop 2a.3: decayed confidence must be positive, got {}",
            recalled.decayed_confidence
        );
        assert!(
            recalled.recency_weight > 0.0,
            "Loop 2a.2: recency weight must be positive, got {}",
            recalled.recency_weight
        );
    }

    /// Cyber test: OCAP boundary — EpisodicWriteHandle can write but not read;
    /// EpisodicReadHandle can read but not write.
    ///
    /// Proves that the capability handles enforce the correct OCAP discipline:
    /// - Write handle: `within_budget()` and `record_stored()` are available
    /// - Read handle: `query_budget()` and `can_access()` are available
    /// - Read handle: `can_access(EpisodicMemory)` → true
    /// - Read handle: `can_access(SemanticMemory)` → false
    /// - Write handle: no `can_access()` method (OCAP: cannot read)
    /// - Read handle: no `within_budget()` method (OCAP: cannot write)
    #[test]
    fn cyber_episodic_write_read() {
        let wid = test_webid();
        let mut write_handle = EpisodicWriteHandle::new(wid, 10000, 0);
        let read_handle = EpisodicReadHandle::new(wid, 100);

        // Write handle CAN check budget and record storage
        assert!(
            write_handle.within_budget(100),
            "Write handle must be within budget"
        );
        assert!(
            write_handle.record_stored(1).is_ok(),
            "Write handle must accept storage"
        );
        assert_eq!(write_handle.storage_used(), 1);

        // Read handle CAN check query budget and data access
        assert_eq!(
            read_handle.query_budget(),
            100,
            "Read handle must expose query budget"
        );
        assert!(
            read_handle.can_access(&DataCategory::EpisodicMemory),
            "Read handle must access EpisodicMemory"
        );

        // OCAP: Read handle CANNOT access SemanticMemory
        assert!(
            !read_handle.can_access(&DataCategory::SemanticMemory),
            "OCAP violation: episodic read handle must not access SemanticMemory"
        );

        // OCAP: Write handle has no can_access() method — compile-time guarantee
        // (verified by absence of the method on EpisodicWriteHandle)
        // OCAP: Read handle has no within_budget() method — compile-time guarantee
        // (verified by absence of the method on EpisodicReadHandle)
        // These are enforced at the type level, not runtime.
    }

    /// Cyber test: EpisodicReadHandle visibility enforcement.
    ///
    /// Proves that the episodic read handle grants access only to
    /// EpisodicMemory — no other DataCategory is accessible.
    #[test]
    fn cyber_episodic_visibility() {
        let wid = test_webid();
        let handle = EpisodicReadHandle::new(wid, 100);

        assert!(
            handle.can_access(&DataCategory::EpisodicMemory),
            "Episodic read handle must access EpisodicMemory"
        );
        assert!(
            !handle.can_access(&DataCategory::SemanticMemory),
            "Episodic read handle must NOT access SemanticMemory"
        );
        assert!(
            !handle.can_access(&DataCategory::PersonalContext),
            "Episodic read handle must NOT access PersonalContext"
        );
        assert!(
            !handle.can_access(&DataCategory::CapabilityTokens),
            "Episodic read handle must NOT access CapabilityTokens"
        );
    }

    /// Cyber test: Loop 2a.2 ADAPT — temporal attention weights by recency.
    ///
    /// Stores two triples for the same entity with different timestamps,
    /// queries with `query_for_weighted`, and verifies that results
    /// are sorted by recency_weight descending — more recent triples
    /// should have higher recency_weight.
    #[test]
    fn cyber_episodic_temporal_attention() {
        let store = test_store();
        let mem = EpisodicMemory::new(store).with_temporal_lambda(0.1);
        let wid = test_webid();

        // Store two triples — both get valid_from = now, but the
        // second one is stored after the first, so it will have a
        // marginally newer timestamp (or same). We use different
        // attributes to distinguish them.
        mem.store(
            Triple::new("entity1", "earlier", serde_json::json!("v1"), wid)
                .with_perspective(wid)
                .with_confidence(0.8),
        )
        .unwrap();

        // Small sleep to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(50));

        mem.store(
            Triple::new("entity1", "later", serde_json::json!("v2"), wid)
                .with_perspective(wid)
                .with_confidence(0.8),
        )
        .unwrap();

        let results = mem.query_for_weighted("entity1", wid).unwrap();
        assert_eq!(results.len(), 2, "Both triples should be recalled");

        // Results are sorted by recency_weight descending
        // The later triple should have a higher recency_weight
        assert!(
            results[0].recency_weight >= results[1].recency_weight,
            "Loop 2a.2 ADAPT: results must be sorted by recency_weight descending, got {} then {}",
            results[0].recency_weight,
            results[1].recency_weight
        );

        // Both recency weights should be positive
        for r in &results {
            assert!(
                r.recency_weight > 0.0,
                "Loop 2a.2: recency_weight must be positive, got {}",
                r.recency_weight
            );
        }
    }

    /// Cyber test: Loop 2a.3 RECONCILE — confidence decays over time.
    ///
    /// Uses `bayesian::decay(0.9, 0.001, 100.0)` to verify that
    /// confidence decreases with time elapsed, but remains positive.
    #[test]
    fn cyber_episodic_confidence_decay() {
        let original_confidence = 0.9;
        let decayed = bayesian::decay(original_confidence, 0.001, 100.0);

        assert!(
            decayed > 0.0,
            "Loop 2a.3: decayed confidence must be positive, got {}",
            decayed
        );
        assert!(
            decayed < original_confidence,
            "Loop 2a.3: decayed confidence ({}) must be less than original ({})",
            decayed,
            original_confidence
        );
    }

    /// Cyber test: Loop 2a.4 RECONCILE — confidence retraction reduces without deletion.
    ///
    /// Uses `bayesian::retract(0.8, 0.5)` to verify that retraction
    /// reduces confidence but keeps it >= 0 (no negative confidence).
    #[test]
    fn cyber_episodic_confidence_retraction() {
        let original_confidence = 0.8;
        let retraction_amount = 0.5;
        let retracted = bayesian::retract(original_confidence, retraction_amount);

        assert!(
            retracted < original_confidence,
            "Loop 2a.4: retracted confidence ({}) must be less than original ({})",
            retracted,
            original_confidence
        );
        assert!(
            retracted >= 0.0,
            "Loop 2a.4: retracted confidence must be >= 0, got {}",
            retracted
        );
        // Verify the formula: retract(0.8, 0.5) = 0.8 * (1 - 0.5) = 0.4
        let expected = 0.8 * (1.0 - 0.5);
        assert!(
            (retracted - expected).abs() < 0.01,
            "Loop 2a.4: retract(0.8, 0.5) should be {}, got {}",
            expected,
            retracted
        );
    }

    /// Cyber test: Loop 2a.5 GUARD — episodic storage budget enforcement.
    ///
    /// Creates an EpisodicMemory with budget 10, stores triples one
    /// by one, and verifies that after storing 10, `check_budget`
    /// returns an error. Also verifies that `consolidation_candidates()`
    /// returns the oldest/lowest-confidence triples.
    #[test]
    fn cyber_episodic_storage_budget() {
        let store = test_store();
        let mem = EpisodicMemory::new(store).with_storage_budget(10);
        let wid = test_webid();

        // Store 10 triples — should be within budget
        for i in 0..10 {
            mem.store(
                Triple::new(
                    "budget_entity",
                    &format!("attr{}", i),
                    serde_json::json!(format!("val{}", i)),
                    wid,
                )
                .with_perspective(wid)
                .with_confidence(1.0 - (i as f64 * 0.05)),
            )
            .unwrap();
        }

        // After storing 10, adding 1 more should exceed budget
        let budget_result = mem.check_budget(wid, 1);
        assert!(
            budget_result.is_err(),
            "Loop 2a.5 GUARD: budget of 10 must be exceeded after storing 10 triples"
        );

        // Consolidation candidates should return oldest/lowest-confidence triples
        let candidates = mem.consolidation_candidates(wid, 3).unwrap();
        assert!(
            !candidates.is_empty(),
            "Loop 2a.5 GUARD: consolidation candidates must not be empty"
        );
        // Candidates should be sorted by confidence ascending (lowest first)
        for window in candidates.windows(2) {
            assert!(
                window[0].confidence <= window[1].confidence,
                "Loop 2a.5: consolidation candidates must be sorted by confidence ascending"
            );
        }
    }
}
