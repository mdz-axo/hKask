//! Bot Evaluation Metrics — Per-bot CNS variety counters
//!
//! BotEvaluationMetrics, BotHealthStatus, CapabilityGap, and GapType are
//! the data contract consumed by Curation's metacognition loop.
//! All bot metric collection is handled by `UnifiedVarietyTracker`
//! (in the `unified_tracker` module).

use hkask_types::WebID;
use serde::{Deserialize, Serialize};

/// Per-bot evaluation metrics for Curator metacognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BotEvaluationMetrics {
    /// The bot's WebID
    pub bot_id: WebID,
    /// Human-readable bot name (e.g., "R7.3")
    pub bot_name: String,
    /// Success rate (0.0 to 1.0) — ratio of Outcome phases to total events
    pub success_rate: f64,
    /// Variety deficit (expected variety minus actual variety)
    pub variety_deficit: u64,
    /// Number of sovereignty violations by this bot
    pub sovereignty_violations: u32,
}

impl BotEvaluationMetrics {
    /// Create new empty metrics for a bot
    pub fn new(bot_id: WebID, bot_name: String) -> Self {
        Self {
            bot_id,
            bot_name,
            success_rate: 0.0,
            variety_deficit: 0,
            sovereignty_violations: 0,
        }
    }

    /// Determine bot health status based on metrics
    pub fn health_status(&self) -> BotHealthStatus {
        if self.sovereignty_violations >= 3 || self.variety_deficit > 500 {
            BotHealthStatus::Critical
        } else if self.success_rate < 0.8 || self.variety_deficit > 100 {
            BotHealthStatus::Degraded
        } else {
            BotHealthStatus::Healthy
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
                description: format!(
                    "Sovereignty violations ({}) at or above escalation threshold (3)",
                    self.sovereignty_violations
                ),
            });
        }

        gaps
    }
}

/// Bot health status derived from evaluation metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum BotHealthStatus {
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
pub(crate) struct CapabilityGap {
    pub bot_id: WebID,
    pub gap_type: GapType,
    pub description: String,
}

/// Types of capability gaps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum GapType {
    LowSuccessRate,
    VarietyDeficit,
    SovereigntyViolations,
    EnergyBudgetCritical,
}
