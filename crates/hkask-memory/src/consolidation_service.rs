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
    ///
    /// REQ: P3-mem-consolidation-service-new
    /// \[P3\] Motivating: Generative Space — user-facing entry point for memory consolidation and cleanup
    /// \[P4\] Constraining: Clear Boundaries — requires Curator-issued ConsolidationToken
    /// pre:  bridge and semantic are initialized
    /// pre:  token.issuer() == expected curator
    /// post: returns ConsolidationService ready for consolidation operations
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
    ///
    /// REQ: P3-mem-consolidation-service-consolidate
    /// \[P3\] Motivating: Generative Space — combines episodic promotion with semantic cleanup
    /// \[P9\] Constraining: Homeostatic Self-Regulation — enforces confidence floor and max triple limits
    /// \[P4\] Constraining: Clear Boundaries — delegates to token-gated bridge
    /// pre:  perspective is a valid WebID
    /// pre:  request.limit > 0
    /// post: episodic triples consolidated into semantic memory
    /// post: low-confidence semantic triples deleted if confidence_floor set
    /// post: excess semantic triples deleted if max_semantic_triples set
    /// post: returns ConsolidationOutcome with counts
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
    ///
    /// REQ: P3-mem-consolidation-service-candidate-count
    /// \[P3\] Motivating: Generative Space — reports how many episodic triples can be promoted
    /// \[P9\] Constraining: Homeostatic Self-Regulation — count-only, graceful degradation on error
    /// pre:  perspective is a valid WebID
    /// post: returns count of episodic triples available for consolidation
    pub fn consolidation_candidate_count(&self, perspective: &WebID) -> usize {
        self.bridge.consolidation_candidate_count(perspective)
    }

    /// Count semantic triples at or below a confidence threshold.
    ///
    /// REQ: P3-mem-consolidation-service-low-confidence-count
    /// \[P3\] Motivating: Generative Space — reports low-confidence semantic triples for cleanup
    /// \[P9\] Constraining: Homeostatic Self-Regulation — threshold-driven pruning signal
    /// pre:  threshold in [0.0, 1.0]
    /// post: returns count of semantic triples with confidence ≤ threshold
    /// post: returns 0 on error (graceful degradation)
    pub fn semantic_low_confidence_count(&self, threshold: f64) -> usize {
        self.semantic.low_confidence_count(threshold).unwrap_or(0)
    }

    /// Get current semantic triple count.
    ///
    /// REQ: P3-mem-consolidation-service-triple-count
    /// \[P3\] Motivating: Generative Space — reports total semantic memory size
    /// \[P9\] Constraining: Homeostatic Self-Regulation — count used for budget monitoring
    /// post: returns total count of triples in semantic memory
    /// post: returns 0 on error (graceful degradation)
    pub fn semantic_triple_count(&self) -> usize {
        self.semantic.triple_count().unwrap_or(0)
    }
}
