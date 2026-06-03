//! CNS observability routes

use axum::{Json, extract::State, routing::Router};
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
    let health = state.cns_runtime.health().await;

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
async fn cns_variety(State(state): State<ApiState>) -> Json<CnsVarietyResponse> {
    let variety_data = state.cns_runtime.variety().await;

    let domains: Vec<String> = variety_data.iter().map(|(d, _)| d.clone()).collect();

    let counters: HashMap<String, VarietyCounterResponse> = variety_data
        .iter()
        .map(|(domain, variety)| {
            (
                domain.clone(),
                VarietyCounterResponse {
                    variety: *variety,
                    total: *variety,
                    entropy: 0.0, // Real entropy requires per-domain tracker access
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
