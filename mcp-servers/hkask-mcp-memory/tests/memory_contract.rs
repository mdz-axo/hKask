//! Contract tests for hkask-mcp-memory — episodic memory invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `EpisodicMemory` (HMemStore-backed, uses TestDb for isolation).

use hkask_database::sqlite::SqliteDriver;
use hkask_mcp_memory::MemoryServer;
use hkask_mcp_memory::types::{RecallRequest, StoreRequest};
use hkask_memory::EpisodicMemory;
use hkask_storage::{HMem, HMemStore};
use hkask_test_harness::TestWebId;
use hkask_types::Visibility;
use hkask_types::visibility::AccessControl;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use std::sync::Arc;

fn setup_store() -> HMemStore {
    let pool = SqliteDriver::in_memory_pool().expect("in-memory pool");
    let driver: Arc<dyn hkask_database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(pool));
    HMemStore::from_driver(driver)
}

fn make_h_mem(
    entity: &str,
    attr: &str,
    value: serde_json::Value,
    perspective: &hkask_types::WebID,
) -> HMem {
    let mut t = HMem::new(entity, attr, value, *perspective);
    t.access = AccessControl::episodic(*perspective, *perspective);
    t
}

// ── Store contract tests ────────────────────────────────────────────────────

// [P1] Constraining: rejects Public visibility — episodic is sovereign
#[test]
fn store_and_recall_episodic_triple() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();
    let h_mem = make_h_mem("session:1", "action", json!("login"), &owner);

    mem.store(h_mem).expect("store should succeed");

    let recalled = mem
        .query_for_deduped("session:1", owner)
        .expect("recall should succeed");
    assert_eq!(recalled.len(), 1);
    assert_eq!(recalled[0].entity, "session:1");
    assert_eq!(recalled[0].attribute, "action");
}

#[test]
fn store_rejects_public_triple() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();
    let mut h_mem = make_h_mem("e", "a", json!("v"), &owner);
    h_mem.access.visibility = Visibility::Public;

    let err = mem.store(h_mem).expect_err("should reject public h_mem");
    assert!(err.to_string().contains("visibility") || err.to_string().contains("Public"));
}

// expect: "I can verify the system rejects anonymous episodic storage" [P12]
#[test]
fn store_requires_perspective() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();
    let mut h_mem = make_h_mem("e", "a", json!("v"), &owner);
    h_mem.access.perspective = None;

    let err = mem
        .store(h_mem)
        .expect_err("should reject missing perspective");
    assert!(err.to_string().contains("perspective") || err.to_string().contains("Perspective"));
}

// ── Recall contract tests ───────────────────────────────────────────────────

#[test]
fn recall_filters_by_perspective() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let alice = TestWebId::alice();
    let bob = TestWebId::bob();

    mem.store(make_h_mem(
        "session:1",
        "action",
        json!("alice did this"),
        &alice,
    ))
    .expect("alice store");
    mem.store(make_h_mem(
        "session:1",
        "action",
        json!("bob did this"),
        &bob,
    ))
    .expect("bob store");

    let alice_recall = mem
        .query_for_deduped("session:1", alice)
        .expect("alice recall");
    assert_eq!(alice_recall.len(), 1);
    assert_eq!(alice_recall[0].access.perspective, Some(alice));

    let bob_recall = mem.query_for_deduped("session:1", bob).expect("bob recall");
    assert_eq!(bob_recall.len(), 1);
    assert_eq!(bob_recall[0].access.perspective, Some(bob));
}

#[test]
fn recall_nonexistent_returns_empty() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();

    let recalled = mem
        .query_for_deduped("nonexistent", owner)
        .expect("recall should succeed");
    assert!(recalled.is_empty());
}

// ── Storage budget contract tests ───────────────────────────────────────────

#[test]
fn storage_usage_reports_count() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();

    let usage_before = mem.storage_usage(&owner).expect("usage before");
    assert_eq!(usage_before, 0);

    mem.store(make_h_mem("e1", "a", json!("v1"), &owner))
        .expect("store 1");
    mem.store(make_h_mem("e2", "a", json!("v2"), &owner))
        .expect("store 2");

    let usage_after = mem.storage_usage(&owner).expect("usage after");
    assert_eq!(usage_after, 2);
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

/// Construct a MemoryServer backed by an in-memory store.
fn test_server() -> MemoryServer {
    let pool = SqliteDriver::in_memory_pool().expect("in-memory pool");
    let driver: Arc<dyn hkask_database::driver::DatabaseDriver> =
        Arc::new(SqliteDriver::new(pool.clone()));
    let h_mem_store = HMemStore::from_driver(Arc::clone(&driver));
    let episodic = EpisodicMemory::new(h_mem_store);
    let h_mem_store2 = HMemStore::from_driver(driver);
    let embedding_store =
        hkask_storage::EmbeddingStore::from_driver(Arc::new(SqliteDriver::new(pool)), 1024);
    let semantic = Arc::new(hkask_memory::SemanticMemory::new(
        h_mem_store2,
        embedding_store,
    ));
    MemoryServer::new(
        TestWebId::alice(),
        "test-replicant".into(),
        None,
        episodic,
        semantic,
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

// REQ: episodic_ping returns liveness and perspective (P5 Testing Discipline).
// expect: episodic_ping returns status=ok and the caller's WebID.
#[tokio::test]
async fn episodic_ping_returns_status_ok_via_parameters_seam() {
    let server = test_server();
    let out = server.episodic_ping().await;
    let content = parse_content(&out);
    assert_eq!(content["status"], "ok");
    assert_eq!(content["server"], "hkask-mcp-memory");
    assert!(
        content.get("perspective").is_some(),
        "should have perspective: {out}"
    );
}

// REQ: episodic_store stores a valid h_mem (P5).
// expect: storing a valid entity/attribute/value returns stored=true.
#[tokio::test]
async fn episodic_store_succeeds_via_parameters_seam() {
    let server = test_server();
    let out = server
        .episodic_store(Parameters(StoreRequest {
            entity: "session:1".into(),
            attribute: "action".into(),
            value: json!("login"),
            confidence: None,
        }))
        .await;
    let content = parse_content(&out);
    assert_eq!(content["stored"], true, "got: {out}");
    assert_eq!(content["entity"], "session:1");
}

// REQ: episodic_store rejects an empty entity identifier (P5, P3).
// expect: an empty entity returns kind=invalid_argument.
#[tokio::test]
async fn episodic_store_rejects_empty_entity_via_parameters_seam() {
    let server = test_server();
    let out = server
        .episodic_store(Parameters(StoreRequest {
            entity: String::new(),
            attribute: "action".into(),
            value: json!("v"),
            confidence: None,
        }))
        .await;
    let kind = error_kind(&out).expect("expected error kind for empty entity");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: episodic_recall returns stored episodes for the entity (P5).
// expect: after storing, recall returns the stored episode.
#[tokio::test]
async fn episodic_recall_returns_stored_episode_via_parameters_seam() {
    let server = test_server();
    // Store first
    server
        .episodic_store(Parameters(StoreRequest {
            entity: "session:42".into(),
            attribute: "action".into(),
            value: json!("test_action"),
            confidence: None,
        }))
        .await;
    let out = server
        .episodic_recall(Parameters(RecallRequest {
            entity: "session:42".into(),
        }))
        .await;
    let content = parse_content(&out);
    let h_mems = content["h_mems"].as_array().expect("h_mems array");
    assert!(
        !h_mems.is_empty(),
        "should recall at least one h_mem: {out}"
    );
}

// REQ: episodic_recall returns empty for a non-existent entity (P5).
// expect: recalling an entity that was never stored returns an empty array.
#[tokio::test]
async fn episodic_recall_returns_empty_for_unknown_entity_via_parameters_seam() {
    let server = test_server();
    let out = server
        .episodic_recall(Parameters(RecallRequest {
            entity: "nonexistent:entity".into(),
        }))
        .await;
    let content = parse_content(&out);
    let h_mems = content["h_mems"].as_array().expect("h_mems array");
    assert!(
        h_mems.is_empty(),
        "should return empty for unknown entity: {out}"
    );
}
