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

use crate::acp::A2AMessage;
use crate::curator::context::CuratorContext;
use crate::curator_agent::bot_metrics::BotHealthStatus;
use hkask_types::BotID;
use hkask_types::WebID;
use hkask_types::cns::CnsHealth;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::TraceId;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal,
};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum MetacognitionError {
    #[error("Escalation error: {0}")]
    Escalation(String),
    #[error("ACP error: {0}")]
    Acp(String),
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
    /// Snapshot from the most recent sense() phase, used by compute()/act().
    last_snapshot: Arc<RwLock<Option<HealthSnapshot>>>,
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
            last_snapshot: Arc::new(RwLock::new(None)),
        }
    }

    /// Access the CuratorContext (capability-disciplined runtime references).
    #[allow(dead_code)] // Metacognition infrastructure
    pub(crate) fn context(&self) -> &Arc<CuratorContext> {
        &self.context
    }

    /// Access the metacognition config (thresholds, intervals).
    #[allow(dead_code)] // Metacognition infrastructure
    pub(crate) fn config(&self) -> &MetacognitionConfig {
        &self.config
    }

    /// Get current bot status reports
    pub(crate) async fn get_bot_reports(&self) -> Vec<BotStatusReport> {
        self.bot_reports.read().await.clone()
    }

    /// Run a full metacognition cycle and return the health snapshot.
    ///
    /// Convenience wrapper around `tick()` that also returns the
    /// `HealthSnapshot` produced during the sense phase.
    pub async fn run_cycle(&self) -> Result<HealthSnapshot, MetacognitionError> {
        info!(target: "curator.metacognition", "Starting metacognition cycle");
        self.tick().await;
        self.last_snapshot
            .read()
            .await
            .clone()
            .ok_or_else(|| MetacognitionError::Escalation("No snapshot available".to_string()))
    }

    /// Metacognitive Adaptation (ADAPT) — evaluate system state and adjust.
    ///
    /// This implements the merged Metacognitive Adaptation subloop (5.2):
    /// - Evaluate variety deficit → calibrate thresholds
    /// - Evaluate critical alerts → escalate for human review
    /// - Evaluate bot health → escalate failed bots
    ///
    /// All three are the same ADAPT primitive: outcome → compare → adjust.
    ///
    /// Note: logic extracted into `sense()/compute()/act()` via `HkaskLoop`.
    /// Retained as reference documentation of the ADAPT subloop pattern.
    #[allow(dead_code)]
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
    pub async fn direct_bot(&self, bot_name: &str, reason: &str) -> Result<(), MetacognitionError> {
        let acp = match self.context.acp() {
            Some(acp) => acp,
            None => {
                warn!(
                    target: "curator.metacognition",
                    bot = %bot_name,
                    "ACP port not configured — cannot direct bot"
                );
                return Ok(());
            }
        };

        let from = *self.context.handle().curator_id();
        let to = WebID::from_persona(bot_name.as_bytes());
        let correlation_id = format!("directive-{}-{}", bot_name, chrono::Utc::now().timestamp());

        let msg = A2AMessage::TemplateDispatch {
            from,
            to: Some(to),
            template_id: "directive".to_string(),
            input: serde_json::json!({ "reason": reason }),
            correlation_id,
        };

        acp.send_message(msg)
            .await
            .map_err(|e| MetacognitionError::Acp(e.to_string()))?;

        info!(
            target: "curator.metacognition",
            bot = %bot_name,
            reason = %reason,
            "Directive sent to bot via ACP"
        );

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
    /// - Energy budget adjustment — AdjustGasBudget
    /// - 6.3 DAMPEN — Suppresses repeated directives within time window
    pub async fn issue_directive(&self, directive: CuratorDirective) -> Option<TraceId> {
        self.context.issue_directive(directive).await
    }
}

// ---------------------------------------------------------------------------
// HkaskLoop — sense → compare → compute → act
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl HkaskLoop for MetacognitionLoop {
    fn id(&self) -> LoopId {
        LoopId::Metacognition
    }

    /// Sense: read CNS health, variety counters, critical alerts, and bot status.
    ///
    /// Produces `Signal`s for metrics that the metacognition loop monitors:
    /// - `variety_deficit` — total deficit across all domains vs threshold
    /// - `critical_alerts` — count of critical CNS alerts vs threshold
    /// - `bot_failures` — count of bots in Critical health vs threshold
    ///
    /// Also builds and stores a `HealthSnapshot` for use by compute/act phases.
    async fn sense(&self) -> Vec<Signal> {
        info!(target: "curator.metacognition", "Starting metacognition sense phase");

        let cns_health = self.context.cns().health().await;
        let cns_health_str = format_health_status(&cns_health);

        let variety_counters = self.context.cns().variety().await;
        let all_alerts = self.context.cns().alerts().await;
        let critical_alerts = self.context.cns().critical_alerts().await;
        let bot_reports = self.get_bot_reports().await;

        // Compute total variety deficit (same logic as evaluate_and_adapt)
        let mut total_variety_deficit = 0u64;
        for (domain, variety) in &variety_counters {
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

        // Compute bot failure count
        let failed_bot_count = bot_reports
            .iter()
            .filter(|r| r.status == BotHealthStatus::Critical)
            .count();

        // Build and store snapshot for compute/act phases
        let snapshot = HealthSnapshot {
            timestamp: chrono::Utc::now(),
            cns_health: cns_health_str,
            variety_counters: variety_counters.clone(),
            critical_alerts: critical_alerts.len(),
            total_alerts: all_alerts.len(),
            bot_status_reports: bot_reports.clone(),
        };
        {
            let mut last = self.last_snapshot.write().await;
            *last = Some(snapshot);
        }

        // Produce afferent signals
        let signals = vec![
            // Variety deficit: act when total_deficit > threshold (strict >)
            Signal::new(
                LoopId::Metacognition,
                "variety_deficit",
                total_variety_deficit as f64,
                self.config.thresholds.variety_deficit as f64,
            ),
            // Critical alerts: act when count >= threshold.
            // Use threshold - 0.5 as set-point so that count == threshold
            // produces an AboveSetPoint deviation.
            Signal::new(
                LoopId::Metacognition,
                "critical_alerts",
                critical_alerts.len() as f64,
                self.config.thresholds.critical_alerts as f64 - 0.5,
            ),
            // Bot failures: act when count >= threshold.
            // Same threshold - 0.5 technique as critical_alerts.
            Signal::new(
                LoopId::Metacognition,
                "bot_failures",
                failed_bot_count as f64,
                self.config.thresholds.bot_failures as f64 - 0.5,
            ),
        ];

        signals
    }

    /// Compute: produce `LoopAction`s for detected deviations.
    ///
    /// Maps deviations to regulatory actions:
    /// - `variety_deficit` AboveSetPoint → Calibrate action (threshold adjustment)
    /// - `critical_alerts` AboveSetPoint → Escalate action (human review)
    /// - `bot_failures` AboveSetPoint → Escalate action (bot attention)
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            match dev.signal.metric.as_str() {
                "variety_deficit" if dev.direction == DeviationDirection::AboveSetPoint => {
                    let deficit = dev.signal.value as u64;
                    let new_threshold =
                        deficit.saturating_add(self.config.thresholds.variety_deficit);
                    actions.push(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Calibrate,
                        serde_json::json!({
                            "domain": "variety",
                            "deficit": deficit,
                            "threshold": dev.signal.set_point as u64,
                            "new_threshold": new_threshold,
                        }),
                    ));
                }
                "critical_alerts" if dev.direction == DeviationDirection::AboveSetPoint => {
                    let count = dev.signal.value as u64;
                    actions.push(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({
                            "metric": "critical_alerts",
                            "count": count,
                            "threshold": self.config.thresholds.critical_alerts,
                        }),
                    ));
                }
                "bot_failures" if dev.direction == DeviationDirection::AboveSetPoint => {
                    let count = dev.signal.value as u64;
                    // Retrieve bot names from the stored snapshot
                    let bot_names: Vec<String> = self
                        .last_snapshot
                        .read()
                        .await
                        .as_ref()
                        .map(|s| {
                            s.bot_status_reports
                                .iter()
                                .filter(|r| r.status == BotHealthStatus::Critical)
                                .map(|r| r.bot_name.clone())
                                .collect()
                        })
                        .unwrap_or_default();
                    actions.push(LoopAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        serde_json::json!({
                            "metric": "bot_failures",
                            "failed_count": count,
                            "threshold": self.config.thresholds.bot_failures,
                            "bot_names": bot_names,
                        }),
                    ));
                }
                _ => {} // BelowSetPoint deviations are in the desired direction
            }
        }

        actions
    }

    /// Act: execute regulatory actions by issuing CuratorDirectives and
    /// posting escalations.
    ///
    /// For each `LoopAction`:
    /// - `Calibrate` → issue `CuratorDirective::CalibrateThreshold`,
    ///   add escalation, calibrate CNS threshold directly
    /// - `Escalate` → add to the escalation queue for human review
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            match action.action_type {
                ActionType::Calibrate => {
                    let domain = action
                        .parameters
                        .get("domain")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let new_threshold = action
                        .parameters
                        .get("new_threshold")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(self.config.thresholds.variety_deficit);
                    let deficit = action
                        .parameters
                        .get("deficit")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    if domain == "variety" {
                        // Issue CalibrateThreshold directive through dispatch (5.3)
                        let directive = CuratorDirective::CalibrateThreshold {
                            domain: "variety".to_string(),
                            new_threshold,
                        };
                        self.issue_directive(directive).await;

                        // Post escalation for variety deficit
                        let template_id = hkask_types::TemplateID::new();
                        let bot_id = BotID::new();
                        let error_context = format!(
                            "Total variety deficit ({}) exceeds threshold ({})",
                            deficit, self.config.thresholds.variety_deficit
                        );
                        if let Err(e) = self.context.escalation_queue().add(
                            template_id,
                            bot_id,
                            format!("Variety deficit: {}", deficit),
                            0.6,
                            0,
                            error_context,
                        ) {
                            warn!(
                                target: "curator.metacognition",
                                error = %e,
                                "Failed to add variety deficit escalation"
                            );
                        }

                        // Calibrate CNS threshold directly (5.3 ADAPT subloop)
                        self.context
                            .cns()
                            .calibrate_threshold("variety", new_threshold)
                            .await;
                    }
                }
                ActionType::Escalate => {
                    let metric = action
                        .parameters
                        .get("metric")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    match metric {
                        "critical_alerts" => {
                            let count = action
                                .parameters
                                .get("count")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as usize;

                            warn!(
                                target: "curator.metacognition",
                                critical_alerts = count,
                                threshold = self.config.thresholds.critical_alerts,
                                "Critical alert count exceeds threshold"
                            );

                            let template_id = hkask_types::TemplateID::new();
                            let bot_id = BotID::new();
                            let error_context = format!(
                                "Critical alert count ({}) exceeds threshold ({})",
                                count, self.config.thresholds.critical_alerts
                            );

                            if let Err(e) = self.context.escalation_queue().add(
                                template_id,
                                bot_id,
                                format!("System has {} critical alerts", count),
                                0.3,
                                0,
                                error_context,
                            ) {
                                warn!(
                                    target: "curator.metacognition",
                                    error = %e,
                                    "Failed to add critical alert escalation"
                                );
                            }
                        }
                        "bot_failures" => {
                            let count = action
                                .parameters
                                .get("failed_count")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as usize;
                            let bot_names: Vec<String> = action
                                .parameters
                                .get("bot_names")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();

                            warn!(
                                target: "curator.metacognition",
                                failed_bots = count,
                                threshold = self.config.thresholds.bot_failures,
                                "Bot failure count exceeds threshold"
                            );

                            let template_id = hkask_types::TemplateID::new();
                            let bot_id = BotID::new();
                            let error_context = format!(
                                "{} bots in critical state: {}",
                                count,
                                bot_names.join(", ")
                            );

                            if let Err(e) = self.context.escalation_queue().add(
                                template_id,
                                bot_id,
                                format!("{} bots require attention", count),
                                0.4,
                                0,
                                error_context,
                            ) {
                                warn!(
                                    target: "curator.metacognition",
                                    error = %e,
                                    "Failed to add bot failure escalation"
                                );
                            }
                        }
                        _ => {
                            warn!(
                                target: "curator.metacognition",
                                metric = %metric,
                                "Unknown escalation metric in MetacognitionLoop act()"
                            );
                        }
                    }
                }
                _ => {
                    info!(
                        target: "curator.metacognition",
                        action_type = ?action.action_type,
                        "Unhandled action type in MetacognitionLoop act()"
                    );
                }
            }
        }
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
