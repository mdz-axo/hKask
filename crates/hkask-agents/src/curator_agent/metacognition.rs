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
use crate::escalation::{EscalationBatch, EscalationEntry, EscalationStatus};
use hkask_types::BotID;
use hkask_types::WebID;
use hkask_types::cns::CnsHealth;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::TraceId;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Default interval between metacognition cycles (1 hour).
pub(crate) const DEFAULT_METACOGNITION_INTERVAL_SECS: u64 = 3600;

/// Default expected variety per domain for deficit calculation.
pub(crate) const DEFAULT_EXPECTED_VARIETY_PER_DOMAIN: u64 = 50;

/// Default maximum concurrent escalations (VSM algedonic paradox — fewer signals = higher fidelity).
pub(crate) const DEFAULT_MAX_CONCURRENT_ESCALATIONS: usize = 3;

/// Default variety deficit threshold for escalation.
pub(crate) const DEFAULT_ESCALATION_VARIETY_DEFICIT: u64 = 100;

/// Default critical alert count threshold for escalation.
pub(crate) const DEFAULT_ESCALATION_CRITICAL_ALERTS: usize = 3;

/// Default bot failure count threshold for escalation.
pub(crate) const DEFAULT_ESCALATION_BOT_FAILURES: usize = 2;

#[derive(Debug, Error)]
pub enum MetacognitionError {
    #[error("Escalation error: {0}")]
    Escalation(#[from] crate::escalation::EscalationError),
    #[error("No snapshot available for metacognition cycle")]
    NoSnapshot,
    #[error("ACP error: {0}")]
    Acp(#[from] crate::acp::AcpError),
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
            variety_deficit: DEFAULT_ESCALATION_VARIETY_DEFICIT,
            critical_alerts: DEFAULT_ESCALATION_CRITICAL_ALERTS,
            bot_failures: DEFAULT_ESCALATION_BOT_FAILURES,
        }
    }
}

/// The trigger that caused an escalation alert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EscalationTrigger {
    /// Variety deficit exceeded a threshold.
    VarietyDeficit,
    /// Critical alert count exceeded a threshold.
    CriticalAlerts,
    /// Bot failure count exceeded a threshold.
    BotFailures,
}

/// Severity of an escalation alert, following the algedonic signal model:
/// - **Warning**: deficit > threshold / 2 (early signal)
/// - **Critical**: deficit > threshold (full escalation)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscalationSeverity {
    Warning,
    Critical,
}

/// An alert produced by the escalation policy when a threshold is breached.
///
/// Carries the trigger, current measured value, configured threshold,
/// and whether this is a warning or critical alert.
#[derive(Debug, Clone)]
pub struct EscalationAlert {
    /// What triggered this alert.
    pub trigger: EscalationTrigger,
    /// The current measured value.
    pub value: f64,
    /// The configured threshold that was compared against.
    pub threshold: f64,
    /// Whether this is a warning (deficit > threshold/2) or critical (deficit > threshold).
    pub severity: EscalationSeverity,
}

/// Determines whether the system should escalate based on health metrics.
///
/// Encapsulates the escalation thresholds and decision logic, making it
/// independently testable from the metacognition loop's sense→compare→compute→act
/// pipeline. The algedonic signal model uses two levels:
/// - **Warning** when a metric exceeds half the configured threshold
/// - **Critical** when a metric exceeds the full threshold
///
/// For variety deficit specifically, this implements the algedonic alert system
/// described in the architecture: deficit > threshold/2 → warning to Curator,
/// deficit > threshold → critical to human.
pub struct EscalationPolicy {
    thresholds: EscalationThresholds,
}

impl EscalationPolicy {
    /// Create a new escalation policy with the given thresholds.
    pub(crate) fn new(thresholds: EscalationThresholds) -> Self {
        Self { thresholds }
    }

    /// Check all escalation conditions and return a list of active alerts.
    ///
    /// Each metric is evaluated against its configured threshold. Variety deficit
    /// uses the algedonic two-level model (warning at threshold/2, critical at
    /// threshold). Critical alerts and bot failures trigger a critical alert
    /// when their count meets or exceeds the threshold.
    pub fn check_conditions(
        &self,
        variety_deficit: f64,
        critical_alerts: u64,
        bot_failures: u64,
    ) -> Vec<EscalationAlert> {
        let mut alerts = Vec::new();

        let variety_threshold = self.thresholds.variety_deficit as f64;
        if variety_deficit > variety_threshold {
            alerts.push(EscalationAlert {
                trigger: EscalationTrigger::VarietyDeficit,
                value: variety_deficit,
                threshold: variety_threshold,
                severity: EscalationSeverity::Critical,
            });
        } else if variety_deficit > variety_threshold / 2.0 {
            alerts.push(EscalationAlert {
                trigger: EscalationTrigger::VarietyDeficit,
                value: variety_deficit,
                threshold: variety_threshold,
                severity: EscalationSeverity::Warning,
            });
        }

        let critical_alerts_threshold = self.thresholds.critical_alerts as f64;
        if critical_alerts >= self.thresholds.critical_alerts as u64 {
            alerts.push(EscalationAlert {
                trigger: EscalationTrigger::CriticalAlerts,
                value: critical_alerts as f64,
                threshold: critical_alerts_threshold,
                severity: EscalationSeverity::Critical,
            });
        }

        let bot_failures_threshold = self.thresholds.bot_failures as f64;
        if bot_failures >= self.thresholds.bot_failures as u64 {
            alerts.push(EscalationAlert {
                trigger: EscalationTrigger::BotFailures,
                value: bot_failures as f64,
                threshold: bot_failures_threshold,
                severity: EscalationSeverity::Critical,
            });
        }

        alerts
    }
}

impl Default for EscalationPolicy {
    fn default() -> Self {
        Self::new(EscalationThresholds::default())
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
    /// Maximum number of concurrent escalations before batching is required.
    /// When the number of simultaneous escalations exceeds this threshold,
    /// they are consolidated into an EscalationBatch for the human operator.
    /// Default: 3 (VSM algedonic paradox — fewer signals = higher fidelity).
    pub max_concurrent_escalations: usize,
}

impl Default for MetacognitionConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(DEFAULT_METACOGNITION_INTERVAL_SECS),
            thresholds: EscalationThresholds::default(),
            expected_variety_per_domain: DEFAULT_EXPECTED_VARIETY_PER_DOMAIN,
            max_concurrent_escalations: DEFAULT_MAX_CONCURRENT_ESCALATIONS,
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
    escalation_policy: EscalationPolicy,
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
        let escalation_policy = EscalationPolicy::new(config.thresholds.clone());
        Self {
            context,
            escalation_policy,
            config,
            bot_reports: Arc::new(RwLock::new(Vec::new())),
            last_snapshot: Arc::new(RwLock::new(None)),
        }
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
            .ok_or(MetacognitionError::NoSnapshot)
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

    // Curator metacognition: evaluate, coach, direct

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

        acp.send_message(msg).await?;

        info!(
            target: "curator.metacognition",
            bot = %bot_name,
            reason = %reason,
            "Directive sent to bot via ACP"
        );

        Ok(())
    }

    // Directive issuance — Curation → Governance/Observability

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

    // Act helpers — extracted from HkaskLoop::act()

    /// Handle a Calibrate (throttle) action: issue threshold directive via
    /// dispatch (which calibrates CNS on arrival), and return an escalation entry.
    // NOTE: EscalationQueue is a Curation-owned durable queue. Direct writes
    // are intentional — it is an exception to the dispatch-only rule per the
    // authority DAG: Curation (L5) owns the escalation queue as its algedonic
    // regulation mechanism. This does NOT bypass the Communication Loop because
    // the queue is not a loop-to-loop message channel.
    async fn act_on_throttle(&self, action: &LoopAction) -> Option<EscalationEntry> {
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

            // Build escalation entry for variety deficit (written in act())
            let template_id = hkask_types::TemplateID::new();
            let bot_id = BotID::new();
            let error_context = format!(
                "Total variety deficit ({}) exceeds threshold ({})",
                deficit, self.config.thresholds.variety_deficit
            );
            Some(EscalationEntry {
                id: format!("esc_{}", uuid::Uuid::new_v4().simple()),
                template_id,
                bot_id,
                output: format!("Variety deficit: {}", deficit),
                confidence: 0.6,
                retry_count: 0,
                error_context,
                created_at: chrono::Utc::now(),
                status: EscalationStatus::Pending,
                resolved_at: None,
                resolved_by: None,
            })
        } else {
            None
        }
    }

    /// Handle an Escalate action: route to the appropriate escalation
    /// handler based on the metric (critical_alerts, bot_failures, or unknown).
    ///
    /// Returns the escalation entry for the caller to write (either
    /// individually or as part of a batch).
    // NOTE: EscalationQueue is a Curation-owned durable queue. Direct writes
    // are intentional — it is an exception to the dispatch-only rule per the
    // authority DAG: Curation (L5) owns the escalation queue as its algedonic
    // regulation mechanism. This does NOT bypass the Communication Loop because
    // the queue is not a loop-to-loop message channel.
    async fn act_on_escalate(&self, action: &LoopAction) -> Option<EscalationEntry> {
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

                Some(EscalationEntry {
                    id: format!("esc_{}", uuid::Uuid::new_v4().simple()),
                    template_id,
                    bot_id,
                    output: format!("System has {} critical alerts", count),
                    confidence: 0.3,
                    retry_count: 0,
                    error_context,
                    created_at: chrono::Utc::now(),
                    status: EscalationStatus::Pending,
                    resolved_at: None,
                    resolved_by: None,
                })
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
                let error_context =
                    format!("{} bots in critical state: {}", count, bot_names.join(", "));

                Some(EscalationEntry {
                    id: format!("esc_{}", uuid::Uuid::new_v4().simple()),
                    template_id,
                    bot_id,
                    output: format!("{} bots require attention", count),
                    confidence: 0.4,
                    retry_count: 0,
                    error_context,
                    created_at: chrono::Utc::now(),
                    status: EscalationStatus::Pending,
                    resolved_at: None,
                    resolved_by: None,
                })
            }
            _ => {
                warn!(
                    target: "curator.metacognition",
                    metric = %metric,
                    "Unknown escalation metric in MetacognitionLoop act()"
                );
                None
            }
        }
    }

    /// Log an unhandled action type (no-op).
    fn act_on_no_action(&self, action: &LoopAction) {
        info!(
            target: "curator.metacognition",
            action_type = ?action.action_type,
            "Unhandled action type in MetacognitionLoop act()"
        );
    }

    // Explicit 4-stage cycle: sense → compare → compute → act

    /// **Sense stage** (sense → compare → compute → act):
    /// Read CNS health, variety counters, critical alerts, and bot status
    /// reports. Produces afferent signals for variety deficit, critical alert
    /// count, and bot failure count. Builds and stores a HealthSnapshot for
    /// use by compare/compute/act phases.
    pub async fn sense(&self) -> Vec<Signal> {
        <Self as HkaskLoop>::sense(self).await
    }

    /// **Compare stage** (sense → compare → compute → act):
    /// Evaluate variety deficit vs threshold, critical alert count vs
    /// threshold, and bot failure count vs threshold. Detects deviations
    /// from set-points in the sensed signals.
    pub async fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        <Self as HkaskLoop>::compare(self, signals).await
    }

    /// **Compute stage** (sense → compare → compute → act):
    /// Map deviations to regulatory actions. Variety deficit above threshold
    /// → Calibrate (threshold adjustment). Critical alerts above threshold
    /// → Escalate (human review). Bot failures above threshold → Escalate
    /// (bot attention).
    pub async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        <Self as HkaskLoop>::compute(self, deviations).await
    }

    /// **Act stage** (sense → compare → compute → act):
    /// Execute regulatory actions by issuing CuratorDirectives and posting
    /// escalations. Calibrate → issue CalibrateThreshold directive. Escalate
    /// → write escalation entries to the queue (individually or batched).
    pub async fn act(&self, actions: &[LoopAction]) {
        <Self as HkaskLoop>::act(self, actions).await
    }
}

// HkaskLoop — sense → compare → compute → act

#[async_trait::async_trait]
impl HkaskLoop for MetacognitionLoop {
    fn id(&self) -> LoopId {
        // Metacognition is a worker within Curation (Loop 5), not a governing loop.
        LoopId::Curation
    }

    /// Metacognition is a worker within the Curation loop.
    fn worker_kind(&self) -> Option<hkask_types::loops::dispatch::WorkerKind> {
        Some(hkask_types::loops::dispatch::WorkerKind::Metacognition)
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

        // Delegate escalation condition checking to the policy.
        // The policy returns structured alerts that can be logged, surfaced
        // through the algedonic channel, or used for downstream decisions.
        let alerts = self.escalation_policy.check_conditions(
            total_variety_deficit as f64,
            critical_alerts.len() as u64,
            failed_bot_count as u64,
        );
        for alert in &alerts {
            match alert.severity {
                EscalationSeverity::Warning => warn!(
                    target: "curator.metacognition",
                    trigger = ?alert.trigger,
                    value = alert.value,
                    threshold = alert.threshold,
                    "Escalation policy: warning condition detected"
                ),
                EscalationSeverity::Critical => warn!(
                    target: "curator.metacognition",
                    trigger = ?alert.trigger,
                    value = alert.value,
                    threshold = alert.threshold,
                    "Escalation policy: critical condition detected"
                ),
            }
        }

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
                LoopId::Curation,
                SignalMetric::MetacognitionVarietyDeficit,
                total_variety_deficit as f64,
                self.config.thresholds.variety_deficit as f64,
            ),
            // Critical alerts: act when count >= threshold.
            // Use threshold - 0.5 as set-point so that count == threshold
            // produces an AboveSetPoint deviation.
            Signal::new(
                LoopId::Curation,
                SignalMetric::MetacognitionCriticalAlerts,
                critical_alerts.len() as f64,
                self.config.thresholds.critical_alerts as f64 - 0.5,
            ),
            // Bot failures: act when count >= threshold.
            // Same threshold - 0.5 technique as critical_alerts.
            Signal::new(
                LoopId::Curation,
                SignalMetric::MetacognitionBotFailures,
                failed_bot_count as f64,
                self.config.thresholds.bot_failures as f64 - 0.5,
            ),
        ];

        signals
    }

    /// Compute: produce `LoopAction`s for detected deviations.
    ///
    /// Maps deviations to regulatory actions:
    /// - `metacognition_variety_deficit` AboveSetPoint → Calibrate action (threshold adjustment)
    /// - `metacognition_critical_alerts` AboveSetPoint → Escalate action (human review)
    /// - `metacognition_bot_failures` AboveSetPoint → Escalate action (bot attention)
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            match dev.signal.metric {
                SignalMetric::MetacognitionVarietyDeficit
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
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
                SignalMetric::MetacognitionCriticalAlerts
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
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
                SignalMetric::MetacognitionBotFailures
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
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
    ///   collect escalation entry for batch processing
    /// - `Escalate` → collect escalation entry for batch processing
    ///
    /// After processing all actions, escalation entries are written to the
    /// queue either individually (if below `max_concurrent_escalations`)
    /// or as a single consolidated `EscalationBatch` (if at/above threshold).
    async fn act(&self, actions: &[LoopAction]) {
        let mut escalation_entries: Vec<EscalationEntry> = Vec::new();

        for action in actions {
            match action.action_type {
                ActionType::Calibrate => {
                    if let Some(entry) = self.act_on_throttle(action).await {
                        escalation_entries.push(entry);
                    }
                }
                ActionType::Escalate => {
                    if let Some(entry) = self.act_on_escalate(action).await {
                        escalation_entries.push(entry);
                    }
                }
                _ => self.act_on_no_action(action),
            }
        }

        // Write escalations: batch if concurrent count meets/exceeds threshold,
        // otherwise write individually.
        let threshold = self.config.max_concurrent_escalations;
        if escalation_entries.len() >= threshold {
            let batch = EscalationBatch::new(escalation_entries, "consolidated", threshold);
            let summary = batch.summary();
            info!(
                target: "curator.metacognition",
                batch_id = %batch.id,
                entry_count = batch.entries.len(),
                threshold,
                "Consolidating escalations into batch"
            );
            if let Err(e) = self.context.escalation_queue().add(
                hkask_types::TemplateID::new(),
                BotID::new(),
                summary,
                batch
                    .entries
                    .iter()
                    .map(|e| e.confidence)
                    .fold(f64::MAX, f64::min),
                0,
                format!("Consolidated batch: {} escalation(s)", batch.entries.len()),
            ) {
                warn!(
                    target: "curator.metacognition",
                    error = %e,
                    "Failed to add consolidated escalation batch"
                );
            }
        } else {
            for entry in escalation_entries {
                if let Err(e) = self.context.escalation_queue().add(
                    entry.template_id,
                    entry.bot_id,
                    entry.output,
                    entry.confidence,
                    entry.retry_count,
                    entry.error_context,
                ) {
                    warn!(
                        target: "curator.metacognition",
                        error = %e,
                        "Failed to add escalation"
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_thresholds() -> EscalationThresholds {
        EscalationThresholds {
            variety_deficit: 100,
            critical_alerts: 3,
            bot_failures: 2,
        }
    }

    #[test]
    fn escalation_policy_below_all_thresholds_produces_no_alerts() {
        let policy = EscalationPolicy::new(default_thresholds());
        let alerts = policy.check_conditions(40.0, 1, 0);
        assert!(
            alerts.is_empty(),
            "expected no alerts when all metrics are below thresholds"
        );
    }

    #[test]
    fn escalation_policy_variety_deficit_warning_at_half_threshold() {
        let policy = EscalationPolicy::new(default_thresholds());
        // 60 > 100/2 = 50, but 60 < 100 → warning
        let alerts = policy.check_conditions(60.0, 0, 0);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].trigger, EscalationTrigger::VarietyDeficit);
        assert_eq!(alerts[0].severity, EscalationSeverity::Warning);
        assert!((alerts[0].value - 60.0).abs() < f64::EPSILON);
        assert!((alerts[0].threshold - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn escalation_policy_variety_deficit_critical_at_threshold() {
        let policy = EscalationPolicy::new(default_thresholds());
        // 101 > 100 → critical
        let alerts = policy.check_conditions(101.0, 0, 0);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].trigger, EscalationTrigger::VarietyDeficit);
        assert_eq!(alerts[0].severity, EscalationSeverity::Critical);
    }

    #[test]
    fn escalation_policy_variety_deficit_exact_half_threshold_is_not_warning() {
        let policy = EscalationPolicy::new(default_thresholds());
        // 50 is NOT > 50, so no alert
        let alerts = policy.check_conditions(50.0, 0, 0);
        assert!(
            alerts.is_empty(),
            "deficit == threshold/2 should not trigger warning (strict >)"
        );
    }

    #[test]
    fn escalation_policy_critical_alerts_at_threshold() {
        let policy = EscalationPolicy::new(default_thresholds());
        // 3 >= 3 → critical
        let alerts = policy.check_conditions(0.0, 3, 0);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].trigger, EscalationTrigger::CriticalAlerts);
        assert_eq!(alerts[0].severity, EscalationSeverity::Critical);
        assert!((alerts[0].value - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn escalation_policy_critical_alerts_below_threshold() {
        let policy = EscalationPolicy::new(default_thresholds());
        // 2 < 3 → no alert
        let alerts = policy.check_conditions(0.0, 2, 0);
        assert!(
            alerts
                .iter()
                .all(|a| a.trigger != EscalationTrigger::CriticalAlerts),
            "critical alerts below threshold should not trigger alert"
        );
    }

    #[test]
    fn escalation_policy_bot_failures_at_threshold() {
        let policy = EscalationPolicy::new(default_thresholds());
        // 2 >= 2 → critical
        let alerts = policy.check_conditions(0.0, 0, 2);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].trigger, EscalationTrigger::BotFailures);
        assert_eq!(alerts[0].severity, EscalationSeverity::Critical);
        assert!((alerts[0].value - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn escalation_policy_bot_failures_below_threshold() {
        let policy = EscalationPolicy::new(default_thresholds());
        // 1 < 2 → no alert
        let alerts = policy.check_conditions(0.0, 0, 1);
        assert!(
            alerts
                .iter()
                .all(|a| a.trigger != EscalationTrigger::BotFailures),
            "bot failures below threshold should not trigger alert"
        );
    }

    #[test]
    fn escalation_policy_multiple_conditions_active_simultaneously() {
        let policy = EscalationPolicy::new(default_thresholds());
        // variety deficit 120 > 100 → critical
        // critical alerts 5 >= 3 → critical
        // bot failures 3 >= 2 → critical
        let alerts = policy.check_conditions(120.0, 5, 3);
        assert_eq!(alerts.len(), 3);

        let triggers: std::collections::HashSet<_> =
            alerts.iter().map(|a| a.trigger.clone()).collect();
        assert!(triggers.contains(&EscalationTrigger::VarietyDeficit));
        assert!(triggers.contains(&EscalationTrigger::CriticalAlerts));
        assert!(triggers.contains(&EscalationTrigger::BotFailures));

        // All should be critical when deficit > full threshold
        assert!(
            alerts
                .iter()
                .all(|a| a.severity == EscalationSeverity::Critical)
        );
    }

    #[test]
    fn escalation_policy_warning_and_critical_can_coexist() {
        let policy = EscalationPolicy::new(default_thresholds());
        // variety deficit 60 > 50 (warning) but < 100
        // critical alerts 4 >= 3 (critical)
        let alerts = policy.check_conditions(60.0, 4, 0);
        assert_eq!(alerts.len(), 2);

        let variety_alert = alerts
            .iter()
            .find(|a| a.trigger == EscalationTrigger::VarietyDeficit)
            .expect("should have variety alert");
        assert_eq!(variety_alert.severity, EscalationSeverity::Warning);

        let critical_alert = alerts
            .iter()
            .find(|a| a.trigger == EscalationTrigger::CriticalAlerts)
            .expect("should have critical alerts trigger");
        assert_eq!(critical_alert.severity, EscalationSeverity::Critical);
    }

    #[test]
    fn escalation_policy_default_matches_thresholds_default() {
        let policy = EscalationPolicy::default();
        // Using default thresholds: variety=100, critical_alerts=3, bot_failures=2
        // Below all thresholds → no alerts
        let alerts = policy.check_conditions(10.0, 1, 0);
        assert!(alerts.is_empty());
    }
}
