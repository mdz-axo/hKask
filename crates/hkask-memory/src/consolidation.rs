//! Consolidation Bridge — Episodic → Semantic (one-way, Curation-directed)
//!
//! When Curation directs consolidation, episodic triples are:
//! 1. Selected via `EpisodicMemory::consolidation_candidates()` (oldest, lowest effective confidence)
//! 2. Stripped of perspective (privacy boundary removal)
//! 3. Seeded into semantic memory with inherited confidence from the episodic source
//! 4. Expired in episodic memory (valid_to set, soft-deleted) to free storage budget
//!
//! This is a ONE-WAY operation: Episodic → Semantic. No reverse flow.
//! Authority: Curation directs, Cybernetics regulates (budget enforcement).


use std::sync::Arc;

use crate::episodic::EpisodicMemory;
use crate::semantic::SemanticMemory;
use hkask_storage::Triple;
use hkask_types::WebID;
use hkask_types::capability::tokens::ConsolidationToken;
use hkask_types::ports::{ConsolidationOutcome, ConsolidationRequest};

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
    /// \[P3\] Motivating: Generative Space — bridges episodic experience into shared semantic memory
    /// \[P4\] Constraining: Clear Boundaries — links stores without bypassing their membranes
    pub fn new(episodic: Arc<EpisodicMemory>, semantic: Arc<SemanticMemory>) -> Self {
        Self { episodic, semantic }
    }

    /// Consolidate episodic triples into semantic memory (one-way).
    ///
    /// For each candidate:
    /// 1. Strip perspective (set to `None`) — removes privacy boundary
    /// 2. Inherit confidence from the episodic source — the triple carries
    ///    its proven confidence into shared memory
    /// 3. Store in semantic memory
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
            "Starting consolidation"
        );

        let mut consolidated_count = 0usize;
        let mut expired_count = 0usize;
        let mut failed_count = 0usize;

        for triple in &candidates {
            // 1. Strip perspective, inherit confidence from episodic source
            let semantic_triple = Triple {
                id: hkask_storage::TripleID::new(),
                entity: triple.entity.clone(),
                attribute: triple.attribute.clone(),
                value: triple.value.clone(),
                temporal: triple.temporal.clone(),
                confidence: triple.confidence,
                access: triple.access.to_semantic(),
            };

            // 2. Store in semantic memory
            match self.semantic.store_consolidated(semantic_triple) {
                Ok(()) => {
                    consolidated_count += 1;

                    // 3. Expire the episodic source triple (soft-delete via valid_to)
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
                        "Triple consolidated into semantic memory, episodic source expired"
                    );
                }
                Err(e) => {
                    failed_count += 1;
                    tracing::warn!(
                        target: "cns.consolidation",
                        entity = %triple.entity,
                        attribute = %triple.attribute,
                        error = %e,
                        "Failed to store semantic triple"
                    );
                    continue;
                }
            }
        }

        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            consolidated_count,
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
    /// \[P3\] Motivating: Generative Space — promotes sovereign episodic triples to shared knowledge
    /// \[P1\] Constraining: User Sovereignty — strips perspective only under Curator authority
    /// \[P4\] Constraining: Clear Boundaries — requires ConsolidationToken from expected curator
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
    /// \[P3\] Motivating: Generative Space — surfaces how much episodic content is ready for promotion
    /// \[P9\] Constraining: Homeostatic Self-Regulation — count-only query avoids loading full store
    pub fn consolidation_candidate_count(&self, perspective: &WebID) -> usize {
        // Use storage_usage (COUNT query) instead of loading all candidates
        // into memory just to count them.
        self.episodic.storage_usage(perspective).unwrap_or(0)
    }
}
