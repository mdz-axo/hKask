//! Git Archival Error Unit Tests
//!
//! Tests for hkask-types error types

use hkask_types::error::GitArchivalError;

#[test]
fn test_error_display() {
    let err = GitArchivalError::AdapterNotFound("git".to_string());
    assert_eq!(err.to_string(), "Adapter not configured: git");
}

#[test]
fn test_error_recovery_classification() {
    let network_err = GitArchivalError::NetworkError("timeout".to_string());
    assert!(network_err.is_recoverable());
    assert!(!network_err.requires_user_intervention());

    let cap_err = GitArchivalError::CapabilityDenied("missing token".to_string());
    assert!(!cap_err.is_recoverable());
    assert!(cap_err.requires_user_intervention());
}

#[test]
fn test_error_serialization() {
    let err = GitArchivalError::RepositoryNotFound {
        owner: "test".to_string(),
        repo: "repo".to_string(),
    };
    let json = serde_json::to_string(&err).unwrap();
    // Serialized error should contain owner and repo
    assert!(json.contains("test"));
    assert!(json.contains("repo"));
}