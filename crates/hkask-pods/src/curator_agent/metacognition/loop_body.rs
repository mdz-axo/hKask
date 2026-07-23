//! MetacognitionLoop struct definition and core methods.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ports::EscalationEntry;
use hkask_regulation::meta_span::{emit_meta_circuit_breaker, emit_meta_self_calibration};
use hkask_regulation::types::loops::{
    ActionType, Deviation, DeviationDirection, Loop, LoopId, RegulationData, RegulatoryAction,
    RegulatoryActionParams, SignalMetric,
};
use hkask_types::WebID;
use hkask_types::curator::CuratorDirective;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::a2a::A2AMessage;
use crate::curation::context::CuratorContext;
use crate::curator_agent::cat;
use crate::pod::CommunicationPosture;

use super::config::{HealthSnapshot, MC_TARGET, MetacognitionConfig};
use super::escalation::EscalationPolicy;

/// Metacognition loop — Curator Agent's system governance mechanism.
pub struct MetacognitionLoop {
    pub(super) context: Arc<CuratorContext>,
    pub(super) config: MetacognitionConfig,
    pub(super) escalation_policy: EscalationPolicy,
    pub(super) last_snapshot_tx: tokio::sync::watch::Sender<Option<HealthSnapshot>>,
    /// Template output from the most recent template-driven compute cycle.
    /// Stored separately from HealthSnapshot to avoid race conditions —
    /// `sense()` wipes the snapshot each cycle but template output must
    /// survive across cycles for `generate_summary()` and `act()`.
    pub(super) last_template_output: RwLock<Option<serde_json::Value>>,
    /// Circuit breaker: consecutive template invocation failures.
    /// After 3 consecutive failures, skip template for 5 cycles.
    pub(super) consecutive_template_failures: std::sync::atomic::AtomicU64,
    pub(super) template_skip_remaining: std::sync::atomic::AtomicU64,
    pub(super) last_cal_dropped: std::sync::atomic::AtomicU64,
    pub(super) last_cal_directives: std::sync::atomic::AtomicU64,
    /// Communication posture — loaded from the agent's persona.
    /// Governs speak/silent decisions and accommodation level.
    pub(super) communication_posture: CommunicationPosture,
    /// Agent name used in template compute and communication dispatch.
    /// Defaults to "curator" for backward compatibility; derived from
    /// `CuratorContext::handle().curator_id()` when constructed via
    /// `CuratorAgent`.
    agent_name: String,
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
        Self::with_posture(
            context,
            config,
            CommunicationPosture::default(),
            "curator".to_string(),
        )
    }

    /// Create a new metacognition loop with a specific communication posture.
    ///
    /// This is the canonical constructor — `new()` delegates to it with defaults.
    pub fn with_posture(
        context: Arc<CuratorContext>,
        config: MetacognitionConfig,
        posture: CommunicationPosture,
        agent_name: String,
    ) -> Self {
        let escalation_policy = EscalationPolicy::new(config.thresholds.clone());
        let (last_snapshot_tx, _) = tokio::sync::watch::channel(None);
        Self {
            context,
            escalation_policy,
            config,
            last_snapshot_tx,
            last_template_output: RwLock::new(None),
            consecutive_template_failures: std::sync::atomic::AtomicU64::new(0),
            template_skip_remaining: std::sync::atomic::AtomicU64::new(0),
            last_cal_dropped: std::sync::atomic::AtomicU64::new(0),
            last_cal_directives: std::sync::atomic::AtomicU64::new(0),
            communication_posture: posture,
            agent_name,
        }
    }

    /// Create a new metacognition loop with a BotHealthEvaluator.
    ///
    /// The evaluator reads gas data from the Regulation runtime and populates
    /// bot health reports at each cycle.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — classify bot energy health for Curator
    /// \[P4\] Constraining: Clear Boundaries — thresholds map consumption ratio to status
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`; `evaluator` is a valid
    ///       `Arc<BotHealthEvaluator>`.
    /// post: Returns a `MetacognitionLoop` with the evaluator wired in.
    /// Access the metacognition configuration.
    pub fn config(&self) -> &MetacognitionConfig {
        &self.config
    }

    /// Return the agent name used in template compute and communication dispatch.
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }

    /// Set the agent name for template compute and communication dispatch.
    ///
    /// Defaults to `"curator"` when constructed via [`new`](Self::new).
    /// Set explicitly when a persona-derived name is available.
    pub fn with_agent_name(mut self, name: String) -> Self {
        self.agent_name = name;
        self
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
        let signals = self.sense().await;
        let deviations = self.compare(&signals).await;
        let actions = self.compute(&deviations).await;
        self.act(&actions).await;
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
    ///       with timestamp, Regulation health, critical/total alerts, variety
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
        let _ = writeln!(s, "**Regulation Health:** {}", snapshot.reg_health);
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

    pub(super) async fn act_on_throttle(
        &self,
        action: &RegulatoryAction,
    ) -> Option<EscalationEntry> {
        // Source data from the HealthSnapshot built in sense(), not from the
        // action's typed RegulationData (which is NoData for metacognition actions).
        match action.parameters.reason.as_str() {
            "variety_deficit" | "calibrate" | "adjust_threshold" => {
                let deficit = self
                    .last_snapshot_tx
                    .borrow()
                    .as_ref()
                    .map(|s| s.variety_deficit)
                    .unwrap_or(0);
                let new_threshold = self.config.thresholds.variety_deficit;
                let directive = CuratorDirective::CalibrateThreshold {
                    domain: "variety".to_string(),
                    new_threshold,
                };
                self.issue_directive(directive).await;

                let error_context = format!(
                    "Total variety deficit ({}) exceeds threshold ({})",
                    deficit, new_threshold
                );
                Some(EscalationEntry::pending(
                    format!("Variety deficit: {}", deficit),
                    0.6,
                    error_context,
                ))
            }
            _ => None,
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
    pub(super) async fn act_on_escalate(
        &self,
        action: &RegulatoryAction,
    ) -> Option<EscalationEntry> {
        let reason = action.parameters.reason.as_str();
        match reason {
            "critical_alerts" => {
                let count = self
                    .last_snapshot_tx
                    .borrow()
                    .as_ref()
                    .map(|s| s.critical_alerts)
                    .unwrap_or(0);
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
            // No bot-health subsystem exists (sense passes bot_failures: 0),
            // so no producer emits this reason and no data source is available.
            "bot_failures" => {
                warn!(
                    target: MC_TARGET,
                    "bot_failures escalation requested but no bot-health subsystem is wired"
                );
                None
            }
            "restart" | "rebalance" | "escalate" => {
                // Template-directed: diagnosis lives in last_template_output,
                // not in the action (which carries only the reason string).
                let diagnosis = {
                    let guard = self.last_template_output.read().await;
                    guard
                        .as_ref()
                        .and_then(|o| o.get("diagnosis").and_then(|v| v.as_str()))
                        .unwrap_or("template-diagnosed")
                        .to_string()
                };
                warn!(target: MC_TARGET, metric = %reason, diagnosis = %diagnosis, "Template-directed bot action");
                Some(EscalationEntry::pending(
                    format!("{} ({})", reason, diagnosis),
                    0.7,
                    format!("Template directed {}: {}", reason, diagnosis),
                ))
            }
            _ => {
                warn!(target: MC_TARGET, metric = %reason, "Unknown escalation metric");
                None
            }
        }
    }

    /// Log an unhandled action type (no-op).
    pub(super) fn act_on_no_action(&self, action: &RegulatoryAction) {
        info!(
            target: MC_TARGET,
            action_type = ?action.action_type,
            "Unhandled action type in MetacognitionLoop act()"
        );
    }

    /// Handle an `OverrideEnergyBudget` action: extract the typed
    /// `CuratorBudgetOverride` data and issue a `CuratorDirective`.
    ///
    /// This closes the gap where the LLM's `adjust_budget` remediation step
    /// was silently dropped (the action carried only a reason string).
    pub(super) async fn act_on_budget_override(&self, action: &RegulatoryAction) {
        if let RegulationData::CuratorBudgetOverride { agent, new_budget } = &action.parameters.data
        {
            let directive = CuratorDirective::OverrideEnergyBudget {
                agent: WebID::from_persona(agent.as_bytes()),
                new_budget: *new_budget,
            };
            self.issue_directive(directive).await;
            info!(
                target: MC_TARGET,
                agent = %agent,
                new_budget,
                "Issued OverrideEnergyBudget from template directive"
            );
        } else {
            warn!(
                target: MC_TARGET,
                "OverrideEnergyBudget action carried no CuratorBudgetOverride data — ignored"
            );
        }
    }

    // Explicit 4-stage cycle: sense → compare → compute → act
    // Delegation methods removed — RegulationLoop trait impl provides tick().

    /// Template-driven compute: invoke KnowAct templates for calibrated decisions.
    /// Self-calibration: the Curator observes its OWN decision quality and
    /// adjusts its escalation sensitivity. This is the meta-cybernetic loop
    /// (Meta -> Curation -> Cybernetics), kept non-circular by reading in-process
    /// SelfQuality counters (not reg.* algedonic events, which CurationLoop
    /// reads for the system).
    ///
    /// Policy (first cut, deliberately conservative — only ever RAISES the
    /// variety-deficit threshold to reduce noise):
    /// - If escalations are being dropped (delta >= 3 since last calibration),
    ///   the feedback loop is overloaded, so raise the variety-deficit threshold
    ///   by 10% so the Curator escalates less aggressively.
    /// - Lowering is intentionally omitted until effectiveness-driven lowering
    ///   is validated (avoid runaway oscillation).
    pub(super) fn self_calibrate(&self) {
        let sq = self.context.self_quality().snapshot();
        let prev_dropped = self
            .last_cal_dropped
            .swap(sq.escalations_dropped, std::sync::atomic::Ordering::Relaxed);
        let prev_directives = self
            .last_cal_directives
            .swap(sq.directives_issued, std::sync::atomic::Ordering::Relaxed);

        let delta_dropped = sq.escalations_dropped.saturating_sub(prev_dropped);
        let delta_directives = sq.directives_issued.saturating_sub(prev_directives);

        // Only calibrate when there is recent activity to learn from.
        if delta_directives == 0 && delta_dropped == 0 {
            return;
        }

        // Escalations being lost means the Curator is too sensitive for the
        // queue capacity. Raise the variety-deficit threshold by 10% (at least
        // +1, capped at 2x to bound runaway growth).
        if delta_dropped >= 3 {
            let current = self.escalation_policy.thresholds();
            let old = current.variety_deficit;
            let cap = old.saturating_mul(2).max(old + 1);
            let new = ((old / 10).max(1) + old).min(cap);
            if new != old {
                let mut next = current.clone();
                next.variety_deficit = new;
                self.escalation_policy.set_thresholds(next);
                if let Some(sink) = self.context.regulation_sink() {
                    emit_meta_self_calibration(
                        sink.as_ref(),
                        self.context.handle().curator_id(),
                        "variety_deficit",
                        old,
                        new,
                    );
                }
                tracing::info!(
                    target: MC_TARGET,
                    old,
                    new,
                    delta_dropped,
                    "Self-calibration: raised variety_deficit threshold (escalations were being dropped)"
                );
            }
        }
    }

    pub(super) async fn compute_with_templates(
        &self,
        executor: &Arc<hkask_templates::ManifestExecutor>,
        deviations: &[Deviation],
    ) -> Vec<RegulatoryAction> {
        let snapshot = self.last_snapshot_tx.borrow().clone();
        let mut ctx = HashMap::new();

        if let Some(ref snap) = snapshot {
            ctx.insert("system_health".into(), serde_json::json!(snap.reg_health));
            ctx.insert(
                "critical_alerts".into(),
                serde_json::json!(snap.critical_alerts),
            );
            ctx.insert("total_alerts".into(), serde_json::json!(snap.total_alerts));
            ctx.insert(
                "variety_deficit".into(),
                serde_json::json!(snap.variety_deficit),
            );
        }

        let issues: Vec<serde_json::Value> = deviations
            .iter()
            .filter(|d| d.direction == DeviationDirection::AboveSetPoint)
            .map(|d| {
                serde_json::json!({
                    "id": d.signal.metric.as_str(),
                    "source_bot": "regulation",
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
            let agent_name = &self.agent_name;
            for event in &comm_events {
                let bias = self.communication_posture.convergence_bias;

                // Score message saliency against persona via condenser MCP tool.
                // Saliency pulls effective bias upward — domain-relevant messages
                // trigger stronger convergence. Graceful degradation: default 0.5 on error.
                let body = event
                    .observation
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let persona_score = match executor
                    .call_tool(
                        "condenser/condenser_score_saliency",
                        serde_json::json!({"text": body, "against": "persona"}),
                    )
                    .await
                {
                    Ok(resp) => resp.get("score").and_then(|v| v.as_f64()).unwrap_or(0.5),
                    Err(e) => {
                        tracing::debug!(
                            target: MC_TARGET,
                            error = %e,
                            "Condenser saliency unavailable, using default 0.5"
                        );
                        0.5
                    }
                };
                let effective_bias = (bias + persona_score * (1.0 - bias)).min(1.0);

                let decision = cat::evaluate(effective_bias, agent_name, event);
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

                    let mut resp_ctx = HashMap::new();
                    resp_ctx.insert("message_body".into(), serde_json::json!(body));
                    resp_ctx.insert("sender".into(), serde_json::json!(sender));
                    resp_ctx.insert("room_id".into(), serde_json::json!(room_id));
                    resp_ctx.insert(
                        "convergence_bias".into(),
                        serde_json::json!(convergence_level),
                    );
                    resp_ctx.insert(
                        "invariant_traits".into(),
                        serde_json::json!(self.communication_posture.invariant_traits),
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
                    self.context.self_quality().record_circuit_breaker();
                    if let Some(sink) = self.context.regulation_sink() {
                        emit_meta_circuit_breaker(
                            sink.as_ref(),
                            self.context.handle().curator_id(),
                            5,
                        );
                    }
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
                    "calibrate" | "adjust_threshold" => actions.push(RegulatoryAction::new(
                        LoopId::Curation,
                        ActionType::Calibrate,
                        RegulatoryActionParams::reason("calibrate"),
                    )),
                    "adjust_budget" => {
                        let new_budget =
                            step.get("new_budget").and_then(|v| v.as_u64()).unwrap_or(0);
                        if new_budget > 0 && !target.is_empty() {
                            actions.push(RegulatoryAction::new(
                                LoopId::Curation,
                                ActionType::OverrideEnergyBudget,
                                RegulatoryActionParams::with_data(
                                    "adjust_budget",
                                    RegulationData::CuratorBudgetOverride {
                                        agent: target.to_string(),
                                        new_budget,
                                    },
                                ),
                            ));
                        }
                    }
                    "escalate" | "restart" | "rebalance" => actions.push(RegulatoryAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        RegulatoryActionParams::reason(action_type),
                    )),
                    _ => actions.push(RegulatoryAction::new(
                        LoopId::Curation,
                        ActionType::Notify,
                        RegulatoryActionParams::reason("notify"),
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
    pub(super) fn compute_with_thresholds(
        &self,
        deviations: &[Deviation],
    ) -> Vec<RegulatoryAction> {
        let mut actions = Vec::new();
        for dev in deviations {
            match dev.signal.metric {
                SignalMetric::MetacognitionVarietyDeficit
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    let _deficit = dev.signal.value as u64;
                    actions.push(RegulatoryAction::new(
                        LoopId::Curation,
                        ActionType::Calibrate,
                        RegulatoryActionParams::reason("variety_deficit"),
                    ));
                }
                SignalMetric::MetacognitionCriticalAlerts
                    if dev.direction == DeviationDirection::AboveSetPoint =>
                {
                    let _count = dev.signal.value as u64;
                    actions.push(RegulatoryAction::new(
                        LoopId::Curation,
                        ActionType::Escalate,
                        RegulatoryActionParams::reason("critical_alerts"),
                    ));
                }
                _ => {}
            }
        }
        actions
    }
}
