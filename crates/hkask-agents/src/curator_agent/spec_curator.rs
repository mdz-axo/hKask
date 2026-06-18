//! DefaultSpecCurator — Curation logic for MDS specifications
//!
//! `DefaultSpecCurator` implements the `SpecCurator` trait, which evaluates
//! specification coherence and makes curation decisions (Merge, Revise,
//! Discard). Curation is a Cybernetics concern (Loop 5) that belongs in
//! `hkask-agents`, not in the storage crate.
//!
//! Moved from `curator::spec_curator` as part of the Curation/Agent separation:
//! spec curation is a persona concern (the Curator Agent curates specs),
//! not a regulatory concern (the Curation Loop regulates).

use hkask_rsolidity as rs;
use hkask_storage::spec_types::{Spec, SpecCurationRecord, SpecCurator, SpecError};
use hkask_types::capability::SYSTEM_MAX_RECURSION;
use hkask_types::cns::CnsSpan;
use hkask_types::curation::{
    CurationDecision, CurationThresholdConfig, OCAPBoundary, OcapTokenKind,
};
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
use hkask_types::id::WebID;
use hkask_types::loops::CurationInput;
use hkask_types::loops::SpecEvent;
use std::collections::HashSet;
use std::sync::Arc;

/// Default implementation of the `SpecCurator` trait.
///
/// Evaluates specification coherence and drift, making curation decisions
/// (Merge, Revise, Discard, Escalate) based on completeness, goal coverage,
/// and spec-tool drift.
pub struct DefaultSpecCurator {
    coherence_threshold: f64,
    drift_threshold: f64,
    max_iterations: u8,
    event_sink: Option<Arc<dyn NuEventSink>>,
    /// Direct spec event channel: SpecCurator → CurationLoop.
    spec_tx: Option<tokio::sync::mpsc::UnboundedSender<CurationInput>>,
}

impl DefaultSpecCurator {
    /// REQ: P9-agt-curator-agent-spec-new
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — initialize spec curator with coherence threshold
    /// \[P7\] Constraining: Evolutionary Architecture — thresholds calibrated from observations
    /// pre:  `coherence_threshold` is in range [0.0, 1.0] (clamped).
    /// post: Returns a `DefaultSpecCurator` with the given coherence
    ///       threshold, drift_threshold=0.5, max_iterations=SYSTEM_MAX_RECURSION,
    ///       and no event sink or spec channel.
    #[rs::contract(id = "P9-agt-curator-agent-spec-new", principle = "P9")]
    pub fn new(coherence_threshold: f64) -> Self {
        Self {
            coherence_threshold: coherence_threshold.clamp(0.0, 1.0),
            drift_threshold: 0.5,
            max_iterations: SYSTEM_MAX_RECURSION,
            event_sink: None,
            spec_tx: None,
        }
    }

    /// Calibrate the coherence threshold from historical curation records.
    ///
    /// Queries `SqliteCurationRecordStore` for all prior coherence scores,
    /// computes the 25th percentile as the empirical threshold, and returns
    /// a recommended value. The 25th percentile is chosen so that specs in
    /// the bottom quartile trigger `Revise` while the top 75% are candidates
    /// for `Merge` or `Defer`. This is a conservative baseline — manual tuning
    /// may tighten the threshold further.
    ///
    /// Returns `None` if there are fewer than 10 records (insufficient data).
    ///
    /// MDS §5: Coherence threshold calibration — FUT-013.
    ///
    /// REQ: P9-agt-curator-agent-spec-calibrate
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — calibrate threshold from historical coherence
    /// \[P7\] Constraining: Evolutionary Architecture — 25th-percentile heuristic emerged from usage
    /// pre:  `curation_store` is a valid `SqliteCurationRecordStore`.
    /// post: Returns `Some(f64)` — the 25th-percentile coherence score —
    ///       if ≥10 records exist; `None` otherwise.
    #[rs::contract(id = "P9-agt-curator-agent-spec-calibrate", principle = "P9")]
    pub fn calibrate_from_history(
        curation_store: &hkask_storage::spec_store::SqliteCurationRecordStore,
    ) -> Option<f64> {
        let records = curation_store
            .load_all_curation_records()
            .unwrap_or_default();

        if records.len() < 10 {
            tracing::info!(
                target: "cns.spec",
                record_count = records.len(),
                "Insufficient curation history for threshold calibration — need ≥10 records"
            );
            return None;
        }

        let mut scores: Vec<f64> = records.iter().map(|r| r.coherence_score).collect();
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // 25th percentile: the score below which 25% of observations fall.
        // Using nearest-rank method (n = ceil(P/100 * N)).
        let p25_idx = (0.25_f64 * scores.len() as f64).ceil() as usize;
        let p25_idx = p25_idx.saturating_sub(1);
        let calibrated = scores[p25_idx.min(scores.len().saturating_sub(1))];

        tracing::info!(
            target: "cns.spec",
            record_count = records.len(),
            min_coherence = scores.first(),
            max_coherence = scores.last(),
            p25_coherence = calibrated,
            "Calibrated coherence threshold from historical curation records"
        );

        Some(calibrated)
    }

    /// Create from a `CurationThresholdConfig` loaded from YAML.
    ///
    /// Logs the actual threshold values at construction time for post-hoc analysis.
    ///
    /// REQ: P9-agt-curator-agent-spec-with-config
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — apply explicit curation threshold config
    /// pre:  `config` is a valid `CurationThresholdConfig` with thresholds
    ///       in [0.0, 1.0] (clamped).
    /// post: Returns a `DefaultSpecCurator` with thresholds from the config,
    ///       max_iterations=SYSTEM_MAX_RECURSION, and no event sink or spec
    ///       channel.
    #[rs::contract(id = "P9-agt-curator-agent-spec-with-config", principle = "P9")]
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
            event_sink: None,
            spec_tx: None,
        }
    }

    /// Create with a custom drift threshold.
    ///
    /// REQ: P9-agt-curator-agent-spec-drift-threshold
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — drift threshold triggers escalation
    /// pre:  `threshold` is in range [0.0, 1.0] (clamped).
    /// post: Returns `self` with `drift_threshold` updated.
    #[rs::contract(id = "P9-agt-curator-agent-spec-drift-threshold", principle = "P9")]
    pub fn with_drift_threshold(mut self, threshold: f64) -> Self {
        self.drift_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Provide a `NuEventSink` for emitting algedonic events on drift escalation.
    ///
    /// REQ: P9-agt-curator-agent-spec-with-sink
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — emit algedonic events on drift escalation
    /// pre:  `sink` is a valid `Arc<dyn NuEventSink>`.
    /// post: Returns `self` with `event_sink` set to `Some(sink)`.
    #[rs::contract(id = "P9-agt-curator-agent-spec-with-sink", principle = "P9")]
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Wire the direct spec event channel: SpecCurator → CurationLoop.
    ///
    /// REQ: P9-agt-curator-agent-spec-channel
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — wire spec events into CurationLoop
    /// pre:  `tx` is a valid `UnboundedSender<CurationInput>`.
    /// post: Returns `self` with `spec_tx` set to `Some(tx)`.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_spec_channel(
        mut self,
        tx: tokio::sync::mpsc::UnboundedSender<CurationInput>,
    ) -> Self {
        self.spec_tx = Some(tx);
        self
    }

    /// Record a sovereignty check for a spec evaluation.
    ///
    /// Emits a `cns.sovereignty.checked` `NuEvent` (Phase::Compare) describing
    /// which data categories the curator consulted. This is the
    /// Curator-as-Enforcer recording the Magna Carta's "sovereignty checking"
    /// responsibility (Curator Responsibilities §2 in `magna-carta.md`).
    ///
    /// This call does not change the existing `evaluate` semantics: it is a
    /// side-channel that records the fact that sovereignty was considered
    /// during curation.
    ///
    /// REQ: P9-agt-curator-agent-spec-check
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — check spec coherence and emit drift alerts
    /// pre:  `spec_id` is a non-empty string; `categories` is a slice of
    ///       category name strings.
    /// post: If `event_sink` is `Some`, emits a `cns.sovereignty.checked`
    ///       NuEvent; if `None`, this is a silent no-op.
    #[rs::contract(id = "P9-agt-curator-agent-spec-check", principle = "P9")]
    pub fn check_sovereignty(&self, spec_id: &str, categories: &[String]) {
        // Emit a NuEvent whenever a sink is wired. The sink is optional
        // (set via `with_event_sink`), so absent one we silently no-op.
        let Some(ref sink) = self.event_sink else {
            return;
        };
        let event = NuEvent::new(
            WebID::from_persona(b"spec-curator"),
            Span::new(SpanNamespace::from(CnsSpan::Sovereignty), "checked"),
            Phase::Compare,
            serde_json::json!({
                "spec_id": spec_id,
                "categories": categories,
            }),
            0,
        );
        if let Err(e) = sink.persist(&event) {
            tracing::warn!(
                target: "cns.sovereignty",
                error = %e,
                "Failed to persist sovereignty check event"
            );
        }
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
        let coherence = spec.coherence();
        let drift_report = spec.drift(registered_verbs);
        let drift_within_tolerance = drift_report.drift_magnitude <= self.drift_threshold;

        // Four-way curation decision gradient (MDS §5):
        //   Merge:   spec is complete (all criteria satisfied)
        //   Discard: spec has no goals
        //   Defer:   partial progress but insufficient for Merge — coherence > 0.5
        //            (exclusive, to exclude the sub_coherence=1.0 artifact) but < threshold,
        //            and drift within tolerance
        //   Revise:  unsatisfied criteria remain, needs immediate revision
        let decision = if complete {
            CurationDecision::Merge
        } else if spec.goals.is_empty() {
            CurationDecision::Discard
        } else if coherence > 0.5 && coherence < self.coherence_threshold && drift_within_tolerance
        {
            CurationDecision::Defer
        } else {
            CurationDecision::Revise
        };

        let rationale = if complete {
            "All criteria satisfied".to_string()
        } else if spec.goals.is_empty() {
            "No goals defined".to_string()
        } else if matches!(decision, CurationDecision::Defer) {
            "Insufficient information — revisit later".to_string()
        } else {
            "Unsatisfied criteria remain".to_string()
        };

        let ocap_boundary = OCAPBoundary::token(OcapTokenKind::SpecCurate);

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

            // Emit algedonic NuEvent so Curation Loop can sense it
            if let Some(ref sink) = self.event_sink {
                let event = NuEvent::new(
                    WebID::from_persona(b"spec-curator"),
                    Span::new(SpanNamespace::from(CnsSpan::Spec), "drift_exceeded"),
                    Phase::Compare,
                    serde_json::json!({
                        "spec_id": spec.id.to_string(),
                        "drift_magnitude": drift_report.drift_magnitude,
                        "drift_threshold": self.drift_threshold,
                        "missing_verbs": drift_report.missing_verbs,
                    }),
                    0,
                );
                if let Err(e) = sink.persist(&event) {
                    tracing::warn!(
                        target: "cns.spec",
                        error = %e,
                        "Failed to persist spec drift algedonic event"
                    );
                }
            }

            // Send SpecDrift through unified CurationInput channel to Curation's inbox
            if let Some(ref spec_tx) = self.spec_tx {
                let event = SpecEvent {
                    spec_id: spec.id.to_string(),
                    drift_magnitude: drift_report.drift_magnitude,
                    drift_threshold: self.drift_threshold,
                    missing_verbs: drift_report.missing_verbs.clone(),
                };
                if let Err(e) = spec_tx.send(CurationInput::SpecDrift(event)) {
                    tracing::warn!(
                        target: "cns.spec",
                        error = %e,
                        "Failed to send CurationInput::SpecDrift"
                    );
                }
            }
        }

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
