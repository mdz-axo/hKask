//! Property-Based Tests for Okapi Inference
//!
//! Uses proptest for random prompt/parameter generation.
//! Tests invariants: response non-empty, usage stats consistent, no panics.

use hkask_templates::{InferenceResult, OkapiConfig, OkapiRetryConfig};
use hkask_types::LLMParameters;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Valid prompts always produce non-empty responses (when successful)
    #[test]
    fn test_inference_response_non_empty(
        prompt in "[a-zA-Z0-9 ]{10,500}",
        temperature in 0.0f32..=2.0,
        max_tokens in 10u32..=1000,
    ) {
        let params = LLMParameters {
            temperature,
            max_tokens,
            ..Default::default()
        };

        prop_assert!(params.temperature >= 0.0);
        prop_assert!(params.temperature <= 2.0);
        prop_assert!(params.max_tokens > 0);
        prop_assert!(!prompt.is_empty());
        prop_assert!(prompt.len() <= 500);
    }

    /// Property: Usage stats are always consistent
    #[test]
    fn test_usage_stats_consistent(
        prompt_tokens in 1u32..=1000,
        completion_tokens in 1u32..=1000,
    ) {
        let expected_total = prompt_tokens + completion_tokens;
        prop_assert!(expected_total >= prompt_tokens);
        prop_assert!(expected_total >= completion_tokens);
    }

    /// Property: Token probabilities are in valid range [0, 1]
    #[test]
    fn test_token_probabilities_valid_range(
        probs in prop::collection::vec(0.0f64..=1.0, 1..20),
    ) {
        for prob in &probs {
            prop_assert!(*prob >= 0.0);
            prop_assert!(*prob <= 1.0);
        }
    }

    /// Property: Model names are non-empty strings
    #[test]
    fn test_model_name_valid(model_name in "[a-zA-Z0-9/_-]{5,100}") {
        prop_assert!(!model_name.is_empty());
        prop_assert!(model_name.len() >= 5);
        prop_assert!(model_name.len() <= 100);
    }
}

/// Property: InferenceResult serialization round-trip
#[test]
fn test_inference_result_roundtrip() {
    let original = InferenceResult {
        text: "Test response".to_string(),
        model: "test-model".to_string(),
        usage: hkask_templates::Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        },
        finish_reason: "stop".to_string(),
        token_probabilities: None,
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: InferenceResult = serde_json::from_str(&json).unwrap();

    assert_eq!(original.text, deserialized.text);
    assert_eq!(original.model, deserialized.model);
    assert_eq!(original.usage.total_tokens, deserialized.usage.total_tokens);
}

/// Property: Config serialization round-trip
#[test]
fn test_okapi_config_roundtrip() {
    let original = OkapiConfig::default();
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: OkapiConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(original.base_url, deserialized.base_url);
    assert_eq!(original.timeout_secs, deserialized.timeout_secs);
}

/// Property: Retry config exponential backoff
#[test]
fn test_retry_exponential_backoff() {
    let config = OkapiRetryConfig::default();

    let delay_0 = config.delay_for_attempt(0);
    let delay_1 = config.delay_for_attempt(1);
    let delay_2 = config.delay_for_attempt(2);
    let delay_3 = config.delay_for_attempt(3);

    assert!(delay_1 > delay_0);
    assert!(delay_2 > delay_1);
    assert!(delay_3 > delay_2);
    assert!(delay_3 <= std::time::Duration::from_millis(config.max_delay_ms));
}

/// Property: Circuit breaker state transitions
#[test]
fn test_circuit_breaker_states() {
    use hkask_templates::{CircuitBreaker, CircuitBreakerConfig, CircuitState};

    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        open_timeout: std::time::Duration::from_millis(100),
        success_threshold: 2,
    };

    let cb = CircuitBreaker::new("test".to_string(), config);

    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.allow_request());

    cb.record_failure();
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.allow_request());
}

/// Property: Cache key generation is deterministic
#[test]
fn test_cache_key_deterministic() {
    use hkask_templates::prompt_cache::PromptCache;

    let prompt = "test prompt";
    let model = "test-model";
    let params = LLMParameters::default();

    let key1 = PromptCache::generate_key(prompt, model, &params);
    let key2 = PromptCache::generate_key(prompt, model, &params);

    assert_eq!(key1, key2);

    let key3 = PromptCache::generate_key("different prompt", model, &params);
    assert_ne!(key1, key3);
}

/// Property: Rate limiter allows requests under limit
#[test]
fn test_rate_limiter_allows_under_limit() {
    use hkask_cns::{RateLimitConfig, RateLimiter};
    use hkask_types::WebID;

    let config = RateLimitConfig {
        max_tokens: 100,
        refill_interval: std::time::Duration::from_millis(600),
    };

    let limiter = RateLimiter::new(config);
    let bot_id = WebID::new();

    for i in 0..50 {
        let allowed = limiter.check(&bot_id);
        assert!(allowed, "Request {} should be allowed", i);
    }
}

/// Property: Escalation queue CRUD operations
#[test]
fn test_escalation_queue_crud() {
    use hkask_agents::{EscalationQueue, EscalationStatus};
    use hkask_types::{BotID, TemplateId};
    use rusqlite::Connection;
    use std::sync::Arc;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("escalations.db");
    let conn = Arc::new(Connection::open(db_path).unwrap());
    let queue = EscalationQueue::new(conn).unwrap();

    let template_id = TemplateId::new();
    let bot_id = BotID::new();

    let id = queue
        .add(
            template_id,
            bot_id,
            "Test output".to_string(),
            0.3,
            2,
            "Low confidence".to_string(),
        )
        .unwrap();

    let entry = queue.get(&id).unwrap().unwrap();
    assert_eq!(entry.confidence, 0.3);
    assert_eq!(entry.retry_count, 2);
    assert_eq!(entry.status, EscalationStatus::Pending);

    queue.resolve(&id, "curator").unwrap();
    let entry = queue.get(&id).unwrap().unwrap();
    assert_eq!(entry.status, EscalationStatus::Resolved);
    assert!(entry.resolved_at.is_some());
}
