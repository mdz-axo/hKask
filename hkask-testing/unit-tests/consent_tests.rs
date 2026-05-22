//! Consent Manager Unit Tests
//!
//! Tests for hkask-agents consent management

use hkask_agents::consent::ConsentManager;
use hkask_storage::SovereigntyBoundaryStore;
use hkask_types::DataCategory;

#[test]
fn test_consent_manager_grant_revoke() {
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let manager = ConsentManager::new(store);
    let webid = "did:web:test.example.com:user1";
    let category = DataCategory::EpisodicMemory;

    // Initially no consent
    assert!(!manager.has_consent(webid, &category));

    // Grant consent
    manager.grant_consent(webid, &category).unwrap();
    assert!(manager.has_consent(webid, &category));

    // Revoke consent
    manager.revoke_consent(webid).unwrap();
    assert!(!manager.has_consent(webid, &category));
}

#[test]
fn test_consent_manager_multiple_categories() {
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let manager = ConsentManager::new(store);
    let webid = "did:web:test.example.com:user2";

    manager
        .grant_consent(webid, &DataCategory::EpisodicMemory)
        .unwrap();
    manager
        .grant_consent(webid, &DataCategory::SemanticMemory)
        .unwrap();

    assert!(manager.has_consent(webid, &DataCategory::EpisodicMemory));
    assert!(manager.has_consent(webid, &DataCategory::SemanticMemory));
    assert!(!manager.has_consent(webid, &DataCategory::PersonalContext));

    let categories = manager.get_granted_categories(webid);
    assert_eq!(categories.len(), 2);
}

#[test]
fn test_consent_manager_clear() {
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let manager = ConsentManager::new(store);
    let webid = "did:web:test.example.com:user3";

    manager
        .grant_consent(webid, &DataCategory::EpisodicMemory)
        .unwrap();
    assert!(manager.has_consent(webid, &DataCategory::EpisodicMemory));

    manager.clear();
    assert!(!manager.has_consent(webid, &DataCategory::EpisodicMemory));
}
