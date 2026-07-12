//! Contract tests for hkask-mcp-memory — episodic memory invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `EpisodicMemory` (HMemStore-backed, uses TestDb for isolation).

use hkask_database::sqlite::SqliteDriver;
use hkask_memory::EpisodicMemory;
use hkask_storage::{HMem, HMemStore};
use hkask_test_harness::TestWebId;
use hkask_types::Visibility;
use hkask_types::visibility::AccessControl;
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
