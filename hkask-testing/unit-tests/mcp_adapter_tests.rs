//! MCP unit tests migrated from inline tests
//!
//! Tests for: adapter_container.rs and archival_service.rs

use hkask_mcp::adapter_container::AdapterContainer;
use hkask_mcp::archival_service::ArchivalService;
use hkask_types::{GitArchivalError, WebID};

// ============================================================================
// Adapter Container Tests
// ============================================================================

#[test]
fn test_adapter_container_new() {
    let container = AdapterContainer::new();
    assert!(!container.has_git_cas());
    assert!(container.get_base_path().is_none());
}

#[test]
fn test_adapter_container_configure() {
    let container = AdapterContainer::new();
    let temp_path = std::env::temp_dir().join("hkask_adapter_test");

    std::fs::create_dir_all(&temp_path).unwrap();

    let result = container.configure_git_cas(temp_path.clone());
    assert!(result.is_ok());
    assert!(container.has_git_cas());
    assert_eq!(container.get_base_path(), Some(temp_path.clone()));

    std::fs::remove_dir_all(&temp_path).ok();
}

#[test]
fn test_adapter_container_clear() {
    let container = AdapterContainer::new();
    let temp_path = std::env::temp_dir().join("hkask_adapter_clear_test");

    std::fs::create_dir_all(&temp_path).unwrap();
    container.configure_git_cas(temp_path.clone()).unwrap();
    assert!(container.has_git_cas());

    container.clear();
    assert!(!container.has_git_cas());
    assert!(container.get_base_path().is_none());

    std::fs::remove_dir_all(&temp_path).ok();
}

#[test]
fn test_adapter_container_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let container = Arc::new(AdapterContainer::new());
    let mut handles = vec![];

    for i in 0..10 {
        let container_clone = Arc::clone(&container);
        let handle = thread::spawn(move || {
            let temp_path = std::env::temp_dir().join(format!("hkask_thread_{}", i));
            std::fs::create_dir_all(&temp_path).unwrap();

            container_clone
                .configure_git_cas(temp_path.clone())
                .unwrap();
            assert!(container_clone.has_git_cas());

            std::fs::remove_dir_all(&temp_path).ok();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// ============================================================================
// Archival Service Tests
// ============================================================================

#[tokio::test]
async fn test_archival_service_new() {
    let container = AdapterContainer::new();
    let owner = WebID::new();
    let _service = ArchivalService::new(container, owner);

    // Service should be created without error
    assert!(true);
}

#[tokio::test]
async fn test_archive_without_adapter() {
    let container = AdapterContainer::new();
    let owner = WebID::new();
    let service = ArchivalService::new(container, owner);

    let result = service
        .archive("owner", "repo", "main", "path", "content", &owner)
        .await;
    assert!(matches!(result, Err(GitArchivalError::AdapterNotFound(_))));
}

#[tokio::test]
async fn test_archive_sovereignty_check() {
    let container = AdapterContainer::new();
    let owner = WebID::new();
    let service = ArchivalService::new(container, owner);

    // Without adapter, should fail (sovereignty check passes for owner)
    let result = service
        .archive("owner", "repo", "main", "path", "content", &owner)
        .await;
    assert!(matches!(result, Err(GitArchivalError::AdapterNotFound(_))));
}
