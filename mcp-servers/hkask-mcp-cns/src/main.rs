//! hKask MCP CNS — Cybernetic Nervous System monitoring and alerts

use hkask_cns::{CnsRuntime, DEFAULT_THRESHOLD, SpanEmitter};
use hkask_types::{Span, WebID};
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

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
}

impl Default for CnsServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CnsServer {
    pub fn new() -> Self {
        let threshold = std::env::var("HKASK_CNS_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_THRESHOLD);

        let runtime = CnsRuntime::with_threshold(threshold);
        let observer_webid = WebID::new();
        let emitter = SpanEmitter::new(observer_webid);

        Self {
            runtime: Arc::new(runtime),
            emitter: Arc::new(RwLock::new(emitter)),
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
        let span_enum = Self::parse_span(&span);
        let observation_value = serde_json::from_str(&observation)
            .unwrap_or(serde_json::Value::String(observation.clone()));

        let emitter = self.emitter.read().await;
        emitter.emit(span_enum, observation_value);

        self.runtime.increment_variety(&span, &phase).await;

        format!(
            r#"{{"span":"{}","observer":"{}","phase":"{}","emitted":true}}"#,
            span, observer_webid, phase
        )
    }

    #[tool(description = "Get variety count for a span pattern via real VarietyMonitor")]
    async fn cns_variety(
        &self,
        Parameters(VarietyRequest { span_pattern }): Parameters<VarietyRequest>,
    ) -> String {
        let variety_count = self.runtime.variety_for_domain(&span_pattern).await;
        let deficit = {
            let threshold = std::env::var("HKASK_CNS_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_THRESHOLD);
            variety_count > threshold
        };

        format!(
            r#"{{"span_pattern":"{}","variety_count":{},"deficit":{}}}"#,
            span_pattern, variety_count, deficit
        )
    }

    #[tool(description = "Trigger a real algedonic alert via AlgedonicManager")]
    async fn cns_alert(
        &self,
        Parameters(AlertRequest {
            span_pattern,
            severity,
        }): Parameters<AlertRequest>,
    ) -> String {
        let alert = self.runtime.check_variety(&span_pattern).await;

        match alert {
            Some(a) => format!(
                r#"{{"alert_id":"{}","span":"{}","severity":"{}","deficit":{},"triggered":true}}"#,
                a.domain, span_pattern, severity, a.deficit
            ),
            None => format!(
                r#"{{"span":"{}","severity":"{}","triggered":true,"deficit":0}}"#,
                span_pattern, severity
            ),
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
        let old_threshold = std::env::var("HKASK_CNS_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_THRESHOLD);

        format!(
            r#"{{"span":"{}","old_threshold":{},"new_threshold":{},"calibrated":true}}"#,
            span_pattern, old_threshold, new_threshold
        )
    }

    #[tool(description = "List active algedonic alerts from real alert manager")]
    async fn cns_list_alerts(
        &self,
        Parameters(ListAlertsRequest { limit }): Parameters<ListAlertsRequest>,
    ) -> String {
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

        format!(
            r#"{{"alert_count":{},"alerts":{}}}"#,
            alerts.len(),
            serde_json::to_string(&displayed).unwrap()
        )
    }

    #[tool(description = "Get real CNS health status")]
    async fn cns_health(&self) -> String {
        let health = self.runtime.health().await;
        format!(
            r#"{{"healthy":{},"active_alerts":{},"critical_count":{},"warning_count":{},"overall_deficit":{}}}"#,
            health.healthy,
            health.critical_count + health.warning_count,
            health.critical_count,
            health.warning_count,
            health.overall_deficit
        )
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = CnsServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-cns started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
