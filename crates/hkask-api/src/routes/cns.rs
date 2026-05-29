//! CNS observability routes

use axum::{Json, extract::State, routing::Router};
use hkask_cns::algedonic::{AlgedonicManager, CnsHealth};
use hkask_cns::variety::VarietyMonitor;
use std::collections::HashMap;

use crate::{ApiState, CnsHealthResponse, CnsVarietyResponse, VarietyCounterResponse};

/// Create CNS router
pub fn cns_router() -> Router<ApiState> {
    Router::new()
        .route("/api/cns/health", axum::routing::get(cns_health))
        .route("/api/cns/alerts", axum::routing::get(cns_alerts))
        .route("/api/cns/variety", axum::routing::get(cns_variety))
}

/// CNS health endpoint
#[utoipa::path(
    get,
    path = "/api/cns/health",
    tag = "cns",
    responses(
        (status = 200, description = "CNS health status", body = CnsHealthResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn cns_health(State(state): State<ApiState>) -> Json<CnsHealthResponse> {
    state.cns_emitter.emit_tool(
        "cns.health.check",
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    let health = CnsHealth::check(&AlgedonicManager::new(100, 10));

    Json(CnsHealthResponse {
        overall_deficit: health.overall_deficit,
        critical_count: health.critical_count,
        warning_count: health.warning_count,
        healthy: health.healthy,
    })
}

/// CNS alerts endpoint
async fn cns_alerts(State(_state): State<ApiState>) -> Json<Vec<String>> {
    Json(vec![])
}

/// CNS variety endpoint
#[utoipa::path(
    get,
    path = "/api/cns/variety",
    tag = "cns",
    responses(
        (status = 200, description = "CNS variety counters", body = CnsVarietyResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn cns_variety(State(_state): State<ApiState>) -> Json<CnsVarietyResponse> {
    let mut monitor = VarietyMonitor::new();

    let domains: Vec<String> = vec![
        "tool.invocation".to_string(),
        "template.render".to_string(),
        "agent.pod".to_string(),
    ];

    for domain in &domains {
        monitor.counter(domain).increment("state_active");
    }

    let counters: HashMap<String, VarietyCounterResponse> = domains
        .iter()
        .map(|d| {
            let counter = monitor.counter(d);
            (
                d.clone(),
                VarietyCounterResponse {
                    variety: counter.variety(),
                    total: counter.total(),
                    entropy: counter.entropy(),
                },
            )
        })
        .collect();

    let total_deficit: u64 = counters.values().map(|c| c.variety).sum();

    Json(CnsVarietyResponse {
        domains,
        total_deficit,
        counters,
    })
}
