//! Curator Agent metacognition: sense→compare→compute→act governance loop.
//! Moved from `curator::metacognition` — persona concern, not regulatory.

use hkask_rsolidity as rs;
use crate::a2a::A2AMessage;
use crate::curator::context::CuratorContext;
use crate::curator_agent::bot_health::BotHealthEvaluator;
use crate::curator_agent::bot_metrics::BotHealthStatus;
use hkask_storage::{EscalationBatch, EscalationEntry};
use hkask_types::BotID;
use hkask_types::WebID;
use hkask_types::cns::CnsHealth;
use hkask_types::event::SpanNamespace;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

const MC_TARGET: &str = "curator.metacognition";

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

/// Metacognition cycle errors.
#[derive(Debug, Error)]
pub enum MetacognitionError {
    #[error(transparent)]
    Core(#[from] crate::error::CoreError),
}

impl From<crate::a2a::A2AError> for MetacognitionError {
    fn from(e: crate::a2a::A2AError) -> Self {
        MetacognitionError::Core(crate::error::CoreError::A2A(e))
    }
}

/// Escalation trigger thresholds.
#[derive(Debug, Clone)]
pub(crate) struct EscalationThresholds {
    pub variety_deficit: u64,
    pub critical_alerts: usize,
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

/// Algedonic signal model: Warning (threshold/2) or Critical (threshold).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscalationSeverity {
    Warning,
    Critical,
}

/// Alert produced when a threshold is breached.
#[derive(Debug, Clone)]
pub struct EscalationAlert {
    pub trigger: EscalationTrigger,
    pub value: f64,
    pub threshold: f64,
    pub severity: EscalationSeverity,
}

/// Encapsulates escalation threshold logic — independently testable.
/// Algedonic: Warning at threshold/2, Critical at threshold.
pub struct EscalationPolicy {
    thresholds: EscalationThresholds,
}

impl EscalationPolicy {
    pub(crate) fn new(thresholds: EscalationThresholds) -> Self {
        Self { thresholds }
    }

    /// Check all escalation conditions, return active alerts.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — escalation policy classifies variety deficit
    /// \[P4\] Constraining: Clear Boundaries — thresholds define explicit boundaries
    /// pre:  `variety_deficit`, `critical_alerts`, `bot_failures` are
    ///       non-negative numeric values.
    /// post: Returns a `Vec<EscalationAlert>` containing alerts for any
    ///       threshold exceeded: VarietyDeficit (Critical if > threshold,
    ///       Warning if > threshold/2), CriticalAlerts (Critical if ≥
    ///       threshold), BotFailures (Critical if ≥ threshold).
    #[rs::contract(id = "P9-agt-curator-agent-escalation-check", principle = "P9")]
    #[rs::contract(id = "P9-agt-curator-agent-escalation-check", principle = "P9")]
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

/// Health snapshot — unified system health state.
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cns_health: String,
    pub variety_counters: HashMap<SpanNamespace, u64>,
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

/// Metacognition loop configuration.
#[derive(Debug, Clone)]
pub struct MetacognitionConfig {
    /// Interval between metacognition cycles (default: 1 hour)
    pub interval: Duration,
    /// Escalation thresholds
    pub(crate) thresholds: EscalationThresholds,
    /// Expected variety per domain (for deficit calculation)
    pub expected_variety_per_domain: u64,
    /// Max concurrent escalations before batching (VSM algedonic paradox). Default: 3.
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

/// Metacognition loop — Curator Agent's system governance mechanism.
pub struct MetacognitionLoop {
    context: Arc<CuratorContext>,
    config: MetacognitionConfig,
    escalation_policy: EscalationPolicy,
    bot_reports: Arc<RwLock<Vec<BotStatusReport>>>,
    last_snapshot_tx: tokio::sync::watch::Sender<Option<HealthSnapshot>>,
    bot_health_evaluator: Option<Arc<BotHealthEvaluator>>,
}

impl MetacognitionLoop {
    /// Create a new metacognition loop without a BotHealthEvaluator.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — MetacognitionLoop monitors agent health
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`.
    /// post: Returns a `MetacognitionLoop` with an `EscalationPolicy`
    ///       derived from `config.thresholds`, empty bot reports, and a
    ///       fresh watch channel for health snapshots.
    #[rs::contract(id = "P9-agt-curator-agent-meta-new", principle = "P9")]
    #[rs::contract(id = "P9-agt-curator-agent-meta-new", principle = "P9")]
    pub fn new(context: Arc<CuratorContext>, config: MetacognitionConfig) -> Self {
        let escalation_policy = EscalationPolicy::new(config.thresholds.clone());
        let (last_snapshot_tx, _) = tokio::sync::watch::channel(None);
        Self {
            context,
            escalation_policy,
            config,
            bot_reports: Arc::new(RwLock::new(Vec::new())),
            last_snapshot_tx,
            bot_health_evaluator: None,
        }
    }

    /// Create a new metacognition loop with a BotHealthEvaluator.
    ///
    /// The evaluator reads gas data from the CNS runtime and populates
    /// bot health reports at each cycle.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — classify bot energy health for Curator
    /// \[P4\] Constraining: Clear Boundaries — thresholds map consumption ratio to status
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`; `evaluator` is a valid
    ///       `Arc<BotHealthEvaluator>`.
    /// post: Returns a `MetacognitionLoop` with the evaluator wired in.
    #[rs::contract(id = "P9-agt-bot-health-classify", principle = "P9")]
    #[rs::contract(id = "P9-agt-bot-health-classify", principle = "P9")]
    pub fn with_evaluator(
        context: Arc<CuratorContext>,
        config: MetacognitionConfig,
        evaluator: Arc<BotHealthEvaluator>,
    ) -> Self {
        let escalation_policy = EscalationPolicy::new(config.thresholds.clone());
        let (last_snapshot_tx, _) = tokio::sync::watch::channel(None);
        Self {
            context,
            escalation_policy,
            config,
            bot_reports: Arc::new(RwLock::new(Vec::new())),
            last_snapshot_tx,
            bot_health_evaluator: Some(evaluator),
        }
    }

    /// Get current bot status reports.
    ///
    /// If a BotHealthEvaluator is wired in, runs evaluation for all agents.
    /// Otherwise, returns the cached reports (which may be empty).
    pub(crate) async fn get_bot_reports(&self) -> Vec<BotStatusReport> {
        if let Some(ref evaluator) = self.bot_health_evaluator {
            match evaluator.evaluate_all(chrono::Utc::now()).await {
                Ok(reports) => return reports,
                Err(e) => {
                    warn!(target: MC_TARGET, error = %e, "BotHealthEvaluator failed, falling back to cached reports");
                }
            }
        }
        self.bot_reports.read().await.clone()
    }

    /// Run a full cycle, returning the health snapshot.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — tick produces latest HealthSnapshot
    /// pre:  The loop has been registered and ticked at least once.
    /// post: On success, returns `Ok(HealthSnapshot)` — the latest
    ///       snapshot from the watch channel. If no snapshot has been
    ///       produced yet, returns `Err(MetacognitionError::Core(...))`.
    #[rs::contract(id = "P9-agt-curator-agent-tick", principle = "P9")]
    #[rs::contract(id = "P9-agt-curator-agent-tick", principle = "P9")]
    pub async fn run_cycle(&self) -> Result<HealthSnapshot, MetacognitionError> {
        info!(target: MC_TARGET, "Starting metacognition cycle");
        self.tick().await;
        self.last_snapshot_tx
            .borrow()
            .clone()
            .ok_or(crate::error::CoreError::NoSnapshot.into())
    }
    /// Generate a system state summary for posting to standing session.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — summary posts system state to standing session
    /// pre:  `snapshot` is a valid `&HealthSnapshot`.
    /// post: Returns a `String` containing a markdown-formatted summary
    ///       with timestamp, CNS health, critical/total alerts, variety
    ///       counters, and bot status reports.
    #[rs::contract(id = "P9-agt-curator-agent-summary", principle = "P9")]
    #[rs::contract(id = "P9-agt-curator-agent-summary", principle = "P9")]
    pub fn generate_summary(&self, snapshot: &HealthSnapshot) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(s, "## Metacognition Update\n");
        let _ = writeln!(
            s,
            "**Timestamp:** {}",
            snapshot.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        let _ = writeln!(s, "**CNS Health:** {}", snapshot.cns_health);
        let _ = writeln!(s, "**Critical Alerts:** {}", snapshot.critical_alerts);
        let _ = writeln!(s, "**Total Alerts:** {}\n", snapshot.total_alerts);
        if !snapshot.variety_counters.is_empty() {
            let _ = writeln!(s, "### Variety Counters");
            for (ns, variety) in &snapshot.variety_counters {
                let _ = writeln!(s, "- {}: {}", ns.as_str(), variety);
            }
            s.push('\n');
        }
        if !snapshot.bot_status_reports.is_empty() {
            let _ = writeln!(s, "### Bot Status");
            for report in &snapshot.bot_status_reports {
                let _ = write!(s, "- **{}**: {}", report.bot_name, report.status);
                if !report.issues.is_empty() {
                    let _ = write!(s, " ({})", report.issues.join(", "));
                }
                s.push('\n');
            }
        }
        s
    }

    // Curator metacognition: evaluate, coach, direct

    /// Direct a bot to take action via A2A message.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — direct a bot to take corrective action
    /// pre:  `bot_name` is a non-empty string; `reason` is a non-empty
    ///       string; `self.context.a2a()` may be `Some` or `None`.
    /// post: If A2A is configured, sends a `TemplateDispatch` directive
    ///       to the bot and returns `Ok(())`. If A2A is not configured,
    ///       logs a warning and returns `Ok(())` (graceful degradation).
    ///       Returns `Err` on A2A send failure.
    #[rs::contract(id = "P9-agt-curator-agent-direct", principle = "P9")]
    #[rs::contract(id = "P9-agt-curator-agent-direct", principle = "P9")]
    pub async fn direct_bot(&self, bot_name: &str, reason: &str) -> Result<(), MetacognitionError> {
        let a2a = match self.context.a2a() {
            Some(a2a) => a2a,
            None => {
                warn!(
                    target: MC_TARGET,
                    bot = %bot_name,
                    "A2A port not configured — cannot direct bot"
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

        a2a.send_message(msg).await?;

        info!(
            target: MC_TARGET,
            bot = %bot_name,
            reason = %reason,
            "Directive sent to bot via A2A"
        );

        Ok(())
    }

    /// Issue a CuratorDirective on the direct channel with DAMPEN filtering.
    /// Delegates to `CuratorContext::issue_directive()`.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — delegate directive to CuratorContext
    /// pre:  `directive` is a valid `CuratorDirective`.
    /// post: Delegates to `self.context.issue_directive(directive)`;
    ///       same post-conditions as `CuratorContext::issue_directive`.
    #[rs::contract(id = "P9-agt-curator-agent-issue-directive", principle = "P9")]
    #[rs::contract(id = "P9-agt-curator-agent-issue-directive", principle = "P9")]
    pub async fn issue_directive(&self, directive: CuratorDirective) {
        self.context.issue_directive(directive).await;
    }

    // Act helpers — parameter extraction

    fn param_str<'a>(action: &'a LoopAction, key: &str, default: &'a str) -> &'a str {
        action
            .parameters
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
    }

    fn param_u64(action: &LoopAction, key: &str, default: u64) -> u64 {
        action
            .parameters
            .get(key)
            .and_then(|v| v.as_u64())
            .unwrap_or(default)
    }

    async fn act_on_throttle(&self, action: &LoopAction) -> Option<EscalationEntry> {
        let domain = Self::param_str(action, "domain", "");
        let new_threshold = Self::param_u64(
            action,
            "new_threshold",
            self.config.thresholds.variety_deficit,
        );
        let deficit = Self::param_u64(action, "deficit", 0);

        if domain == "variety" {
            let directive = CuratorDirective::CalibrateThreshold {
                domain: "variety".to_string(),
                new_threshold,
            };
            self.issue_directive(directive).await;

            let error_context = format!(
                "Total variety deficit ({}) exceeds threshold ({})",
                deficit, self.config.thresholds.variety_deficit
            );
            Some(EscalationEntry::pending(
                format!("Variety deficit: {}", deficit),
                0.6,
                error_context,
            ))
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
        let metric = Self::param_str(action, "metric", "");
        match metric {
            "critical_alerts" => {
                let count = Self::param_u64(action, "count", 0) as usize;
                warn!(
                    target: MC_TARGET,
                    critical_alerts = count,
                    threshold = self.config.thresholds.critical_alerts,
                    "Critical alert count exceeds threshold"
                );
                Some(EscalationEntry::pending(
                    format!("System has {} critical alerts", count),
                    0.3,
                    format!(
                        "Critical alert count ({}) exceeds threshold ({})",
                        count, self.config.thresholds.critical_alerts
                    ),
                ))
            }
            "bot_failures" => {
                let count = Self::param_u64(action, "failed_count", 0) as usize;
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
                    target: MC_TARGET,
                    failed_bots = count,
                    threshold = self.config.thresholds.bot_failures,
                    "Bot failure count exceeds threshold"
                );
                Some(EscalationEntry::pending(
                    format!("{} bots require attention", count),
                    0.4,
                    format!("{} bots in critical state: {}", count, bot_names.join(", ")),
                ))
            }
            _ => {
                warn!(target: MC_TARGET, metric = %metric, "Unknown escalation metric");
                None
            }
        }
    }

    /// Log an unhandled action type (no-op).
    fn act_on_no_action(&self, action: &LoopAction) {
        info!(
            target: MC_TARGET,
            action_type = ?action.action_type,
            "Unhandled action type in MetacognitionLoop act()"
        );
    }

    // Explicit 4-stage cycle: sense → compare → compute → act
    // Delegation methods removed — HkaskLoop trait impl provides tick().
}

// HkaskLoop — sense → compare → compute → act
#[async_trait::async_trait]
impl HkaskLoop for MetacognitionLoop {
    fn id(&self) -> LoopId {
        // Metacognition is a worker within Curation (Loop 5), not a governing loop.
        LoopId::Curation
    }

    /// Sense: read CNS health, variety counters, alerts, and bot status.
    /// Builds and stores a HealthSnapshot.
    async fn sense(&self) -> Vec<Signal> {
        info!(target: MC_TARGET, "Starting metacognition sense phase");

        let cns_health = self.context.cns().health().await;
        let cns_health_str = format_health_status(&cns_health);

        let variety_counters = self.context.cns().variety().await;
        let all_alerts = self.context.cns().alerts().await;
        let critical_alerts = self.context.cns().critical_alerts().await;
        let bot_reports = self.get_bot_reports().await;

        // Compute total variety deficit (same logic as evaluate_and_adapt)
        let mut total_variety_deficit = 0u64;
        for (ns, variety) in &variety_counters {
            let deficit = self
                .config
                .expected_variety_per_domain
                .saturating_sub(*variety);
            if deficit > 0 {
                total_variety_deficit += deficit;
                if deficit > self.config.thresholds.variety_deficit {
                    warn!(
                        target: MC_TARGET,
                        domain = %ns.as_str(),
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
                    target: MC_TARGET,
                    trigger = ?alert.trigger,
                    value = alert.value,
                    threshold = alert.threshold,
                    "Escalation policy: warning condition detected"
                ),
                EscalationSeverity::Critical => warn!(
                    target: MC_TARGET,
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
        // `send_replace` returns the previous value and Errs only if the
        // channel is closed — which can't happen here because we own the
        // `Sender`. Ignore the previous value (we just wrote).
        let _ = self.last_snapshot_tx.send_replace(Some(snapshot));

        // Produce afferent signals
        let lid = LoopId::Curation;
        let t = &self.config.thresholds;
        vec![
            Signal::new(
                lid,
                SignalMetric::MetacognitionVarietyDeficit,
                total_variety_deficit as f64,
                t.variety_deficit as f64,
            ),
            Signal::new(
                lid,
                SignalMetric::MetacognitionCriticalAlerts,
                critical_alerts.len() as f64,
                t.critical_alerts as f64 - 0.5,
            ),
            Signal::new(
                lid,
                SignalMetric::MetacognitionBotFailures,
                failed_bot_count as f64,
                t.bot_failures as f64 - 0.5,
            ),
        ]
    }

    /// Compute: map deviations to regulatory actions.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();
        for dev in deviations {
            match dev.signal.metric {
                SignalMetric::MetacognitionVarietyDeficit
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    let deficit = dev.signal.value as u64;
                    actions.push(LoopAction::new(LoopId::Curation, ActionType::Calibrate, serde_json::json!({"domain": "variety", "deficit": deficit, "threshold": dev.signal.set_point as u64, "new_threshold": deficit.saturating_add(self.config.thresholds.variety_deficit)})));
                }
                SignalMetric::MetacognitionCriticalAlerts
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    let count = dev.signal.value as u64;
                    actions.push(LoopAction::new(LoopId::Curation, ActionType::Escalate, serde_json::json!({"metric": "critical_alerts", "count": count, "threshold": self.config.thresholds.critical_alerts})));
                }
                SignalMetric::MetacognitionBotFailures
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    let count = dev.signal.value as u64;
                    let bot_names: Vec<String> = self
                        .last_snapshot_tx
                        .borrow()
                        .as_ref()
                        .map(|s| {
                            s.bot_status_reports
                                .iter()
                                .filter(|r| r.status == BotHealthStatus::Critical)
                                .map(|r| r.bot_name.clone())
                                .collect()
                        })
                        .unwrap_or_default();
                    actions.push(LoopAction::new(LoopId::Curation, ActionType::Escalate, serde_json::json!({"metric": "bot_failures", "failed_count": count, "threshold": self.config.thresholds.bot_failures, "bot_names": bot_names})));
                }
                _ => {}
            }
        }
        actions
    }

    /// Act: issue CuratorDirectives and post escalations (batched if above threshold).
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

        // Write escalations: batch if concurrent count meets/exceeds threshold, otherwise write individually.
        let threshold = self.config.max_concurrent_escalations;
        if escalation_entries.len() >= threshold {
            let batch = EscalationBatch::new(escalation_entries, "consolidated", threshold);
            let summary = batch.summary();
            info!(target: MC_TARGET, batch_id = %batch.id, entry_count = batch.entries.len(), threshold, "Consolidating escalations into batch");
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
                warn!(target: MC_TARGET, error = %e, "Failed to add consolidated escalation batch");
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
                    warn!(target: MC_TARGET, error = %e, "Failed to add escalation");
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
