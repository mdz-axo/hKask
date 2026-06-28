//! Consolidation Bridge — Episodic → Semantic (one-way)
//!
//! When currency pressure triggers consolidation, episodic triples are:
//! 1. Selected via `EpisodicMemory::consolidation_candidates()` (oldest, lowest effective confidence)
//! 2. Stripped of perspective (privacy boundary removal)
//! 3. Checked against existing semantic triples with same EAV:
//!    a. **Match found:** Bayesian combine confidences, update existing
//!    b. **No match:** Seed as new semantic triple
//! 4. Expired in episodic memory (valid_to set, soft-deleted) to free storage budget
//!
//! This is a ONE-WAY operation: Episodic → Semantic. No reverse flow.

use std::sync::Arc;

use crate::bayesian::combine_confidences;
use crate::episodic::EpisodicMemory;
use crate::semantic::SemanticMemory;
use hkask_ports::{ConsolidationOutcome, ConsolidationRequest};
use hkask_storage::{Triple, TripleID};
use hkask_types::WebID;

/// Consolidation Bridge — Episodic → Semantic
///
/// One-way operation called from `EpisodicLoop::act()` when budget pressure
/// requires freeing episodic storage.
pub struct ConsolidationBridge {
    episodic: Arc<EpisodicMemory>,
    semantic: Arc<SemanticMemory>,
}

impl ConsolidationBridge {
    pub fn new(episodic: Arc<EpisodicMemory>, semantic: Arc<SemanticMemory>) -> Self {
        Self { episodic, semantic }
    }

    /// Consolidate episodic triples into semantic memory (one-way).
    ///
    /// For each candidate:
    /// 1. Strip perspective (set to `None`) — removes privacy boundary
    /// 2. Check semantic memory for existing triple with same EAV hash:
    ///    a. **Match:** Bayesian combine episodic + semantic confidence,
    ///       update existing semantic triple
    ///    b. **No match:** Insert as new semantic triple
    /// 3. Expire episodic source (soft-delete via valid_to)
    #[allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]
    pub fn consolidate(
        &self,
        perspective: WebID,
        request: ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, String> {
        let span = tracing::span!(target: "cns.consolidation", tracing::Level::INFO, "consolidate");
        let _enter = span.enter();

        let candidates = self
            .episodic
            .consolidation_candidates(perspective, request.limit)
            .map_err(|e| format!("Episodic error: {e}"))?;

        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            candidate_count = candidates.len(),
            limit = request.limit,
            "Starting consolidation"
        );

        let mut consolidated_count = 0usize;
        let mut combined_count = 0usize;
        let mut expired_count = 0usize;
        let mut failed_count = 0usize;

        let now = chrono::Utc::now();
        for triple in &candidates {
            let days_since = (now - triple.recalled_at).num_seconds() as f64 / 86400.0;
            let episodic_c = triple
                .confidence
                .memory_decay(days_since, self.episodic.memory_life_days());

            if let Some(existing) = self.semantic.find_existing_by_eav(triple) {
                let combined = combine_confidences(existing.confidence, episodic_c);

                match self
                    .semantic
                    .update_confidence(&existing.id, triple.value.clone(), combined)
                {
                    Ok(()) => {
                        combined_count += 1;
                        consolidated_count += 1;
                        if let Err(e) = self.episodic.expire_triple(&triple.id) {
                            tracing::warn!(target: "cns.consolidation", triple_id = %triple.id.as_uuid(), error = %e, "Failed to expire episodic triple");
                        } else {
                            expired_count += 1;
                        }
                        tracing::debug!(
                            target: "cns.consolidation",
                            entity = %triple.entity, attribute = %triple.attribute,
                            stored = %triple.confidence, days_since_recall = days_since,
                            episodic = %episodic_c, semantic = %existing.confidence, combined = %combined,
                            "Bayesian combined"
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        tracing::warn!(target: "cns.consolidation", entity = %triple.entity, error = %e, "Failed to update semantic triple");
                        continue;
                    }
                }
            } else {
                let semantic_triple = Triple {
                    id: TripleID::new(),
                    entity: triple.entity.clone(),
                    attribute: triple.attribute.clone(),
                    value: triple.value.clone(),
                    temporal: triple.temporal.clone(),
                    confidence: episodic_c,
                    access: triple.access.to_semantic(),
                    recalled_at: now,
                };

                match self.semantic.store_consolidated(semantic_triple) {
                    Ok(()) => {
                        consolidated_count += 1;
                        if let Err(e) = self.episodic.expire_triple(&triple.id) {
                            tracing::warn!(target: "cns.consolidation", triple_id = %triple.id.as_uuid(), error = %e, "Failed to expire episodic triple");
                        } else {
                            expired_count += 1;
                        }
                        tracing::debug!(
                            target: "cns.consolidation",
                            entity = %triple.entity, attribute = %triple.attribute,
                            stored = %triple.confidence, days_since_recall = days_since,
                            episodic = %episodic_c,
                            "New semantic triple seeded"
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        tracing::warn!(target: "cns.consolidation", entity = %triple.entity, error = %e, "Failed to store new semantic triple");
                        continue;
                    }
                }
            }
        }

        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            consolidated_count, combined_count,
            newly_seeded = consolidated_count - combined_count,
            expired_count, failed_count,
            "Consolidation complete"
        );

        Ok(ConsolidationOutcome {
            consolidated_count,
            deleted_count: expired_count,
            failed_count,
        })
    }

    /// Count consolidation candidates for a perspective.
    ///
    /// Returns the number of episodic triples eligible for consolidation
    /// (sorted by decayed confidence, oldest/lowest first), not total storage usage.
    pub fn consolidation_candidate_count(&self, perspective: &WebID) -> usize {
        self.episodic.consolidation_candidate_count(perspective)
    }
}
