//! Bot Evaluation Metrics — Per-bot CNS variety counters
//!
//! Each R7 bot's pod emits spans with its own WebID as observer_id.
//! BotMetricsCollector groups NuEvent observations by observer_id and span
//! category, producing BotEvaluationMetrics on demand for the Curator's
//! metacognition loop.

use crate::spans::SpanCategory;
use crate::variety::VarietyTracker;
use chrono::{DateTime, Utc};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-bot evaluation metrics for Curator metacognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotEvaluationMetrics {
    /// The bot's WebID
    pub bot_id: WebID,
    /// Human-readable bot name (e.g., "R7.3")
    pub bot_name: String,
    /// Span counts per category
    pub span_counts: HashMap<SpanCategory, u64>,
    /// Success rate (0.0 to 1.0) — ratio of Outcome phases to total events
    pub success_rate: f64,
    /// Total energy consumed
    pub energy_consumed: u64,
    /// Energy budget allocated
    pub energy_budget: u64,
    /// Variety deficit (expected variety minus actual variety)
    pub variety_deficit: u64,
    /// Number of algedonic alerts received by this bot
    pub algedonic_alerts: u32,
    /// Number of sovereignty violations by this bot
    pub sovereignty_violations: u32,
    /// Last report timestamp
    pub last_report: DateTime<Utc>,
}

impl BotEvaluationMetrics {
    /// Create new empty metrics for a bot
    pub fn new(bot_id: WebID, bot_name: String) -> Self {
        Self {
            bot_id,
            bot_name,
            span_counts: HashMap::new(),
            success_rate: 0.0,
            energy_consumed: 0,
            energy_budget: 10_000, // default from manifests
            variety_deficit: 0,
            algedonic_alerts: 0,
            sovereignty_violations: 0,
            last_report: Utc::now(),
        }
    }

    /// Determine bot health status based on metrics
    pub fn health_status(&self) -> BotHealthStatus {
        if self.sovereignty_violations >= 3 || self.variety_deficit > 500 {
            BotHealthStatus::Critical
        } else if self.success_rate < 0.8
            || self.variety_deficit > 100
            || self.algedonic_alerts >= 2
        {
            BotHealthStatus::Degraded
        } else {
            BotHealthStatus::Healthy
        }
    }

    /// Calculate energy utilization ratio (0.0 to 1.0)
    pub fn energy_utilization(&self) -> f64 {
        if self.energy_budget == 0 {
            0.0
        } else {
            self.energy_consumed as f64 / self.energy_budget as f64
        }
    }

    /// Identify capability gaps based on thresholds
    pub fn capability_gaps(
        &self,
        success_threshold: f64,
        deficit_threshold: u64,
    ) -> Vec<CapabilityGap> {
        let mut gaps = Vec::new();

        if self.success_rate < success_threshold {
            gaps.push(CapabilityGap {
                bot_id: self.bot_id,
                gap_type: GapType::LowSuccessRate,
                current_value: self.success_rate,
                threshold: success_threshold,
                description: format!(
                    "Success rate {:.1}% below threshold {:.1}%",
                    self.success_rate * 100.0,
                    success_threshold * 100.0
                ),
            });
        }

        if self.variety_deficit > deficit_threshold {
            gaps.push(CapabilityGap {
                bot_id: self.bot_id,
                gap_type: GapType::VarietyDeficit,
                current_value: self.variety_deficit as f64,
                threshold: deficit_threshold as f64,
                description: format!(
                    "Variety deficit {} exceeds threshold {}",
                    self.variety_deficit, deficit_threshold
                ),
            });
        }

        if self.sovereignty_violations >= 3 {
            gaps.push(CapabilityGap {
                bot_id: self.bot_id,
                gap_type: GapType::SovereigntyViolations,
                current_value: self.sovereignty_violations as f64,
                threshold: 3.0,
                description: format!(
                    "Sovereignty violations ({}) at or above escalation threshold (3)",
                    self.sovereignty_violations
                ),
            });
        }

        if self.energy_utilization() > 0.9 {
            gaps.push(CapabilityGap {
                bot_id: self.bot_id,
                gap_type: GapType::EnergyBudgetCritical,
                current_value: self.energy_utilization(),
                threshold: 0.9,
                description: format!(
                    "Energy utilization {:.1}% exceeds 90% threshold",
                    self.energy_utilization() * 100.0
                ),
            });
        }

        gaps
    }
}

/// Bot health status derived from evaluation metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotHealthStatus {
    Healthy,
    Degraded,
    Critical,
}

impl std::fmt::Display for BotHealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotHealthStatus::Healthy => write!(f, "healthy"),
            BotHealthStatus::Degraded => write!(f, "degraded"),
            BotHealthStatus::Critical => write!(f, "critical"),
        }
    }
}

/// Capability gap identified by Curator metacognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGap {
    pub bot_id: WebID,
    pub gap_type: GapType,
    pub current_value: f64,
    pub threshold: f64,
    pub description: String,
}

/// Types of capability gaps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapType {
    LowSuccessRate,
    VarietyDeficit,
    SovereigntyViolations,
    EnergyBudgetCritical,
}

/// Collector for per-bot evaluation metrics
///
/// Aggregates NuEvent observations by observer_id and span category,
/// producing BotEvaluationMetrics on demand. Wired into CnsRuntime
/// alongside the existing VarietyMonitor.
pub struct BotMetricsCollector {
    /// Per-bot metrics, keyed by WebID
    metrics: HashMap<WebID, BotEvaluationMetrics>,
    /// Per-bot variety trackers, keyed by WebID
    variety_trackers: HashMap<WebID, VarietyTracker>,
    /// Per-bot success tracking (observe_count, outcome_count)
    success_tracking: HashMap<WebID, (u64, u64)>,
    /// Mapping from WebID to bot name
    bot_names: HashMap<WebID, String>,
    /// Expected variety per domain (default: 100)
    #[allow(dead_code)]
    expected_variety: u64,
}

impl BotMetricsCollector {
    /// Create a new collector with default expected variety
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            variety_trackers: HashMap::new(),
            success_tracking: HashMap::new(),
            bot_names: HashMap::new(),
            expected_variety: 100,
        }
    }

    /// Create a new collector with custom expected variety
    pub fn with_expected_variety(expected_variety: u64) -> Self {
        Self {
            metrics: HashMap::new(),
            variety_trackers: HashMap::new(),
            success_tracking: HashMap::new(),
            bot_names: HashMap::new(),
            expected_variety,
        }
    }

    /// Register a bot in the collector
    pub fn register_bot(&mut self, bot_id: WebID, bot_name: String) {
        self.bot_names.insert(bot_id, bot_name.clone());
        self.metrics
            .entry(bot_id)
            .or_insert_with(|| BotEvaluationMetrics::new(bot_id, bot_name));
        self.variety_trackers.entry(bot_id).or_default();
        self.success_tracking.entry(bot_id).or_insert((0, 0));
    }

    /// Record a span observation for a bot
    pub fn record_span(&mut self, bot_id: &WebID, category: SpanCategory) {
        if let Some(metrics) = self.metrics.get_mut(bot_id) {
            *metrics.span_counts.entry(category).or_insert(0) += 1;
            metrics.last_report = Utc::now();
        }
        if let Some(tracker) = self.variety_trackers.get_mut(bot_id) {
            tracker.increment(category.as_str());
        }
    }

    /// Record a success (Outcome phase) for a bot
    pub fn record_success(&mut self, bot_id: &WebID) {
        if let Some((_observe, outcome)) = self.success_tracking.get_mut(bot_id) {
            *outcome += 1;
        }
        self.update_success_rate(bot_id);
    }

    /// Record an observation (Observe phase) for a bot
    pub fn record_observation(&mut self, bot_id: &WebID) {
        if let Some((observe, _)) = self.success_tracking.get_mut(bot_id) {
            *observe += 1;
        }
        self.update_success_rate(bot_id);
    }

    /// Record energy consumption for a bot
    pub fn record_energy(&mut self, bot_id: &WebID, amount: u64) {
        if let Some(metrics) = self.metrics.get_mut(bot_id) {
            metrics.energy_consumed += amount;
            metrics.last_report = Utc::now();
        }
    }

    /// Set the energy budget for a bot
    pub fn set_energy_budget(&mut self, bot_id: &WebID, budget: u64) {
        if let Some(metrics) = self.metrics.get_mut(bot_id) {
            metrics.energy_budget = budget;
        }
    }

    /// Record an algedonic alert for a bot
    pub fn record_alert(&mut self, bot_id: &WebID) {
        if let Some(metrics) = self.metrics.get_mut(bot_id) {
            metrics.algedonic_alerts += 1;
            metrics.last_report = Utc::now();
        }
    }

    /// Record a sovereignty violation for a bot
    pub fn record_sovereignty_violation(&mut self, bot_id: &WebID) {
        if let Some(metrics) = self.metrics.get_mut(bot_id) {
            metrics.sovereignty_violations += 1;
            metrics.last_report = Utc::now();
        }
    }

    /// Get evaluation metrics for a specific bot
    pub fn evaluate(&self, bot_id: &WebID) -> Option<BotEvaluationMetrics> {
        self.metrics.get(bot_id).cloned()
    }

    /// Get evaluation metrics for all bots
    pub fn evaluate_all(&self) -> Vec<BotEvaluationMetrics> {
        self.metrics.values().cloned().collect()
    }

    /// Get the health status for a specific bot
    pub fn health_status(&self, bot_id: &WebID) -> Option<BotHealthStatus> {
        self.metrics.get(bot_id).map(|m| m.health_status())
    }

    /// Identify capability gaps for a specific bot
    pub fn identify_gaps(
        &self,
        bot_id: &WebID,
        success_threshold: f64,
        deficit_threshold: u64,
    ) -> Vec<CapabilityGap> {
        if let Some(metrics) = self.metrics.get(bot_id) {
            metrics.capability_gaps(success_threshold, deficit_threshold)
        } else {
            Vec::new()
        }
    }

    /// Identify capability gaps across all bots
    pub fn identify_all_gaps(
        &self,
        success_threshold: f64,
        deficit_threshold: u64,
    ) -> Vec<CapabilityGap> {
        self.metrics
            .values()
            .flat_map(|m| m.capability_gaps(success_threshold, deficit_threshold))
            .collect()
    }

    /// Update success rate for a bot from tracking data
    fn update_success_rate(&mut self, bot_id: &WebID) {
        if let Some((observe, outcome)) = self.success_tracking.get(bot_id)
            && let Some(metrics) = self.metrics.get_mut(bot_id)
            && *observe > 0
        {
            metrics.success_rate = *outcome as f64 / *observe as f64;
        }
    }
}

impl Default for BotMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
