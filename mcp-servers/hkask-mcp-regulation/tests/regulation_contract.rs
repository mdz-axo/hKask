//! Contract tests for hkask-mcp-cns — CNS span history query invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `cns_query_spans` and `reg_span_stats` MCP tool methods
//! invoked through the public `Parameters<T>` seam — the same surface an
//! agent uses.

use hkask_database::sqlite::SqliteDriver;
use hkask_mcp_regulation::CnsServer;
use hkask_storage::RegulationArchive;
use hkask_types::WebID;
use hkask_types::event::{CyclePhase, RegulationRecord, RegulationSink, Span, SpanNamespace};
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

/// Build a CnsServer backed by an in-memory RegulationArchive (no on-disk DB).
fn test_server() -> CnsServer {
    let pool = SqliteDriver::in_memory_pool().expect("in-memory SQLite pool");
    let driver: Arc<dyn hkask_database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(pool));
    let store = RegulationArchive::from_driver(driver);
    CnsServer::new(
        WebID::new(),
        "test-userpod".into(),
        None,
        Some(Arc::new(store)),
    )
}

/// Build a CnsServer with NO store attached — simulates the
/// `HKASK_DB_PASSPHRASE`-missing degradation path.
fn test_server_no_store() -> CnsServer {
    CnsServer::new(WebID::new(), "test-userpod".into(), None, None)
}

/// Insert a single ν-event into the in-memory store for the given namespace.
fn insert_event(store: &RegulationArchive, namespace: &str, local_path: &str) {
    let ns = SpanNamespace::new(namespace)
        .unwrap_or_else(|| SpanNamespace::new("reg.gas").expect("reg.gas must be canonical"));
    let span = Span::new(ns, local_path);
    let event = RegulationRecord::new(
        WebID::from_persona(b"observer"),
        span,
        CyclePhase::Act,
        serde_json::json!({"test": true}),
        0,
    );
    store.persist(&event).expect("persist test event");
}

/// Parse the success envelope `{"content": <value>}`; falls back to the raw
/// value for non-envelope outputs.
fn parse_content(out: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("content").cloned().unwrap_or(v)
}

/// Extract the `kind` field from an error envelope, if present.
fn error_kind(out: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("kind").and_then(|e| e.as_str()).map(String::from)
}

// REQ: cns_query_spans returns an empty events array when no events match (P5).
// expect: a fresh server with no events returns count=0 and an empty events array.
#[tokio::test]
async fn cns_query_spans_returns_empty_array_when_no_events() {
    let server = test_server();
    let req: hkask_mcp_regulation::QuerySpansRequest = serde_json::from_value(serde_json::json!({
        "namespace": "reg.guard",
        "since_hours": 1.0,
        "limit": 100
    }))
    .expect("deserialize QuerySpansRequest");
    let out = server.cns_query_spans(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(
        content["count"], 0,
        "expected count=0 for empty store: {out}"
    );
    assert!(
        content["events"].is_array(),
        "events should be an array: {out}"
    );
    assert_eq!(
        content["events"].as_array().unwrap().len(),
        0,
        "events array should be empty: {out}"
    );
}

// REQ: cns_query_spans returns matching events for a populated namespace (P5).
// expect: after inserting a cns.guard.input event, cns_query_spans with
// namespace="reg.guard" returns count=1 and the event in the array.
#[tokio::test]
async fn cns_query_spans_returns_matching_events() {
    let pool = SqliteDriver::in_memory_pool().expect("in-memory SQLite pool");
    let driver: Arc<dyn hkask_database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(pool));
    let store = RegulationArchive::from_driver(driver);
    insert_event(&store, "reg.guard.input", "guard.input.violation");

    let server = CnsServer::new(
        WebID::new(),
        "test-userpod".into(),
        None,
        Some(Arc::new(store)),
    );

    let req: hkask_mcp_regulation::QuerySpansRequest = serde_json::from_value(serde_json::json!({
        "namespace": "reg.guard",
        "since_hours": 1.0,
        "limit": 100
    }))
    .expect("deserialize QuerySpansRequest");
    let out = server.cns_query_spans(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(content["count"], 1, "expected count=1: {out}");
    let events = content["events"].as_array().expect("events is array");
    assert_eq!(events.len(), 1, "expected 1 event: {out}");
    assert_eq!(
        events[0]["namespace"], "reg.guard.input",
        "namespace should match: {out}"
    );
}

// REQ: cns_query_spans rejects an empty namespace with invalid_argument (P5).
// expect: an empty namespace string returns kind=invalid_argument.
#[tokio::test]
async fn cns_query_spans_rejects_empty_namespace() {
    let server = test_server();
    let req: hkask_mcp_regulation::QuerySpansRequest = serde_json::from_value(serde_json::json!({
        "namespace": "",
        "since_hours": 1.0,
        "limit": 100
    }))
    .expect("deserialize QuerySpansRequest");
    let out = server.cns_query_spans(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for empty namespace");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: cns_query_spans rejects a whitespace-only namespace with invalid_argument (P5).
// expect: a whitespace-only namespace string returns kind=invalid_argument.
#[tokio::test]
async fn cns_query_spans_rejects_whitespace_namespace() {
    let server = test_server();
    let req: hkask_mcp_regulation::QuerySpansRequest = serde_json::from_value(serde_json::json!({
        "namespace": "   ",
        "since_hours": 1.0,
        "limit": 100
    }))
    .expect("deserialize QuerySpansRequest");
    let out = server.cns_query_spans(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for whitespace namespace");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: cns_query_spans returns permission_denied when no store is attached (P5).
// expect: when the RegulationArchive is None (no DB passphrase), the tool returns
// kind=permission_denied with a clear message.
#[tokio::test]
async fn cns_query_spans_returns_permission_denied_without_store() {
    let server = test_server_no_store();
    let req: hkask_mcp_regulation::QuerySpansRequest = serde_json::from_value(serde_json::json!({
        "namespace": "reg.guard",
        "since_hours": 1.0,
        "limit": 100
    }))
    .expect("deserialize QuerySpansRequest");
    let out = server.cns_query_spans(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for missing store");
    assert_eq!(kind, "permission_denied", "got: {out}");
}

// REQ: cns_query_spans applies default values when optional fields are omitted (P5).
// expect: omitting since_hours and limit still returns a valid response with
// the documented defaults (1.0 hour, 100 events).
#[tokio::test]
async fn cns_query_spans_applies_defaults() {
    let server = test_server();
    let req: hkask_mcp_regulation::QuerySpansRequest = serde_json::from_value(serde_json::json!({
        "namespace": "reg.guard"
    }))
    .expect("deserialize QuerySpansRequest with defaults");
    let out = server.cns_query_spans(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(content["count"], 0, "expected count=0: {out}");
    assert_eq!(content["limit"], 100, "default limit should be 100: {out}");
    assert!(
        content["since"].is_string(),
        "since should be an RFC3339 string: {out}"
    );
}

// REQ: reg_span_stats returns an empty categories object when no events match (P5).
// expect: a fresh server with no events returns total_events=0 and an empty
// categories object.
#[tokio::test]
async fn reg_span_stats_returns_empty_object_when_no_events() {
    let server = test_server();
    let req: hkask_mcp_regulation::SpanStatsRequest = serde_json::from_value(serde_json::json!({
        "namespace": "reg.outcome",
        "since_hours": 1.0
    }))
    .expect("deserialize SpanStatsRequest");
    let out = server.reg_span_stats(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(content["total_events"], 0, "expected total_events=0: {out}");
    assert!(
        content["categories"].is_object(),
        "categories should be an object: {out}"
    );
    assert_eq!(
        content["categories"].as_object().unwrap().len(),
        0,
        "categories should be empty: {out}"
    );
}

// REQ: reg_span_stats returns aggregated counts by span_category (P5).
// expect: after inserting two cns.regulation events with different local paths
// (both stored under span_category="regulation"), reg_span_stats with
// namespace="reg.outcome" returns total_events=2 and a categories object
// mapping "regulation" to 2.
#[tokio::test]
async fn reg_span_stats_returns_aggregated_counts() {
    let pool = SqliteDriver::in_memory_pool().expect("in-memory SQLite pool");
    let driver: Arc<dyn hkask_database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(pool));
    let store = RegulationArchive::from_driver(driver);
    insert_event(&store, "reg.outcome", "regulation.action_blocked");
    insert_event(&store, "reg.outcome", "regulation.plateau_detected");

    let server = CnsServer::new(
        WebID::new(),
        "test-userpod".into(),
        None,
        Some(Arc::new(store)),
    );

    let req: hkask_mcp_regulation::SpanStatsRequest = serde_json::from_value(serde_json::json!({
        "namespace": "reg.outcome",
        "since_hours": 1.0
    }))
    .expect("deserialize SpanStatsRequest");
    let out = server.reg_span_stats(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(content["total_events"], 2, "expected total_events=2: {out}");
    let categories = content["categories"]
        .as_object()
        .expect("categories is object");
    assert_eq!(
        categories.len(),
        1,
        "expected 1 distinct span_category: {out}"
    );
    assert_eq!(
        categories["regulation"], 2,
        "regulation category should have count=2: {out}"
    );
}

// REQ: reg_span_stats rejects an empty namespace with invalid_argument (P5).
// expect: an empty namespace string returns kind=invalid_argument.
#[tokio::test]
async fn reg_span_stats_rejects_empty_namespace() {
    let server = test_server();
    let req: hkask_mcp_regulation::SpanStatsRequest = serde_json::from_value(serde_json::json!({
        "namespace": "",
        "since_hours": 1.0
    }))
    .expect("deserialize SpanStatsRequest");
    let out = server.reg_span_stats(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for empty namespace");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: reg_span_stats returns permission_denied when no store is attached (P5).
// expect: when the RegulationArchive is None, the tool returns kind=permission_denied.
#[tokio::test]
async fn reg_span_stats_returns_permission_denied_without_store() {
    let server = test_server_no_store();
    let req: hkask_mcp_regulation::SpanStatsRequest = serde_json::from_value(serde_json::json!({
        "namespace": "reg.guard",
        "since_hours": 1.0
    }))
    .expect("deserialize SpanStatsRequest");
    let out = server.reg_span_stats(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for missing store");
    assert_eq!(kind, "permission_denied", "got: {out}");
}

// REQ: cns_query_spans strips the cns. prefix before querying (P5).
// expect: querying "reg.guard" finds events stored under span_category="guard.input".
// This verifies the short-name normalization — the column stores short names,
// not full cns.* namespaces.
#[tokio::test]
async fn cns_query_spans_strips_cns_prefix() {
    let pool = SqliteDriver::in_memory_pool().expect("in-memory SQLite pool");
    let driver: Arc<dyn hkask_database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(pool));
    let store = RegulationArchive::from_driver(driver);
    insert_event(&store, "reg.guard.input", "guard.input.violation");

    let server = CnsServer::new(
        WebID::new(),
        "test-userpod".into(),
        None,
        Some(Arc::new(store)),
    );

    // Query with the full "reg.guard" namespace — should find the event
    // stored under span_category="guard.input".
    let req: hkask_mcp_regulation::QuerySpansRequest = serde_json::from_value(serde_json::json!({
        "namespace": "reg.guard",
        "since_hours": 1.0,
        "limit": 100
    }))
    .expect("deserialize QuerySpansRequest");
    let out = server.cns_query_spans(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(content["count"], 1, "expected count=1: {out}");
}

// REQ: cns_query_spans handles non-cns namespaces (e.g. hkask performative) (P5).
// expect: querying "hkask" (no cns. prefix) does not panic and returns an
// empty result (no hkask.* events in the test store).
#[tokio::test]
async fn cns_query_spans_handles_non_cns_namespace() {
    let server = test_server();
    let req: hkask_mcp_regulation::QuerySpansRequest = serde_json::from_value(serde_json::json!({
        "namespace": "hkask",
        "since_hours": 1.0,
        "limit": 100
    }))
    .expect("deserialize QuerySpansRequest");
    let out = server.cns_query_spans(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(content["count"], 0, "expected count=0: {out}");
}
