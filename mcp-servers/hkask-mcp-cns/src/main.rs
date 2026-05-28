//! hKask MCP CNS — Cybernetic Nervous System monitoring and alerts
//!
//! Starts an MCP server over stdio exposing 6 tools:
//! - `cns_emit` — Emit a CNS observation event via real SpanEmitter
//! - `cns_variety` — Get variety count for a span pattern
//! - `cns_alert` — Trigger a real algedonic alert
//! - `cns_calibrate` — Calibrate a span threshold
//! - `cns_list_alerts` — List active algedonic alerts
//! - `cns_health` — Get CNS health status

use hkask_cns::{CnsRuntime, DEFAULT_THRESHOLD, SpanEmitter};
use hkask_mcp::server::{
    CredentialRequirement, McpToolOutput, ServerContext, emit_tool_span, run_stdio_server,
    validate_identifier,
};
use hkask_types::{Span, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

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
    emitter: Arc<RwLock<SpanEmitter>>,
    threshold: u64,
}

impl CnsServer {
    pub fn new(threshold: Option<u64>) -> Self {
        let threshold = threshold.unwrap_or(DEFAULT_THRESHOLD);

        let runtime = CnsRuntime::with_threshold(threshold);
        let observer_webid = WebID::new();
        let emitter = SpanEmitter::new(observer_webid);

        Self {
            runtime: Arc::new(runtime),
            emitter: Arc::new(RwLock::new(emitter)),
            threshold,
        }
    }

    fn parse_span(raw: &str) -> Span {
        let parts: Vec<&str> = raw.splitn(3, '.').collect();
        match parts.get(1).copied() {
            Some("connector") => Span::Connector(raw.to_string()),
            Some("pipeline") => Span::Pipeline(raw.to_string()),
            Some("tool") => Span::Tool(raw.to_string()),
            Some("prompt") => Span::Prompt(raw.to_string()),
            Some("agent_pod") => Span::AgentPod(raw.to_string()),
            Some("energy") => Span::Energy(raw.to_string()),
            Some("sovereignty") => Span::Sovereignty(raw.to_string()),
            Some("goal") => Span::Goal(raw.to_string()),
            Some("review") => Span::Review(raw.to_string()),
            Some("spec") => Span::Spec(raw.to_string()),
            _ => Span::Tool(raw.to_string()),
        }
    }
}

#[tool_router(server_handler)]
impl CnsServer {
    #[tool(description = "Emit a CNS observation event via real SpanEmitter")]
    async fn cns_emit(
        &self,
        Parameters(EmitRequest {
            span,
            observer_webid,
            phase,
            observation,
        }): Parameters<EmitRequest>,
    ) -> String {
        let start = Instant::now();

        // Validate identifiers
        if let Err(e) = validate_identifier("span", &span, 256) {
            emit_tool_span(
                "cns_emit",
                "error",
                start.elapsed().as_millis() as u64,
                Some(&hkask_types::McpErrorKind::InvalidArgument),
            );
            return e.to_json_string();
        }
        if let Err(e) = validate_identifier("observer_webid", &observer_webid, 128) {
            emit_tool_span(
                "cns_emit",
                "error",
                start.elapsed().as_millis() as u64,
                Some(&hkask_types::McpErrorKind::InvalidArgument),
            );
            return e.to_json_string();
        }

        let span_enum = Self::parse_span(&span);
        let observation_value = serde_json::from_str(&observation)
            .unwrap_or(serde_json::Value::String(observation.clone()));

        let emitter = self.emitter.read().await;
        emitter.emit(span_enum, observation_value);

        self.runtime.increment_variety(&span, &phase).await;

        emit_tool_span("cns_emit", "ok", start.elapsed().as_millis() as u64, None);

        McpToolOutput::new(serde_json::json!({
            "span": span,
            "observer": observer_webid,
            "phase": phase,
            "emitted": true,
        }))
        .to_json_string()
    }

    #[tool(description = "Get variety count for a span pattern via real VarietyMonitor")]
    async fn cns_variety(
        &self,
        Parameters(VarietyRequest { span_pattern }): Parameters<VarietyRequest>,
    ) -> String {
        let start = Instant::now();

        // Validate identifiers
        if let Err(e) = validate_identifier("span_pattern", &span_pattern, 256) {
            emit_tool_span(
                "cns_variety",
                "error",
                start.elapsed().as_millis() as u64,
                Some(&hkask_types::McpErrorKind::InvalidArgument),
            );
            return e.to_json_string();
        }

        let variety_count = self.runtime.variety_for_domain(&span_pattern).await;
        let deficit = variety_count > self.threshold;

        emit_tool_span(
            "cns_variety",
            "ok",
            start.elapsed().as_millis() as u64,
            None,
        );

        McpToolOutput::new(serde_json::json!({
            "span_pattern": span_pattern,
            "variety_count": variety_count,
            "deficit": deficit,
        }))
        .to_json_string()
    }

    #[tool(description = "Trigger a real algedonic alert via AlgedonicManager")]
    async fn cns_alert(
        &self,
        Parameters(AlertRequest {
            span_pattern,
            severity,
        }): Parameters<AlertRequest>,
    ) -> String {
        let start = Instant::now();

        // Validate identifiers
        if let Err(e) = validate_identifier("span_pattern", &span_pattern, 256) {
            emit_tool_span(
                "cns_alert",
                "error",
                start.elapsed().as_millis() as u64,
                Some(&hkask_types::McpErrorKind::InvalidArgument),
            );
            return e.to_json_string();
        }
        if let Err(e) = validate_identifier("severity", &severity, 32) {
            emit_tool_span(
                "cns_alert",
                "error",
                start.elapsed().as_millis() as u64,
                Some(&hkask_types::McpErrorKind::InvalidArgument),
            );
            return e.to_json_string();
        }

        let alert = self.runtime.check_variety(&span_pattern).await;

        match alert {
            Some(a) => {
                emit_tool_span("cns_alert", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::new(serde_json::json!({
                    "alert_id": a.domain,
                    "span": span_pattern,
                    "severity": severity,
                    "deficit": a.deficit,
                    "triggered": true,
                }))
                .to_json_string()
            }
            None => {
                emit_tool_span("cns_alert", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::new(serde_json::json!({
                    "span": span_pattern,
                    "severity": severity,
                    "triggered": true,
                    "deficit": 0,
                }))
                .to_json_string()
            }
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
        let start = Instant::now();

        // Validate identifiers
        if let Err(e) = validate_identifier("span_pattern", &span_pattern, 256) {
            emit_tool_span(
                "cns_calibrate",
                "error",
                start.elapsed().as_millis() as u64,
                Some(&hkask_types::McpErrorKind::InvalidArgument),
            );
            return e.to_json_string();
        }

        let old_threshold = self.threshold;

        emit_tool_span(
            "cns_calibrate",
            "ok",
            start.elapsed().as_millis() as u64,
            None,
        );

        McpToolOutput::new(serde_json::json!({
            "span": span_pattern,
            "old_threshold": old_threshold,
            "new_threshold": new_threshold,
            "calibrated": true,
        }))
        .to_json_string()
    }

    #[tool(description = "List active algedonic alerts from real alert manager")]
    async fn cns_list_alerts(
        &self,
        Parameters(ListAlertsRequest { limit }): Parameters<ListAlertsRequest>,
    ) -> String {
        let start = Instant::now();

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

        emit_tool_span(
            "cns_list_alerts",
            "ok",
            start.elapsed().as_millis() as u64,
            None,
        );

        McpToolOutput::new(serde_json::json!({
            "alert_count": alerts.len(),
            "alerts": displayed,
        }))
        .to_json_string()
    }

    #[tool(description = "Get real CNS health status")]
    async fn cns_health(&self) -> String {
        let start = Instant::now();

        let health = self.runtime.health().await;

        emit_tool_span("cns_health", "ok", start.elapsed().as_millis() as u64, None);

        McpToolOutput::new(serde_json::json!({
            "healthy": health.healthy,
            "active_alerts": health.critical_count + health.warning_count,
            "critical_count": health.critical_count,
            "warning_count": health.warning_count,
            "overall_deficit": health.overall_deficit,
        }))
        .to_json_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-cns",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            let threshold: Option<u64> = ctx
                .credentials
                .get("HKASK_CNS_THRESHOLD")
                .and_then(|s| s.parse().ok());
            Ok(CnsServer::new(threshold))
        },
        vec![CredentialRequirement::optional(
            "HKASK_CNS_THRESHOLD",
            "CNS variety deficit threshold (default: 100)",
        )],
    )
    .await
}
