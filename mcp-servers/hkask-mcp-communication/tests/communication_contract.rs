//! Contract tests for hkask-mcp-communication — agent registry invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `AgentRegistry` (in-memory HashMap, no external dependencies).

use hkask_communication::matrix::UserId;
use hkask_mcp_communication::agent_registration::AgentRegistry;

// ── Registration contract tests ─────────────────────────────────────────────

// [P1] Goal: User Sovereignty — agents communicate through user-owned channels
// [P12] Constraining: both WebID and UserId carry authenticated identity
#[tokio::test]
async fn record_mapping_stores_webid_to_userid() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let user_id = UserId::new("@alice:localhost");

    registry.record_mapping(&webid, &user_id).await;

    let found = registry.resolve(&webid).await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().as_str(), "@alice:localhost");
}

#[tokio::test]
async fn record_mapping_is_idempotent() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let first = UserId::new("@alice:localhost");
    let second = UserId::new("@alice-v2:localhost");

    registry.record_mapping(&webid, &first).await;
    registry.record_mapping(&webid, &second).await;

    let found = registry.resolve(&webid).await;
    assert_eq!(found.unwrap().as_str(), "@alice-v2:localhost");
}

#[tokio::test]
async fn deregister_removes_mapping() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let user_id = UserId::new("@bob:localhost");

    registry.record_mapping(&webid, &user_id).await;
    registry
        .deregister(&webid)
        .await
        .expect("deregister should succeed");

    let found = registry.resolve(&webid).await;
    assert!(found.is_none());
}

#[tokio::test]
async fn deregister_nonexistent_is_ok() {
    let registry = AgentRegistry::new();
    let unknown = hkask_types::WebID::new();

    let result = registry.deregister(&unknown).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn lookup_unregistered_returns_none() {
    let registry = AgentRegistry::new();
    let unknown = hkask_types::WebID::new();

    let found = registry.resolve(&unknown).await;
    assert!(found.is_none());
}
