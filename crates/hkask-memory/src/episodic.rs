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
use crate::recall_dedup;
use chrono::Utc;
use hkask_storage::{Triple, TripleError, TripleStore};
use hkask_types::WebID;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EpisodicMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
    #[error("Triple not found for retraction: {entity}/{attribute}")]
    TripleNotFound { entity: String, attribute: String },
}

/// Default decay rate for episodic memory confidence.
///
/// A rate of 0.001 means confidence halves approximately every 693 time units
/// (half-life = ln(2)/rate ≈ 693 for rate 0.001).
/// Time units are seconds (matching `valid_from` timestamps).
pub(crate) const DEFAULT_DECAY_RATE: f64 = 0.001;

/// Default per-agent storage budget (max triples).
pub(crate) const DEFAULT_EPISODIC_BUDGET: usize = 10_000;

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
    /// Per-agent storage budget (max triples). Default: 10,000
    storage_budget: usize,
}

impl EpisodicMemory {
    pub fn new(triple_store: TripleStore) -> Self {
        Self {
            triple_store,
            decay_rate: DEFAULT_DECAY_RATE,
            storage_budget: DEFAULT_EPISODIC_BUDGET,
        }
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

    // ========================================================================
    // Retraction (5b / Loop 2a.4) — Cybernetics membrane operation
    // ========================================================================

    /// Retract a triple by reducing its confidence without deleting (5b).
    ///
    /// **Membrane-sealed:** Only callable from within this crate.
    /// External consumers should route retraction through `EpisodicLoop::act()`.
    pub(crate) fn retract_triple(
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

    // ========================================================================
    // Storage Budget (5d / Loop 2a.5) — Cybernetics membrane operations
    // ========================================================================

    /// Get the current storage usage for a perspective (number of triples).
    pub fn storage_usage(&self, perspective: &WebID) -> Result<usize, EpisodicMemoryError> {
        let count = self.triple_store.query_by_perspective(perspective)?.len();
        Ok(count)
    }

    /// Identify triples eligible for consolidation (oldest, lowest-confidence)
    /// when budget is exceeded (5d).
    ///
    /// **Membrane-sealed:** Only callable from within this crate.
    pub(crate) fn consolidation_candidates(
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
    ///
    /// **Membrane-sealed:** Only callable from within this crate.
    pub(crate) fn decay_rate(&self) -> f64 {
        self.decay_rate
    }
}
