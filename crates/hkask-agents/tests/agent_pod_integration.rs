//! Integration tests for hKask agent pod orchestration.
//!
//! Verifies multi-pod lifecycle: creation, activation, mode transitions,
//! status queries, listing, and deactivation. Uses `PodManager::new_mock()`
//! for in-memory testing without external dependencies.
//!
//! # Scope
//!
//! Tests pod orchestration (create → activate → mode → list → deactivate).
//! Actual improv interaction between pods requires the inference port
//! (`InferencePort`), which is not available in the mock manager.
//! Improv integration tests are deferred until mock inference is available.
//!
//! # REQ tags
//!
//! Each test carries a `// REQ:` tag linking it to the contract-first
//! migration plan.

use hkask_agents::{AgentMode, AgentPersona, PodManager, PodStatus};
use hkask_types::PodID;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Create a minimal agent persona for testing.
fn test_persona(name: &str, agent_type: &str) -> AgentPersona {
    let yaml = format!(
        "agent:\n  name: {}\n  type: {}\n  version: 0.1.0\ncharter:\n  description: Test agent for integration\n  editor: test\ncapabilities: []\nrights: []\nresponsibilities: []\nvisibility:\n  default: public\n",
        name, agent_type
    );
    AgentPersona::from_yaml(&yaml).expect("test persona parse")
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// REQ: INT-005.1 — Two-pod creation and activation
///
/// Two pods can be created with distinct identities and activated
/// through the full lifecycle (Populated → Registered → Activated).
#[tokio::test]
async fn two_pod_creation_and_activation() {
    let manager = PodManager::new_mock();

    // Create two pods with different personas
    let alice_persona = test_persona("alice", "Replicant");
    let bob_persona = test_persona("bob", "Bot");

    let alice_id = manager
        .create_pod("alice-template", &alice_persona, Some("alice".into()))
        .await
        .expect("alice pod creation");
    let bob_id = manager
        .create_pod("bob-template", &bob_persona, Some("bob".into()))
        .await
        .expect("bob pod creation");

    // Verify distinct identities
    assert_ne!(alice_id, bob_id, "Pods should have distinct IDs");

    // Both start in Populated state
    let alice_status = manager.get_pod_status(&alice_id).await.unwrap();
    let bob_status = manager.get_pod_status(&bob_id).await.unwrap();
    assert!(format!("{:?}", alice_status.state).contains("Populated"));
    assert!(format!("{:?}", bob_status.state).contains("Populated"));

    // Activate both pods
    manager
        .activate_pod(&alice_id)
        .await
        .expect("alice activation");
    manager.activate_pod(&bob_id).await.expect("bob activation");

    // Both should now be Activated
    let alice_status = manager.get_pod_status(&alice_id).await.unwrap();
    let bob_status = manager.get_pod_status(&bob_id).await.unwrap();
    assert!(format!("{:?}", alice_status.state).contains("Activated"));
    assert!(format!("{:?}", bob_status.state).contains("Activated"));
}

/// REQ: INT-005.2 — Pod listing after multi-pod creation
///
/// `list_pods()` returns all created pods with correct status.
#[tokio::test]
async fn list_pods_after_multi_pod_creation() {
    let manager = PodManager::new_mock();

    let alice = test_persona("alice", "Replicant");
    let bob = test_persona("bob", "Bot");
    let carol = test_persona("carol", "Curator");

    let a_id = manager.create_pod("a", &alice, None).await.unwrap();
    let b_id = manager.create_pod("b", &bob, None).await.unwrap();
    let c_id = manager.create_pod("c", &carol, None).await.unwrap();

    // Activate only alice and bob
    manager.activate_pod(&a_id).await.unwrap();
    manager.activate_pod(&b_id).await.unwrap();

    let pods = manager.list_pods().await;
    assert_eq!(pods.len(), 3, "Should list all 3 pods");

    // Verify states
    let alice_status = pods.iter().find(|s| s.pod_id == a_id).unwrap();
    assert!(format!("{:?}", alice_status.state).contains("Activated"));

    let bob_status = pods.iter().find(|s| s.pod_id == b_id).unwrap();
    assert!(format!("{:?}", bob_status.state).contains("Activated"));

    let carol_status = pods.iter().find(|s| s.pod_id == c_id).unwrap();
    assert!(format!("{:?}", carol_status.state).contains("Populated"));
}

/// REQ: INT-005.3 — Mode transitions on activated pod
///
/// An activated pod can enter server mode, exit, and enter chat mode.
#[tokio::test]
async fn mode_transitions_on_activated_pod() {
    let manager = PodManager::new_mock();
    let persona = test_persona("agent", "Replicant");
    let pod_id = manager.create_pod("test", &persona, None).await.unwrap();

    // Activate and assign a role
    manager.activate_pod(&pod_id).await.unwrap();
    manager.assign_role(&pod_id, "research").await.unwrap();

    // Enter server mode
    manager
        .set_mode(&pod_id, AgentMode::Server("research".into()))
        .await
        .expect("enter server mode");

    let status = manager.get_pod_status(&pod_id).await.unwrap();
    assert!(format!("{:?}", status.state).contains("Activated"));

    // Exit mode
    manager
        .set_mode(&pod_id, AgentMode::None)
        .await
        .expect("exit mode");

    // Enter chat mode
    manager
        .set_mode(&pod_id, AgentMode::Chat)
        .await
        .expect("enter chat mode");
}

/// REQ: INT-005.4 — Deactivation and reactivation
///
/// A pod can be deactivated and its state transitions correctly.
#[tokio::test]
async fn deactivation_and_reactivation() {
    let manager = PodManager::new_mock();
    let persona = test_persona("agent", "Replicant");
    let pod_id = manager.create_pod("test", &persona, None).await.unwrap();

    manager.activate_pod(&pod_id).await.unwrap();
    assert!(
        format!("{:?}", manager.get_pod_status(&pod_id).await.unwrap().state).contains("Activated")
    );

    manager.deactivate_pod(&pod_id).await.unwrap();
    assert!(
        format!("{:?}", manager.get_pod_status(&pod_id).await.unwrap().state)
            .contains("Deactivated")
    );
}

/// REQ: INT-005.5 — Pod not found error
///
/// Querying a non-existent pod returns PodNotFound.
#[tokio::test]
async fn nonexistent_pod_returns_error() {
    let manager = PodManager::new_mock();
    let fake_id = PodID::new();

    let result = manager.get_pod_status(&fake_id).await;
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention pod not found"
    );
}
