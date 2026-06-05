//! CNS observability routes

use axum::{Json, extract::State, routing::Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::ApiState;

/// Create CNS router
pub fn cns_router() -> Router<ApiState> {
    Router::new()
        .route("/api/cns/health", axum::routing::get(cns_health))
        .route("/api/cns/alerts", axum::routing::get(cns_alerts))
        .route("/api/cns/variety", axum::routing::get(cns_variety))
        .route("/api/cns/subscribe", axum::routing::post(cns_subscribe))
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

// ── CNS Subscribe ──

/// CNS health response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CnsHealthResponse {
    pub overall_deficit: u64,
    pub critical_count: usize,
    pub warning_count: usize,
    pub healthy: bool,
}

/// CNS variety counter response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VarietyCounterResponse {
    pub variety: u64,
    pub total: u64,
    pub entropy: f64,
}

/// CNS variety response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CnsVarietyResponse {
    pub domains: Vec<String>,
    pub total_deficit: u64,
    pub counters: HashMap<String, VarietyCounterResponse>,
}

/// Request body for CNS subscription
#[derive(Debug, Deserialize, ToSchema)]
struct SubscribeRequest {
    /// Agent WebID to observe events for
    agent: String,
    /// Span namespaces to subscribe to (e.g., ["cns.tool", "cns.inference"])
    spans: Vec<String>,
}

/// Response body for CNS subscription
#[derive(Debug, Serialize, ToSchema)]
struct SubscribeResponse {
    status: String,
    agent: String,
    spans: Vec<String>,
    message: String,
}

/// Subscribe to CNS events for an agent
///
/// Stub endpoint that validates the request and returns confirmation.
/// The actual subscription wiring will be connected when the runtime
/// integration is complete.
#[utoipa::path(
    post,
    path = "/api/cns/subscribe",
    tag = "cns",
    request_body = SubscribeRequest,
    responses(
        (status = 200, description = "Subscription confirmed", body = SubscribeResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn cns_subscribe(
    State(_state): State<ApiState>,
    Json(req): Json<SubscribeRequest>,
) -> Json<SubscribeResponse> {
    // Validate spans are valid CNS namespaces
    let valid_spans: Vec<String> = req
        .spans
        .iter()
        .filter(|s| hkask_types::event::SpanNamespace::parse(s).is_some())
        .cloned()
        .collect();

    let rejected_count = req.spans.len() - valid_spans.len();

    Json(SubscribeResponse {
        status: "confirmed".to_string(),
        agent: req.agent,
        spans: valid_spans,
        message: if rejected_count > 0 {
            format!(
                "Subscription confirmed. {} span namespace(s) were invalid and ignored.",
                rejected_count
            )
        } else {
            "Subscription confirmed. Events matching the specified namespaces will be delivered."
                .to_string()
        },
    })
}
