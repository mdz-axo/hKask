//! Semantic Loop — knowledge → store → index → recall → dedup → combine → context (Loop 2b)
//!
//! Wraps `SemanticMemory` and provides two regulatory triggers:
//!
//! 1. **Storage budget** — when triple count exceeds the configurable budget
//!    (default 25,000), delete lowest-confidence triples to free space.
//!
//! 2. **Consolidation trigger** — review and delete semantic triples with
//!    confidence at or below the low-confidence threshold (default 0.33).
//!    These triples are too uncertain to be useful and should be pruned.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::semantic::SemanticMemory;
use hkask_cns::types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use hkask_storage::Triple;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};

/// Default storage budget for semantic triple count.
pub const DEFAULT_SEMANTIC_STORAGE_BUDGET: usize = 25_000;

/// Default low-confidence threshold for the consolidation trigger.
///
/// Semantic triples at or below this confidence (0.33 = 33%) are candidates
/// for review and deletion. These triples carry insufficient signal to
/// justify their storage cost.
pub const DEFAULT_LOW_CONFIDENCE_THRESHOLD: f64 = 0.33;

/// Default condensation window in days.
///
/// Semantic triples with `valid_from` older than this many days are
/// candidates for entity-grouped condensation.
pub const DEFAULT_CONDENSATION_WINDOW_DAYS: u32 = 30;

/// Confidence assigned to condensed summary triples.
///
/// Summary triples carry 0.6 confidence — lower than directly observed
/// facts (1.0) but higher than the low-confidence threshold (0.33).
pub const CONDENSED_SUMMARY_CONFIDENCE: f64 = 0.6;

/// Semantic Loop — monitors semantic memory with two regulatory triggers.
///
/// Wraps `SemanticMemory` and reads:
/// - `triple_count` — current count vs storage budget
/// - `low_confidence_count` — triples at or below the confidence threshold
///
/// When `auto_condense` is enabled (default), the loop also performs
/// entity-grouped condensation: triples older than `condensation_window_days`
/// are grouped by entity, the highest-confidence + most recent triple is kept,
/// and the rest are soft-deleted with a provenance summary triple stored.
///
/// Both the storage budget and low-confidence threshold are configurable
/// per-loop instance, enabling per-user and per-agent customization.
pub struct SemanticLoop {
    memory: Arc<SemanticMemory>,
    storage_budget: usize,
    low_confidence_threshold: f64,
    /// Enable automatic semantic condensation (default: true).
    auto_condense: bool,
    /// Triples older than this many days are condensation candidates.
    condensation_window_days: u32,
}

impl SemanticLoop {
    /// Create a new Semantic Loop with default settings.
    ///
    /// Default storage budget: 25,000 triples.
    /// Default low-confidence threshold: 0.33 (33%).
    ///
    /// expect: "The system wraps semantic memory in a regulated knowledge loop"
    /// \[P3\] Motivating: Generative Space — wraps semantic memory in a regulated knowledge loop
    /// \[P9\] Constraining: Homeostatic Self-Regulation — default budget and low-confidence threshold are set-points
    /// pre:  memory is initialized
    /// post: returns SemanticLoop with DEFAULT_SEMANTIC_STORAGE_BUDGET and DEFAULT_LOW_CONFIDENCE_THRESHOLD
    pub fn new(memory: Arc<SemanticMemory>) -> Self {
        Self {
            memory,
            storage_budget: DEFAULT_SEMANTIC_STORAGE_BUDGET,
            low_confidence_threshold: DEFAULT_LOW_CONFIDENCE_THRESHOLD,
            auto_condense: true,
            condensation_window_days: DEFAULT_CONDENSATION_WINDOW_DAYS,
        }
    }

    /// Create a new Semantic Loop with a custom storage budget.
    ///
    /// Use this for per-user or per-agent budget customization.
    ///
    /// expect: "The system wraps semantic memory in a regulated knowledge loop"
    /// \[P3\] Motivating: Generative Space — customizes storage budget per user or agent
    /// \[P9\] Constraining: Homeostatic Self-Regulation — configurable set-point for memory homeostasis
    /// pre:  memory is initialized, storage_budget > 0
    /// post: returns SemanticLoop with custom budget, default threshold
    pub fn with_budget(memory: Arc<SemanticMemory>, storage_budget: usize) -> Self {
        Self {
            memory,
            storage_budget,
            low_confidence_threshold: DEFAULT_LOW_CONFIDENCE_THRESHOLD,
            auto_condense: true,
            condensation_window_days: DEFAULT_CONDENSATION_WINDOW_DAYS,
        }
    }

    /// Create a new Semantic Loop with custom storage budget and
    /// low-confidence threshold.
    ///
    /// Use this for full per-user or per-agent customization.
    ///
    /// expect: "The system wraps semantic memory in a regulated knowledge loop"
    /// \[P3\] Motivating: Generative Space — customizes both budget and cleanup threshold
    /// \[P7\] Constraining: Evolutionary Architecture — thresholds emerge from usage patterns
    /// pre:  memory is initialized, storage_budget > 0
    /// pre:  low_confidence_threshold in [0.0, 1.0]
    /// post: returns SemanticLoop with custom budget and threshold
    pub fn with_budget_and_threshold(
        memory: Arc<SemanticMemory>,
        storage_budget: usize,
        low_confidence_threshold: f64,
    ) -> Self {
        Self {
            memory,
            storage_budget,
            low_confidence_threshold,
            auto_condense: true,
            condensation_window_days: DEFAULT_CONDENSATION_WINDOW_DAYS,
        }
    }

    /// Get the configured storage budget (set-point).
    ///
    /// expect: "The system wraps semantic memory in a regulated knowledge loop"
    /// \[P3\] Motivating: Generative Space — exposes the semantic storage set-point
    /// \[P9\] Constraining: Homeostatic Self-Regulation — immutable budget reference for regulation
    /// post: returns the storage_budget value set at construction
    pub fn storage_budget(&self) -> usize {
        self.storage_budget
    }

    /// Get the configured low-confidence threshold.
    ///
    /// expect: "The system wraps semantic memory in a regulated knowledge loop"
    /// \[P3\] Motivating: Generative Space — exposes the low-confidence cleanup set-point
    /// \[P9\] Constraining: Homeostatic Self-Regulation — threshold triggers pruning of uncertain knowledge
    /// post: returns the low_confidence_threshold value set at construction
    pub fn low_confidence_threshold(&self) -> f64 {
        self.low_confidence_threshold
    }

    /// Create with condensation disabled (for testing or manual curation).
    ///
    /// expect: "The system wraps semantic memory in a regulated knowledge loop"
    /// pre:  memory is initialized
    /// post: returns SemanticLoop with auto_condense and window set
    pub fn with_condensation(
        memory: Arc<SemanticMemory>,
        auto_condense: bool,
        condensation_window_days: u32,
    ) -> Self {
        Self {
            memory,
            storage_budget: DEFAULT_SEMANTIC_STORAGE_BUDGET,
            low_confidence_threshold: DEFAULT_LOW_CONFIDENCE_THRESHOLD,
            auto_condense,
            condensation_window_days,
        }
    }

    /// Check whether auto-condensation is enabled.
    pub fn auto_condense(&self) -> bool {
        self.auto_condense
    }

    /// Get the condensation window in days.
    pub fn condensation_window_days(&self) -> u32 {
        self.condensation_window_days
    }

    /// Emit a CNS NuEvent through the memory's event sink.
    fn emit_cns(&self, verb: &str, observation: serde_json::Value) {
        if let Some(sink) = self.memory.event_sink() {
            let span = Span::new(SpanNamespace::from(CnsSpan::MemoryEncode), verb);
            let event = NuEvent::new(hkask_types::WebID::new(), span, Phase::Act, observation, 0);
            let _ = sink.persist(&event);
        }
    }
}

#[async_trait::async_trait]
impl HkaskLoop for SemanticLoop {
    fn id(&self) -> LoopId {
        LoopId::Semantic
    }

    /// Sense: read semantic triple count and low-confidence count.
    ///
    /// Produces signals for:
    /// - `triple_count` — current count vs storage budget
    /// - `low_confidence_count` — triples at or below confidence threshold
    ///   (set-point = 0, any non-zero count is a deviation)
    async fn sense(&self) -> Vec<Signal> {
        let count = self.memory.triple_count().unwrap_or(0);
        let low_count = self
            .memory
            .low_confidence_count(self.low_confidence_threshold)
            .unwrap_or(0);

        vec![
            Signal::new(
                LoopId::Memory,
                SignalMetric::TripleCount,
                count as f64,
                self.storage_budget as f64,
            ),
            Signal::new(
                LoopId::Memory,
                SignalMetric::LowConfidenceCount,
                low_count as f64,
                0.0, // set-point = 0: any low-confidence triples are a deviation
            ),
        ]
    }

    /// Compute: produce actions based on deviations.
    ///
    /// - `triple_count` above set-point → Calibrate (budget exceeded)
    ///   If `auto_condense` is enabled, condensation is attempted first.
    /// - `low_confidence_count` above 0 → Calibrate (consolidation trigger)
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            match dev.signal.metric {
                SignalMetric::TripleCount if dev.direction == DeviationDirection::AboveSetPoint => {
                    // Try condensation first if enabled
                    if self.auto_condense {
                        // Check if there are old triples worth condensing
                        if let Ok(old_triples) = self
                            .memory
                            .triples_older_than(self.condensation_window_days, 200)
                            && !old_triples.is_empty()
                        {
                            actions.push(LoopAction::new(
                                LoopId::Memory,
                                ActionType::Calibrate,
                                serde_json::json!({
                                    "reason": "semantic_condense",
                                    "count": dev.signal.value,
                                    "set_point": dev.signal.set_point,
                                    "old_triple_count": old_triples.len(),
                                    "window_days": self.condensation_window_days,
                                }),
                            ));
                            // Don't also emit budget enforcement — condensation may resolve it
                            continue;
                        }
                    }

                    let overage = (dev.signal.value - dev.signal.set_point) as usize;
                    actions.push(LoopAction::new(
                        LoopId::Memory,
                        ActionType::Calibrate,
                        serde_json::json!({
                            "reason": "semantic_triple_count_exceeded",
                            "count": dev.signal.value,
                            "set_point": dev.signal.set_point,
                            "overage": overage,
                        }),
                    ));
                }
                SignalMetric::LowConfidenceCount
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    actions.push(LoopAction::new(
                        LoopId::Memory,
                        ActionType::Calibrate,
                        serde_json::json!({
                            "reason": "semantic_low_confidence_review",
                            "low_confidence_count": dev.signal.value,
                            "threshold": self.low_confidence_threshold,
                        }),
                    ));
                }
                _ => {}
            }
        }

        actions
    }

    /// Act: enforce regulation via deletion.
    ///
    /// Two triggers:
    ///
    /// - `semantic_low_confidence_review`: delete all semantic triples at or
    ///   below the low-confidence threshold (default 33%). These triples
    ///   carry insufficient signal to justify their storage cost.
    ///
    /// - `semantic_triple_count_exceeded`: delete lowest-confidence semantic
    ///   triples to bring count back within budget. Fires after the
    ///   low-confidence review — if budget is still exceeded after clearing
    ///   low-confidence entries, progressively delete the next-lowest.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            match action.action_type {
                ActionType::Calibrate => {
                    let reason = action.parameters.get("reason").and_then(|v| v.as_str());
                    match reason {
                        Some("semantic_low_confidence_review") => {
                            let count = action
                                .parameters
                                .get("low_confidence_count")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as usize;

                            if count == 0 {
                                continue;
                            }

                            // Delete all semantic triples at or below the threshold
                            match self
                                .memory
                                .low_confidence_triples(self.low_confidence_threshold, count)
                            {
                                Ok(candidates) if !candidates.is_empty() => {
                                    tracing::warn!(
                                        target: "cns.semantic",
                                        candidates = candidates.len(),
                                        threshold = self.low_confidence_threshold,
                                        "Deleting low-confidence semantic triples (consolidation trigger)"
                                    );
                                    for triple in &candidates {
                                        if let Err(e) = self.memory.delete_triple(&triple.id) {
                                            tracing::debug!(
                                                target: "cns.semantic",
                                                triple_id = %triple.id,
                                                entity = %triple.entity,
                                                attribute = %triple.attribute,
                                                confidence = %triple.confidence,
                                                error = %e,
                                                "Failed to delete low-confidence semantic triple"
                                            );
                                        }
                                    }
                                }
                                Ok(_) => {
                                    tracing::debug!(
                                        target: "cns.semantic",
                                        "No low-confidence semantic triples found for deletion"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        target: "cns.semantic",
                                        error = %e,
                                        "Failed to query low-confidence semantic triples"
                                    );
                                }
                            }
                        }
                        Some("semantic_triple_count_exceeded") => {
                            let overage = action
                                .parameters
                                .get("overage")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as usize;

                            // Delete lowest-confidence triples to free budget
                            match self.memory.lowest_confidence_triples(overage) {
                                Ok(candidates) if !candidates.is_empty() => {
                                    tracing::warn!(
                                        target: "cns.semantic",
                                        candidates = candidates.len(),
                                        overage = overage,
                                        "Deleting lowest-confidence semantic triples to enforce budget"
                                    );
                                    for triple in &candidates {
                                        if let Err(e) = self.memory.delete_triple(&triple.id) {
                                            tracing::debug!(
                                                target: "cns.semantic",
                                                triple_id = %triple.id,
                                                entity = %triple.entity,
                                                attribute = %triple.attribute,
                                                error = %e,
                                                "Failed to delete semantic triple"
                                            );
                                        }
                                    }
                                }
                                Ok(_) => {
                                    tracing::debug!(
                                        target: "cns.semantic",
                                        "No low-confidence semantic triples found for deletion"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        target: "cns.semantic",
                                        error = %e,
                                        "Failed to query low-confidence semantic triples"
                                    );
                                }
                            }
                        }
                        Some("semantic_condense") => {
                            let window_days = action
                                .parameters
                                .get("window_days")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(DEFAULT_CONDENSATION_WINDOW_DAYS as u64)
                                as u32;

                            // Query old triples for condensation
                            match self.memory.triples_older_than(window_days, 500) {
                                Ok(candidates) if !candidates.is_empty() => {
                                    let total = candidates.len();
                                    // Group by entity
                                    let mut groups: BTreeMap<String, Vec<&Triple>> =
                                        BTreeMap::new();
                                    for t in &candidates {
                                        groups.entry(t.entity.clone()).or_default().push(t);
                                    }

                                    let mut condensed_count = 0usize;
                                    let mut summary_count = 0usize;

                                    for (entity, group) in &groups {
                                        if group.len() < 2 {
                                            // Single triple per entity — nothing to condense
                                            continue;
                                        }

                                        // Keep the best: highest confidence, then most recent valid_from
                                        let mut sorted: Vec<&&Triple> = group.iter().collect();
                                        sorted.sort_by(|a, b| {
                                            b.confidence
                                                .value()
                                                .partial_cmp(&a.confidence.value())
                                                .unwrap_or(std::cmp::Ordering::Equal)
                                                .then_with(|| {
                                                    b.temporal
                                                        .valid_from
                                                        .cmp(&a.temporal.valid_from)
                                                })
                                        });

                                        // Best triple is kept; rest are soft-deleted
                                        let condensed_ids: Vec<String> =
                                            sorted[1..].iter().map(|t| t.id.to_string()).collect();

                                        for id_str in &condensed_ids {
                                            let tid: hkask_storage::TripleID = match id_str.parse()
                                            {
                                                Ok(id) => id,
                                                Err(e) => {
                                                    tracing::debug!(
                                                        target: "cns.semantic",
                                                        triple_id = %id_str,
                                                        entity = %entity,
                                                        error = %e,
                                                        "Failed to parse condensed triple ID"
                                                    );
                                                    continue;
                                                }
                                            };
                                            if let Err(e) = self.memory.close_triple(&tid) {
                                                tracing::debug!(
                                                    target: "cns.semantic",
                                                    triple_id = %id_str,
                                                    entity = %entity,
                                                    error = %e,
                                                    "Failed to soft-delete condensed triple"
                                                );
                                            } else {
                                                condensed_count += 1;
                                            }
                                        }

                                        // Store provenance summary
                                        let summary = Triple::new(
                                            entity,
                                            "condensed_summary",
                                            serde_json::json!({
                                                "condensed_from": condensed_ids,
                                                "condensed_at": chrono::Utc::now().to_rfc3339(),
                                                "original_count": group.len(),
                                                "kept_triple_id": sorted[0].id.to_string(),
                                            }),
                                            sorted[0].access.owner_webid,
                                        )
                                        .with_confidence(CONDENSED_SUMMARY_CONFIDENCE);

                                        if let Err(e) = self.memory.store_consolidated(summary) {
                                            tracing::debug!(
                                                target: "cns.semantic",
                                                entity = %entity,
                                                error = %e,
                                                "Failed to store condensation summary"
                                            );
                                        } else {
                                            summary_count += 1;
                                        }
                                    }

                                    tracing::info!(
                                        target: "cns.semantic",
                                        total_candidates = total,
                                        condensed = condensed_count,
                                        summaries = summary_count,
                                        entities = groups.len(),
                                        window_days = window_days,
                                        "Semantic condensation completed"
                                    );

                                    self.emit_cns(
                                        "semantic_condensed",
                                        serde_json::json!({
                                            "total_candidates": total,
                                            "condensed": condensed_count,
                                            "summaries_stored": summary_count,
                                            "entity_groups": groups.len(),
                                            "window_days": window_days,
                                        }),
                                    );
                                }
                                Ok(_) => {
                                    tracing::debug!(
                                        target: "cns.semantic",
                                        window_days = window_days,
                                        "No old semantic triples found for condensation"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        target: "cns.semantic",
                                        error = %e,
                                        "Failed to query old semantic triples for condensation"
                                    );
                                }
                            }
                        }
                        _ => {
                            tracing::info!(
                                target: "cns.semantic",
                                action_type = ?action.action_type,
                                target_loop = %action.target,
                                "Semantic Loop calibration action"
                            );
                        }
                    }
                }
                _ => {
                    tracing::info!(
                        target: "cns.semantic",
                        action_type = ?action.action_type,
                        target_loop = %action.target,
                        "Semantic Loop regulatory action"
                    );
                }
            }
        }
    }
}
