//! DefaultSpecCurator — Curation logic for DDMVSS specifications
//!
//! `DefaultSpecCurator` implements the `SpecCurator` trait, which evaluates
//! specification coherence and makes curation decisions (Merge, Revise,
//! Discard). Curation is a Cybernetics concern (Loop 5) that belongs in
//! `hkask-agents`, not in the storage crate.
//!
//! Moved from `curator::spec_curator` as part of the Curation/Agent separation:
//! spec curation is a persona concern (the Curator Agent curates specs),
//! not a regulatory concern (the Curation Loop regulates).

use hkask_cns::CurationThresholdConfig;
use hkask_storage::spec_types::{Spec, SpecCurationRecord, SpecCurator, SpecError};
use hkask_types::capability::SYSTEM_MAX_RECURSION;
use hkask_types::curation::{CurationDecision, OCAPBoundary};
use std::collections::HashSet;

/// Default implementation of the `SpecCurator` trait.
///
/// Evaluates specification coherence and drift, making curation decisions
/// (Merge, Revise, Discard, Escalate) based on completeness, goal coverage,
/// and spec-tool drift.
pub struct DefaultSpecCurator {
    coherence_threshold: f64,
    drift_threshold: f64,
    max_iterations: u8,
}

impl DefaultSpecCurator {
    pub fn new(coherence_threshold: f64) -> Self {
        Self {
            coherence_threshold: coherence_threshold.clamp(0.0, 1.0),
            drift_threshold: 0.5,
            max_iterations: SYSTEM_MAX_RECURSION,
        }
    }

    /// Create from a `CurationThresholdConfig` loaded from YAML.
    ///
    /// Logs the actual threshold values at construction time for post-hoc analysis.
    pub fn from_config(config: &CurationThresholdConfig) -> Self {
        tracing::info!(
            target: "cns.spec",
            coherence_threshold = config.coherence_threshold,
            drift_threshold = config.drift_threshold,
            "DefaultSpecCurator initialized with YAML-configured thresholds"
        );
        Self {
            coherence_threshold: config.coherence_threshold.clamp(0.0, 1.0),
            drift_threshold: config.drift_threshold.clamp(0.0, 1.0),
            max_iterations: SYSTEM_MAX_RECURSION,
        }
    }

    /// Create with a custom drift threshold.
    pub fn with_drift_threshold(mut self, threshold: f64) -> Self {
        self.drift_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

impl Default for DefaultSpecCurator {
    fn default() -> Self {
        Self::new(0.7)
    }
}

impl SpecCurator for DefaultSpecCurator {
    fn evaluate(
        &self,
        spec: &Spec,
        registered_verbs: &[String],
    ) -> Result<SpecCurationRecord, SpecError> {
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

        // Compute drift between declared verbs and registered tools
        let drift_report = spec.drift(registered_verbs);

        // Emit cns.spec.drift NuEvent span with configured thresholds
        tracing::info!(
            target: "cns.spec",
            spec_id = %spec.id,
            drift_magnitude = drift_report.drift_magnitude,
            drift_threshold = self.drift_threshold,
            missing_verbs = ?drift_report.missing_verbs,
            extra_verbs = ?drift_report.extra_verbs,
            coherence = coherence,
            coherence_threshold = self.coherence_threshold,
            "Spec drift detection with configured thresholds"
        );

        // Escalate if drift exceeds threshold
        if drift_report.drift_magnitude > self.drift_threshold {
            tracing::warn!(
                target: "cns.spec",
                spec_id = %spec.id,
                drift_magnitude = drift_report.drift_magnitude,
                drift_threshold = self.drift_threshold,
                missing_verbs = ?drift_report.missing_verbs,
                "Spec drift exceeded threshold — escalation recommended"
            );
        }

        let ocap_boundary = OCAPBoundary::explicit("spec:curate".to_string());

        Ok(SpecCurationRecord::new(
            spec.id,
            decision,
            &rationale,
            coherence,
            ocap_boundary,
        ))
    }

    fn reconcile(
        &self,
        specs: &[Spec],
        registered_verbs: &[String],
    ) -> Result<Vec<SpecCurationRecord>, SpecError> {
        let records: Result<Vec<SpecCurationRecord>, SpecError> = specs
            .iter()
            .map(|s| self.evaluate(s, registered_verbs))
            .collect();

        records
    }

    fn cultivate(&self, specs: &mut Vec<Spec>) -> Result<f64, SpecError> {
        for _ in 0..self.max_iterations {
            let coherence = Spec::collection_coherence(specs);
            if coherence >= self.coherence_threshold {
                return Ok(coherence);
            }

            let records = self.reconcile(specs, &[])?;

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
