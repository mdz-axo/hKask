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
