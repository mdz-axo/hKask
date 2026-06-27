//! Curator Agent metacognition: sense→compare→compute→act governance loop.
//! Moved from `curator::metacognition` — persona concern, not regulatory.

use crate::a2a::A2AMessage;
use crate::curator::context::CuratorContext;
use crate::curator_agent::bot_health::BotHealthEvaluator;
use crate::curator_agent::bot_metrics::BotHealthStatus;
use crate::curator_agent::cat;
use hkask_cns::types::loops::{
    ActionType, Deviation, DeviationDirection, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use hkask_storage::{EscalationBatch, EscalationEntry};
use hkask_types::BotID;
use hkask_types::WebID;
use hkask_types::cns::CnsHealth;
use hkask_types::curator::CuratorDirective;
use hkask_types::event::SpanNamespace;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
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
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — escalation policy classifies variety deficit
    /// \[P4\] Constraining: Clear Boundaries — thresholds define explicit boundaries
    /// pre:  `variety_deficit`, `critical_alerts`, `bot_failures` are
    ///       non-negative numeric values.
    /// post: Returns a `Vec<EscalationAlert>` containing alerts for any
    ///       threshold exceeded: VarietyDeficit (Critical if > threshold,
    ///       Warning if > threshold/2), CriticalAlerts (Critical if ≥
    ///       threshold), BotFailures (Critical if ≥ threshold).
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
    pub variety_deficit: u64,
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
    /// Template output from the most recent template-driven compute cycle.
    /// Stored separately from HealthSnapshot to avoid race conditions —
    /// `sense()` wipes the snapshot each cycle but template output must
    /// survive across cycles for `generate_summary()` and `act()`.
    last_template_output: RwLock<Option<serde_json::Value>>,
    /// Circuit breaker: consecutive template invocation failures.
    /// After 3 consecutive failures, skip template for 5 cycles.
    consecutive_template_failures: std::sync::atomic::AtomicU64,
    template_skip_remaining: std::sync::atomic::AtomicU64,
    /// Persona name for communication posture evaluation.
    curator_name: String,
    /// Convergence bias for CAT decision evaluation.
    convergence_bias: f64,
    /// Core traits never compromised by accommodation.
    invariant_traits: Vec<String>,
}

impl MetacognitionLoop {
    /// Create a new metacognition loop without a BotHealthEvaluator.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — MetacognitionLoop monitors agent health
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`.
    /// post: Returns a `MetacognitionLoop` with an `EscalationPolicy`
    ///       derived from `config.thresholds`, empty bot reports, and a
    ///       fresh watch channel for health snapshots.
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
            last_template_output: RwLock::new(None),
            consecutive_template_failures: std::sync::atomic::AtomicU64::new(0),
            template_skip_remaining: std::sync::atomic::AtomicU64::new(0),
            curator_name: "curator".to_string(),
            convergence_bias: 0.5,
            invariant_traits: Vec::new(),
        }
    }

    /// Create a new metacognition loop with a BotHealthEvaluator.
    ///
    /// The evaluator reads gas data from the CNS runtime and populates
    /// bot health reports at each cycle.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — classify bot energy health for Curator
    /// \[P4\] Constraining: Clear Boundaries — thresholds map consumption ratio to status
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`; `evaluator` is a valid
    ///       `Arc<BotHealthEvaluator>`.
    /// post: Returns a `MetacognitionLoop` with the evaluator wired in.
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
            last_template_output: RwLock::new(None),
            consecutive_template_failures: std::sync::atomic::AtomicU64::new(0),
            template_skip_remaining: std::sync::atomic::AtomicU64::new(0),
            curator_name: "curator".to_string(),
            convergence_bias: 0.5,
            invariant_traits: Vec::new(),
        }
    }

    /// Builder: set the communication posture (persona name, convergence bias, and invariant traits).
    pub fn with_communication_posture(mut self, name: String, bias: f64, traits: Vec<String>) -> Self {
        self.curator_name = name;
        self.convergence_bias = bias;
        self.invariant_traits = traits;
        self
    }

    /// Access the metacognition configuration.
    pub fn config(&self) -> &MetacognitionConfig {
        &self.config
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
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — tick produces latest HealthSnapshot
    /// pre:  The loop has been registered and ticked at least once.
    /// post: On success, returns `Ok(HealthSnapshot)` — the latest
    ///       snapshot from the watch channel. If no snapshot has been
    ///       produced yet, returns `Err(CoreError::NoSnapshot)`.
    pub async fn run_cycle(&self) -> Result<HealthSnapshot, crate::error::CoreError> {
        info!(target: MC_TARGET, "Starting metacognition cycle");
        self.tick().await;
        self.last_snapshot_tx
            .borrow()
            .clone()
            .ok_or(crate::error::CoreError::NoSnapshot)
    }
    /// Generate a system state summary for posting to standing session.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — summary posts system state to standing session
    /// pre:  `snapshot` is a valid `&HealthSnapshot`.
    /// post: Returns a `String` containing a markdown-formatted summary
    ///       with timestamp, CNS health, critical/total alerts, variety
    ///       counters, and bot status reports.
    pub fn generate_summary(&self, snapshot: &HealthSnapshot) -> String {
        use std::fmt::Write;
        if let Some(ref output) = *self.last_template_output.blocking_read() {
            let mut s = String::new();
            let _ = writeln!(s, "## Metacognition Update (LLM)");
            let _ = writeln!(
                s,
                "**Timestamp:** {}",
                snapshot.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            );
            if let Some(diag) = output.get("diagnosis").and_then(|v| v.as_str()) {
                let _ = writeln!(s, "**Diagnosis:** {}", diag);
            }
            if let Some(plan) = output.get("remediation_plan").and_then(|v| v.as_array()) {
                for step in plan {
                    let action = step.get("action").and_then(|v| v.as_str()).unwrap_or("?");
                    let target = step
                        .get("target")
                        .and_then(|v| v.as_str())
                        .unwrap_or("system");
                    let _ = writeln!(s, "- {} -> {}", action, target);
                }
            }
            return s;
        }

        let mut s = String::new();
        let _ = writeln!(s, "## Metacognition Update\n");
        let _ = writeln!(
            s,
            "**Timestamp:** {}",
            snapshot.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        let _ = writeln!(s, "**CNS Health:** {}", snapshot.cns_health);
        let _ = writeln!(s, "**Variety Deficit:** {}", snapshot.variety_deficit);
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
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — direct a bot to take corrective action
    /// pre:  `bot_name` is a non-empty string; `reason` is a non-empty
    ///       string; `self.context.a2a()` may be `Some` or `None`.
    /// post: If A2A is configured, sends a `TemplateDispatch` directive
    ///       to the bot and returns `Ok(())`. If A2A is not configured,
    ///       logs a warning and returns `Ok(())` (graceful degradation).
    ///       Returns `Err` on A2A send failure.
    pub async fn direct_bot(
        &self,
        bot_name: &str,
        reason: &str,
    ) -> Result<(), crate::error::CoreError> {
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
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — delegate directive to CuratorContext
    /// pre:  `directive` is a valid `CuratorDirective`.
    /// post: Delegates to `self.context.issue_directive(directive)`;
    ///       same post-conditions as `CuratorContext::issue_directive`.
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
        let target = Self::param_str(action, "target", "");
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
            "restart" | "rebalance" => {
                let diagnosis = action
                    .parameters
                    .get("diagnosis")
                    .and_then(|v| v.as_str())
                    .unwrap_or("template-diagnosed");
                warn!(target: MC_TARGET, bot = %target, metric, diagnosis, "Template-directed bot action");
                Some(EscalationEntry::pending(
                    format!("{} bot {} ({})", metric, target, diagnosis),
                    0.7,
                    format!(
                        "Template {} directed {} on {}: {}",
                        action
                            .parameters
                            .get("template")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown"),
                        metric,
                        target,
                        diagnosis
                    ),
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

    /// Template-driven compute: invoke KnowAct templates for calibrated decisions.
    async fn compute_with_templates(
        &self,
        executor: &Arc<hkask_templates::ManifestExecutor>,
        deviations: &[Deviation],
    ) -> Vec<LoopAction> {
        let snapshot = self.last_snapshot_tx.borrow().clone();
        let mut ctx = std::collections::HashMap::new();

        if let Some(ref snap) = snapshot {
            ctx.insert("system_health".into(), serde_json::json!(snap.cns_health));
            ctx.insert(
                "critical_alerts".into(),
                serde_json::json!(snap.critical_alerts),
            );
            ctx.insert("total_alerts".into(), serde_json::json!(snap.total_alerts));
            ctx.insert(
                "variety_deficit".into(),
                serde_json::json!(snap.variety_deficit),
            );
            // Build bot_status for template context
            let bot_status: Vec<serde_json::Value> = snap
                .bot_status_reports
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "name": r.bot_name,
                        "status": r.status.to_string(),
                        "issues": r.issues,
                    })
                })
                .collect();
            ctx.insert("bot_status".into(), serde_json::json!(bot_status));
        }

        let issues: Vec<serde_json::Value> = deviations
            .iter()
            .filter(|d| d.direction == DeviationDirection::AboveSetPoint)
            .map(|d| {
                serde_json::json!({
                    "id": d.signal.metric.as_str(),
                    "source_bot": "cns",
                    "type": d.signal.metric.as_str(),
                    "severity": if d.magnitude > 2.0 { "critical" } else { "warning" },
                    "first_observed": d.signal.timestamp.to_rfc3339(),
                    "occurrence_count": 1,
                    "current_impact": format!("value {} > set-point {}", d.signal.value, d.signal.set_point),
                    "resolution_attempts": [],
                })
            })
            .collect();
        ctx.insert("issues".into(), serde_json::json!(issues));

        // ── Communication events: drain and process via respond template ──
        let mut actions = Vec::new();
        let comm_events = self.context.drain_communication_events().await;
        if !comm_events.is_empty() {
            let curator_name = &self.curator_name;
            for event in &comm_events {
                let bias = self.convergence_bias;
                let decision = cat::evaluate(bias, curator_name, event);
                if let cat::Decision::Speak { convergence_level } = decision {
                    let sender = event
                        .observation
                        .get("sender")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let body = event
                        .observation
                        .get("body")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let room_id = event
                        .observation
                        .get("room_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let mut resp_ctx = std::collections::HashMap::new();
                    resp_ctx.insert("message_body".into(), serde_json::json!(body));
                    resp_ctx.insert("sender".into(), serde_json::json!(sender));
                    resp_ctx.insert("room_id".into(), serde_json::json!(room_id));
                    resp_ctx.insert(
                        "convergence_bias".into(),
                        serde_json::json!(convergence_level),
                    );
                    resp_ctx.insert(
                        "invariant_traits".into(),
                        serde_json::json!(self.invariant_traits),
                    );

                    match executor
                        .execute_knowact("curator/metacognition-respond.j2", &resp_ctx)
                        .await
                    {
                        Ok(output) => {
                            if output
                                .get("should_respond")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                            {
                                let response_body = output
                                    .get("response_body")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if !response_body.is_empty() {
                                    let tool_input = serde_json::json!({
                                        "room_id": room_id,
                                        "body": response_body,
                                    });
                                    match executor
                                        .call_tool("communication/send_message", tool_input)
                                        .await
                                    {
                                        Ok(_) => {
                                            tracing::info!(
                                                target: MC_TARGET,
                                                sender = %sender,
                                                room_id = %room_id,
                                                "Communication response sent via MCP"
                                            );
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                target: MC_TARGET,
                                                error = %e,
                                                "Failed to send communication response via MCP"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(target: MC_TARGET, error = %e, "Communication respond template failed");
                        }
                    }
                }
            }
        }

        // Circuit breaker: skip template after 3 consecutive failures,
        // retry after 5 skip cycles.
        let skip = self
            .template_skip_remaining
            .load(std::sync::atomic::Ordering::Relaxed);
        if skip > 0 {
            self.template_skip_remaining
                .store(skip - 1, std::sync::atomic::Ordering::Relaxed);
            return self.compute_with_thresholds(deviations);
        }

        let result = match executor
            .execute_knowact("curator/metacognition-diagnose.j2", &ctx)
            .await
        {
            Ok(r) => {
                self.consecutive_template_failures
                    .store(0, std::sync::atomic::Ordering::Relaxed);
                r
            }
            Err(e) => {
                let failures = self
                    .consecutive_template_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1;
                tracing::warn!(target: MC_TARGET, error = %e, consecutive_failures = failures, "Template failed");
                if failures >= 3 {
                    self.template_skip_remaining
                        .store(5, std::sync::atomic::Ordering::Relaxed);
                    tracing::warn!(target: MC_TARGET, "Circuit breaker tripped — skipping template for 5 cycles");
                }
                return self.compute_with_thresholds(deviations);
            }
        };

        // actions declared above for communication events; continued here
        if let Some(plan) = result.get("remediation_plan").and_then(|v| v.as_array()) {
            if plan.is_empty() {
                tracing::info!(target: MC_TARGET, "LLM returned empty remediation_plan — no actions");
            }
            for step in plan {
                let action_type = step.get("action").and_then(|v| v.as_str()).unwrap_or("");
                let target = step.get("target").and_then(|v| v.as_str()).unwrap_or("");
                if action_type.is_empty() {
                    tracing::warn!(target: MC_TARGET, ?step, "LLM produced malformed remediation step — missing 'action' field");
                }
                match action_type {
                    "calibrate" | "adjust_threshold" => actions.push(LoopAction::new(
                        LoopId::Curation, ActionType::Calibrate,
                        serde_json::json!({"domain": target, "template": "metacognition-diagnose"}),
                    )),
                    "adjust_budget" => {
                        let new_budget = step.get("new_budget").and_then(|v| v.as_u64()).unwrap_or(0);
                        if new_budget > 0 && !target.is_empty() {
                            actions.push(LoopAction::new(
                                LoopId::Curation, ActionType::OverrideEnergyBudget,
                                serde_json::json!({"metric": "adjust_budget", "target": target, "new_budget": new_budget, "template": "metacognition-diagnose"}),
                            ));
                        }
                    }
                    "escalate" | "restart" | "rebalance" => actions.push(LoopAction::new(
                        LoopId::Curation, ActionType::Escalate,
                        serde_json::json!({"metric": action_type, "target": target, "diagnosis": result.get("diagnosis"), "template": "metacognition-diagnose"}),
                    )),
                    _ => actions.push(LoopAction::new(
                        LoopId::Curation, ActionType::Notify,
                        serde_json::json!({"action": action_type, "target": target}),
                    )),
                }
            }
        }

        // Store template output for act phase and generate_summary.
        // Uses a dedicated RwLock to avoid the race condition where
        // sense() would wipe template_output from HealthSnapshot.
        *self.last_template_output.write().await = Some(result.clone());

        actions
    }

    /// Fallback: Rust threshold comparison (standalone CLI, no executor).
    fn compute_with_thresholds(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
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
            variety_deficit: total_variety_deficit,
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
    ///
    /// Per P3 (Generative Space), when a ManifestExecutor is available,
    /// calibrated decisions are produced by KnowAct templates, not Rust
    /// threshold comparison. Falls back to Rust logic when no executor
    /// is configured (standalone CLI).
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        if let Some(executor) = self.context.manifest_executor().await {
            return self.compute_with_templates(&executor, deviations).await;
        }
        self.compute_with_thresholds(deviations)
    }

    /// Act: issue CuratorDirectives, direct bots, and post escalations.
    ///
    /// When a template output is available (from compute_with_templates),
    /// "restart" and "rebalance" actions trigger bot direction via A2A
    /// in addition to escalation queue audit entries.
    async fn act(&self, actions: &[LoopAction]) {
        let mut escalation_entries: Vec<EscalationEntry> = Vec::new();

        for action in actions {
            // Template-driven bot direction: when the LLM says restart/rebalance,
            // send an A2A directive to the target bot before posting the escalation.
            let metric = action
                .parameters
                .get("metric")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let target = action
                .parameters
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if matches!(metric, "restart" | "rebalance")
                && !target.is_empty()
                && let Err(e) = self.direct_bot(target, metric).await
            {
                tracing::warn!(target: MC_TARGET, bot = %target, error = %e, "Failed to direct bot");
            }

            // OverrideEnergyBudget from template (LLM-computed, replaces hardcoded 5000)
            if matches!(metric, "adjust_budget") {
                let new_budget = action
                    .parameters
                    .get("new_budget")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if new_budget > 0 && !target.is_empty() {
                    let directive = CuratorDirective::OverrideEnergyBudget {
                        agent: hkask_types::WebID::from_persona(target.as_bytes()),
                        new_budget,
                    };
                    self.context.issue_directive(directive).await;
                }
            }

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
            // Invoke escalate template for LLM-formatted notification when executor available
            let summary = if let Some(executor) = self.context.manifest_executor().await {
                let mut ctx = std::collections::HashMap::new();
                ctx.insert("critical_issues".into(), serde_json::json!([]));
                ctx.insert("system_health".into(), serde_json::json!("degraded"));
                ctx.insert("variety_deficit".into(), serde_json::json!(0));
                ctx.insert("active_alerts".into(), serde_json::json!([]));
                ctx.insert("bot_failures".into(), serde_json::json!([]));
                ctx.insert("energy_budget_status".into(), serde_json::json!("unknown"));
                ctx.insert("required_actions".into(), serde_json::json!([]));
                match executor
                    .execute_knowact("curator/metacognition-escalate.j2", &ctx)
                    .await
                {
                    Ok(output) => output
                        .get("notification")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&summary)
                        .to_string(),
                    Err(_) => summary,
                }
            } else {
                summary
            };
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
                tracing::error!(
                    target: "cns.curation.escalation",
                    error = %e,
                    batch_size = batch.entries.len(),
                    "Failed to add consolidated escalation batch — escalations LOST"
                );
            }
        } else {
            let mut lost_count = 0u32;
            for entry in escalation_entries {
                if let Err(e) = self.context.escalation_queue().add(
                    entry.template_id,
                    entry.bot_id,
                    entry.output,
                    entry.confidence,
                    entry.retry_count,
                    entry.error_context,
                ) {
                    lost_count += 1;
                    tracing::error!(
                        target: "cns.curation.escalation",
                        error = %e,
                        template_id = %entry.template_id,
                        bot_id = %entry.bot_id,
                        "Failed to add escalation — escalation LOST"
                    );
                }
            }
            if lost_count > 0 {
                tracing::error!(
                    target: "cns.curation.escalation",
                    lost = lost_count,
                    "{} escalation(s) could not be persisted — check escalation queue health",
                    lost_count
                );
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
