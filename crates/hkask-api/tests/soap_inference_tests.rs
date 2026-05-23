//! SOAP Inference Endpoint Integration Tests

use hkask_api::{
    EventRecord, ObjectiveData, SeverityCounts, SoapInferAuthRequest, SoapInferRequest,
    SoapInferenceConfig,
};

/// Test valid SOAP request structure
#[test]
fn test_soap_infer_valid_request_structure() {
    let req = SoapInferAuthRequest {
        request: SoapInferRequest {
            subjective: Some("Machine feels sluggish".to_string()),
            objective: ObjectiveData {
                severity_counts: SeverityCounts {
                    crit: 0,
                    alert: 1,
                    warn: 2,
                    info: 5,
                },
                recent_events: vec![EventRecord {
                    probe: "host/mem_available_mib".to_string(),
                    severity: "Alert".to_string(),
                    message: "Low memory".to_string(),
                    ts: "2026-05-22T12:00:00Z".to_string(),
                }],
            },
            assessment: String::new(),
            plan: String::new(),
        },
        capability_token: "test-token".to_string(),
    };

    assert_eq!(req.request.objective.recent_events.len(), 1);
    assert_eq!(req.request.objective.severity_counts.crit, 0);
}

/// Test ACTION extraction from response
#[test]
fn test_extract_actions() {
    let response = r#"
**Assessment:** Memory pressure detected.

**Plan:**
1. Monitor memory usage
ACTION: okapi-watcher/restart-okapi
3. Check disk space
ACTION: sysadmin/clear-disk-space
"#;

    let actions: Vec<&str> = response
        .lines()
        .filter_map(|line| line.trim().strip_prefix("ACTION:"))
        .map(|a| a.trim())
        .collect();

    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0], "okapi-watcher/restart-okapi");
    assert_eq!(actions[1], "sysadmin/clear-disk-space");
}

/// Test ACTION extraction with no actions
#[test]
fn test_extract_actions_empty() {
    let response = r#"
**Assessment:** System is healthy.
**Plan:** Continue monitoring.
"#;

    let actions: Vec<&str> = response
        .lines()
        .filter_map(|line| line.trim().strip_prefix("ACTION:"))
        .map(|a| a.trim())
        .collect();

    assert_eq!(actions.len(), 0);
}

/// Test SOAP request with capability token
#[test]
fn test_soap_auth_request_has_token() {
    let req = SoapInferAuthRequest {
        request: SoapInferRequest {
            subjective: None,
            objective: ObjectiveData {
                severity_counts: SeverityCounts {
                    crit: 0,
                    alert: 0,
                    warn: 0,
                    info: 1,
                },
                recent_events: vec![],
            },
            assessment: String::new(),
            plan: String::new(),
        },
        capability_token: "macaroon-token-here".to_string(),
    };

    assert!(!req.capability_token.is_empty());
    assert!(req.capability_token.contains("macaroon"));
}

/// Test validation passes for valid request
#[test]
fn test_validation_passes_valid_request() {
    use hkask_api::routes::validate_soap_request;

    let config = SoapInferenceConfig::default();
    let req = SoapInferRequest {
        subjective: Some("Normal note".to_string()),
        objective: ObjectiveData {
            severity_counts: SeverityCounts {
                crit: 0,
                alert: 1,
                warn: 2,
                info: 5,
            },
            recent_events: vec![EventRecord {
                probe: "host/test".to_string(),
                severity: "Info".to_string(),
                message: "Test message".to_string(),
                ts: "2026-05-22T12:00:00Z".to_string(),
            }],
        },
        assessment: String::new(),
        plan: String::new(),
    };

    let result = validate_soap_request(&req, &config);
    assert!(result.is_ok());
}

/// Test validation fails for too many events
#[test]
fn test_validation_fails_too_many_events() {
    use hkask_api::routes::validate_soap_request;

    let config = SoapInferenceConfig::default();
    let events = (0..config.max_events + 1)
        .map(|i| EventRecord {
            probe: format!("probe_{}", i),
            severity: "Info".to_string(),
            message: "Test".to_string(),
            ts: "2026-05-22T12:00:00Z".to_string(),
        })
        .collect();

    let req = SoapInferRequest {
        subjective: None,
        objective: ObjectiveData {
            severity_counts: SeverityCounts::default(),
            recent_events: events,
        },
        assessment: String::new(),
        plan: String::new(),
    };

    let result = validate_soap_request(&req, &config);
    assert!(result.is_err());
}

/// Test validation fails for subjective too long
#[test]
fn test_validation_fails_subjective_too_long() {
    use hkask_api::routes::validate_soap_request;

    let config = SoapInferenceConfig::default();
    let long_text = "x".repeat(config.max_subjective_len + 1);

    let req = SoapInferRequest {
        subjective: Some(long_text),
        objective: ObjectiveData {
            severity_counts: SeverityCounts::default(),
            recent_events: vec![],
        },
        assessment: String::new(),
        plan: String::new(),
    };

    let result = validate_soap_request(&req, &config);
    assert!(result.is_err());
}

/// Test validation fails for message too long
#[test]
fn test_validation_fails_message_too_long() {
    use hkask_api::routes::validate_soap_request;

    let config = SoapInferenceConfig::default();
    let long_message = "x".repeat(config.max_message_len + 1);

    let req = SoapInferRequest {
        subjective: None,
        objective: ObjectiveData {
            severity_counts: SeverityCounts::default(),
            recent_events: vec![EventRecord {
                probe: "host/test".to_string(),
                severity: "Info".to_string(),
                message: long_message,
                ts: "2026-05-22T12:00:00Z".to_string(),
            }],
        },
        assessment: String::new(),
        plan: String::new(),
    };

    let result = validate_soap_request(&req, &config);
    assert!(result.is_err());
}

/// Test config defaults are reasonable
#[test]
fn test_config_defaults() {
    let config = SoapInferenceConfig::default();

    assert_eq!(config.max_events, 100);
    assert_eq!(config.max_subjective_len, 4096);
    assert_eq!(config.max_message_len, 1024);
    assert_eq!(config.timeout_secs, 30);
    assert_eq!(config.capability_secret.len(), 32);
    assert_eq!(config.model, "qwen3:8b");
    assert_eq!(config.temperature, 0.2);
    assert_eq!(config.max_tokens, 2048);
}

/// Test config from_env uses defaults when no env vars set
#[test]
fn test_config_from_env_defaults() {
    let config = SoapInferenceConfig::from_env();

    assert_eq!(config.max_events, 100);
    assert_eq!(config.model, "qwen3:8b");
}

/// Test CNS span type names
#[test]
fn test_inference_span_names() {
    use hkask_api::InferenceSpan;

    let start_span = InferenceSpan::Start {
        timestamp: "2026-05-22T12:00:00Z".to_string(),
        events_count: 5,
        severity_total: 8,
    };
    assert_eq!(start_span.span_name(), "cns.tool.inference.start");

    let error_span = InferenceSpan::InferenceError {
        error: "test".to_string(),
    };
    assert_eq!(error_span.span_name(), "cns.tool.inference.error");

    let timeout_span = InferenceSpan::Timeout { timeout_secs: 30 };
    assert_eq!(timeout_span.span_name(), "cns.tool.inference.timeout");
}

/// Test CNS span observations are valid JSON
#[test]
fn test_inference_span_observations() {
    use hkask_api::InferenceSpan;

    let outcome_span = InferenceSpan::Outcome {
        latency_ms: 1234,
        actions_count: 2,
        success: true,
    };
    let obs = outcome_span.observation();
    assert!(obs.get("latency_ms").is_some());
    assert!(obs.get("actions_count").is_some());
    assert!(obs.get("success").is_some());
}

/// Test capability token holder extraction (Miller authority)
#[test]
fn test_capability_token_holder() {
    use hkask_types::capability::CapabilityToken;
    use hkask_types::{CapabilityAction, CapabilityResource, WebID};

    let secret = b"test-secret-key";
    let issuer = WebID::new();
    let holder = WebID::new();

    let token = CapabilityToken::new(
        CapabilityResource::Tool,
        "soap_inference".to_string(),
        CapabilityAction::Execute,
        issuer,
        holder,
        secret,
    );

    // Test holder extraction
    assert_eq!(token.holder(), holder);
    assert_eq!(token.issuer(), issuer);

    // Test serialization/deserialization
    let encoded = token.to_base64().expect("should encode");
    let decoded = CapabilityToken::from_base64(&encoded).expect("should decode");

    assert_eq!(decoded.holder(), holder);
    assert_eq!(decoded.issuer(), issuer);
    assert!(decoded.verify(secret));
}
