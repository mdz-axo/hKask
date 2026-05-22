//! OCAP Unit Tests
//!
//! Tests for hkask-agents capability operations

use hkask_agents::ocap::OCAP;
use hkask_agents::security::ExpiryEnforcer;
use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, WebID};
use std::time::Duration;

#[tokio::test]
async fn test_ocap_attenuation_history() {
    let ocap = OCAP::new();
    let webid1 = WebID::new();
    let webid2 = WebID::new();
    let webid3 = WebID::new();

    ocap.record_attenuation("root-nonce", &webid1, &webid2, 1000, 0)
        .await;
    ocap.record_attenuation("root-nonce", &webid2, &webid3, 1001, 1)
        .await;

    let history = ocap.get_attenuation_history("root-nonce").await;
    assert!(history.is_some());
    assert_eq!(history.unwrap().chain_length(), 2);
}

#[tokio::test]
async fn test_ocap_verify_attenuation() {
    let ocap = OCAP::new();
    let webid1 = WebID::new();
    let webid2 = WebID::new();

    ocap.record_attenuation("root-nonce", &webid1, &webid2, 1000, 0)
        .await;

    let token = CapabilityToken::new(
        CapabilityResource::Tool,
        "test-tool".to_string(),
        CapabilityAction::Execute,
        webid1,
        webid2,
        b"test-secret",
    );

    // Token should verify (internal chain check)
    assert!(ocap.verify_attenuation(&token).await);
}

#[test]
fn test_ocap_expiry_enforcement() {
    let ocap = OCAP::new();
    let creation_time = 1000;
    let expiry = ocap.calculate_expiry(creation_time);

    assert_eq!(expiry, creation_time + 3600); // Default 1 hour

    // Create token with expiry set
    let mut token_valid = CapabilityToken::new(
        CapabilityResource::Tool,
        "test".to_string(),
        CapabilityAction::Execute,
        WebID::new(),
        WebID::new(),
        b"secret",
    );
    token_valid.expires_at = Some(expiry - 1);
    assert!(ocap.validate_expiry(&token_valid, expiry - 1));

    // Create token with expiry set
    let mut token_expired = CapabilityToken::new(
        CapabilityResource::Tool,
        "test".to_string(),
        CapabilityAction::Execute,
        WebID::new(),
        WebID::new(),
        b"secret",
    );
    token_expired.expires_at = Some(expiry);
    assert!(!ocap.validate_expiry(&token_expired, expiry + 1));
}

#[tokio::test]
async fn test_ocp_attenuate_with_history() {
    let ocap = OCAP::new();
    let webid1 = WebID::new();
    let webid2 = WebID::new();
    let secret = b"test-secret";
    let current_time = 1000;

    let parent = CapabilityToken::new(
        CapabilityResource::Tool,
        "test-tool".to_string(),
        CapabilityAction::Execute,
        webid1,
        webid1,
        secret,
    );

    let child = ocap
        .attenuate_with_history(&parent, webid2, secret, current_time)
        .await;
    assert!(child.is_some());

    let child_token = child.unwrap();
    assert_eq!(child_token.attenuation_level, 1);
    assert!(child_token.expires_at.is_some());

    // Verify history was recorded
    let history = ocap
        .get_attenuation_history(parent.root_context_nonce())
        .await;
    assert!(history.is_some());
    assert_eq!(history.unwrap().chain_length(), 1);
}

#[test]
fn test_ocap_custom_expiry() {
    let enforcer = ExpiryEnforcer::new(Duration::from_secs(7200)); // 2 hours
    let ocap = OCAP::with_expiry(enforcer);

    let creation_time = 1000;
    let expiry = ocap.calculate_expiry(creation_time);

    assert_eq!(expiry, creation_time + 7200);
    assert_eq!(ocap.max_lifetime_secs(), 7200);
}
