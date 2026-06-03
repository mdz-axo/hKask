//! DefaultSpecCurator — Curation logic for DDMVSS specifications
//!
//! `DefaultSpecCurator` implements the `SpecCurator` trait, which evaluates
//! specification coherence and makes curation decisions (Merge, Revise,
//! Discard). Curation is a Cybernetics concern (Loop 5) that belongs in
//! `hkask-agents`, not in the storage crate.

use hkask_storage::spec_types::{Spec, SpecCurationRecord, SpecCurator, SpecError};
use hkask_types::capability::SYSTEM_MAX_RECURSION;
use hkask_types::curation::{CurationDecision, OCAPBoundary};
use std::collections::HashSet;

/// Default implementation of the `SpecCurator` trait.
///
/// Evaluates specification coherence and makes curation decisions
/// (Merge, Revise, Discard) based on completeness and goal coverage.
pub struct DefaultSpecCurator {
    coherence_threshold: f64,
    max_iterations: u8,
}

impl DefaultSpecCurator {
    pub fn new(coherence_threshold: f64) -> Self {
        Self {
            coherence_threshold: coherence_threshold.clamp(0.0, 1.0),
            max_iterations: SYSTEM_MAX_RECURSION,
        }
    }
}

impl Default for DefaultSpecCurator {
    fn default() -> Self {
        Self::new(0.7)
    }
}

impl SpecCurator for DefaultSpecCurator {
    fn evaluate(&self, spec: &Spec) -> Result<SpecCurationRecord, SpecError> {
        let complete = spec.is_complete();
        let decision = if complete {
            CurationDecision::Merge
        } else if spec.goals.is_empty() {
            CurationDecision::Discard
        } else {
            CurationDecision::Revise
        };

        let rationale = if complete {
            "All criteria satisfied".to_string()
        } else if spec.goals.is_empty() {
            "No goals defined".to_string()
        } else {
            "Unsatisfied criteria remain".to_string()
        };

        let coherence = spec.coherence();
        let ocap_boundary = OCAPBoundary::explicit("spec:curate".to_string());

        Ok(SpecCurationRecord::new(
            spec.id,
            decision,
            &rationale,
            coherence,
            ocap_boundary,
        ))
    }

    fn reconcile(&self, specs: &[Spec]) -> Result<Vec<SpecCurationRecord>, SpecError> {
        let records: Result<Vec<SpecCurationRecord>, SpecError> =
            specs.iter().map(|s| self.evaluate(s)).collect();

        records
    }

    fn cultivate(&self, specs: &mut Vec<Spec>) -> Result<f64, SpecError> {
        for _ in 0..self.max_iterations {
            let coherence = Spec::collection_coherence(specs);
            if coherence >= self.coherence_threshold {
                return Ok(coherence);
            }

            let records = self.reconcile(specs)?;

            // Remove specs marked for discard
            let discard_ids: HashSet<_> = records
                .iter()
                .filter(|r| r.decision == CurationDecision::Discard)
                .map(|r| r.spec_id)
                .collect();
            specs.retain(|s| !discard_ids.contains(&s.id));

            // If all remaining records are Merge, check coherence again
            let all_merge = records
                .iter()
                .filter(|r| r.decision != CurationDecision::Discard)
                .all(|r| r.decision == CurationDecision::Merge);
            if all_merge {
                let coherence = Spec::collection_coherence(specs);
                if coherence >= self.coherence_threshold {
                    return Ok(coherence);
                }
            }
        }

        // Coherence still below threshold after all iterations
        Err(SpecError::CurationDepthExceeded)
    }
}
