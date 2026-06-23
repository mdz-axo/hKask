//! Consolidation Bridge — Episodic → Semantic (one-way, Curation-directed)
//!
//! When Curation directs consolidation, episodic triples are:
//! 1. Selected via `EpisodicMemory::consolidation_candidates()` (oldest, lowest effective confidence)
//! 2. Stripped of perspective (privacy boundary removal)
//! 3. Checked against existing semantic triples with same EAV:
//!    a. **Match found:** Bayesian combine confidences, update existing
//!    b. **No match:** Seed as new semantic triple
//! 4. Expired in episodic memory (valid_to set, soft-deleted) to free storage budget
//!
//! This is a ONE-WAY operation: Episodic → Semantic. No reverse flow.
//! Authority: Curation directs, Cybernetics regulates (budget enforcement).

use std::sync::Arc;

use crate::bayesian::combine_confidences;
use crate::episodic::EpisodicMemory;
use crate::semantic::SemanticMemory;
use hkask_capability::tokens::ConsolidationToken;
use hkask_ports::{ConsolidationOutcome, ConsolidationRequest};
use hkask_storage::{Triple, TripleID};
use hkask_types::WebID;

/// Consolidation Bridge — Episodic → Semantic
///
/// Curation-directed one-way operation. Called from `CurationLoop::act()`
/// when a `CuratorDirective::OverrideEnergyBudget` or equivalent consolidation
/// trigger fires.
pub struct ConsolidationBridge {
    episodic: Arc<EpisodicMemory>,
    semantic: Arc<SemanticMemory>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConsolidationError {
    #[error("Episodic memory error: {0}")]
    Episodic(String),
    #[error("Unauthorized consolidation token: issuer {0} is not the expected curator")]
    UnauthorizedToken(String),
}

#[derive(Debug, Clone)]
pub(crate) struct ConsolidationResult {
    pub consolidated_count: usize,
    pub deleted_count: usize,
    pub failed_count: usize,
}

impl ConsolidationBridge {
    /// Create a new ConsolidationBridge.
    ///
    /// expect: "The system bridges episodic experience into shared semantic memory"
    /// \[P3\] Motivating: Generative Space — bridges episodic experience into shared semantic memory
    /// \[P4\] Constraining: Clear Boundaries — links stores without bypassing their membranes
    /// pre:  episodic and semantic are initialized memory stores
    /// post: returns ConsolidationBridge linking the two stores
    pub fn new(episodic: Arc<EpisodicMemory>, semantic: Arc<SemanticMemory>) -> Self {
        Self { episodic, semantic }
    }

    /// Consolidate episodic triples into semantic memory (one-way).
    ///
    /// For each candidate:
    /// 1. Strip perspective (set to `None`) — removes privacy boundary
    /// 2. Check semantic memory for existing triple with same EAV hash:
    ///    a. **Match:** Bayesian combine episodic + semantic confidence,
    ///       update existing semantic triple (no duplicate insertion)
    ///    b. **No match:** Insert as new semantic triple
    /// 3. Expire episodic source (soft-delete via valid_to)
    pub(crate) fn consolidate_inner(
        &self,
        perspective: WebID,
        request: ConsolidationRequest,
    ) -> Result<ConsolidationResult, ConsolidationError> {
        let span = tracing::span!(target: "cns.consolidation", tracing::Level::INFO, "consolidate");
        let _enter = span.enter();

        let candidates = self
            .episodic
            .consolidation_candidates(perspective, request.limit)
            .map_err(|e| ConsolidationError::Episodic(e.to_string()))?;

        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            candidate_count = candidates.len(),
            limit = request.limit,
            "Starting consolidation (with Bayesian evidence pooling)"
        );

        let mut consolidated_count = 0usize;
        let mut combined_count = 0usize;
        let mut expired_count = 0usize;
        let mut failed_count = 0usize;

        for triple in &candidates {
            // 1. Check for existing semantic triple with same EAV
            if let Some(existing) = self.semantic.find_existing_by_eav(triple) {
                // 1a. BAYESIAN COMBINE: existing semantic fact + new episodic evidence
                let combined = combine_confidences(existing.confidence, triple.confidence);

                match self
                    .semantic
                    .update_confidence(&existing.id, triple.value.clone(), combined)
                {
                    Ok(()) => {
                        combined_count += 1;
                        consolidated_count += 1;

                        // Expire episodic source
                        if let Err(e) = self.episodic.expire_triple(&triple.id) {
                            tracing::warn!(
                                target: "cns.consolidation",
                                triple_id = %triple.id.as_uuid(),
                                error = %e,
                                "Failed to expire episodic triple after Bayesian combination"
                            );
                        } else {
                            expired_count += 1;
                        }

                        tracing::debug!(
                            target: "cns.consolidation",
                            entity = %triple.entity,
                            attribute = %triple.attribute,
                            episodic_confidence = %triple.confidence,
                            semantic_confidence = %existing.confidence,
                            combined_confidence = %combined,
                            "Bayesian combined: episodic + existing semantic evidence"
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        tracing::warn!(
                            target: "cns.consolidation",
                            entity = %triple.entity,
                            attribute = %triple.attribute,
                            error = %e,
                            "Failed to update semantic triple confidence via Bayesian combination"
                        );
                        continue;
                    }
                }
            } else {
                // 1b. NO MATCH: seed as new semantic triple
                let semantic_triple = Triple {
                    id: TripleID::new(),
                    entity: triple.entity.clone(),
                    attribute: triple.attribute.clone(),
                    value: triple.value.clone(),
                    temporal: triple.temporal.clone(),
                    confidence: triple.confidence,
                    access: triple.access.to_semantic(),
                    recalled_at: chrono::Utc::now(),
                };

                match self.semantic.store_consolidated(semantic_triple) {
                    Ok(()) => {
                        consolidated_count += 1;

                        // Expire episodic source
                        if let Err(e) = self.episodic.expire_triple(&triple.id) {
                            tracing::warn!(
                                target: "cns.consolidation",
                                triple_id = %triple.id.as_uuid(),
                                error = %e,
                                "Failed to expire episodic triple after consolidation"
                            );
                        } else {
                            expired_count += 1;
                        }

                        tracing::debug!(
                            target: "cns.consolidation",
                            entity = %triple.entity,
                            attribute = %triple.attribute,
                            inherited_confidence = %triple.confidence,
                            "New semantic triple seeded (no prior EAV match)"
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        tracing::warn!(
                            target: "cns.consolidation",
                            entity = %triple.entity,
                            attribute = %triple.attribute,
                            error = %e,
                            "Failed to store new semantic triple"
                        );
                        continue;
                    }
                }
            }
        }

        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            consolidated_count,
            combined_count,
            newly_seeded = consolidated_count - combined_count,
            expired_count,
            failed_count,
            "Consolidation complete (episodic sources expired)"
        );

        Ok(ConsolidationResult {
            consolidated_count,
            deleted_count: expired_count,
            failed_count,
        })
    }
}

impl ConsolidationBridge {
    /// Consolidate episodic triples into semantic memory (public API).
    ///
    /// Requires ConsolidationToken proving Cybernetics authority.
    ///
    /// expect: "The system bridges episodic experience into shared semantic memory"
    /// \[P3\] Motivating: Generative Space — promotes sovereign episodic triples to shared knowledge
    /// \[P1\] Constraining: User Sovereignty — strips perspective only under Curator authority
    /// \[P4\] Constraining: Clear Boundaries — requires ConsolidationToken from expected curator
    /// pre:  token.issuer() == expected curator WebID
    /// pre:  perspective is a valid WebID
    /// post: episodic triples stripped of perspective, stored in semantic memory
    /// post: consolidated episodic sources expired (soft-deleted)
    /// post: returns ConsolidationOutcome with counts
    /// post: returns Err if token is unauthorized
    pub fn consolidate(
        &self,
        token: &ConsolidationToken,
        perspective: &WebID,
        request: ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, String> {
        // Verify the token issuer matches the expected curator
        let expected_curator = hkask_types::id::WebID::from_persona(b"curator");
        if token.issuer() != &expected_curator {
            return Err(
                ConsolidationError::UnauthorizedToken(token.issuer().to_string()).to_string(),
            );
        }
        let result = self
            .consolidate_inner(
                *perspective,
                ConsolidationRequest {
                    limit: request.limit,
                    confidence_floor: None, // bridge doesn't handle cleanup
                    max_semantic_triples: None,
                },
            )
            .map_err(|e| e.to_string())?;
        Ok(ConsolidationOutcome {
            consolidated_count: result.consolidated_count,
            deleted_count: result.deleted_count, // episodic triples expired after consolidation
            failed_count: result.failed_count,
        })
    }

    /// Count consolidation candidates for a perspective.
    ///
    /// expect: "The system bridges episodic experience into shared semantic memory"
    /// \[P3\] Motivating: Generative Space — surfaces how much episodic content is ready for promotion
    /// \[P9\] Constraining: Homeostatic Self-Regulation — count-only query avoids loading full store
    /// pre:  perspective is a valid WebID
    /// post: returns count of triples in episodic storage for this perspective
    /// post: returns 0 on error (graceful degradation)
    pub fn consolidation_candidate_count(&self, perspective: &WebID) -> usize {
        // Use storage_usage (COUNT query) instead of loading all candidates
        // into memory just to count them.
        self.episodic.storage_usage(perspective).unwrap_or(0)
    }
}
