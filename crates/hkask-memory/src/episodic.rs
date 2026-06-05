//! Episodic memory pipeline — first-person experience
//!
//! Subloops (Loop 2a):
//! - 2a.1 Experience Encoding (FILTER) — filter and classify incoming experience
//! - 2a.2 Temporal Attention (ADAPT) — weight by recency: weight = e^(-λ × time_since_storage)
//! - 2a.3 Confidence Decay (RECONCILE) — confidence decreases over time via Bayesian decay
//! - 2a.4 Episodic Storage Budget (GUARD) — per-agent storage limit, mark oldest for consolidation
//! - 2a.5 Episodic Context Assembly (FILTER+ADAPT) — temporal-ordered, recency-weighted, budget-constrained

use crate::bayesian;
use crate::recall_dedup;
use chrono::Utc;
use hkask_storage::{Triple, TripleError, TripleStore};
use hkask_types::Visibility;
use hkask_types::WebID;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EpisodicMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
    #[error("Invalid visibility for episodic store: {0}")]
    InvalidVisibility(String),
    #[error("Episodic memory requires a perspective (agent WebID)")]
    MissingPerspective,
}

/// Default decay rate for episodic memory confidence.
///
/// Derived from a 3-month (90-day) half-life: λ = ln(2) / (90 × 86400) ≈ 8.913 × 10⁻⁸.
/// Time units are seconds (matching `valid_from` timestamps).
pub(crate) const DEFAULT_DECAY_RATE: f64 = crate::bayesian::DEFAULT_DECAY_RATE;

/// Default per-agent storage budget (max triples).
pub(crate) const DEFAULT_EPISODIC_BUDGET: usize = 10_000;

// =============================================================================
// EpisodicMemory — first-person experience with subloops
// =============================================================================

/// Episodic memory — first-person experience
///
/// Provides the following subloops:
/// - **Confidence decay** (2a.3): Decays confidence based on time since
///   storage using `bayesian::decay()`. Applied at recall time, not persisted.
/// - **Temporal attention** (2a.2): Weights recalled triples by recency.
/// - **Storage budget** (2a.4): Per-agent storage limit with consolidation
///   candidate identification (uses decayed confidence for prioritization).
pub struct EpisodicMemory {
    triple_store: TripleStore,
    /// Decay rate for confidence (λ in e^(-λt)). Default derived from 30-day half-life.
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
        if triple.visibility == Visibility::Shared {
            return Err(EpisodicMemoryError::InvalidVisibility(
                "Episodic memory is sovereign — Shared triples belong in semantic memory"
                    .to_string(),
            ));
        }
        if triple.perspective.is_none() {
            return Err(EpisodicMemoryError::MissingPerspective);
        }
        self.triple_store.insert(&triple)?;
        Ok(())
    }

    // ========================================================================
    // Recall — basic queries
    // ========================================================================

    /// Query by entity for specific perspective with deduplication,
    /// confidence decay, and temporal attention applied (2a.3 + 2a.2).
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
                // Apply confidence decay (2a.3): e^(-λt)
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

        // Sort by recency (most recent first) — temporal attention (2a.2)
        filtered.sort_by(|a, b| b.valid_from.cmp(&a.valid_from));

        Ok(recall_dedup::dedup_triples(filtered))
    }

    // ========================================================================
    // Query — all episodic memories
    // ========================================================================

    // ========================================================================
    // Storage Budget (2a.5) — Cybernetics membrane operations
    // ========================================================================

    /// Get the current storage usage for a perspective (number of triples).
    ///
    /// Uses a COUNT query instead of loading all triples into memory.
    pub fn storage_usage(&self, perspective: &WebID) -> Result<usize, EpisodicMemoryError> {
        let count = self.triple_store.count_by_perspective(perspective)?;
        Ok(count)
    }

    /// Identify triples eligible for consolidation (oldest, lowest effective confidence)
    /// when budget is exceeded (2a.4).
    ///
    /// Uses recall-time decayed confidence (not stored confidence) so that
    /// old triples with high stored confidence but low effective confidence
    /// are correctly prioritized for consolidation.
    ///
    /// **Membrane-sealed:** Only callable from within this crate.
    pub(crate) fn consolidation_candidates(
        &self,
        perspective: WebID,
        limit: usize,
    ) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let mut triples = self.triple_store.query_by_perspective(&perspective)?;
        let now = Utc::now();

        // Sort by decayed confidence ascending, then by valid_from ascending (oldest first)
        triples.sort_by(|a, b| {
            let a_effective = bayesian::decay(
                a.confidence,
                self.decay_rate,
                (now - a.valid_from).num_seconds() as f64,
            );
            let b_effective = bayesian::decay(
                b.confidence,
                self.decay_rate,
                (now - b.valid_from).num_seconds() as f64,
            );
            a_effective
                .partial_cmp(&b_effective)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.valid_from.cmp(&b.valid_from))
        });

        triples.truncate(limit);
        Ok(triples)
    }

    /// Expire a triple by setting its `valid_to` timestamp (soft-delete).
    ///
    /// Used by consolidation to mark episodic triples as expired after
    /// they have been promoted to semantic memory. The triple remains in
    /// the store for audit but is excluded from all current queries.
    ///
    /// **Membrane-sealed:** Only callable from within this crate.
    pub(crate) fn expire_triple(
        &self,
        id: &hkask_storage::TripleID,
    ) -> Result<(), EpisodicMemoryError> {
        self.triple_store.close_by_id(id)?;
        tracing::debug!(
            target: "cns.episodic",
            triple_id = %id.0,
            "Episodic triple expired (consolidated to semantic memory)"
        );
        Ok(())
    }

    /// Get the configured storage budget.
    pub fn storage_budget(&self) -> usize {
        self.storage_budget
    }

    /// Count consolidation candidates for a perspective.
    ///
    /// Returns the number of episodic triples eligible for consolidation
    /// (sorted by decayed confidence, oldest/lowest first). This is the
    /// count-only version of `consolidation_candidates` — safe to expose
    /// publicly because it doesn't return triple data.
    pub fn consolidation_candidate_count(&self, perspective: &WebID) -> usize {
        match self.consolidation_candidates(*perspective, usize::MAX) {
            Ok(candidates) => candidates.len(),
            Err(_) => 0,
        }
    }

    /// Get the configured decay rate.
    ///
    /// **Membrane-sealed:** Only callable from within this crate.
    pub(crate) fn decay_rate(&self) -> f64 {
        self.decay_rate
    }
}
