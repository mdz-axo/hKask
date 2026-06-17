//! CNS observability routes — including SSE event stream

use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use hkask_types::event::{NuEvent, SpanNamespace};
use hkask_types::ports::{BackpressureSignal, CnsObserver, DepletionSignal};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiState;

/// Create CNS router
///
/// REQ: API-006
/// pre:  none
/// post: returns OpenApiRouter<ApiState> with CNS routes registered
pub fn cns_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(cns_health))
        .route("/api/cns/alerts", axum::routing::get(cns_alerts))
        .routes(routes!(cns_variety))
        .routes(routes!(cns_subscribe))
}

/// Broadcast channel capacity for SSE events.
const SSE_CHANNEL_CAPACITY: usize = 256;

// ── SSE Event Envelope ──

/// Union type for all CNS events that can be streamed over SSE.
/// Wraps NuEvent, DepletionSignal, and BackpressureSignal so a single
/// broadcast channel can carry all observer callbacks.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
enum CnsSseEvent {
    #[serde(rename = "event")]
    NuEvent(NuEvent),
    #[serde(rename = "depletion")]
    Depletion(DepletionSignal),
    #[serde(rename = "backpressure")]
    Backpressure(BackpressureSignal),
}

// ── SSE Observer Bridge ──

/// Bridge between CnsRuntime's observer pattern and tokio broadcast channel.
///
/// Implements `CnsObserver` so the CNS can deliver events via its standard
/// callback interface. Each callback forwards the event into a broadcast channel
/// whose receiver is consumed by the SSE response stream.
struct SseObserver {
    sender: broadcast::Sender<CnsSseEvent>,
    interest_mask: Vec<SpanNamespace>,
}

impl SseObserver {
    fn new(interest_mask: Vec<SpanNamespace>) -> (Self, broadcast::Receiver<CnsSseEvent>) {
        let (sender, receiver) = broadcast::channel(SSE_CHANNEL_CAPACITY);
        let observer = Self {
            sender,
            interest_mask,
        };
        (observer, receiver)
    }
}
impl CnsObserver for SseObserver {
    fn interest_mask(&self) -> Vec<SpanNamespace> {
        self.interest_mask.clone()
    }

    async fn on_event(&self, event: &NuEvent) {
        let interested =
            self.interest_mask.is_empty() || self.interest_mask.contains(&event.span.namespace);
        if interested {
            let _ = self.sender.send(CnsSseEvent::NuEvent(event.clone()));
        }
    }

    async fn on_depletion(&self, signal: &DepletionSignal) {
        let _ = self.sender.send(CnsSseEvent::Depletion(signal.clone()));
    }

    async fn on_backpressure(&self, signal: &BackpressureSignal) {
        let _ = self.sender.send(CnsSseEvent::Backpressure(signal.clone()));
    }
}

// ── CNS Health ──

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
pub(crate) async fn cns_health(State(state): State<ApiState>) -> axum::Json<CnsHealthResponse> {
    let cns_runtime = state.agent_service.cns_runtime();
    let health = cns_runtime.read().await.health().await;

    axum::Json(CnsHealthResponse {
        overall_deficit: health.overall_deficit,
        critical_count: health.critical_count,
        warning_count: health.warning_count,
        healthy: health.healthy,
    })
}

/// CNS alerts endpoint
async fn cns_alerts(State(_state): State<ApiState>) -> axum::Json<Vec<String>> {
    axum::Json(vec![])
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
pub(crate) async fn cns_variety(State(state): State<ApiState>) -> axum::Json<CnsVarietyResponse> {
    let cns_runtime = state.agent_service.cns_runtime();
    let variety_data = cns_runtime.read().await.variety().await;

    let domains: Vec<String> = variety_data
        .keys()
        .map(|ns| ns.as_str().to_string())
        .collect();

    let counters: std::collections::HashMap<String, VarietyCounterResponse> = variety_data
        .iter()
        .map(|(ns, variety)| {
            (
                ns.as_str().to_string(),
                VarietyCounterResponse {
                    variety: *variety,
                    total: *variety,
                    entropy: 0.0, // Real entropy requires per-domain tracker access
                },
            )
        })
        .collect();

    let total_deficit: u64 = counters.values().map(|c| c.variety).sum();

    axum::Json(CnsVarietyResponse {
        domains,
        total_deficit,
        counters,
    })
}

// ── CNS Subscribe (SSE) ──

/// Query parameters for CNS SSE subscription
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub(crate) struct CnsSubscribeParams {
    /// Span namespaces to subscribe to (e.g., ["cns.tool", "cns.inference"])
    #[serde(default)]
    spans: Vec<String>,
}

/// Subscribe to CNS events as an SSE stream.
///
/// The endpoint upgrades the HTTP response to a long-lived SSE connection.
/// Events matching the requested span namespaces are forwarded in real time.
/// Lag notifications are emitted when the client falls behind.
#[utoipa::path(
    get,
    path = "/api/cns/subscribe",
    tag = "cns",
    params(CnsSubscribeParams),
    responses(
        (status = 200, description = "SSE event stream", content_type = "text/event-stream"),
        (status = 400, description = "Invalid request"),
    ),
)]
pub(crate) async fn cns_subscribe(
    State(state): State<ApiState>,
    Query(params): Query<CnsSubscribeParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Validate spans — filter to only canonical CNS namespaces
    let valid_spans: Vec<SpanNamespace> = params
        .spans
        .iter()
        .filter_map(|s| SpanNamespace::parse(s))
        .collect();

    let (observer, mut receiver) = SseObserver::new(valid_spans);
    let cns_runtime = state.agent_service.cns_runtime();
    cns_runtime
        .read()
        .await
        .subscribe_async(Arc::new(observer))
        .await;

    let stream = async_stream::stream! {
        loop {
            match receiver.recv().await {
                Ok(cns_event) => {
                    let data = serde_json::to_string(&cns_event).unwrap_or_default();
                    let event_type = match cns_event {
                        CnsSseEvent::NuEvent(_) => "cns-event",
                        CnsSseEvent::Depletion(_) => "cns-depletion",
                        CnsSseEvent::Backpressure(_) => "cns-backpressure",
                    };
                    yield Ok(Event::default().data(data).event(event_type));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    let data = format!(r#"{{"type":"lagged","count":{n}}}"#);
                    yield Ok(Event::default().data(data).event("cns-warning"));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
}

// ── Response Types ──

/// CNS health response — P9 (Homeostatic Self-Regulation).
///
/// `healthy: false` means one or more variety domains are in deficit (below
/// their configured threshold). Check `critical_count` and `warning_count` for
/// severity. `overall_deficit` is the sum of all per-domain deficits.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CnsHealthResponse {
    /// Sum of all per-domain variety deficits
    pub overall_deficit: u64,
    /// Count of domains in critical deficit (escalation-triggered)
    pub critical_count: usize,
    /// Count of domains in warning deficit
    pub warning_count: usize,
    /// Whether all variety domains are within healthy thresholds
    pub healthy: bool,
}

/// CNS variety counter for a single domain.
///
/// `variety` is the tracked behavioral diversity for this domain.
/// `entropy` is the Shannon entropy of the domain's observation distribution
/// (0.0 when not yet computed).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VarietyCounterResponse {
    /// Tracked behavioral diversity count
    pub variety: u64,
    /// Total observations in this domain
    pub total: u64,
    /// Shannon entropy (0.0 = not computed yet)
    pub entropy: f64,
}

/// CNS variety response — per-domain variety counters for the Cybernetic Nervous System (Pattern B).
///
/// `domains` lists all canonical CNS span namespaces registered (e.g., cns.tool,
/// cns.inference, cns.memory). `total_deficit` is the aggregate variety gap across
/// all domains.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CnsVarietyResponse {
    /// All registered CNS span namespace domains
    pub domains: Vec<String>,
    /// Aggregate variety deficit across all domains
    pub total_deficit: u64,
    /// Per-domain variety counters keyed by namespace
    pub counters: std::collections::HashMap<String, VarietyCounterResponse>,
}
