//! Bot Health Status — Per-bot health classification
//!
//! BotHealthStatus is used by the Curator Agent to filter critical bots
//! and by BotStatusReport for per-bot status tracking.
//!
//! Moved from `curator::bot_metrics` as part of the Curation/Agent separation:
//! bot health status is a persona concern (Curator reads bot status for
//! escalation), not a regulatory concern.

use serde::{Deserialize, Serialize};

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
