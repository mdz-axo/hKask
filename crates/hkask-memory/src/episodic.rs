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
    /// if the budget would be exceeded.
    ///
    /// **Superseded by EpisodicLoop::act()** — budget enforcement is now owned
    /// by the loop membrane (Cybernetics concern). This method remains for
    /// pre-write validation by callers that need a synchronous check before
    /// the loop's next tick, but the loop's `act()` is the authority that
    /// actually prunes triples when budget is exceeded.
    ///
    /// New code should prefer querying `storage_usage()` and letting the
    /// loop handle enforcement asynchronously.
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
