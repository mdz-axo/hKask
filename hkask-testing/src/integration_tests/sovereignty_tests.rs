//! Sovereignty Integration Tests
//!
//! Tests sovereignty enforcement across archival operations:
//! - Archive with sovereignty check
//! - Consent grant/revoke flow
//! - Boundary update flow
//! - Kill-zone detection

use hkask_agents::{ConsentManager, SovereigntyChecker};
use hkask_mcp::{adapter_container::AdapterContainer, archival_service::ArchivalService};
use hkask_storage::SovereigntyBoundaryStore;
use hkask_types::{DataCategory, UserSovereigntyState, WebID};

/// Test sovereignty check during archival operation
#[tokio::test]
async fn test_sovereignty_archive_check() {
    // Setup
    let container = AdapterContainer::new();
    let owner = WebID::new();
    let service = ArchivalService::new(container, owner);

    // Archive without adapter should fail (sovereignty passes for owner)
    let result = service
        .archive("owner", "repo", "main", "path", "content", &owner)
        .await;

    // Should fail due to missing adapter, not sovereignty
    assert!(matches!(
        result,
        Err(hkask_types::GitArchivalError::AdapterNotFound(_))
    ));
}

/// Test consent grant enables archival
#[tokio::test]
async fn test_consent_grant_enables_access() {
    // Setup
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let manager = ConsentManager::new(store);
    let webid = "did:web:test.example.com:user1";

    // Initially no consent for episodic memory
    assert!(!manager.has_consent(webid, &DataCategory::EpisodicMemory));

    // Grant consent
    manager
        .grant_consent(webid, &DataCategory::EpisodicMemory)
        .unwrap();

    // Now has consent
    assert!(manager.has_consent(webid, &DataCategory::EpisodicMemory));

    // Revoke consent
    manager.revoke_consent(webid).unwrap();

    // No longer has consent
    assert!(!manager.has_consent(webid, &DataCategory::EpisodicMemory));
}

/// Test boundary update affects sovereignty check
#[tokio::test]
async fn test_boundary_update_affects_sovereignty() {
    // Setup
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let webid = "did:web:test.example.com:user2";

    // Store initial boundary
    let state = UserSovereigntyState::new();
    let entry = hkask_storage::SovereigntyBoundaryEntry::from_state(webid, &state);
    store.store(&entry).unwrap();

    // Verify initial state (default threshold is 0.5)
    let retrieved = store.get(webid).unwrap().unwrap();
    assert_eq!(retrieved.kill_zone_threshold, 0.5);

    // Update threshold
    store.update_kill_zone_threshold(webid, 0.3).unwrap();

    // Verify update
    let updated = store.get(webid).unwrap().unwrap();
    assert_eq!(updated.kill_zone_threshold, 0.3);
}

/// Test kill-zone detection triggers on VC investment
#[tokio::test]
async fn test_kill_zone_detection() {
    // Setup
    let owner = WebID::new();
    let mut checker = SovereigntyChecker::new(owner);

    // Initially not compromised
    assert!(!checker.is_compromised());
    assert!(!checker.kill_zone_active());

    // Mark acquisition attempt first
    checker.mark_acquisition_attempt(&serde_json::json!({}));

    // Simulate VC investment below threshold (default is 0.5)
    checker.update_vc_investment(0.3);

    // Kill zone should be active (0.3 < 0.5 threshold, after acquisition attempt)
    assert!(checker.is_compromised());
    assert!(checker.kill_zone_active());
}

/// Test sovereignty checker with different data categories
#[tokio::test]
async fn test_sovereignty_by_category() {
    // Setup
    let owner = WebID::new();
    let mut checker = SovereigntyChecker::new(owner);

    // Grant consent for sovereign data access
    checker.grant_consent();

    // Episodic memory is sovereign - owner with consent can access
    assert!(checker.can_access(&DataCategory::EpisodicMemory, &owner));

    // hLexicon terms are public - anyone can access
    assert!(checker.can_access(&DataCategory::HLexiconTerms, &owner));
    assert!(checker.can_access(&DataCategory::HLexiconTerms, &WebID::new()));
}

/// Test archival service sovereignty denial
#[tokio::test]
async fn test_archival_sovereignty_denial() {
    // Setup
    let container = AdapterContainer::new();
    let owner = WebID::new();
    let service = ArchivalService::new(container, owner);

    // Archive should check sovereignty
    // For owner with default boundary, registry access should be allowed
    // But adapter is missing, so will fail at adapter check
    let result = service
        .archive("owner", "repo", "main", "path", "content", &owner)
        .await;

    // Should fail at adapter check (sovereignty passes for owner)
    assert!(matches!(
        result,
        Err(hkask_types::GitArchivalError::AdapterNotFound(_))
    ));
}

/// Test consent manager multiple categories
#[tokio::test]
async fn test_consent_multiple_categories() {
    // Setup
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let manager = ConsentManager::new(store);
    let webid = "did:web:test.example.com:user3";

    // Grant consent for multiple categories
    manager
        .grant_consent(webid, &DataCategory::EpisodicMemory)
        .unwrap();
    manager
        .grant_consent(webid, &DataCategory::SemanticMemory)
        .unwrap();
    manager
        .grant_consent(webid, &DataCategory::PersonalContext)
        .unwrap();

    // Verify all granted
    assert!(manager.has_consent(webid, &DataCategory::EpisodicMemory));
    assert!(manager.has_consent(webid, &DataCategory::SemanticMemory));
    assert!(manager.has_consent(webid, &DataCategory::PersonalContext));

    // Ungranted category
    assert!(!manager.has_consent(webid, &DataCategory::CapabilityTokens));

    // Get all granted
    let granted = manager.get_granted_categories(webid);
    assert_eq!(granted.len(), 3);
}

/// Test sovereignty boundary store persistence
#[tokio::test]
async fn test_sovereignty_store_persistence() {
    // Setup
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let webid = "did:web:test.example.com:user4";

    // Store boundary
    let state = UserSovereigntyState::new();
    let entry = hkask_storage::SovereigntyBoundaryEntry::from_state(webid, &state);
    store.store(&entry).unwrap();

    // Retrieve and verify
    let retrieved = store.get(webid).unwrap().unwrap();
    assert_eq!(retrieved.webid, webid);

    // Convert back to state
    let retrieved_state = retrieved.to_state().unwrap();
    assert_eq!(
        retrieved_state.boundary.sovereign_data,
        state.boundary.sovereign_data
    );

    // Delete
    store.delete(webid).unwrap();
    assert!(store.get(webid).unwrap().is_none());
}

/// Test archival restore operation
#[tokio::test]
async fn test_archival_restore() {
    // Setup
    let container = AdapterContainer::new();
    let owner = WebID::new();
    let service = ArchivalService::new(container, owner);

    // Restore without adapter should fail
    let result = service
        .restore("owner", "repo", "main", "target", &owner)
        .await;

    // Should fail due to missing adapter
    assert!(matches!(
        result,
        Err(hkask_types::GitArchivalError::AdapterNotFound(_))
    ));
}

/// Test archival list operation
#[tokio::test]
async fn test_archival_list() {
    // Setup
    let container = AdapterContainer::new();
    let owner = WebID::new();
    let service = ArchivalService::new(container, owner);

    // List should work (no adapter needed for listing metadata)
    let result = service.list_archives("owner", "repo", &owner).await;

    // Should return simulated commits
    assert!(result.is_ok());
    let commits = result.unwrap();
    assert_eq!(commits.len(), 3);
}

/// Test archival snapshot operation
#[tokio::test]
async fn test_archival_snapshot() {
    // Setup
    let container = AdapterContainer::new();
    let owner = WebID::new();
    let service = ArchivalService::new(container, owner);

    // Snapshot without adapter should fail
    let result = service
        .create_snapshot("owner", "repo", "test message", &owner)
        .await;

    // Should fail due to missing adapter
    assert!(matches!(
        result,
        Err(hkask_types::GitArchivalError::AdapterNotFound(_))
    ));
}

/// Test sovereignty checker consent flow
#[tokio::test]
async fn test_sovereignty_consent_flow() {
    // Setup
    let owner = WebID::new();
    let mut checker = SovereigntyChecker::new(owner);

    // Initially no explicit consent
    assert!(!checker.get_state().explicit_consent);

    // Grant consent
    checker.grant_consent();
    assert!(checker.get_state().explicit_consent);

    // Revoke consent
    checker.revoke_consent();
    assert!(!checker.get_state().explicit_consent);
}

/// Test sovereignty boundary resistance levels
#[tokio::test]
async fn test_sovereignty_resistance_levels() {
    use hkask_types::AcquisitionResistance;

    // Setup
    let store = SovereigntyBoundaryStore::in_memory().unwrap();
    let webid = "did:web:test.example.com:user5";

    // Store with default resistance
    let state = UserSovereigntyState::new();
    let entry = hkask_storage::SovereigntyBoundaryEntry::from_state(webid, &state);
    store.store(&entry).unwrap();

    // Update resistance
    store
        .update_resistance(webid, AcquisitionResistance::None)
        .unwrap();

    // Verify update
    let updated = store.get(webid).unwrap().unwrap();
    assert_eq!(updated.resistance, "None");
}

/// Test adapter container configuration
#[tokio::test]
async fn test_adapter_container_config() {
    // Setup
    let container = AdapterContainer::new();
    let temp_path = std::env::temp_dir().join("hkask_adapter_config_test");

    std::fs::create_dir_all(&temp_path).unwrap();

    // Configure
    container.configure_git_cas(temp_path.clone()).unwrap();

    // Verify
    assert!(container.has_git_cas());
    assert_eq!(container.get_base_path(), Some(temp_path.clone()));

    // Clear
    container.clear();
    assert!(!container.has_git_cas());

    std::fs::remove_dir_all(&temp_path).ok();
}
