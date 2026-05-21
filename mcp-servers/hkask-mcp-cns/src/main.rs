//! hKask MCP CNS — Cybernetic Nervous System monitoring and alerts

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router, transport::stdio, ServiceExt};
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

#[derive(Debug, Default)]
pub struct CnsServer {
    alerts: Arc<RwLock<Vec<String>>>,
}

impl CnsServer {
    pub fn new() -> Self {
        Self::default()
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
        format!(
            r#"{{"span":"{}","observer":"{}","phase":"{}","observation":"{}","emitted":true}}"#,
            span, observer_webid, phase, observation
        )
    }

    #[tool(description = "Get variety count for a span pattern")]
    async fn cns_variety(
        &self,
        Parameters(VarietyRequest { span_pattern }): Parameters<VarietyRequest>,
    ) -> String {
        let count = span_pattern.len() as u64;
        format!(
            r#"{{"span_pattern":"{}","variety_count":{},"deficit":false}}"#,
            span_pattern, count
        )
    }

    #[tool(description = "Trigger an algedonic alert")]
    async fn cns_alert(
        &self,
        Parameters(AlertRequest {
            span_pattern,
            severity,
        }): Parameters<AlertRequest>,
    ) -> String {
        let mut alerts = self.alerts.write().await;
        let alert_id = format!("alert_{}", alerts.len());
        alerts.push(alert_id.clone());
        format!(
            r#"{{"alert_id":"{}","span":"{}","severity":"{}","triggered":true}}"#,
            alert_id, span_pattern, severity
        )
    }

    #[tool(description = "Calibrate a span threshold")]
    async fn cns_calibrate(
        &self,
        Parameters(CalibrateRequest {
            span_pattern,
            new_threshold,
        }): Parameters<CalibrateRequest>,
    ) -> String {
        format!(
            r#"{{"span":"{}","old_threshold":100,"new_threshold":{},"calibrated":true}}"#,
            span_pattern, new_threshold
        )
    }

    #[tool(description = "List active alerts")]
    async fn cns_list_alerts(
        &self,
        Parameters(ListAlertsRequest { limit }): Parameters<ListAlertsRequest>,
    ) -> String {
        let alerts = self.alerts.read().await;
        let limit = limit.unwrap_or(10) as usize;
        let displayed: Vec<&String> = alerts.iter().take(limit).collect();
        format!(
            r#"{{"alert_count":{},"alerts":{}}}"#,
            alerts.len(),
            serde_json::to_string(&displayed).unwrap()
        )
    }

    #[tool(description = "Get CNS health status")]
    async fn cns_health(&self) -> String {
        let alerts = self.alerts.read().await;
        let active_alerts = alerts.len();
        let healthy = active_alerts < 5;
        format!(
            r#"{{"healthy":{},"active_alerts":{},"variety_deficit":false}}"#,
            healthy, active_alerts
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
