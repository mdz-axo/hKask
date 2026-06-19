//! Contract tests for hkask-mcp-memory — episodic memory invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `EpisodicMemory` (TripleStore-backed, uses TestDb for isolation).

use hkask_memory::EpisodicMemory;
use hkask_storage::{Triple, TripleStore};
use hkask_test_harness::TestWebId;
use hkask_types::Visibility;
use hkask_types::visibility::AccessControl;
use serde_json::json;
use std::sync::Arc;

fn setup_store() -> TripleStore {
    let conn = Arc::new(std::sync::Mutex::new(
        rusqlite::Connection::open_in_memory().expect("in-memory SQLite"),
    ));
    conn.lock()
        .expect("mutex not poisoned")
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS triples (
            id TEXT PRIMARY KEY,
            entity TEXT NOT NULL,
            attribute TEXT NOT NULL,
            value TEXT NOT NULL,
            valid_from TEXT NOT NULL,
            valid_to TEXT,
            confidence REAL NOT NULL,
            perspective TEXT,
            visibility TEXT NOT NULL,
            owner_webid TEXT NOT NULL
        );",
        )
        .expect("DDL must succeed");
    TripleStore::new(conn)
}

fn make_triple(
    entity: &str,
    attr: &str,
    value: serde_json::Value,
    perspective: &hkask_types::WebID,
) -> Triple {
    let mut t = Triple::new(entity, attr, value, *perspective);
    t.access = AccessControl::episodic(*perspective, *perspective);
    t
}

// ── Store contract tests ────────────────────────────────────────────────────

// contract: MEM-STORE-001
// expect: "I can store first-person experience triples in my sovereign episodic memory" [P3]
// [P1] Constraining: rejects Public visibility — episodic is sovereign
#[test]
fn store_and_recall_episodic_triple() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();
    let triple = make_triple("session:1", "action", json!("login"), &owner);

    mem.store(triple).expect("store should succeed");

    let recalled = mem
        .query_for_deduped("session:1", owner)
        .expect("recall should succeed");
    assert_eq!(recalled.len(), 1);
    assert_eq!(recalled[0].entity, "session:1");
    assert_eq!(recalled[0].attribute, "action");
}

// contract: MEM-STORE-002
// expect: "I can verify the system enforces sovereignty boundaries on episodic memory" [P1]
#[test]
fn store_rejects_public_triple() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();
    let mut triple = make_triple("e", "a", json!("v"), &owner);
    triple.access.visibility = Visibility::Public;

    let err = mem.store(triple).expect_err("should reject public triple");
    assert!(err.to_string().contains("visibility") || err.to_string().contains("Public"));
}

// contract: MEM-STORE-003
// expect: "I can verify the system rejects anonymous episodic storage" [P12]
#[test]
fn store_requires_perspective() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();
    let mut triple = make_triple("e", "a", json!("v"), &owner);
    triple.access.perspective = None;

    let err = mem
        .store(triple)
        .expect_err("should reject missing perspective");
    assert!(err.to_string().contains("perspective") || err.to_string().contains("Perspective"));
}

// ── Recall contract tests ───────────────────────────────────────────────────

// contract: MEM-RECALL-001
// expect: "I can verify that episodic recall respects sovereignty — mine vs. yours" [P1]
#[test]
fn recall_filters_by_perspective() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let alice = TestWebId::alice();
    let bob = TestWebId::bob();

    mem.store(make_triple(
        "session:1",
        "action",
        json!("alice did this"),
        &alice,
    ))
    .expect("alice store");
    mem.store(make_triple(
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

// contract: MEM-RECALL-002
// expect: "I can query for nonexistent memories and get a clean empty result" [P8]
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

// contract: MEM-BUDGET-001
// expect: "I can query my episodic storage usage" [P9]
#[test]
fn storage_usage_reports_count() {
    let store = setup_store();
    let mem = EpisodicMemory::new(store);
    let owner = TestWebId::alice();

    let usage_before = mem.storage_usage(&owner).expect("usage before");
    assert_eq!(usage_before, 0);

    mem.store(make_triple("e1", "a", json!("v1"), &owner))
        .expect("store 1");
    mem.store(make_triple("e2", "a", json!("v2"), &owner))
        .expect("store 2");

    let usage_after = mem.storage_usage(&owner).expect("usage after");
    assert_eq!(usage_after, 2);
}
