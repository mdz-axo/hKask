//! Metacognition Loop — Curator's periodic system governance
//!
//! The Curator performs metacognition on system performance:
//! - Queries CNS spans for health metrics
//! - Checks variety counters (algedonic alerts if deficit > 100)
//! - Collects bot status reports from standing session
//! - Synthesizes system state updates
//! - Triggers escalations when thresholds are exceeded
//! - Posts summaries to standing session

use crate::adapters::MetacognitionStoreAdapter;
use crate::curator::context::CuratorContext;
#[allow(deprecated)]
use crate::ports::metacognition::StoredHealthSnapshot;
use crate::ports::metacognition::{
    BotDirective, EvaluationResult, KataDirective, KataType, RecommendedAction,
};
use hkask_cns::algedonic::CnsHealth;
use hkask_cns::bot_metrics::{
    BotEvaluationMetrics, BotHealthStatus as CnsBotHealthStatus, GapType,
};
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::TraceId;
use hkask_types::{BotID, WebID};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum MetacognitionError {
    #[error("Escalation error: {0}")]
    Escalation(String),
    #[error("CNS query error: {0}")]
    CnsQuery(String),
}

/// Escalation trigger thresholds
#[derive(Debug, Clone)]
pub struct EscalationThresholds {
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
    pub bot_status_reports: Vec<BotStatusReport>,
}

/// Bot status report from standing session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BotStatusReport {
    pub bot_name: String,
    pub status: BotHealthStatus,
    pub last_report: Option<chrono::DateTime<chrono::Utc>>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Metacognition loop configuration
#[derive(Debug, Clone)]
pub struct MetacognitionConfig {
    /// Interval between metacognition cycles (default: 1 hour)
    pub interval: Duration,
    /// Escalation thresholds
    pub thresholds: EscalationThresholds,
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

/// Metacognition loop — Curator's system governance mechanism
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
    store: Option<Arc<MetacognitionStoreAdapter>>,
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
            store: None,
        }
    }

    pub fn with_store(mut self, store: Arc<MetacognitionStoreAdapter>) -> Self {
        self.store = Some(store);
        self
    }

    /// Submit a bot status report
    pub async fn submit_bot_report(&self, report: BotStatusReport) {
        let mut reports = self.bot_reports.write().await;
        // Replace existing report for this bot
        if let Some(existing) = reports.iter_mut().find(|r| r.bot_name == report.bot_name) {
            *existing = report;
        } else {
            reports.push(report);
        }
    }

    /// Get current bot status reports
    pub async fn get_bot_reports(&self) -> Vec<BotStatusReport> {
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

        self.check_escalation_triggers(&snapshot).await?;

        if let Some(ref store) = self.store {
            #[allow(deprecated)]
            let stored: StoredHealthSnapshot = snapshot.clone().into();
            if let Err(e) = store.save_snapshot(&stored) {
                warn!(
                    target: "curator.metacognition",
                    error = %e,
                    "Failed to persist metacognition snapshot"
                );
            }
        }

        info!(
            target: "curator.metacognition",
            health = %snapshot.cns_health,
            critical_alerts = snapshot.critical_alerts,
            bot_reports = snapshot.bot_status_reports.len(),
            "Metacognition cycle complete"
        );

        Ok(snapshot)
    }

    /// Check escalation triggers and post escalations if needed
    async fn check_escalation_triggers(
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

    /// Evaluate a single bot's performance using its metrics
    pub fn evaluate_bot(&self, bot_id: &WebID, metrics: &BotEvaluationMetrics) -> EvaluationResult {
        let health = metrics.health_status();
        let gaps = metrics.capability_gaps(0.8, 100);

        let recommended_action = if gaps.is_empty() {
            RecommendedAction::None
        } else if gaps
            .iter()
            .any(|g| g.gap_type == GapType::SovereigntyViolations)
        {
            RecommendedAction::Escalate
        } else if gaps.iter().any(|g| g.gap_type == GapType::VarietyDeficit) {
            if metrics.success_rate < 0.5 {
                RecommendedAction::Coach(KataType::Improvement)
            } else {
                RecommendedAction::Coach(KataType::Coaching)
            }
        } else if gaps.iter().any(|g| g.gap_type == GapType::LowSuccessRate) {
            RecommendedAction::Coach(KataType::Starter)
        } else {
            RecommendedAction::Monitor
        };

        EvaluationResult {
            bot_id: *bot_id,
            bot_name: metrics.bot_name.clone(),
            health,
            gaps,
            recommended_action,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Identify a capability gap and create a Kata directive
    pub fn identify_capability_gap(&self, evaluation: &EvaluationResult) -> Option<KataDirective> {
        let primary_gap = evaluation.gaps.first()?;

        let kata_type = match primary_gap.gap_type {
            GapType::LowSuccessRate => {
                if evaluation.health == CnsBotHealthStatus::Critical {
                    KataType::Improvement
                } else {
                    KataType::Starter
                }
            }
            GapType::VarietyDeficit => KataType::Coaching,
            GapType::SovereigntyViolations => KataType::Coaching,
            GapType::EnergyBudgetCritical => KataType::Starter,
        };

        Some(KataDirective {
            bot_id: evaluation.bot_id,
            bot_name: evaluation.bot_name.clone(),
            kata_type,
            gap_description: primary_gap.description.clone(),
            gap: primary_gap.clone(),
        })
    }

    /// Direct a bot to take action via ACP message
    pub async fn direct_bot(&self, directive: BotDirective) -> Result<(), MetacognitionError> {
        info!(
            target: "curator.metacognition",
            bot = %directive.bot_name,
            directive_type = ?directive.directive_type,
            "Directing bot"
        );

        // The actual ACP message delivery happens through the standing session
        // which is wired in the bootstrap sequence. For now, log the directive.
        // The standing session integration (Task 2) will deliver this.

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
