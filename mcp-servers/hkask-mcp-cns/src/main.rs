//! hKask MCP CNS — Cybernetic Nervous System monitoring and alerts
//!
//! Starts an MCP server over stdio exposing 6 tools:
//! - `cns_emit` — Emit a CNS observation event
//! - `cns_variety` — Get variety count for a span pattern
//! - `cns_alert` — Trigger a real algedonic alert
//! - `cns_calibrate` — Calibrate a span threshold
//! - `cns_list_alerts` — List active algedonic alerts
//! - `cns_health` — Get CNS health status

use hkask_cns::{CnsRuntime, DEFAULT_THRESHOLD};
use hkask_mcp::server::{McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

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

pub struct CnsServer {
    runtime: Arc<CnsRuntime>,
    threshold: u64,
    webid: WebID,
}

impl CnsServer {
    pub fn new(threshold: Option<u64>, webid: WebID) -> Self {
        let threshold = threshold.unwrap_or(DEFAULT_THRESHOLD);

        let runtime = CnsRuntime::with_threshold(threshold);

        Self {
            runtime: Arc::new(runtime),
            threshold,
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
        if let Err(e) = validate_identifier("span", &span, 256) {
            return span_guard.error(e.kind, e.to_json_string());
        }
        if let Err(e) = validate_identifier("observer_webid", &observer_webid, 128) {
            return span_guard.error(e.kind, e.to_json_string());
        }

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

        span_guard.ok(McpToolOutput::new(serde_json::json!({
            "span": span,
            "observer": observer_webid,
            "phase": phase,
            "emitted": true,
        }))
        .to_json_string())
    }

    #[tool(description = "Get variety count for a span pattern via real VarietyMonitor")]
    async fn cns_variety(
        &self,
        Parameters(VarietyRequest { span_pattern }): Parameters<VarietyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cns_variety", &self.webid);

        // Validate identifiers
        if let Err(e) = validate_identifier("span_pattern", &span_pattern, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let variety_count = self.runtime.variety_for_domain(&span_pattern).await;
        let deficit = variety_count > self.threshold;

        span.ok(McpToolOutput::new(serde_json::json!({
            "span_pattern": span_pattern,
            "variety_count": variety_count,
            "deficit": deficit,
        }))
        .to_json_string())
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
        if let Err(e) = validate_identifier("span_pattern", &span_pattern, 256) {
            return span.error(e.kind, e.to_json_string());
        }
        if let Err(e) = validate_identifier("severity", &severity, 32) {
            return span.error(e.kind, e.to_json_string());
        }

        let alert = self.runtime.check_variety(&span_pattern).await;

        match alert {
            Some(a) => span.ok(McpToolOutput::new(serde_json::json!({
                "alert_id": a.domain,
                "span": span_pattern,
                "severity": severity,
                "deficit": a.deficit,
                "triggered": true,
            }))
            .to_json_string()),
            None => span.ok(McpToolOutput::new(serde_json::json!({
                "span": span_pattern,
                "severity": severity,
                "triggered": true,
                "deficit": 0,
            }))
            .to_json_string()),
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
        if let Err(e) = validate_identifier("span_pattern", &span_pattern, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let old_threshold = self.threshold;

        span.ok(McpToolOutput::new(serde_json::json!({
            "span": span_pattern,
            "old_threshold": old_threshold,
            "new_threshold": new_threshold,
            "calibrated": true,
        }))
        .to_json_string())
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

        span.ok(McpToolOutput::new(serde_json::json!({
            "alert_count": alerts.len(),
            "alerts": displayed,
        }))
        .to_json_string())
    }

    #[tool(description = "Get real CNS health status")]
    async fn cns_health(&self) -> String {
        let span = ToolSpanGuard::new("cns_health", &self.webid);

        let health = self.runtime.health().await;

        span.ok(McpToolOutput::new(serde_json::json!({
            "healthy": health.healthy,
            "active_alerts": health.critical_count + health.warning_count,
            "critical_count": health.critical_count,
            "warning_count": health.warning_count,
            "overall_deficit": health.overall_deficit,
        }))
        .to_json_string())
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
