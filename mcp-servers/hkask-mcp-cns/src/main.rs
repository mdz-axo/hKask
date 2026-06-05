//! hKask MCP CNS — Cybernetic Nervous System monitoring and alerts
//!
//! Starts an MCP server over stdio exposing 10 tools:
//! - `cns_emit` — Emit a CNS observation event
//! - `cns_variety` — Get variety count for a span pattern
//! - `cns_alert` — Trigger a real algedonic alert
//! - `cns_calibrate` — Calibrate a span threshold
//! - `cns_list_alerts` — List active algedonic alerts
//! - `cns_health` — Get CNS health status
//! - `cns_kill_zone` — Check or update kill-zone state
//! - `cns_replenish_budget` — Replenish an agent's gas budget
//! - `cns_energy` — Get an agent's gas budget status
//! - `cns_backpressure` — Emit a backpressure signal

use hkask_cns::{CnsRuntime, DEFAULT_THRESHOLD};
use hkask_mcp::server::ToolSpanGuard;
use hkask_mcp::validate_field;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmitRequest {
    pub span: String,
    pub observer_webid: String,
    pub phase: String,
    pub observation: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VarietyRequest {
    pub span_pattern: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AlertRequest {
    pub span_pattern: String,
    pub severity: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalibrateRequest {
    pub span_pattern: String,
    pub new_threshold: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListAlertsRequest {
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KillZoneRequest {
    /// Optional: update VC investment level (0.0 to 1.0)
    pub vc_investment: Option<f32>,
    /// Optional: mark acquisition attempt detected
    pub acquisition_attempt: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReplenishBudgetRequest {
    /// Agent WebID to replenish
    pub agent_id: String,
    /// Amount of gas to add
    pub amount: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnergyRequest {
    /// Agent WebID to check (optional — if omitted, uses the calling agent)
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BackpressureRequest {
    /// Backpressure severity (0.0–1.0)
    pub severity: f64,
    /// Reason for the backpressure signal
    pub reason: String,
}

pub struct CnsServer {
    runtime: Arc<CnsRuntime>,
    threshold: AtomicU64,
    webid: WebID,
}

impl CnsServer {
    pub fn new(threshold: Option<u64>, webid: WebID) -> Self {
        let threshold = threshold.unwrap_or(DEFAULT_THRESHOLD);

        let runtime = CnsRuntime::with_threshold(threshold);

        Self {
            runtime: Arc::new(runtime),
            threshold: AtomicU64::new(threshold),
            webid,
        }
    }
}

#[tool_router(server_handler)]
impl CnsServer {
    #[tool(description = "Emit a CNS observation event")]
    async fn cns_emit(
        &self,
        Parameters(EmitRequest {
            span,
            observer_webid,
            phase,
            observation,
        }): Parameters<EmitRequest>,
    ) -> String {
        let span_guard = ToolSpanGuard::new("cns_emit", &self.webid);

        // Validate identifiers
        validate_field!(span_guard, "span", &span, 256);
        validate_field!(span_guard, "observer_webid", &observer_webid, 128);

        let observation_value = serde_json::from_str(&observation)
            .unwrap_or(serde_json::Value::String(observation.clone()));

        tracing::debug!(
            target: "cns.mcp",
            span = %span,
            verb = "observe",
            payload = ?observation_value,
            confidence = 1.0,
            "CNS event"
        );

        self.runtime.increment_variety(&span, &phase).await;

        span_guard.ok_json(serde_json::json!({
            "span": span,
            "observer": observer_webid,
            "phase": phase,
            "emitted": true,
        }))
    }

    #[tool(description = "Get variety count for a span pattern via real VarietyMonitor")]
    async fn cns_variety(
        &self,
        Parameters(VarietyRequest { span_pattern }): Parameters<VarietyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_variety", &self.webid);

        // Validate identifiers
        validate_field!(span, "span_pattern", &span_pattern, 256);

        let variety_count = self.runtime.variety_for_domain(&span_pattern).await;
        let deficit = variety_count > self.threshold.load(Ordering::Relaxed);

        span.ok_json(serde_json::json!({
            "span_pattern": span_pattern,
            "variety_count": variety_count,
            "deficit": deficit,
        }))
    }

    #[tool(description = "Trigger a real algedonic alert via AlgedonicManager")]
    async fn cns_alert(
        &self,
        Parameters(AlertRequest {
            span_pattern,
            severity,
        }): Parameters<AlertRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_alert", &self.webid);

        // Validate identifiers
        validate_field!(span, "span_pattern", &span_pattern, 256);
        validate_field!(span, "severity", &severity, 32);

        let alert = self.runtime.check_variety(&span_pattern).await;

        match alert {
            Some(a) => span.ok_json(serde_json::json!({
                "alert_id": a.domain,
                "span": span_pattern,
                "severity": severity,
                "deficit": a.deficit,
                "triggered": true,
            })),
            None => span.ok_json(serde_json::json!({
                "span": span_pattern,
                "severity": severity,
                "triggered": true,
                "deficit": 0,
            })),
        }
    }

    #[tool(description = "Calibrate a span threshold")]
    async fn cns_calibrate(
        &self,
        Parameters(CalibrateRequest {
            span_pattern,
            new_threshold,
        }): Parameters<CalibrateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_calibrate", &self.webid);

        // Validate identifiers
        validate_field!(span, "span_pattern", &span_pattern, 256);

        let old_threshold = self.threshold.load(Ordering::Relaxed);
        self.runtime
            .calibrate_threshold(&span_pattern, new_threshold)
            .await;
        self.threshold.store(new_threshold, Ordering::Relaxed);

        span.ok_json(serde_json::json!({
            "span": span_pattern,
            "old_threshold": old_threshold,
            "new_threshold": new_threshold,
            "calibrated": true,
        }))
    }

    #[tool(description = "List active algedonic alerts from real alert manager")]
    async fn cns_list_alerts(
        &self,
        Parameters(ListAlertsRequest { limit }): Parameters<ListAlertsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_list_alerts", &self.webid);

        let alerts = self.runtime.alerts().await;
        let limit = limit.unwrap_or(10) as usize;
        let displayed: Vec<serde_json::Value> = alerts
            .iter()
            .take(limit)
            .map(|a| {
                serde_json::json!({
                    "domain": a.domain,
                    "severity": format!("{:?}", a.severity),
                    "deficit": a.deficit,
                    "escalated": a.escalated,
                    "message": a.message,
                })
            })
            .collect();

        span.ok_json(serde_json::json!({
            "alert_count": alerts.len(),
            "alerts": displayed,
        }))
    }

    #[tool(description = "Get real CNS health status")]
    async fn cns_health(&self) -> String {
        let span = ToolSpanGuard::new("cns_health", &self.webid);

        let health = self.runtime.health().await;

        span.ok_json(serde_json::json!({
            "healthy": health.healthy,
            "active_alerts": health.critical_count + health.warning_count,
            "critical_count": health.critical_count,
            "warning_count": health.warning_count,
            "overall_deficit": health.overall_deficit,
        }))
    }

    #[tool(description = "Check or update kill-zone state (VC investment, acquisition detection)")]
    async fn cns_kill_zone(
        &self,
        Parameters(KillZoneRequest {
            vc_investment,
            acquisition_attempt,
        }): Parameters<KillZoneRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_kill_zone", &self.webid);

        // If parameters provided, update and check
        let triggered = if vc_investment.is_some() || acquisition_attempt.is_some() {
            self.runtime
                .check_kill_zone(
                    vc_investment.unwrap_or(1.0),
                    acquisition_attempt.unwrap_or(false),
                )
                .await
        } else {
            // Just read current state
            let state = self.runtime.kill_zone_state().await;
            state.kill_zone_active
        };

        let state = self.runtime.kill_zone_state().await;

        span.ok_json(serde_json::json!({
            "kill_zone_active": state.kill_zone_active,
            "vc_investment": state.vc_investment,
            "acquisition_attempt": state.acquisition_attempt,
            "triggered": triggered,
        }))
    }

    #[tool(description = "Replenish an agent's gas budget (Curator authority required)")]
    async fn cns_replenish_budget(
        &self,
        Parameters(ReplenishBudgetRequest { agent_id, amount }): Parameters<ReplenishBudgetRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_replenish_budget", &self.webid);

        validate_field!(span, "agent_id", &agent_id, 128);

        // The MCP server's calling WebID acts as the authority check.
        // OCAP gating on the GovernedTool membrane ensures only authorized
        // callers can invoke this tool.
        let agent = WebID::from_string(&agent_id);

        let remaining = self.runtime.replenish_agent_budget(&agent, amount).await;

        span.ok_json(serde_json::json!({
            "agent_id": agent_id,
            "replenished": amount,
            "remaining": remaining,
        }))
    }

    #[tool(description = "Get an agent's gas budget status (energy level, usage, limits)")]
    async fn cns_energy(
        &self,
        Parameters(EnergyRequest { agent_id }): Parameters<EnergyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_energy", &self.webid);

        // Resolve agent: use provided agent_id or fall back to calling agent
        let agent_str = agent_id.unwrap_or_else(|| self.webid.to_string());
        validate_field!(span, "agent_id", &agent_str, 128);
        let agent = WebID::from_string(&agent_str);

        match self.runtime.agent_gas_status(&agent).await {
            Some(status) => span.ok_json(serde_json::json!({
                "agent_id": agent_str,
                "cap": status.cap,
                "remaining": status.remaining,
                "reserved": status.reserved,
                "available": status.available,
                "usage_ratio": (status.usage_ratio * 100.0).round() / 100.0,
                "hard_limit": status.hard_limit,
                "alert_threshold": status.alert_threshold,
            })),
            None => span.ok_json(serde_json::json!({
                "agent_id": agent_str,
                "error": "No budget registered for agent",
            })),
        }
    }

    #[tool(description = "Emit a backpressure signal to throttle downstream loops")]
    async fn cns_backpressure(
        &self,
        Parameters(BackpressureRequest { severity, reason }): Parameters<BackpressureRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_backpressure", &self.webid);

        validate_field!(span, "reason", &reason, 256);

        let clamped = severity.clamp(0.0, 1.0);
        let signal = hkask_types::ports::BackpressureSignal {
            source: hkask_types::loops::LoopId::Cybernetics,
            reason: reason.clone(),
            severity: clamped,
        };
        self.runtime.emit_backpressure(signal.clone()).await;

        span.ok_json(serde_json::json!({
            "source": "Cybernetics",
            "severity": clamped,
            "reason": reason,
            "emitted": true,
        }))
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-cns",
    factory: |ctx: hkask_mcp::ServerContext| {
        let threshold: Option<u64> = ctx
            .credentials
            .get("HKASK_CNS_THRESHOLD")
            .and_then(|s| s.parse().ok());
        Ok(CnsServer::new(threshold, ctx.webid))
    },
    credentials: vec![hkask_mcp::CredentialRequirement::optional(
        "HKASK_CNS_THRESHOLD",
        "CNS variety deficit threshold (default: 100)",
    )]
);
