//! Contract tests for hkask-mcp-curator — request types and structural construction.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: Request type deserialization (no DB dependency).

// ── Request type deserialization tests ─────────────────────────────────────

#[test]
fn ping_request_parses_empty_object() {
    let json = serde_json::json!({});
    let req: hkask_mcp_curator::types::PingRequest =
        serde_json::from_value(json).expect("should parse ping request");
    // PingRequest is a unit struct — just verifies it compiles and parses
    let _ = req;
}

#[test]
fn escalation_resolve_request_parses_valid_json() {
    let json = serde_json::json!({
        "id": "esc-123",
        "resolution": "Resolved by restarting service"
    });
    let req: hkask_mcp_curator::types::EscalationResolveRequest =
        serde_json::from_value(json).expect("should parse resolve request");
    assert_eq!(req.id, "esc-123");
    assert!(req.resolution.contains("restarting"));
}

#[test]
fn escalation_dismiss_request_parses_valid_json() {
    let json = serde_json::json!({
        "id": "esc-456",
        "reason": "False positive — system recovered"
    });
    let req: hkask_mcp_curator::types::EscalationDismissRequest =
        serde_json::from_value(json).expect("should parse dismiss request");
    assert_eq!(req.id, "esc-456");
    assert!(req.reason.contains("False positive"));
}

#[test]
fn semantic_search_request_parses_with_limit() {
    let json = serde_json::json!({
        "query": "test query",
        "limit": 10
    });
    let req: hkask_mcp_curator::types::SemanticSearchRequest =
        serde_json::from_value(json).expect("should parse semantic search");
    assert_eq!(req.query, "test query");
    assert_eq!(req.limit, Some(10));
}

#[test]
fn semantic_search_request_parses_without_limit() {
    let json = serde_json::json!({
        "query": "test query"
    });
    let req: hkask_mcp_curator::types::SemanticSearchRequest =
        serde_json::from_value(json).expect("should parse without limit");
    assert_eq!(req.limit, None);
}

#[test]
fn cns_query_request_parses_with_all_fields() {
    let json = serde_json::json!({
        "namespace": "cns.sovereignty",
        "window_seconds": 3600,
        "limit": 50
    });
    let req: hkask_mcp_curator::types::CnsQueryRequest =
        serde_json::from_value(json).expect("should parse CNS query");
    assert_eq!(req.namespace, Some("cns.sovereignty".to_string()));
    assert_eq!(req.window_seconds, Some(3600));
    assert_eq!(req.limit, Some(50));
}

#[test]
fn cns_query_request_parses_with_minimal_fields() {
    let json = serde_json::json!({});
    let req: hkask_mcp_curator::types::CnsQueryRequest =
        serde_json::from_value(json).expect("should parse empty CNS query");
    assert_eq!(req.namespace, None);
    assert_eq!(req.window_seconds, None);
    assert_eq!(req.limit, None);
}

#[test]
fn memory_recall_request_parses_valid_json() {
    let json = serde_json::json!({
        "entity": "test-entity",
        "memory_type": "episodic"
    });
    let req: hkask_mcp_curator::types::MemoryRecallRequest =
        serde_json::from_value(json).expect("should parse memory recall");
    assert_eq!(req.entity, "test-entity");
    assert_eq!(req.memory_type, Some("episodic".to_string()));
}

#[test]
fn token_list_request_parses_with_filters() {
    let json = serde_json::json!({
        "window_seconds": 86400,
        "issuer": "webid:alice",
        "recipient": "webid:bob"
    });
    let req: hkask_mcp_curator::types::TokenListRequest =
        serde_json::from_value(json).expect("should parse token list request");
    assert_eq!(req.window_seconds, Some(86400));
    assert_eq!(req.issuer, Some("webid:alice".to_string()));
    assert_eq!(req.recipient, Some("webid:bob".to_string()));
}

// ── Server struct existence test ───────────────────────────────────────────

#[test]
fn curator_server_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_curator::CuratorServer>();
    assert!(_type_name.contains("hkask_mcp_curator"));
}

// ── Schema generation tests ────────────────────────────────────────────────

#[test]
fn request_types_have_schemas() {
    // Verify all request types implement JsonSchema (compile-time check)
    let schemas = vec![
        schemars::schema_for!(hkask_mcp_curator::types::PingRequest),
        schemars::schema_for!(hkask_mcp_curator::types::EscalationResolveRequest),
        schemars::schema_for!(hkask_mcp_curator::types::EscalationDismissRequest),
        schemars::schema_for!(hkask_mcp_curator::types::SemanticSearchRequest),
        schemars::schema_for!(hkask_mcp_curator::types::CnsQueryRequest),
        schemars::schema_for!(hkask_mcp_curator::types::MemoryRecallRequest),
        schemars::schema_for!(hkask_mcp_curator::types::TokenListRequest),
    ];
    for schema in &schemas {
        let schema_json = serde_json::to_value(schema).expect("schema should serialize");
        assert!(schema_json.is_object());
    }
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

use hkask_mcp_curator::CuratorServer;
use hkask_mcp_curator::types::PingRequest;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;

/// Construct a CuratorServer with no backing stores — all optional fields None.
/// Tools that require stores return permission_denied or unavailable.
fn test_server() -> CuratorServer {
    CuratorServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        None,
        None,
        None,
        None,
        None,
    )
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

// REQ: curator_ping returns liveness and store availability (P5 Testing Discipline).
// expect: curator_ping returns status=ok and reports store availability.
#[tokio::test]
async fn curator_ping_returns_status_ok_via_parameters_seam() {
    let server = test_server();
    let out = server.curator_ping(Parameters(PingRequest {})).await;
    let content = parse_content(&out);
    assert_eq!(content["status"], "ok");
    assert_eq!(content["server"], "hkask-mcp-curator");
    assert!(
        content.get("stores").is_some(),
        "should have stores info: {out}"
    );
}

// REQ: curator_escalations rejects when no escalation queue is configured (P5).
// expect: without an escalation queue, returns kind=permission_denied.
#[tokio::test]
async fn curator_escalations_rejects_without_queue_via_parameters_seam() {
    let server = test_server();
    let out = server.curator_escalations(Parameters(PingRequest {})).await;
    let kind = error_kind(&out).expect("expected error kind for missing queue");
    assert_eq!(kind, "permission_denied", "got: {out}");
}

// REQ: curator_health rejects when no daemon is connected (P5).
// expect: without a daemon, returns kind=unavailable.
#[tokio::test]
async fn curator_health_rejects_without_daemon_via_parameters_seam() {
    let server = test_server();
    let out = server.curator_health(Parameters(PingRequest {})).await;
    let kind = error_kind(&out).expect("expected error kind for missing daemon");
    assert_eq!(kind, "unavailable", "got: {out}");
}

// REQ: curator_semantic_search rejects when no semantic memory is configured (P5).
// expect: without semantic memory, returns kind=permission_denied.
#[tokio::test]
async fn curator_semantic_search_rejects_without_memory_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_curator::types::SemanticSearchRequest =
        serde_json::from_value(serde_json::json!({"query": "test", "limit": 5}))
            .expect("deserialize SemanticSearchRequest");
    let out = server.curator_semantic_search(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for missing semantic memory");
    assert_eq!(kind, "permission_denied", "got: {out}");
}

// REQ: curator_memory_recall reports unavailable status when no memory is configured (P5).
// expect: without memory stores, returns success with episodic.status=unavailable.
#[tokio::test]
async fn curator_memory_recall_reports_unavailable_without_memory_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_curator::types::MemoryRecallRequest =
        serde_json::from_value(serde_json::json!({"entity": "test", "memory_type": "both"}))
            .expect("deserialize MemoryRecallRequest");
    let out = server.curator_memory_recall(Parameters(req)).await;
    let content = parse_content(&out);
    // The tool returns Ok with status=unavailable for missing stores (not an error).
    assert_eq!(content["episodic"]["status"], "unavailable", "got: {out}");
    assert_eq!(content["semantic"]["status"], "unavailable", "got: {out}");
}
