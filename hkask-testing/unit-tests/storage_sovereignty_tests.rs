//! Sovereignty Boundary Store Unit Tests
//!
//! Tests for hkask-storage sovereignty operations

use hkask_storage::sovereignty::{SovereigntyBoundaryEntry, SovereigntyBoundaryStore};
use hkask_types::sovereignty::UserSovereigntyState;

#[test]
fn test_sovereignty_store_in_memory() {
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_boundaries, 0);
}

#[test]
fn test_sovereignty_store_roundtrip() {
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let webid = "did:web:test.example.com:user1";

    let state = UserSovereigntyState::new();
    let entry = SovereigntyBoundaryEntry::from_state(webid, &state);

    store.store(&entry).unwrap();

    let retrieved = store.get(webid).unwrap();
    assert!(retrieved.is_some());

    let retrieved_entry = retrieved.unwrap();
    assert_eq!(retrieved_entry.webid, webid);

    let retrieved_state = retrieved_entry.to_state().unwrap();
    assert_eq!(
        retrieved_state.boundary.sovereign_data,
        state.boundary.sovereign_data
    );
}

#[test]
fn test_sovereignty_store_update_threshold() {
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let webid = "did:web:test.example.com:user2";

    let state = UserSovereigntyState::new();
    let entry = SovereigntyBoundaryEntry::from_state(webid, &state);
    store.store(&entry).unwrap();

    store.update_kill_zone_threshold(webid, 0.5).unwrap();

    let retrieved = store.get(webid).unwrap().unwrap();
    assert_eq!(retrieved.kill_zone_threshold, 0.5);
}

#[test]
fn test_sovereignty_store_delete() {
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let webid = "did:web:test.example.com:user3";

    let state = UserSovereigntyState::new();
    let entry = SovereigntyBoundaryEntry::from_state(webid, &state);
    store.store(&entry).unwrap();

    store.delete(webid).unwrap();

    let retrieved = store.get(webid).unwrap();
    assert!(retrieved.is_none());
}
