//! hKask MCP CNS — Cybernetic Nervous System monitoring and algedonic alerts

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router, tool_handler,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_VARIETY_THRESHOLD: u64 = 100;

/// ν-event span emitted by CNS
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NuEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub observer_webid: String,
    pub phase: String,
    pub observation: serde_json::Value,
    pub regulation: Option<serde_json::Value>,
    pub outcome: Option<serde_json::Value>,
    pub recursion_depth: u32,
    pub parent_event: Option<String>,
    pub visibility: String,
}

/// Variety counter for Ashby's Law monitoring
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct VarietyCounter {
    pub span_pattern: String,
    pub required_variety: u64,
    pub available_variety: u64,
    pub deficit: i64,
    pub last_updated: DateTime<Utc>,
}

/// Algedonic alert triggered by variety deficit
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AlgedonicAlert {
    pub id: String,
    pub severity: String,
    pub span_pattern: String,
    pub deficit: i64,
    pub threshold: u64,
    pub triggered_at: DateTime<Utc>,
    pub escalated_to: Option<String>,
}

/// Emit span request
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EmitSpanRequest {
    pub span: String,
    pub observer_webid: String,
    pub phase: String,
    pub observation: serde_json::Value,
    pub outcome: Option<serde_json::Value>,
    pub parent_event: Option<String>,
}

/// CNS server implementation
pub struct CnsServer {
    tool_router: ToolRouter<CnsServer>,
    events: std::sync::Arc<tokio::sync::RwLock<Vec<NuEvent>>>,
    counters: std::sync::Arc<tokio::sync::RwLock<HashMap<String, VarietyCounter>>>,
    alerts: std::sync::Arc<tokio::sync::RwLock<Vec<AlgedonicAlert>>>,
    threshold: std::sync::Arc<tokio::sync::RwLock<u64>>,
}

impl CnsServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            events: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            counters: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            alerts: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            threshold: std::sync::Arc::new(tokio::sync::RwLock::new(DEFAULT_VARIETY_THRESHOLD)),
        }
    }
}

#[tool_router]
impl CnsServer {
    #[tool(description = "Emit a ν-event span to the CNS")]
    async fn cns_emit(&self, Parameters(req): Parameters<EmitSpanRequest>) -> String {
        let event = NuEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            observer_webid: req.observer_webid.clone(),
            phase: req.phase.clone(),
            observation: req.observation.clone(),
            regulation: None,
            outcome: req.outcome.clone(),
            recursion_depth: 0,
            parent_event: req.parent_event.clone(),
            visibility: "private".to_string(),
        };

        let mut events = self.events.write().await;
        events.push(event.clone());

        // Update variety counter for span pattern
        let mut counters = self.counters.write().await;
        let counter = counters.entry(req.span.clone()).or_insert_with(|| VarietyCounter {
            span_pattern: req.span.clone(),
            required_variety: 1000,
            available_variety: 1,
            deficit: 999,
            last_updated: Utc::now(),
        });
        counter.available_variety += 1;
        counter.deficit = counter.required_variety as i64 - counter.available_variety as i64;
        counter.last_updated = Utc::now();

        tracing::info!(span = %req.span, event_id = %event.id, "emitted ν-event");
        serde_json::to_string_pretty(&event).unwrap_or_else(|_| "error serializing".to_string())
    }

    #[tool(description = "Query variety counters for a span pattern")]
    async fn cns_variety(&self, span_pattern: String) -> String {
        let counters = self.counters.read().await;
        match counters.get(&span_pattern) {
            Some(counter) => serde_json::to_string_pretty(&counter).unwrap_or_else(|_| "error".to_string()),
            None => serde_json::json!({
                "span_pattern": span_pattern,
                "found": false,
                "required_variety": *self.threshold.read().await,
                "available_variety": 0,
                "deficit": *self.threshold.read().await as i64
            }).to_string()
        }
    }

    #[tool(description = "Trigger an algedonic alert for variety deficit escalation")]
    async fn cns_alert(&self, span_pattern: String, severity: Option<String>) -> String {
        let counters = self.counters.read().await;
        let threshold = *self.threshold.read().await;
        
        let deficit = counters.get(&span_pattern).map(|c| c.deficit).unwrap_or(threshold as i64 + 1);
        
        if deficit > 0 {
            let alert = AlgedonicAlert {
                id: Uuid::new_v4().to_string(),
                severity: severity.unwrap_or_else(|| "warning".to_string()),
                span_pattern: span_pattern.clone(),
                deficit,
                threshold,
                triggered_at: Utc::now(),
                escalated_to: Some("curator".to_string()),
            };

            let mut alerts = self.alerts.write().await;
            alerts.push(alert.clone());

            tracing::warn!(span = %span_pattern, deficit = deficit, "algedonic alert triggered");
            serde_json::to_string_pretty(&alert).unwrap_or_else(|_| "error".to_string())
        } else {
            serde_json::json!({
                "alert": false,
                "reason": "no variety deficit",
                "span_pattern": span_pattern
            }).to_string()
        }
    }

    #[tool(description = "Recalibrate drift detection thresholds")]
    async fn cns_calibrate(&self, span_pattern: String, new_threshold: u64) -> String {
        let mut counters = self.counters.write().await;
        if let Some(counter) = counters.get_mut(&span_pattern) {
            counter.required_variety = new_threshold;
            counter.deficit = counter.required_variety as i64 - counter.available_variety as i64;
            
            tracing::info!(span = %span_pattern, threshold = new_threshold, "recalibrated CNS");
            serde_json::json!({
                "success": true,
                "span_pattern": span_pattern,
                "new_threshold": new_threshold,
                "deficit": counter.deficit
            }).to_string()
        } else {
            serde_json::json!({
                "success": false,
                "reason": "span pattern not found"
            }).to_string()
        }
    }

    #[tool(description = "List recent algedonic alerts")]
    async fn cns_list_alerts(&self, limit: Option<usize>) -> String {
        let alerts = self.alerts.read().await;
        let limit = limit.unwrap_or(10);
        let recent: Vec<&AlgedonicAlert> = alerts.iter().rev().take(limit).collect();
        
        serde_json::json!({
            "alerts": recent,
            "count": recent.len()
        }).to_string()
    }

    #[tool(description = "Get CNS system health summary")]
    async fn cns_health(&self) -> String {
        let events = self.events.read().await;
        let counters = self.counters.read().await;
        let alerts = self.alerts.read().await;
        let threshold = *self.threshold.read().await;

        let critical_deficits: Vec<&VarietyCounter> = counters
            .values()
            .filter(|c| c.deficit > threshold as i64)
            .collect();

        serde_json::json!({
            "status": if critical_deficits.is_empty() { "healthy" } else { "degraded" },
            "total_events": events.len(),
            "span_patterns": counters.len(),
            "active_alerts": alerts.len(),
            "critical_deficits": critical_deficits.iter().map(|c| &c.span_pattern).collect::<Vec<_>>(),
            "threshold": threshold
        }).to_string()
    }
}

#[tool_handler]
impl ServerHandler for CnsServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = CnsServer::new();
    let service = server.serve_stdio();
    tracing::info!("hkask-mcp-cns MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
