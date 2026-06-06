//! CNS observability routes — including SSE event stream

use async_trait::async_trait;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::Router;
use futures_util::stream::Stream;
use hkask_types::event::{NuEvent, SpanNamespace};
use hkask_types::ports::{BackpressureSignal, CnsObserver, DepletionSignal};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use utoipa::{IntoParams, ToSchema};

use crate::ApiState;

/// Create CNS router
pub fn cns_router() -> Router<ApiState> {
    Router::new()
        .route("/api/cns/health", axum::routing::get(cns_health))
        .route("/api/cns/alerts", axum::routing::get(cns_alerts))
        .route("/api/cns/variety", axum::routing::get(cns_variety))
        .route("/api/cns/subscribe", axum::routing::get(cns_subscribe))
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

#[async_trait]
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
async fn cns_health(State(state): State<ApiState>) -> axum::Json<CnsHealthResponse> {
    let health = state.cns_runtime.health().await;

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
async fn cns_variety(State(state): State<ApiState>) -> axum::Json<CnsVarietyResponse> {
    let variety_data = state.cns_runtime.variety().await;

    let domains: Vec<String> = variety_data.iter().map(|(d, _)| d.clone()).collect();

    let counters: std::collections::HashMap<String, VarietyCounterResponse> = variety_data
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

    axum::Json(CnsVarietyResponse {
        domains,
        total_deficit,
        counters,
    })
}

// ── CNS Subscribe (SSE) ──

/// Query parameters for CNS SSE subscription
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
struct CnsSubscribeParams {
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
async fn cns_subscribe(
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
    state.cns_runtime.subscribe_async(Arc::new(observer)).await;

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
    pub counters: std::collections::HashMap<String, VarietyCounterResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
    use hkask_types::id::WebID;
    use hkask_types::loops::LoopId;
    use hkask_types::ports::{BackpressureSignal, CnsObserver, DepletionSignal};
    use serde_json;

    /// Helper to create a minimal NuEvent with the given span.
    fn make_event(span: Span) -> NuEvent {
        NuEvent::new(
            WebID::new(),
            span,
            Phase::Sense,
            serde_json::json!({"test": true}),
            0,
        )
    }

    #[tokio::test]
    async fn sse_observer_forwards_nu_events() {
        let mask = vec![SpanNamespace::new("cns.tool")];
        let (observer, mut receiver) = SseObserver::new(mask);
        let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        let event = make_event(span);
        observer.on_event(&event).await;
        let received = receiver.try_recv().expect("should receive NuEvent");
        assert!(matches!(received, CnsSseEvent::NuEvent(_)));
    }

    #[tokio::test]
    async fn sse_observer_ignores_uninterested_events() {
        let mask = vec![SpanNamespace::new("cns.variety")];
        let (observer, mut receiver) = SseObserver::new(mask);
        let span = Span::new(SpanNamespace::new("cns.inference"), "invoked");
        let event = make_event(span);
        observer.on_event(&event).await;
        let result = receiver.try_recv();
        assert!(
            result.is_err(),
            "should not receive event for uninterested span namespace"
        );
    }

    #[tokio::test]
    async fn sse_observer_forwards_depletion_signals() {
        let (observer, mut receiver) = SseObserver::new(vec![]);
        let signal = DepletionSignal {
            agent: WebID::new(),
            remaining: 100,
            cap: 1000,
            usage_ratio: 0.9,
        };
        observer.on_depletion(&signal).await;
        let received = receiver
            .try_recv()
            .expect("should receive Depletion signal");
        assert!(matches!(received, CnsSseEvent::Depletion(_)));
    }

    #[tokio::test]
    async fn sse_observer_forwards_backpressure_signals() {
        let (observer, mut receiver) = SseObserver::new(vec![]);
        let signal = BackpressureSignal {
            source: LoopId::Cybernetics,
            reason: "overload".to_string(),
            severity: 0.8,
        };
        observer.on_backpressure(&signal).await;
        let received = receiver
            .try_recv()
            .expect("should receive Backpressure signal");
        assert!(matches!(received, CnsSseEvent::Backpressure(_)));
    }

    #[tokio::test]
    async fn sse_observer_lag_produces_warning() {
        // Construct an SseObserver with a tiny broadcast channel to force lag
        let (sender, mut receiver) = broadcast::channel::<CnsSseEvent>(2);
        let observer = SseObserver {
            sender,
            interest_mask: vec![],
        };

        // Send more events than the channel capacity without draining
        for _ in 0..5 {
            let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
            let event = make_event(span);
            observer.on_event(&event).await;
        }

        // The sender must not block — broadcast::send() always succeeds even when
        // the receiver falls behind.
        //
        // The receiver should encounter a Lagged error due to overflow.
        let mut saw_lagged = false;
        loop {
            match receiver.try_recv() {
                Ok(_) => continue,
                Err(broadcast::error::TryRecvError::Lagged(_)) => {
                    saw_lagged = true;
                    break;
                }
                Err(broadcast::error::TryRecvError::Empty)
                | Err(broadcast::error::TryRecvError::Closed) => break,
            }
        }
        assert!(
            saw_lagged,
            "receiver should encounter a Lagged error due to channel overflow"
        );
    }

    #[test]
    fn cns_subscribe_validates_spans() {
        // Valid full-form spans
        assert!(SpanNamespace::parse("cns.tool").is_some());
        assert!(SpanNamespace::parse("cns.inference").is_some());
        assert!(SpanNamespace::parse("cns.variety").is_some());

        // Valid short-form spans (auto-prefixed with "cns.")
        assert!(SpanNamespace::parse("tool").is_some());
        assert!(SpanNamespace::parse("inference").is_some());

        // Invalid spans
        assert!(SpanNamespace::parse("cns.nonexistent").is_none());
        assert!(SpanNamespace::parse("invalid").is_none());
        assert!(SpanNamespace::parse("").is_none());

        // Simulate the filtering logic from cns_subscribe handler
        let spans = vec![
            "cns.tool".to_string(),
            "cns.inference".to_string(),
            "cns.nonexistent".to_string(),
            "invalid".to_string(),
            "variety".to_string(),
        ];
        let valid: Vec<SpanNamespace> = spans
            .iter()
            .filter_map(|s| SpanNamespace::parse(s))
            .collect();

        assert_eq!(valid.len(), 3);
        assert_eq!(valid[0].as_str(), "cns.tool");
        assert_eq!(valid[1].as_str(), "cns.inference");
        assert_eq!(valid[2].as_str(), "cns.variety");
    }

    //
    // These tests validate that CnsSseEvent variants serialize to the correct
    // JSON structure and that the event type mapping produces the right SSE
    // event names. This exercises the same serialization path used by the
    // `cns_subscribe` handler when producing SSE responses.

    #[test]
    fn sse_event_nu_event_serializes_with_type_tag() {
        let event = NuEvent::new(
            WebID::from_persona(b"test-agent"),
            Span::new(SpanNamespace::new("cns.tool"), "invoked"),
            Phase::Act,
            serde_json::json!({"tool": "hkask-mcp-ocap", "action": "sign"}),
            0,
        );
        let cns_event = CnsSseEvent::NuEvent(event);
        let json = serde_json::to_string(&cns_event).expect("serialize NuEvent");
        // CnsSseEvent uses serde(tag = "type", content = "payload")
        assert!(
            json.contains("\"type\":\"event\""),
            "should have type=event tag"
        );
        assert!(
            json.contains("cns.tool.invoked"),
            "should contain span path"
        );
    }

    #[test]
    fn sse_event_depletion_serializes_with_type_tag() {
        let signal = DepletionSignal {
            agent: WebID::from_persona(b"depleted-agent"),
            remaining: 50,
            cap: 1000,
            usage_ratio: 0.95,
        };
        let cns_event = CnsSseEvent::Depletion(signal);
        let json = serde_json::to_string(&cns_event).expect("serialize Depletion");
        assert!(
            json.contains("\"type\":\"depletion\""),
            "should have type=depletion tag"
        );
        // WebID::from_persona produces a UUID, not the literal persona string,
        // so we verify the JSON structure rather than the specific agent ID.
        assert!(json.contains("\"agent\":"), "should contain agent field");
    }

    #[test]
    fn sse_event_backpressure_serializes_with_type_tag() {
        let signal = BackpressureSignal {
            source: LoopId::Cybernetics,
            reason: "gas budget exceeded".to_string(),
            severity: 0.9,
        };
        let cns_event = CnsSseEvent::Backpressure(signal);
        let json = serde_json::to_string(&cns_event).expect("serialize Backpressure");
        assert!(
            json.contains("\"type\":\"backpressure\""),
            "should have type=backpressure tag"
        );
        assert!(
            json.contains("gas budget exceeded"),
            "should contain reason"
        );
    }

    #[test]
    fn sse_event_type_mapping_covers_all_variants() {
        // Verify the event type mapping used in the SSE stream yields correct
        // strings for each CnsSseEvent variant, matching the handler logic:
        // NuEvent → "cns-event", Depletion → "cns-depletion", Backpressure → "cns-backpressure"
        let nu_event = CnsSseEvent::NuEvent(NuEvent::new(
            WebID::new(),
            Span::new(SpanNamespace::new("cns.tool"), "test"),
            Phase::Sense,
            serde_json::json!({}),
            0,
        ));
        let depletion = CnsSseEvent::Depletion(DepletionSignal {
            agent: WebID::new(),
            remaining: 0,
            cap: 100,
            usage_ratio: 1.0,
        });
        let backpressure = CnsSseEvent::Backpressure(BackpressureSignal {
            source: LoopId::Cybernetics,
            reason: "test".to_string(),
            severity: 1.0,
        });

        assert_eq!(sse_event_type(&nu_event), "cns-event");
        assert_eq!(sse_event_type(&depletion), "cns-depletion");
        assert_eq!(sse_event_type(&backpressure), "cns-backpressure");
    }

    /// Helper: extract the SSE event type string from a CnsSseEvent variant.
    /// Mirrors the match logic in `cns_subscribe`.
    fn sse_event_type(event: &CnsSseEvent) -> &'static str {
        match event {
            CnsSseEvent::NuEvent(_) => "cns-event",
            CnsSseEvent::Depletion(_) => "cns-depletion",
            CnsSseEvent::Backpressure(_) => "cns-backpressure",
        }
    }
}
