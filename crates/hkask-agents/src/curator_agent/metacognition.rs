//! Metacognition Loop — Curator Agent's periodic system governance
//!
//! The Curator Agent performs metacognition on system performance:
//! - Queries CNS spans for health metrics
//! - Checks variety counters (algedonic alerts if deficit > 100)
//! - Collects bot status reports from standing session
//! - Synthesizes system state updates
//! - Triggers escalations when thresholds are exceeded
//! - Posts summaries to standing session
//!
//! Moved from `curator::metacognition` as part of the Curation/Agent separation:
//! metacognition is a persona concern (the Curator Agent observes and adapts),
//! not a regulatory concern (the Curation Loop regulates).

use crate::curator::context::CuratorContext;
use crate::curator_agent::bot_metrics::BotHealthStatus;
use hkask_types::BotID;
use hkask_types::cns::CnsHealth;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::TraceId;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum MetacognitionError {
    #[error("Escalation error: {0}")]
    Escalation(String),
}

/// Escalation trigger thresholds
#[derive(Debug, Clone)]
pub(crate) struct EscalationThresholds {
    /// Variety deficit threshold (default: 100)
    pub variety_deficit: u64,
    /// Critical alert count threshold (default: 3)
    pub critical_alerts: usize,
    /// Bot failure count threshold (default: 2)
    pub bot_failures: usize,
}

impl Default for EscalationThresholds {
    fn default() -> Self {
        Self {
            variety_deficit: 100,
            critical_alerts: 3,
            bot_failures: 2,
        }
    }
}

/// Health snapshot — unified type for system health state.
///
/// Collapses the former `SystemHealthSnapshot` and `StoredHealthSnapshot` into
/// a single type with rich types. `StoredHealthSnapshot` is deprecated in favor
/// of this type; use `From<HealthSnapshot> for StoredHealthSnapshot` for
/// storage-layer conversion.
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cns_health: String,
    pub variety_counters: Vec<(String, u64)>,
    pub critical_alerts: usize,
    pub total_alerts: usize,
    pub(crate) bot_status_reports: Vec<BotStatusReport>,
}

/// Bot status report from standing session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct BotStatusReport {
    pub bot_name: String,
    pub status: BotHealthStatus,
    pub last_report: Option<chrono::DateTime<chrono::Utc>>,
    pub issues: Vec<String>,
}

/// Metacognition loop configuration
#[derive(Debug, Clone)]
pub struct MetacognitionConfig {
    /// Interval between metacognition cycles (default: 1 hour)
    pub interval: Duration,
    /// Escalation thresholds
    pub(crate) thresholds: EscalationThresholds,
    /// Expected variety per domain (for deficit calculation)
    pub expected_variety_per_domain: u64,
}

impl Default for MetacognitionConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(3600), // 1 hour
            thresholds: EscalationThresholds::default(),
            expected_variety_per_domain: 50,
        }
    }
}

/// Metacognition loop — Curator Agent's system governance mechanism
///
/// Uses `CuratorContext` for capability-disciplined access to all
/// Curation subloops. The context provides:
/// - CNS governance writes (threshold calibration)
/// - Message dispatch (inter-loop directive delivery)
/// - Escalation queue (human review routing)
pub struct MetacognitionLoop {
    context: Arc<CuratorContext>,
    config: MetacognitionConfig,
    bot_reports: Arc<RwLock<Vec<BotStatusReport>>>,
}

impl MetacognitionLoop {
    /// Create a new metacognition loop with a CuratorContext.
    ///
    /// Uses `CuratorContext` which provides capability-disciplined access
    /// to all Curation subloops: CNS governance writes (threshold calibration),
    /// message dispatch (inter-loop directives), and escalation queue
    /// (human review routing).
    pub fn new(context: Arc<CuratorContext>, config: MetacognitionConfig) -> Self {
        Self {
            context,
            config,
            bot_reports: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Access the CuratorContext (capability-disciplined runtime references).
    pub(crate) fn context(&self) -> &Arc<CuratorContext> {
        &self.context
    }

    /// Access the metacognition config (thresholds, intervals).
    pub(crate) fn config(&self) -> &MetacognitionConfig {
        &self.config
    }

    /// Get current bot status reports
    pub(crate) async fn get_bot_reports(&self) -> Vec<BotStatusReport> {
        self.bot_reports.read().await.clone()
    }

    pub async fn run_cycle(&self) -> Result<HealthSnapshot, MetacognitionError> {
        info!(target: "curator.metacognition", "Starting metacognition cycle");

        let cns_health = self.context.cns().health().await;
        let cns_health_str = format_health_status(&cns_health);

        let variety_counters = self.context.cns().variety().await;

        let all_alerts = self.context.cns().alerts().await;
        let critical_alerts = self.context.cns().critical_alerts().await;

        let bot_reports = self.get_bot_reports().await;

        let snapshot = HealthSnapshot {
            timestamp: chrono::Utc::now(),
            cns_health: cns_health_str,
            variety_counters: variety_counters.clone(),
            critical_alerts: critical_alerts.len(),
            total_alerts: all_alerts.len(),
            bot_status_reports: bot_reports.clone(),
        };

        self.evaluate_and_adapt(&snapshot).await?;

        Ok(snapshot)
    }

    /// Metacognitive Adaptation (ADAPT) — evaluate system state and adjust.
    ///
    /// This implements the merged Metacognitive Adaptation subloop (5.2):
    /// - Evaluate variety deficit → calibrate thresholds
    /// - Evaluate critical alerts → escalate for human review
    /// - Evaluate bot health → escalate failed bots
    ///
    /// All three are the same ADAPT primitive: outcome → compare → adjust.
    async fn evaluate_and_adapt(
        &self,
        snapshot: &HealthSnapshot,
    ) -> Result<(), MetacognitionError> {
        // Check variety deficit
        let mut total_variety_deficit = 0u64;
        for (domain, variety) in &snapshot.variety_counters {
            let deficit = self
                .config
                .expected_variety_per_domain
                .saturating_sub(*variety);
            if deficit > 0 {
                total_variety_deficit += deficit;
                if deficit > self.config.thresholds.variety_deficit {
                    warn!(
                        target: "curator.metacognition",
                        domain = %domain,
                        variety = variety,
                        deficit = deficit,
                        "Variety deficit exceeds threshold"
                    );
                }
            }
        }

        if total_variety_deficit > self.config.thresholds.variety_deficit {
            let template_id = hkask_types::TemplateID::new();
            let bot_id = BotID::new();
            let error_context = format!(
                "Total variety deficit ({}) exceeds threshold ({})",
                total_variety_deficit, self.config.thresholds.variety_deficit
            );

            self.context
                .escalation_queue()
                .add(
                    template_id,
                    bot_id,
                    format!("Variety deficit: {}", total_variety_deficit),
                    0.6,
                    0,
                    error_context,
                )
                .map_err(|e| MetacognitionError::Escalation(e.to_string()))?;

            // Issue CalibrateThreshold directive through dispatch (5.3 Threshold Calibration)
            let directive = CuratorDirective::CalibrateThreshold {
                domain: "variety".to_string(),
                new_threshold: total_variety_deficit
                    .saturating_add(self.config.thresholds.variety_deficit),
            };
            self.issue_directive(directive).await;

            // Calibrate CNS threshold directly (5.3 ADAPT subloop)
            self.context
                .cns()
                .calibrate_threshold(
                    "variety",
                    total_variety_deficit.saturating_add(self.config.thresholds.variety_deficit),
                )
                .await;
        }

        // Check critical alerts
        if snapshot.critical_alerts >= self.config.thresholds.critical_alerts {
            warn!(
                target: "curator.metacognition",
                critical_alerts = snapshot.critical_alerts,
                threshold = self.config.thresholds.critical_alerts,
                "Critical alert count exceeds threshold"
            );

            // Post escalation for critical alerts
            let template_id = hkask_types::TemplateID::new();
            let bot_id = BotID::new(); // System-level escalation
            let error_context = format!(
                "Critical alert count ({}) exceeds threshold ({})",
                snapshot.critical_alerts, self.config.thresholds.critical_alerts
            );

            self.context
                .escalation_queue()
                .add(
                    template_id,
                    bot_id,
                    format!("System has {} critical alerts", snapshot.critical_alerts),
                    0.3, // Low confidence — needs human review
                    0,
                    error_context,
                )
                .map_err(|e| MetacognitionError::Escalation(e.to_string()))?;
        }

        // Check bot failures
        let failed_bots: Vec<_> = snapshot
            .bot_status_reports
            .iter()
            .filter(|r| r.status == BotHealthStatus::Critical)
            .collect();

        if failed_bots.len() >= self.config.thresholds.bot_failures {
            warn!(
                target: "curator.metacognition",
                failed_bots = failed_bots.len(),
                threshold = self.config.thresholds.bot_failures,
                "Bot failure count exceeds threshold"
            );

            // Post escalation for bot failures
            let template_id = hkask_types::TemplateID::new();
            let bot_id = BotID::new();
            let error_context = format!(
                "{} bots in critical state: {}",
                failed_bots.len(),
                failed_bots
                    .iter()
                    .map(|b| b.bot_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            self.context
                .escalation_queue()
                .add(
                    template_id,
                    bot_id,
                    format!("{} bots require attention", failed_bots.len()),
                    0.4,
                    0,
                    error_context,
                )
                .map_err(|e| MetacognitionError::Escalation(e.to_string()))?;
        }

        Ok(())
    }

    /// Generate a system state summary for posting to standing session
    pub fn generate_summary(&self, snapshot: &HealthSnapshot) -> String {
        let mut summary = String::new();
        summary.push_str("## Metacognition Update\n\n");
        summary.push_str(&format!(
            "**Timestamp:** {}\n",
            snapshot.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        summary.push_str(&format!("**CNS Health:** {}\n", snapshot.cns_health));
        summary.push_str(&format!(
            "**Critical Alerts:** {}\n",
            snapshot.critical_alerts
        ));
        summary.push_str(&format!("**Total Alerts:** {}\n\n", snapshot.total_alerts));

        if !snapshot.variety_counters.is_empty() {
            summary.push_str("### Variety Counters\n");
            for (domain, variety) in &snapshot.variety_counters {
                summary.push_str(&format!("- {}: {}\n", domain, variety));
            }
            summary.push('\n');
        }

        if !snapshot.bot_status_reports.is_empty() {
            summary.push_str("### Bot Status\n");
            for report in &snapshot.bot_status_reports {
                summary.push_str(&format!("- **{}**: {}", report.bot_name, report.status));
                if !report.issues.is_empty() {
                    summary.push_str(&format!(" ({})", report.issues.join(", ")));
                }
                summary.push('\n');
            }
        }

        summary
    }

    // -----------------------------------------------------------------------
    // Curator metacognition: evaluate, coach, direct
    // -----------------------------------------------------------------------

    /// Direct a bot to take action via ACP message
    pub async fn direct_bot(
        &self,
        _bot_name: &str,
        _reason: &str,
    ) -> Result<(), MetacognitionError> {
        // The actual ACP message delivery happens through the standing session
        // which is wired in the bootstrap sequence.
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Directive issuance — Curation → Governance/Observability
    // -----------------------------------------------------------------------

    /// Issue a CuratorDirective through the message dispatch with DAMPEN filtering.
    ///
    /// Delegates to `CuratorContext::issue_directive()` which:
    /// 1. Checks the dampener (6.3 DAMPEN) for repeated directives
    /// 2. If dampened, returns `None` without sending
    /// 3. If not dampened, sends through dispatch and returns the `TraceId`
    ///
    /// # Subloops served
    ///
    /// - 5.2 Bot Evaluation / Kata Coaching (ADAPT) — UpdateCapabilities
    /// - 5.3 Threshold Calibration (ADAPT) — CalibrateThreshold
    /// - Energy budget adjustment — AdjustEnergyBudget
    /// - 6.3 DAMPEN — Suppresses repeated directives within time window
    pub async fn issue_directive(&self, directive: CuratorDirective) -> Option<TraceId> {
        self.context.issue_directive(directive).await
    }
}

fn format_health_status(h: &CnsHealth) -> String {
    if h.healthy {
        format!(
            "Healthy (deficit={}, warnings={})",
            h.overall_deficit, h.warning_count
        )
    } else {
        format!(
            "Degraded (deficit={}, critical={}, warnings={})",
            h.overall_deficit, h.critical_count, h.warning_count
        )
    }
}
