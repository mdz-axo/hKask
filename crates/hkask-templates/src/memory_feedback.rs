//! Memory Feedback Adapter — Closes the feedback loop from memory recall relevance
//! back to template selection.
//!
//! `MemoryFeedbackAdapter` bridges the gap between `AppMemoryAdapter`'s one-directional
//! query access and the template engine. It consumes `MemoryFragment.confidence` (computed
//! by bayesian `combine`) and emits `cns.pipeline.relevance` spans, enabling `CuratorPipeline`
//! and the template engine to factor recall quality into future selections.

use hkask_cns::spans::SpanEmitter;
use hkask_types::{Phase, Span};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Running relevance statistics per template, used for the weighted-average computation.
#[derive(Debug, Clone, Default)]
struct RelevanceStats {
    /// Weighted sum of (confidence × recall_count) so far.
    weighted_sum: f64,
    /// Total recall count across all observations.
    total_recalls: usize,
}

/// Adapter that closes the feedback loop from memory recall relevance to template selection.
///
/// Emits `cns.pipeline.relevance` spans through CNS and tracks a running weighted average
/// of recall quality per template, which can be queried later to influence template selection.
pub struct MemoryFeedbackAdapter {
    span_emitter: SpanEmitter,
    /// Per-template relevance statistics: template_id → running stats.
    relevance: Arc<Mutex<HashMap<String, RelevanceStats>>>,
}

impl MemoryFeedbackAdapter {
    /// Create a new adapter from a `SpanEmitter` (CNS observability) and an
    /// `AppMemoryAdapter` reference (the read-side is not stored here — callers
    /// query it themselves and pass results to `record_relevance`).
    pub fn new(span_emitter: SpanEmitter) -> Self {
        Self {
            span_emitter,
            relevance: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record a relevance observation from memory recall.
    ///
    /// Emits a `cns.pipeline.relevance` span via CNS and updates the running
    /// weighted average of recall quality for the given template.
    ///
    /// # Arguments
    /// * `entity` — The entity key that was queried in memory.
    /// * `recall_count` — Number of memory fragments returned for this entity.
    /// * `avg_confidence` — Average confidence across those fragments.
    /// * `template_id` — The template that triggered the memory query.
    pub async fn record_relevance(
        &self,
        entity: &str,
        recall_count: usize,
        avg_confidence: f64,
        template_id: &str,
    ) {
        // Emit CNS span
        self.span_emitter.emit_with_phase(
            Span::pipeline("relevance"),
            Phase::Observe,
            serde_json::json!({
                "entity": entity,
                "recall_count": recall_count,
                "avg_confidence": avg_confidence,
                "template_id": template_id,
            }),
        );

        // Update running weighted average: new_quality = Σ(confidence × count) / Σ(count)
        let mut relevance = self.relevance.lock().await;
        let stats = relevance.entry(template_id.to_string()).or_default();
        stats.weighted_sum += avg_confidence * recall_count as f64;
        stats.total_recalls += recall_count;
    }

    /// Get the accumulated relevance score for a template.
    ///
    /// Returns the weighted average confidence across all recorded observations
    /// for this template. Defaults to `1.0` (neutral/positive) if no data has
    /// been recorded yet — this ensures templates with no feedback are not
    /// penalised.
    pub async fn get_template_relevance(&self, template_id: &str) -> f64 {
        let relevance = self.relevance.lock().await;
        match relevance.get(template_id) {
            Some(stats) if stats.total_recalls > 0 => {
                stats.weighted_sum / stats.total_recalls as f64
            }
            _ => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::WebID;

    #[tokio::test]
    async fn test_default_relevance_is_one() {
        let emitter = SpanEmitter::new(WebID::new());
        let adapter = MemoryFeedbackAdapter::new(emitter);

        let score = adapter.get_template_relevance("test-template").await;
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_record_and_retrieve_relevance() {
        let emitter = SpanEmitter::new(WebID::new());
        let adapter = MemoryFeedbackAdapter::new(emitter);

        // Record: entity "foo", 10 fragments at avg confidence 0.8
        adapter.record_relevance("foo", 10, 0.8, "tmpl-a").await;

        let score = adapter.get_template_relevance("tmpl-a").await;
        assert!((score - 0.8).abs() < 1e-9);

        // Record again: entity "bar", 5 fragments at avg confidence 0.6
        adapter.record_relevance("bar", 5, 0.6, "tmpl-a").await;

        // Weighted average: (0.8*10 + 0.6*5) / (10+5) = (8+3) / 15 ≈ 0.7333
        let score = adapter.get_template_relevance("tmpl-a").await;
        let expected = (0.8 * 10.0 + 0.6 * 5.0) / 15.0;
        assert!((score - expected).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_independent_template_scores() {
        let emitter = SpanEmitter::new(WebID::new());
        let adapter = MemoryFeedbackAdapter::new(emitter);

        adapter.record_relevance("x", 2, 0.9, "tmpl-x").await;
        adapter.record_relevance("y", 3, 0.5, "tmpl-y").await;

        let score_x = adapter.get_template_relevance("tmpl-x").await;
        let score_y = adapter.get_template_relevance("tmpl-y").await;

        assert!((score_x - 0.9).abs() < 1e-9);
        assert!((score_y - 0.5).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_zero_recall_count_defaults_to_one() {
        let emitter = SpanEmitter::new(WebID::new());
        let adapter = MemoryFeedbackAdapter::new(emitter);

        // Zero recall count — the weighted sum contribution is 0, but total_recalls stays 0
        // so the branch falls through to the default (1.0).
        adapter.record_relevance("z", 0, 0.0, "tmpl-z").await;

        let score = adapter.get_template_relevance("tmpl-z").await;
        // total_recalls is 0 (no fragments), so defaults to 1.0
        assert!((score - 1.0).abs() < f64::EPSILON);
    }
}
