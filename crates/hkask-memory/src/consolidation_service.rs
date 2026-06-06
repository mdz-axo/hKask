//! Consolidation Service — user-facing combined consolidation + cleanup
//!
//! Provides a single operation that:
//! 1. Promotes episodic triples to semantic memory (via ConsolidationBridge)
//! 2. Deletes semantic triples at or below a configurable confidence floor
//! 3. Enforces a maximum semantic triple count by deleting lowest-confidence

use std::sync::Arc;

use crate::consolidation::ConsolidationBridge;
use crate::semantic::SemanticMemory;
use hkask_types::WebID;
use hkask_types::capability::tokens::ConsolidationToken;
use hkask_types::ports::{ConsolidationOutcome, ConsolidationRequest};

/// Consolidation Service — user-facing combined consolidation + semantic cleanup.
///
/// Wraps a `ConsolidationBridge` and `SemanticMemory` to provide a single
/// operation that promotes episodic triples, deletes low-confidence semantic
/// entries, and enforces max triple counts.
///
/// The service requires a `ConsolidationToken` and a passphrase-verified
/// `WebID` to authorize the operation.
pub struct ConsolidationService {
    bridge: Arc<ConsolidationBridge>,
    semantic: Arc<SemanticMemory>,
    token: ConsolidationToken,
}

impl ConsolidationService {
    /// Create a new ConsolidationService.
    ///
    /// The token must be issued by the Curator (system-level authority).
    pub fn new(
        bridge: Arc<ConsolidationBridge>,
        semantic: Arc<SemanticMemory>,
        token: ConsolidationToken,
    ) -> Self {
        Self {
            bridge,
            semantic,
            token,
        }
    }

    /// Execute a user-triggered consolidation operation.
    ///
    /// Three phases:
    /// 1. **Consolidate** — promote episodic triples to semantic memory
    /// 2. **Confidence floor** — delete semantic triples at or below the
    ///    confidence floor (if specified)
    /// 3. **Max triples** — delete lowest-confidence semantic triples until
    ///    count is at or below `max_semantic_triples` (if specified)
    pub fn consolidate(
        &self,
        perspective: &WebID,
        request: ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, String> {
        let span = tracing::span!(target: "cns.consolidation", tracing::Level::INFO, "consolidate_service");
        let _enter = span.enter();

        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            limit = request.limit,
            confidence_floor = ?request.confidence_floor,
            max_semantic_triples = ?request.max_semantic_triples,
            "User-triggered consolidation starting"
        );

        // Phase 1: Consolidate episodic → semantic
        let bridge_outcome = self.bridge.consolidate(
            &self.token,
            perspective,
            ConsolidationRequest {
                limit: request.limit,
                confidence_floor: None, // bridge doesn't handle cleanup
                max_semantic_triples: None,
            },
        )?;

        let mut deleted_count = 0usize;

        // Phase 2: Delete semantic triples at or below confidence floor
        if let Some(floor) = request.confidence_floor {
            match self.semantic.low_confidence_triples(floor, usize::MAX) {
                Ok(candidates) if !candidates.is_empty() => {
                    tracing::info!(
                        target: "cns.consolidation",
                        floor = floor,
                        candidates = candidates.len(),
                        "Deleting semantic triples at or below confidence floor"
                    );
                    for triple in &candidates {
                        if let Err(e) = self.semantic.delete_triple(&triple.id) {
                            tracing::debug!(
                                target: "cns.consolidation",
                                triple_id = %triple.id,
                                error = %e,
                                "Failed to delete low-confidence semantic triple"
                            );
                        } else {
                            deleted_count += 1;
                        }
                    }
                }
                Ok(_) => {
                    tracing::debug!(
                        target: "cns.consolidation",
                        floor = floor,
                        "No semantic triples at or below confidence floor"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.consolidation",
                        error = %e,
                        "Failed to query low-confidence semantic triples"
                    );
                }
            }
        }

        // Phase 3: Enforce max semantic triple count
        if let Some(max) = request.max_semantic_triples {
            match self.semantic.triple_count() {
                Ok(count) if count > max => {
                    let overage = count - max;
                    match self.semantic.lowest_confidence_triples(overage) {
                        Ok(candidates) if !candidates.is_empty() => {
                            tracing::info!(
                                target: "cns.consolidation",
                                max = max,
                                current = count,
                                overage = overage,
                                "Deleting lowest-confidence semantic triples to enforce max"
                            );
                            for triple in &candidates {
                                if let Err(e) = self.semantic.delete_triple(&triple.id) {
                                    tracing::debug!(
                                        target: "cns.consolidation",
                                        triple_id = %triple.id,
                                        error = %e,
                                        "Failed to delete semantic triple for budget"
                                    );
                                } else {
                                    deleted_count += 1;
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!(
                                target: "cns.consolidation",
                                error = %e,
                                "Failed to query lowest-confidence semantic triples"
                            );
                        }
                    }
                }
                Ok(count) => {
                    tracing::debug!(
                        target: "cns.consolidation",
                        current = count,
                        max = max,
                        "Semantic triple count within max"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.consolidation",
                        error = %e,
                        "Failed to count semantic triples"
                    );
                }
            }
        }

        tracing::info!(
            target: "cns.consolidation",
            consolidated = bridge_outcome.consolidated_count,
            deleted = deleted_count,
            failed = bridge_outcome.failed_count,
            "User-triggered consolidation complete"
        );

        Ok(ConsolidationOutcome {
            consolidated_count: bridge_outcome.consolidated_count,
            deleted_count,
            failed_count: bridge_outcome.failed_count,
        })
    }

    /// Count episodic consolidation candidates for a perspective.
    pub fn consolidation_candidate_count(&self, perspective: &WebID) -> usize {
        self.bridge.consolidation_candidate_count(perspective)
    }

    /// Count semantic triples at or below a confidence threshold.
    pub fn semantic_low_confidence_count(&self, threshold: f64) -> usize {
        self.semantic.low_confidence_count(threshold).unwrap_or(0)
    }

    /// Get current semantic triple count.
    pub fn semantic_triple_count(&self) -> usize {
        self.semantic.triple_count().unwrap_or(0)
    }
}
