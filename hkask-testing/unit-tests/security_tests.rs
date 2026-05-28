//! Security Unit Tests
//!
//! Tests for hkask-agents security operations

use hkask_agents::security::{
    AgentPersonaInput, AttenuationHistory, ExpiryEnforcer, RateLimiter, ValidationError,
};
use std::time::Duration;

#[test]
fn test_validate_valid_persona() {
    let input = AgentPersonaInput {
        name: "test-bot".to_string(),
        agent_type: "bot".to_string(),
        version: "0.1.0".to_string(),
        description: "A test bot".to_string(),
        editor: "test-editor".to_string(),
        capabilities: vec!["tool:inference".to_string()],
    };

    assert!(input.validate().is_ok());
}

#[test]
fn test_validate_empty_name() {
    let input = AgentPersonaInput {
        name: "".to_string(),
        agent_type: "bot".to_string(),
        version: "0.1.0".to_string(),
        description: "".to_string(),
        editor: "editor".to_string(),
        capabilities: vec![],
    };

    assert!(matches!(
        input.validate(),
        Err(ValidationError::MissingField(_))
    ));
}

#[test]
fn test_validate_invalid_agent_type() {
    let input = AgentPersonaInput {
        name: "test-bot".to_string(),
        agent_type: "invalid".to_string(),
        version: "0.1.0".to_string(),
        description: "".to_string(),
        editor: "editor".to_string(),
        capabilities: vec![],
    };

    assert!(matches!(
        input.validate(),
        Err(ValidationError::InvalidFormat { .. })
    ));
}

#[test]
fn test_validate_name_too_long() {
    let input = AgentPersonaInput {
        name: "a".repeat(65),
        agent_type: "bot".to_string(),
        version: "0.1.0".to_string(),
        description: "".to_string(),
        editor: "editor".to_string(),
        capabilities: vec![],
    };

    assert!(matches!(
        input.validate(),
        Err(ValidationError::FieldTooLong { .. })
    ));
}

#[tokio::test]
async fn test_rate_limiter_basic() {
    let limiter = RateLimiter::new(5.0, 1.0);

    // Should allow 5 requests
    for _ in 0..5 {
        assert!(limiter.acquire("test-key", 1.0).await.is_ok());
    }

    // 6th request should fail
    assert!(matches!(
        limiter.acquire("test-key", 1.0).await,
        Err(ValidationError::RateLimitExceeded)
    ));
}

#[tokio::test]
async fn test_rate_limiter_refill() {
    let limiter = RateLimiter::new(2.0, 10.0); // Fast refill for testing

    // Consume all tokens
    limiter.acquire("test-key", 2.0).await.unwrap();

    // Wait for refill
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Should have tokens again
    assert!(limiter.acquire("test-key", 1.0).await.is_ok());
}

#[test]
fn test_expiry_enforcer() {
    let enforcer = ExpiryEnforcer::new(Duration::from_secs(3600));
    let creation_time = 1000;
    let expiry = enforcer.calculate_expiry(creation_time);

    assert_eq!(expiry, 4600);
    assert!(!enforcer.is_expired(expiry, 4599));
    assert!(enforcer.is_expired(expiry, 4601));
}

#[test]
fn test_attenuation_history() {
    let mut history = AttenuationHistory::new("root-nonce".to_string());
    history.add_entry("webid1".to_string(), "webid2".to_string(), 1000, 0);
    history.add_entry("webid2".to_string(), "webid3".to_string(), 1001, 1);
    history.add_entry("webid3".to_string(), "webid4".to_string(), 1002, 2);

    assert!(history.verify_chain());
    assert_eq!(history.chain_length(), 3);
}

#[test]
fn test_attenuation_history_invalid_chain() {
    let mut history = AttenuationHistory::new("root-nonce".to_string());
    history.add_entry("webid1".to_string(), "webid2".to_string(), 1000, 0);
    history.add_entry("webid2".to_string(), "webid3".to_string(), 1001, 2); // Skips level 1

    assert!(!history.verify_chain());
}
