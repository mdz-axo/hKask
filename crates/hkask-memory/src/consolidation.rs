//! Consolidation Bridge — Episodic → Semantic (one-way, Curation-directed)
//!
//! When Curation directs consolidation, episodic triples are:
//! 1. Selected via `EpisodicMemory::consolidation_candidates()` (oldest, lowest-confidence)
//! 2. Stripped of perspective (privacy boundary removal)
//! 3. Seeded into semantic memory with confidence = 0.5 (Bayesian seeding baseline)
//! 4. Retracted from episodic memory (confidence halved, not deleted)
//!
//! This is a ONE-WAY operation: Episodic → Semantic. No reverse flow.
//! Authority: Curation directs, Cybernetics regulates (budget enforcement).

use std::sync::Arc;

use crate::episodic::EpisodicMemory;
use crate::semantic::SemanticMemory;
use hkask_storage::Triple;
use hkask_types::WebID;
use hkask_types::capability::tokens::ConsolidationToken;
use hkask_types::ports::{ConsolidationOutcome, ConsolidationPort};

/// Consolidation Bridge — Episodic → Semantic
///
/// Curation-directed one-way operation. Called from `CurationLoop::act()`
/// when a `CuratorDirective::OverrideGasBudget` or equivalent consolidation
/// trigger fires.
pub struct ConsolidationBridge {
    episodic: Arc<EpisodicMemory>,
    semantic: Arc<SemanticMemory>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConsolidationError {
    #[error("Episodic memory error: {0}")]
    Episodic(String),
    #[allow(dead_code)] // Symmetric counterpart to Episodic; needed for semantic consolidation failure path
    #[error("Semantic memory error: {0}")]
    Semantic(String),
}

#[derive(Debug, Clone)]
pub(crate) struct ConsolidationResult {
    pub consolidated_count: usize,
    pub retracted_count: usize,
    pub failed_count: usize,
}

impl ConsolidationBridge {
    pub fn new(episodic: Arc<EpisodicMemory>, semantic: Arc<SemanticMemory>) -> Self {
        Self { episodic, semantic }
    }

    /// Consolidate episodic triples into semantic memory (one-way).
    ///
    /// For each candidate:
    /// 1. Strip perspective (set to `None`) — removes privacy boundary
    /// 2. Set confidence to 0.5 (Bayesian seeding baseline)
    /// 3. Store in semantic memory
    /// 4. Retract from episodic memory (confidence halved, not deleted)
    pub(crate) fn consolidate(
        &self,
        perspective: WebID,
        limit: usize,
    ) -> Result<ConsolidationResult, ConsolidationError> {
        let span = tracing::span!(target: "cns.consolidation", tracing::Level::INFO, "consolidate");
        let _enter = span.enter();

        let candidates = self
            .episodic
            .consolidation_candidates(perspective, limit)
            .map_err(|e| ConsolidationError::Episodic(e.to_string()))?;

        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            candidate_count = candidates.len(),
            limit,
            "Starting consolidation"
        );

        let mut consolidated_count = 0usize;
        let mut retracted_count = 0usize;
        let mut failed_count = 0usize;

        for triple in &candidates {
            // 1. Strip perspective, set confidence to Bayesian seeding baseline
            let semantic_triple = Triple {
                id: hkask_storage::TripleID::new(),
                entity: triple.entity.clone(),
                attribute: triple.attribute.clone(),
                value: triple.value.clone(),
                valid_from: triple.valid_from,
                valid_to: triple.valid_to,
                confidence: 0.5,
                perspective: None,
                visibility: hkask_types::Visibility::Shared,
                owner_webid: triple.owner_webid,
            };

            // 2. Store in semantic memory
            match self.semantic.store_consolidated(semantic_triple) {
                Ok(()) => {
                    consolidated_count += 1;
                    tracing::debug!(
                        target: "cns.consolidation",
                        entity = %triple.entity,
                        attribute = %triple.attribute,
                        original_confidence = triple.confidence,
                        seeded_confidence = 0.5,
                        "Triple seeded into semantic memory"
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

            // 3. Retract from episodic memory (confidence halved, not deleted)
            match self
                .episodic
                .retract_triple(&triple.entity, &triple.attribute, 0.5, perspective)
            {
                Ok(_new_confidence) => {
                    retracted_count += 1;
                    tracing::debug!(
                        target: "cns.consolidation",
                        entity = %triple.entity,
                        attribute = %triple.attribute,
                        retraction = 0.5,
                        "Episodic triple retracted (confidence halved)"
                    );
                }
                Err(e) => {
                    failed_count += 1;
                    tracing::warn!(
                        target: "cns.consolidation",
                        entity = %triple.entity,
                        attribute = %triple.attribute,
                        error = %e,
                        "Failed to retract episodic triple"
                    );
                }
            }
        }

        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            consolidated_count,
            retracted_count,
            failed_count,
            "Consolidation complete"
        );

        Ok(ConsolidationResult {
            consolidated_count,
            retracted_count,
            failed_count,
        })
    }
}

impl ConsolidationPort for ConsolidationBridge {
    fn consolidate(
        &self,
        token: &ConsolidationToken,
        perspective: &WebID,
        limit: usize,
    ) -> Result<ConsolidationOutcome, String> {
        let _token = token; // Capability gate: token proves Cybernetics authority
        let result = ConsolidationBridge::consolidate(self, *perspective, limit)
            .map_err(|e| e.to_string())?;
        Ok(ConsolidationOutcome {
            consolidated_count: result.consolidated_count,
            retracted_count: result.retracted_count,
            failed_count: result.failed_count,
        })
    }

    fn consolidation_candidate_count(&self, perspective: &WebID) -> usize {
        self.episodic
            .consolidation_candidates(*perspective, usize::MAX)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}
