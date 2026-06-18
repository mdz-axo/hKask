//! Contract tests for hkask-mcp-communication — agent registry invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `AgentRegistry` (in-memory HashMap, no external dependencies).

use hkask_mcp_communication::agent_registration::AgentRegistry;
use hkask_communication::matrix::UserId;
use std::str::FromStr;

// ── Registration contract tests ─────────────────────────────────────────────

// REQ: COMM-REG-001 — record_mapping stores a WebID → Matrix UserId mapping
// expect: "I can register agents with their Matrix identities for sovereign communication" [P1]
// [P1] Goal: User Sovereignty — agents communicate through user-owned channels
// [P12] Constraining: both WebID and UserId carry authenticated identity
#[tokio::test]
async fn record_mapping_stores_webid_to_userid() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let user_id = UserId::from_str("@alice:localhost").expect("valid user id");

    registry.record_mapping(&webid, &user_id).await;

    let found = registry.lookup(&webid).await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().as_str(), "@alice:localhost");
}

// REQ: COMM-REG-002 — record_mapping is idempotent
// expect: "I can re-register the same agent and the latest mapping is used" [P8]
#[tokio::test]
async fn record_mapping_is_idempotent() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let first = UserId::from_str("@alice:localhost").expect("valid");
    let second = UserId::from_str("@alice-v2:localhost").expect("valid");

    registry.record_mapping(&webid, &first).await;
    registry.record_mapping(&webid, &second).await;

    let found = registry.lookup(&webid).await;
    assert_eq!(found.unwrap().as_str(), "@alice-v2:localhost");
}

// REQ: COMM-REG-003 — deregister removes the mapping
// expect: "I can deregister an agent and their Matrix mapping is removed" [P1]
#[tokio::test]
async fn deregister_removes_mapping() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let user_id = UserId::from_str("@bob:localhost").expect("valid");

    registry.record_mapping(&webid, &user_id).await;
    registry.deregister(&webid).await.expect("deregister should succeed");

    let found = registry.lookup(&webid).await;
    assert!(found.is_none());
}

// REQ: COMM-REG-004 — deregister of nonexistent agent is Ok(())
// expect: "I can safely deregister an agent that is not registered" [P8]
#[tokio::test]
async fn deregister_nonexistent_is_ok() {
    let registry = AgentRegistry::new();
    let unknown = hkask_types::WebID::new();

    let result = registry.deregister(&unknown).await;
    assert!(result.is_ok());
}

// REQ: COMM-REG-005 — lookup of unregistered agent returns None
// expect: "I can look up an unregistered agent and get a clean empty result" [P8]
#[tokio::test]
async fn lookup_unregistered_returns_none() {
    let registry = AgentRegistry::new();
    let unknown = hkask_types::WebID::new();

    let found = registry.lookup(&unknown).await;
    assert!(found.is_none());
}
