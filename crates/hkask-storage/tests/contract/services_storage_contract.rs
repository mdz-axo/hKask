//! Service→Storage contract tests — Wave 4 Task 4.2
//!
//! Verifies that storage operations produce correct and deduplicated state.
//! Uses TestDb from hkask-test-harness for isolated database instances.
//!
//! # Principle grounding
//! - P4 (Clear Boundaries): storage schema changes must not break service expectations
//! - P8 (Semantic Grounding): each contract asserts a stated behavioral property

use hkask_storage::TripleStore;
use hkask_test_harness::{TestDb, TestWebId, test_triple};
use serde_json::json;

// REQ: CTR-002 — Service→Storage contract (P4, P8)
// Storage operations produce correct and deduplicated state.

#[test]
fn triple_insert_and_query() {
    let db = TestDb::new();
    let store = TripleStore::new(db.conn_arc());

    let triple = test_triple("entity:test", "attr:name", json!("value"));
    store.insert(&triple).expect("insert should succeed");

    let results = store
        .query_by_entity("entity:test")
        .expect("query should succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity, "entity:test");
    assert_eq!(results[0].attribute, "attr:name");
}

#[test]
fn triple_query_by_attribute() {
    let db = TestDb::new();
    let store = TripleStore::new(db.conn_arc());

    let t1 = test_triple("entity:a", "attr:shared", json!("v1"));
    let t2 = test_triple("entity:b", "attr:shared", json!("v2"));
    store.insert(&t1).expect("insert t1");
    store.insert(&t2).expect("insert t2");

    let results = store
        .query_by_attribute("attr:shared")
        .expect("query should succeed");
    assert_eq!(results.len(), 2);
}

#[test]
fn triple_count_is_accurate() {
    let db = TestDb::new();
    let store = TripleStore::new(db.conn_arc());

    assert_eq!(store.count_semantic().unwrap(), 0);

    store.insert(&test_triple("e1", "a1", json!("v1"))).unwrap();
    store.insert(&test_triple("e2", "a2", json!("v2"))).unwrap();
    store.insert(&test_triple("e3", "a3", json!("v3"))).unwrap();

    assert_eq!(store.count_semantic().unwrap(), 3);
}

#[test]
fn triple_delete_removes_correctly() {
    let db = TestDb::new();
    let store = TripleStore::new(db.conn_arc());

    let triple = test_triple("entity:del", "attr:test", json!("value"));
    store.insert(&triple).unwrap();
    assert_eq!(store.count_semantic().unwrap(), 1);

    store
        .delete_by_id(&triple.id)
        .expect("delete should succeed");
    assert_eq!(store.count_semantic().unwrap(), 0);
}

#[test]
fn triple_owner_webid_is_preserved() {
    let db = TestDb::new();
    let store = TripleStore::new(db.conn_arc());

    let owner = TestWebId::alice();
    let triple = hkask_test_harness::test_triple_with_owner(
        "entity:owned",
        "attr:owner",
        json!("data"),
        owner,
    );
    store.insert(&triple).unwrap();

    let results = store.query_by_entity("entity:owned").unwrap();
    assert_eq!(results.len(), 1);
    // Verify owner is stored (access.owner_webid)
    assert_eq!(results[0].access.owner_webid, owner);
}
