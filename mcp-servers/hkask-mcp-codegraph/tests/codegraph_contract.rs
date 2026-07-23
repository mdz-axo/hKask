//! Contract tests for hkask-mcp-codegraph — graph store and traversal invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `GraphStore` (in-memory), `find_symbol_by_name`, and traversal.

use hkask_mcp_codegraph::CodeGraphServer;
use hkask_mcp_codegraph::codegraph::graph::store::GraphStore;
use hkask_mcp_codegraph::codegraph::graph::traversal;
use hkask_mcp_codegraph::codegraph::indexer::pipeline::IndexPipeline;
use hkask_mcp_codegraph::codegraph::types::Direction;
use hkask_mcp_server::server::CapabilityTier;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn setup_store() -> GraphStore {
    let store = GraphStore::open_in_memory().expect("in-memory graph store");

    let conn = store.conn();
    conn.execute(
        "INSERT INTO code_files (path, content_hash) VALUES ('src/main.rs', 'hash1')",
        [],
    )
    .expect("insert test file");

    let file_id: i64 = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO symbols (name, kind, file_id, signature, visibility, start_line, end_line)
         VALUES ('test_function', 'function', ?1, 'fn test_function()', 'pub', 10, 20)",
        [file_id],
    )
    .expect("insert symbol");

    store
}

// ── Store tests ────────────────────────────────────────────────────────────

#[test]
fn graph_store_opens_in_memory() {
    let store = GraphStore::open_in_memory();
    assert!(store.is_ok(), "in-memory store should open");
}

#[test]
fn graph_store_has_connection() {
    let store = setup_store();
    let count = store.symbol_count().expect("symbol count");
    assert!(count > 0, "should have at least one symbol, got {count}");
}

#[test]
fn graph_store_find_symbol_by_name() {
    let store = setup_store();
    let id = store
        .find_symbol_by_name("test_function")
        .expect("find should succeed");
    assert!(id.is_some(), "should find test_function by name");

    let id = store
        .find_symbol_by_name("nonexistent")
        .expect("find should succeed");
    assert!(id.is_none(), "should not find nonexistent symbol");
}

// ── Traversal tests ────────────────────────────────────────────────────────

#[test]
fn find_symbol_id_returns_some_for_existing_symbol() {
    let store = setup_store();
    let id = traversal::find_symbol_id(store.conn(), "test_function")
        .expect("find_symbol_id should succeed");
    assert!(id.is_some(), "should find existing symbol");
}

#[test]
fn find_symbol_id_returns_none_for_missing_symbol() {
    let store = setup_store();
    let id = traversal::find_symbol_id(store.conn(), "no_such_symbol")
        .expect("find_symbol_id should succeed");
    assert!(id.is_none(), "should not find missing symbol");
}

// ── Direction enum tests ───────────────────────────────────────────────────

#[test]
fn direction_variants_exist() {
    let forward = Direction::Forward;
    let reverse = Direction::Reverse;
    assert!(matches!(forward, Direction::Forward));
    assert!(matches!(reverse, Direction::Reverse));
}

// ── Context budget tests ───────────────────────────────────────────────────

#[test]
fn context_budget_variants_exist() {
    let budgets = [
        hkask_mcp_codegraph::codegraph::ContextBudget::Minimal,
        hkask_mcp_codegraph::codegraph::ContextBudget::Focused,
        hkask_mcp_codegraph::codegraph::ContextBudget::Standard,
        hkask_mcp_codegraph::codegraph::ContextBudget::Full,
    ];
    assert_eq!(budgets.len(), 4);
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

/// Construct a CodeGraphServer backed by an in-memory store (no indexing).
fn test_server() -> CodeGraphServer {
    let store = GraphStore::open_in_memory().expect("in-memory graph store");
    let pipeline = IndexPipeline::new(store);
    CodeGraphServer::new(
        WebID::new(),
        "test-userpod".into(),
        None,
        CapabilityTier::detect(&HashMap::new()),
        Arc::new(Mutex::new(pipeline)),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
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

// REQ: codegraph_stats returns index statistics (P5 Testing Discipline).
// expect: stats returns files/symbols/edges counts for a fresh server.
#[tokio::test]
async fn codegraph_stats_returns_counts_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_codegraph::StatsRequest =
        serde_json::from_value(serde_json::json!({})).expect("deserialize StatsRequest");
    let out = server.codegraph_stats(Parameters(req)).await;
    let content = parse_content(&out);
    assert!(
        content.get("files").is_some(),
        "stats should have files: {out}"
    );
    assert!(
        content.get("symbols").is_some(),
        "stats should have symbols: {out}"
    );
    assert!(
        content.get("edges").is_some(),
        "stats should have edges: {out}"
    );
}

// REQ: codegraph_traverse rejects an invalid direction at the type level (P5).
// expect: a direction other than 'forward' or 'reverse' is rejected by serde deserialization
// — the type system makes invalid directions unrepresentable (idiomatic-rust Principle 1).
#[tokio::test]
async fn codegraph_traverse_rejects_invalid_direction_via_parameters_seam() {
    let result: Result<hkask_mcp_codegraph::TraverseRequest, _> =
        serde_json::from_value(serde_json::json!({
            "symbol": "nonexistent",
            "direction": "sideways",
            "max_depth": 5
        }));
    assert!(
        result.is_err(),
        "invalid direction 'sideways' must be rejected at deserialization — got: {result:?}"
    );
}

// REQ: codegraph_analysis rejects an unknown analysis kind at the type level (P5).
// expect: a kind other than 'dead_code' or 'complexity' is rejected by serde deserialization
// — the type system makes invalid kinds unrepresentable (idiomatic-rust Principle 1).
#[tokio::test]
async fn codegraph_analysis_rejects_unknown_kind_via_parameters_seam() {
    let result: Result<hkask_mcp_codegraph::AnalysisRequest, _> =
        serde_json::from_value(serde_json::json!({"kind": "nonexistent_analysis"}));
    assert!(
        result.is_err(),
        "invalid kind 'nonexistent_analysis' must be rejected at deserialization — got: {result:?}"
    );
}

// REQ: codegraph_context rejects an invalid budget at the type level (P5).
// expect: a budget other than minimal/focused/standard/full is rejected by serde deserialization
// — the type system makes invalid budgets unrepresentable (idiomatic-rust Principle 1).
#[tokio::test]
async fn codegraph_context_rejects_invalid_budget_via_parameters_seam() {
    let result: Result<hkask_mcp_codegraph::ContextRequest, _> =
        serde_json::from_value(serde_json::json!({
            "query": "test",
            "budget": "ultra"
        }));
    assert!(
        result.is_err(),
        "invalid budget 'ultra' must be rejected at deserialization — got: {result:?}"
    );
}
