//! Consolidation Service — combined consolidation + cleanup

use std::sync::Arc;

use crate::consolidation::ConsolidationBridge;
use crate::semantic::SemanticMemory;
use hkask_ports::{ConsolidationOutcome, ConsolidationRequest};
use hkask_types::WebID;

pub struct ConsolidationService {
    bridge: Arc<ConsolidationBridge>,
    semantic: Arc<SemanticMemory>,
}

impl ConsolidationService {
    pub fn new(bridge: Arc<ConsolidationBridge>, semantic: Arc<SemanticMemory>) -> Self {
        Self { bridge, semantic }
    }

    /// Execute a consolidation operation — three phases:
    /// 1. Promote episodic triples to semantic memory
    /// 2. Delete semantic triples at or below confidence floor (if specified)
    /// 3. Delete lowest-confidence semantic triples until within max count (if specified)
    pub fn consolidate(
        &self,
        perspective: &WebID,
        request: ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, String> {
        tracing::info!(
            target: "cns.consolidation",
            perspective = %perspective,
            limit = request.limit,
            confidence_floor = ?request.confidence_floor,
            max_semantic_triples = ?request.max_semantic_triples,
            "Consolidation starting"
        );

        let bridge_outcome = self.bridge.consolidate(
            *perspective,
            ConsolidationRequest {
                limit: request.limit,
                confidence_floor: None,
                max_semantic_triples: None,
            },
        )?;

        let mut deleted_count = 0usize;

        if let Some(floor) = request.confidence_floor {
            if let Ok(candidates) = self.semantic.low_confidence_triples(floor, usize::MAX) {
                if !candidates.is_empty() {
                    for triple in &candidates {
                        if self.semantic.delete_triple(&triple.id).is_ok() {
                            deleted_count += 1;
                        }
                    }
                }
            }
        }

        if let Some(max) = request.max_semantic_triples {
            if let Ok(count) = self.semantic.triple_count() {
                if count > max {
                    if let Ok(candidates) = self.semantic.lowest_confidence_triples(count - max) {
                        for triple in &candidates {
                            if self.semantic.delete_triple(&triple.id).is_ok() {
                                deleted_count += 1;
                            }
                        }
                    }
                }
            }
        }

        tracing::info!(
            target: "cns.consolidation",
            consolidated = bridge_outcome.consolidated_count,
            deleted = deleted_count,
            failed = bridge_outcome.failed_count,
            "Consolidation complete"
        );

        Ok(ConsolidationOutcome {
            consolidated_count: bridge_outcome.consolidated_count,
            deleted_count,
            failed_count: bridge_outcome.failed_count,
        })
    }

    pub fn consolidation_candidate_count(&self, perspective: &WebID) -> usize {
        self.bridge.consolidation_candidate_count(perspective)
    }

    pub fn semantic_low_confidence_count(&self, threshold: f64) -> usize {
        self.semantic.low_confidence_count(threshold).unwrap_or(0)
    }

    pub fn semantic_triple_count(&self) -> usize {
        self.semantic.triple_count().unwrap_or(0)
    }
}
