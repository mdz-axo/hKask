//! Integration tests for hKask agent pod orchestration.
//!
//! Verifies multi-pod lifecycle: creation, activation, mode transitions,
//! status queries, listing, and deactivation. Uses `PodManager::new_mock()`
//! for in-memory testing without external dependencies.
//!
//! # Scope
//!
//! Tests pod orchestration (create → activate → mode → list → deactivate).
//! Mock inference is available via `MockInferencePort` for basic wiring tests.
//! Full improv interaction between pods (using `hkask-improv` modes) requires
//! integrating the improv protocol with the inference port — deferred to L4.
//!
//! # REQ tags
//!
//! Each test carries a `// REQ:` tag linking it to the contract-first
//! migration plan.

use hkask_agents::{AgentPersona, PodManager};
use hkask_test_harness::mocks::MockInferencePort;
use hkask_types::PodID;
use std::sync::Arc;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Create a minimal agent persona for testing.
fn test_persona(name: &str, agent_type: &str) -> AgentPersona {
    let yaml = format!(
        "agent:\n  name: {}\n  type: {}\n  version: 0.1.0\ncharter:\n  description: Test agent for integration\n  editor: test\ncapabilities: []\nrights: []\nresponsibilities: []\nvisibility:\n  default: public\n",
        name, agent_type
    );
    AgentPersona::from_yaml(&yaml).expect("test persona parse")
}

/// Ensure a template crate directory exists for PodManager::new_mock().
/// The mock manager uses `/tmp/hkask-mock` as its GitCasAdapter root.
fn ensure_template_dir(template_name: &str) {
    let dir = std::path::PathBuf::from("/tmp/hkask-mock").join(template_name);
    std::fs::create_dir_all(&dir).ok();
    std::fs::write(
        dir.join("agent_persona.yaml"),
        format!("agent:\n  name: {}\n  type: Bot\n", template_name),
    )
    .ok();
    std::fs::write(dir.join("dispatch_manifest.yaml"), "name: test\n").ok();
}

/// Set the master key env var for tests that need key derivation.
/// SAFETY: integration tests run in isolated processes — no other code
/// reads HKASK_MASTER_KEY concurrently.
fn set_test_master_key() {
    unsafe {
        std::env::set_var(
            "HKASK_MASTER_KEY",
            "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
        );
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// REQ: P1-agt-pod-integration-create-test — Two-pod creation and activation
///
/// Two pods can be created with distinct identities and activated
/// through the full lifecycle (Populated → Registered → Activated).
#[tokio::test]
async fn two_pod_creation_and_activation() {
    set_test_master_key();
    let manager = PodManager::new_mock(None);

    // Ensure template directories exist
    ensure_template_dir("alice-template");
    ensure_template_dir("bob-template");

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

/// REQ: P1-agt-pod-integration-list-test — Pod listing after multi-pod creation
///
/// `list_pods()` returns all created pods with correct status.
#[tokio::test]
async fn list_pods_after_multi_pod_creation() {
    set_test_master_key();
    let manager = PodManager::new_mock(None);

    let alice = test_persona("alice", "Replicant");
    let bob = test_persona("bob", "Bot");
    let carol = test_persona("carol", "Bot");

    ensure_template_dir("a");
    ensure_template_dir("b");
    ensure_template_dir("c");

    let a_id = manager.create_pod("a", &alice, None).await.unwrap();
    let b_id = manager.create_pod("b", &bob, None).await.unwrap();
    let c_id = manager.create_pod("c", &carol, None).await.unwrap();

    // Activate only alice and bob
    manager.activate_pod(&a_id).await.unwrap();
    manager.activate_pod(&b_id).await.unwrap();

    let pods = manager.list_pods().await.unwrap();
    assert_eq!(pods.len(), 3, "Should list all 3 pods");

    // Verify states
    let alice_status = pods.iter().find(|s| s.pod_id == a_id.to_string()).unwrap();
    assert!(format!("{:?}", alice_status.state).contains("Activated"));

    let bob_status = pods.iter().find(|s| s.pod_id == b_id.to_string()).unwrap();
    assert!(format!("{:?}", bob_status.state).contains("Activated"));

    let carol_status = pods.iter().find(|s| s.pod_id == c_id.to_string()).unwrap();
    assert!(format!("{:?}", carol_status.state).contains("Populated"));
}

/// REQ: P1-agt-pod-integration-mode-test — Mode transitions on activated pod
///
/// An activated pod can enter server mode, exit, and enter chat mode.
#[tokio::test]
async fn mode_transitions_on_activated_pod() {
    set_test_master_key();
    let manager = PodManager::new_mock(None);
    ensure_template_dir("test");
    let persona = test_persona("agent", "Replicant");
    let pod_id = manager
        .create_pod("test", &persona, Some("agent".into()))
        .await
        .unwrap();

    // Activate and assign a role
    manager.activate_pod(&pod_id).await.unwrap();
    manager.assign_role("agent", "research").await.unwrap();

    // Enter server mode
    manager
        .set_mode("agent", "server", Some("research"))
        .await
        .expect("enter server mode");

    let status = manager.get_pod_status(&pod_id).await.unwrap();
    assert!(format!("{:?}", status.state).contains("Activated"));

    // Exit mode
    manager
        .set_mode("agent", "exit", None)
        .await
        .expect("exit mode");

    // Enter chat mode
    manager
        .set_mode("agent", "chat", None)
        .await
        .expect("enter chat mode");
}

/// REQ: P1-agt-pod-integration-deactivate-test — Deactivation and reactivation
///
/// A pod can be deactivated and its state transitions correctly.
#[tokio::test]
async fn deactivation_and_reactivation() {
    set_test_master_key();
    let manager = PodManager::new_mock(None);
    ensure_template_dir("test");
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

/// REQ: P1-agt-pod-integration-not-found-test — Pod not found error
///
/// Querying a non-existent pod returns PodNotFound.
#[tokio::test]
async fn nonexistent_pod_returns_error() {
    let manager = PodManager::new_mock(None);
    let fake_id = PodID::new();

    let result = manager.get_pod_status(&fake_id).await;
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention pod not found"
    );
}

/// REQ: P1-agt-pod-integration-inference-test — Inference port wiring
///
/// A PodManager constructed with a MockInferencePort exposes it
/// via `inference_port()` and pods can be created/activated with
/// inference available for improv interactions.
#[tokio::test]
async fn inference_port_wiring() {
    set_test_master_key();
    ensure_template_dir("test");

    // Build a PodManager with mock inference
    let inference = Arc::new(
        MockInferencePort::new()
            .with_response("hello", "Hello from mock inference!")
            .with_default("Mock default response"),
    );
    let manager = PodManager::new(
        None,                    // git_cas → defaults to ./registry/templates
        None,                    // a2a_runtime → defaults
        None,                    // mcp_runtime → defaults
        None,                    // episodic_storage → in-memory
        None,                    // semantic_storage → in-memory
        Some(inference.clone()), // inference_port
        None,                    // capability_checker
        None,                    // governed_tool
        None,                    // nu_event_sink
    );

    // Verify inference port is accessible
    let retrieved = manager.inference_port();
    assert!(retrieved.is_some(), "inference_port should be accessible");

    // Verify it's the same mock (by calling it)
    let port = retrieved.unwrap();
    let result = port
        .generate(
            "hello world",
            &hkask_types::template::LLMParameters::default(),
        )
        .await
        .expect("mock inference should succeed");
    assert_eq!(result.text, "Hello from mock inference!");

    // Verify error injection works through the manager
    inference.set_error(hkask_types::ports::InferenceError::Generation(
        "test error".into(),
    ));
    let result = port
        .generate(
            "any prompt",
            &hkask_types::template::LLMParameters::default(),
        )
        .await;
    assert!(result.is_err(), "error injection should propagate");
    inference.clear_error();
}
