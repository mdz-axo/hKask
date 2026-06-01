//! Memory Feedback Adapter — Closes the feedback loop from memory recall relevance
//! back to template selection.
//!
//! `MemoryFeedbackAdapter` bridges the gap between `AppMemoryAdapter`'s one-directional
//! query access and the template engine. It consumes `MemoryFragment.confidence` (computed
//! by bayesian `combine`) and tracks a running weighted average of recall quality per
//! template, which can be queried later to influence template selection.

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
/// Tracks a running weighted average of recall quality per template, which can be queried
/// later to influence template selection.
pub struct MemoryFeedbackAdapter {
    /// Per-template relevance statistics: template_id → running stats.
    relevance: Arc<Mutex<HashMap<String, RelevanceStats>>>,
}

impl MemoryFeedbackAdapter {
    pub fn new() -> Self {
        Self {
            relevance: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record a relevance observation from memory recall.
    ///
    /// Updates the running weighted average of recall quality for the given template.
    ///
    /// # Arguments
    /// * `entity` — The entity key that was queried in memory.
    /// * `recall_count` — Number of memory fragments returned for this entity.
    /// * `avg_confidence` — Average confidence across those fragments.
    /// * `template_id` — The template that triggered the memory query.
    pub async fn record_relevance(
        &self,
        _entity: &str,
        recall_count: usize,
        avg_confidence: f64,
        template_id: &str,
    ) {
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
