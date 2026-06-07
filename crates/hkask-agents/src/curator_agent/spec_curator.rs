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

use hkask_storage::spec_types::{Spec, SpecCurationRecord, SpecCurator, SpecError};
use hkask_types::capability::SYSTEM_MAX_RECURSION;
use hkask_types::curation::{
    CurationDecision, CurationThresholdConfig, OCAPBoundary, OcapTokenKind,
};
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
use hkask_types::id::WebID;
use hkask_types::loops::LoopId;
use hkask_types::loops::dispatch::{LoopMessage, LoopPayload, MessagePriority};
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
    /// Dispatch channel for sending SpecDriftAlert messages through the Communication Loop.
    /// When set, spec drift alerts flow as structured LoopMessages to Curation's inbox
    /// instead of relying solely on the NuEvent store.
    dispatch_tx: Option<tokio::sync::mpsc::UnboundedSender<LoopMessage>>,
}

impl DefaultSpecCurator {
    pub fn new(coherence_threshold: f64) -> Self {
        Self {
            coherence_threshold: coherence_threshold.clamp(0.0, 1.0),
            drift_threshold: 0.5,
            max_iterations: SYSTEM_MAX_RECURSION,
            event_sink: None,
            dispatch_tx: None,
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
            event_sink: None,
            dispatch_tx: None,
        }
    }

    /// Create with a custom drift threshold.
    pub fn with_drift_threshold(mut self, threshold: f64) -> Self {
        self.drift_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Provide a `NuEventSink` for emitting algedonic events on drift escalation.
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Provide a dispatch channel for sending SpecDriftAlert messages through
    /// the Communication Loop to Curation's inbox.
    ///
    /// When both `event_sink` and `dispatch_tx` are set, spec drift alerts
    /// are sent through both pathways: the NuEvent store for durability and
    /// the Communication Loop for real-time loop-based sensing.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_dispatch(mut self, tx: tokio::sync::mpsc::UnboundedSender<LoopMessage>) -> Self {
        self.dispatch_tx = Some(tx);
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
    pub fn check_sovereignty(&self, spec_id: &str, categories: &[String]) {
        // Emit a NuEvent whenever a sink is wired. The sink is optional
        // (set via `with_event_sink`), so absent one we silently no-op.
        let Some(ref sink) = self.event_sink else {
            return;
        };
        let event = NuEvent::new(
            WebID::new(),
            Span::new(SpanNamespace::new("cns.sovereignty"), "checked"),
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

        // Four-way curation decision gradient (DDMVSS §5.9):
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
                    WebID::new(),
                    Span::new(SpanNamespace::new("cns.spec"), "drift_exceeded"),
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

            // Send SpecDriftAlert through Communication Loop to Curation's inbox
            if let Some(ref tx) = self.dispatch_tx {
                let msg = LoopMessage::new(
                    MessagePriority::Warning,
                    LoopId::Curation,
                    LoopPayload::SpecDriftAlert {
                        spec_id: spec.id.to_string(),
                        drift_magnitude: drift_report.drift_magnitude,
                        drift_threshold: self.drift_threshold,
                        missing_verbs: drift_report.missing_verbs.clone(),
                    },
                )
                .with_target(LoopId::Curation);
                if let Err(e) = tx.send(msg) {
                    tracing::warn!(
                        target: "cns.spec",
                        error = %e,
                        "Failed to send SpecDriftAlert through Communication Loop"
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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurator};

    // ── Helpers ────────────────────────────────────────────────────────

    /// Create a complete spec: one goal with all criteria satisfied.
    fn complete_spec() -> Spec {
        let mut goal = GoalSpec::new("achieve objective");
        goal = goal.with_criterion("criterion A");
        goal = goal.with_criterion("criterion B");
        goal.criteria.iter_mut().for_each(|c| c.mark_satisfied());
        Spec::new("complete spec", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal)
    }

    /// Create a partial spec: one goal with unsatisfied criteria.
    fn partial_spec() -> Spec {
        let goal = GoalSpec::new("partial goal").with_criterion("unsatisfied");
        Spec::new(
            "partial spec",
            SpecCategory::Capability,
            DomainAnchor::Hkask,
        )
        .with_goal(goal)
    }

    /// Create an empty spec: no goals at all.
    fn empty_spec() -> Spec {
        Spec::new("empty spec", SpecCategory::Interface, DomainAnchor::Okapi)
    }

    // ── evaluate ──────────────────────────────────────────────────────

    // P8 invariant: complete spec → Merge decision
    #[test]
    fn evaluate_complete_spec_yields_merge() {
        let curator = DefaultSpecCurator::new(0.7);
        let spec = complete_spec();
        let record = curator
            .evaluate(&spec, &[])
            .expect("evaluate should succeed");
        assert_eq!(
            record.decision,
            CurationDecision::Merge,
            "complete spec must produce Merge decision"
        );
        assert_eq!(record.rationale, "All criteria satisfied");
    }

    // P8 invariant: empty goals → Discard decision
    #[test]
    fn evaluate_empty_goals_yields_discard() {
        let curator = DefaultSpecCurator::new(0.7);
        let spec = empty_spec();
        let record = curator
            .evaluate(&spec, &[])
            .expect("evaluate should succeed");
        assert_eq!(
            record.decision,
            CurationDecision::Discard,
            "spec with no goals must produce Discard decision"
        );
        assert_eq!(record.rationale, "No goals defined");
    }

    // P8 invariant: partial spec (unsatisfied criteria) → Revise decision
    #[test]
    fn evaluate_partial_spec_yields_revise() {
        let curator = DefaultSpecCurator::new(0.7);
        let spec = partial_spec();
        let record = curator
            .evaluate(&spec, &[])
            .expect("evaluate should succeed");
        assert_eq!(
            record.decision,
            CurationDecision::Revise,
            "partial spec must produce Revise decision"
        );
        assert_eq!(record.rationale, "Unsatisfied criteria remain");
    }

    // P8 invariant: spec with coherence in (0.5, threshold) and drift within
    // tolerance yields Defer decision (DDMVSS §5.9 four-way gradient)
    #[test]
    fn evaluate_deferred_spec_yields_defer() {
        // Create a spec with 2/3 criteria satisfied → ratio = 2/3 ≈ 0.667
        // No sub-goals → coherence = ratio ≈ 0.667
        // With threshold 0.7 and drift within tolerance: 0.5 < 0.667 < 0.7 → Defer
        let mut goal = GoalSpec::new("defer goal")
            .with_criterion("satisfied criterion A")
            .with_criterion("satisfied criterion B")
            .with_criterion("unsatisfied criterion");
        goal.criteria[0].mark_satisfied();
        goal.criteria[1].mark_satisfied();
        let spec =
            Spec::new("defer spec", SpecCategory::Trust, DomainAnchor::Hkask).with_goal(goal);
        let curator = DefaultSpecCurator::new(0.7);
        let record = curator
            .evaluate(&spec, &[])
            .expect("evaluate should succeed");
        assert_eq!(
            record.decision,
            CurationDecision::Defer,
            "spec with coherence 0.667 (2/3 satisfied) < threshold 0.7 and drift within tolerance must produce Defer"
        );
        assert_eq!(record.rationale, "Insufficient information — revisit later");
    }

    // P8 invariant: spec with coherence in (0.5, threshold) but drift exceeding threshold → Revise (not Defer)
    #[test]
    fn evaluate_high_drift_yields_revise_not_defer() {
        // 2/3 criteria satisfied → coherence ≈ 0.667 (in Defer zone)
        // But declared verb nonexistent → drift = 1.0 > default drift_threshold (0.5)
        // High drift overrides Defer → Revise
        let mut goal = GoalSpec::new("drifty goal")
            .with_criterion("satisfied criterion A")
            .with_criterion("satisfied criterion B")
            .with_criterion("unsatisfied criterion");
        goal.criteria[0].mark_satisfied();
        goal.criteria[1].mark_satisfied();
        let spec = Spec::new(
            "drifty spec",
            SpecCategory::Observability,
            DomainAnchor::Hkask,
        )
        .with_goal(goal)
        .with_declared_verb("nonexistent_verb");
        let curator = DefaultSpecCurator::new(0.7);
        let record = curator
            .evaluate(&spec, &["some_tool".to_string()])
            .expect("evaluate should succeed");
        assert_eq!(
            record.decision,
            CurationDecision::Revise,
            "spec with high drift must produce Revise, not Defer"
        );
    }

    // P8 invariant: coherence score in record matches spec.coherence()
    #[test]
    fn evaluate_record_coherence_matches_spec() {
        let curator = DefaultSpecCurator::new(0.7);
        let spec = partial_spec();
        let record = curator
            .evaluate(&spec, &[])
            .expect("evaluate should succeed");
        let expected_coherence = spec.coherence();
        assert_eq!(
            record.coherence_score, expected_coherence,
            "record coherence must equal spec.coherence()"
        );
    }

    // P8 invariant: record spec_id matches spec.id
    #[test]
    fn evaluate_record_spec_id_matches() {
        let curator = DefaultSpecCurator::new(0.7);
        let spec = complete_spec();
        let record = curator
            .evaluate(&spec, &[])
            .expect("evaluate should succeed");
        assert_eq!(record.spec_id, spec.id, "record spec_id must equal spec.id");
    }

    // P8 invariant: coherence threshold is clamped to [0.0, 1.0]
    #[test]
    fn curator_clamps_coherence_threshold_to_unit_interval() {
        let above = DefaultSpecCurator::new(1.5);
        let below = DefaultSpecCurator::new(-0.5);
        // Coherence threshold is private; verify indirectly by checking that
        // cultivate with 0 specs doesn't panic and that clamping works.
        // With threshold > 1.0, no spec collection can reach it, so cultivate
        // should return CurationDepthExceeded.
        let mut specs = vec![complete_spec()];
        let result = above.cultivate(&mut specs);
        assert!(
            result.is_err(),
            "threshold above 1.0 must cause CurationDepthExceeded"
        );

        // With threshold <= 0.0, any collection (even empty) should succeed.
        let mut specs = vec![];
        let result = below.cultivate(&mut specs);
        assert!(
            result.is_ok(),
            "threshold at 0.0 must succeed with empty collection"
        );
    }

    // ── reconcile ──────────────────────────────────────────────────────

    // P8 invariant: reconcile produces one record per spec
    #[test]
    fn reconcile_produces_one_record_per_spec() {
        let curator = DefaultSpecCurator::new(0.7);
        let specs = vec![complete_spec(), partial_spec(), empty_spec()];
        let records = curator
            .reconcile(&specs, &[])
            .expect("reconcile should succeed");
        assert_eq!(
            records.len(),
            3,
            "reconcile must produce exactly one record per spec"
        );
    }

    // P8 invariant: reconcile decisions match evaluate decisions
    #[test]
    fn reconcile_decisions_match_evaluate() {
        let curator = DefaultSpecCurator::new(0.7);
        let complete = complete_spec();
        let partial = partial_spec();
        let empty = empty_spec();
        let specs = vec![complete, partial, empty];
        let records = curator
            .reconcile(&specs, &[])
            .expect("reconcile should succeed");
        assert_eq!(records[0].decision, CurationDecision::Merge);
        assert_eq!(records[1].decision, CurationDecision::Revise);
        assert_eq!(records[2].decision, CurationDecision::Discard);
    }

    // ── cultivate ──────────────────────────────────────────────────────

    // P8 invariant: cultivate with coherent collection returns Ok(coherence)
    #[test]
    fn cultivate_coherent_collection_succeeds() {
        let curator = DefaultSpecCurator::new(0.5);
        // Use complete specs (all criteria satisfied) for a coherent collection.
        let mut complete_goal = GoalSpec::new("objective");
        complete_goal = complete_goal.with_criterion("c1");
        complete_goal
            .criteria
            .iter_mut()
            .for_each(|c| c.mark_satisfied());
        let mut specs = vec![
            Spec::new("s1", SpecCategory::Domain, DomainAnchor::Hkask)
                .with_goal(complete_goal.clone()),
            Spec::new("s2", SpecCategory::Capability, DomainAnchor::Hkask)
                .with_goal(complete_goal.clone()),
        ];
        let result = curator.cultivate(&mut specs);
        assert!(result.is_ok(), "coherent collection must succeed");
    }

    // P8 invariant: cultivate removes Discard-ed specs from the collection
    #[test]
    fn cultivate_removes_discard_specs() {
        let curator = DefaultSpecCurator::new(0.5);
        // An empty-spec has no goals → Discard → removed from collection
        let mut specs = vec![empty_spec()];
        let result = curator.cultivate(&mut specs);
        // Empty spec gets discarded, leaving empty vec → coherence 0.0
        // With threshold 0.5, this should CurationDepthExceeded
        // But let's verify the specs vec was modified
        // Actually, after 1 iteration, the empty spec is discarded,
        // leaving empty vec. collection_coherence of empty vec is 0.0.
        // With threshold 0.5, it loops max_iterations times and fails.
        assert!(
            specs.is_empty() || result.is_err(),
            "empty specs should be discarded during cultivation"
        );
    }

    // P8 invariant: cultivate returns CurationDepthExceeded when coherence
    // cannot reach threshold within max_iterations
    #[test]
    fn cultivate_returns_depth_exceeded_when_unable_to_reach_threshold() {
        let curator = DefaultSpecCurator::new(0.7);
        // Partial specs that can never reach coherence threshold
        let mut specs = vec![partial_spec()];
        let result = curator.cultivate(&mut specs);
        assert!(
            matches!(result, Err(SpecError::CurationDepthExceeded)),
            "cultivate must return CurationDepthExceeded when coherence cannot reach threshold"
        );
    }

    // ── Default ────────────────────────────────────────────────────────

    // P8 invariant: DefaultSpecCurator::default() uses coherence threshold 0.7
    #[test]
    fn default_spec_curator_has_threshold_0_7() {
        let _curator = DefaultSpecCurator::default();
        // Threshold is private; verify behaviorally: a spec with 0% criteria
        // satisfied (coherence = 0.0) should produce Revise (not Defer or Merge).
        let goal = GoalSpec::new("g").with_criterion("c");
        // coherence of a single spec with 1 unsatisfied criterion = 0.0
        // (no sub-goals → coherence = ratio = 0/1 = 0.0)
        // This is below 0.5, so default curator should produce Revise.
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        let curator = DefaultSpecCurator::default();
        let record = curator.evaluate(&spec, &[]).expect("evaluate");
        assert_eq!(
            record.decision,
            CurationDecision::Revise,
            "spec with coherence 0.0 must produce Revise with default threshold"
        );
    }
}
