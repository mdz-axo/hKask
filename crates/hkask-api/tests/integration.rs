//! Integration tests for hkask-api public route types.
//!
//! Tests the request/response types for all API endpoints:
//! serialization round-trips, validation, and structure.
//!
//! Full HTTP integration tests (spinning up the axum server) require
//! AgentService infrastructure (keystore, SQLite, CNS). Those are deferred
//! to the end-to-end test suite. See OPEN_QUESTIONS.md.

use hkask_api::routes::{
    A2ARegisterRequest, A2ARegisterResponse, ChatRequest, ChatResponse, CnsHealthResponse,
    CnsVarietyResponse, CreatePodRequest, CreatePodResponse, GrantCapabilityRequest,
    ListPodsResponse, ModelEntry, ModelListResponse, PodStatusInfoResponse, SovereigntyStatusResponse,
    SpecCoherenceResponse, SpecListResponse, SpecWritingQualityResponse, TemplateResponse,
    VarietyCounterResponse, WithdrawalFeeEstimateResponse,
};
use std::collections::HashMap;

// ── Chat Types ────────────────────────────────────────────────────────────

#[test]
fn chat_request_serialization_round_trip() {
    let req = ChatRequest {
        input: "Hello, Curator!".to_string(),
        template_id: Some("greeting".to_string()),
        model: Some("qwen3:8b".to_string()),
    };
    let json = serde_json::to_string(&req).expect("ChatRequest should serialize");
    let parsed: ChatRequest = serde_json::from_str(&json).expect("ChatRequest should deserialize");
    assert_eq!(parsed.input, "Hello, Curator!");
    assert_eq!(parsed.template_id, Some("greeting".to_string()));
    assert_eq!(parsed.model, Some("qwen3:8b".to_string()));
}

#[test]
fn chat_request_minimal_fields() {
    let req = ChatRequest {
        input: "Hi".to_string(),
        template_id: None,
        model: None,
    };
    let json = serde_json::to_string(&req).expect("ChatRequest should serialize");
    let parsed: ChatRequest = serde_json::from_str(&json).expect("ChatRequest should deserialize");
    assert_eq!(parsed.input, "Hi");
    assert!(parsed.template_id.is_none());
    assert!(parsed.model.is_none());
}

#[test]
fn chat_response_serialization_round_trip() {
    let resp = ChatResponse {
        output: "Hello! How can I help?".to_string(),
        template_id: "greeting".to_string(),
        model: "qwen3:8b".to_string(),
    };
    let json = serde_json::to_string(&resp).expect("ChatResponse should serialize");
    let parsed: ChatResponse =
        serde_json::from_str(&json).expect("ChatResponse should deserialize");
    assert_eq!(parsed.output, "Hello! How can I help?");
    assert_eq!(parsed.template_id, "greeting");
    assert_eq!(parsed.model, "qwen3:8b");
}

// ── Pod Types ─────────────────────────────────────────────────────────────

#[test]
fn create_pod_request_serialization_round_trip() {
    let req = CreatePodRequest {
        template: "curator".to_string(),
        persona_yaml: "name: test-bot\ntype: bot".to_string(),
        name: Some("my-bot".to_string()),
    };
    let json = serde_json::to_string(&req).expect("CreatePodRequest should serialize");
    let parsed: CreatePodRequest =
        serde_json::from_str(&json).expect("CreatePodRequest should deserialize");
    assert_eq!(parsed.template, "curator");
    assert_eq!(parsed.persona_yaml, "name: test-bot\ntype: bot");
    assert_eq!(parsed.name, Some("my-bot".to_string()));
}

#[test]
fn create_pod_response_carries_pod_id() {
    let resp = CreatePodResponse {
        pod_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    };
    let json = serde_json::to_string(&resp).expect("CreatePodResponse should serialize");
    let parsed: CreatePodResponse =
        serde_json::from_str(&json).expect("CreatePodResponse should deserialize");
    assert_eq!(parsed.pod_id, "550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn pod_status_response_carries_all_fields() {
    let status = PodStatusInfoResponse {
        pod_id: "p1".to_string(),
        name: Some("curator".to_string()),
        state: "Activated".to_string(),
        webid: "webid-123".to_string(),
        agent_type: "Bot".to_string(),
        template: "curator".to_string(),
        created_at: 1700000000,
    };
    let json = serde_json::to_string(&status).expect("PodStatusInfoResponse should serialize");
    let parsed: PodStatusInfoResponse =
        serde_json::from_str(&json).expect("PodStatusInfoResponse should deserialize");
    assert_eq!(parsed.pod_id, "p1");
    assert_eq!(parsed.state, "Activated");
    assert_eq!(parsed.agent_type, "Bot");
}

#[test]
fn list_pods_response_wraps_pod_statuses() {
    let resp = ListPodsResponse {
        pods: vec![
            PodStatusInfoResponse {
                pod_id: "p1".to_string(),
                name: Some("bot-a".to_string()),
                state: "Activated".to_string(),
                webid: "w1".to_string(),
                agent_type: "Bot".to_string(),
                template: "t1".to_string(),
                created_at: 1000,
            },
            PodStatusInfoResponse {
                pod_id: "p2".to_string(),
                name: None,
                state: "Populated".to_string(),
                webid: "w2".to_string(),
                agent_type: "Replicant".to_string(),
                template: "t2".to_string(),
                created_at: 2000,
            },
        ],
    };
    let json = serde_json::to_string(&resp).expect("ListPodsResponse should serialize");
    let parsed: ListPodsResponse =
        serde_json::from_str(&json).expect("ListPodsResponse should deserialize");
    assert_eq!(parsed.pods.len(), 2);
    assert_eq!(parsed.pods[0].pod_id, "p1");
    assert_eq!(parsed.pods[1].pod_id, "p2");
}

// ── Model Types ───────────────────────────────────────────────────────────

#[test]
fn model_entry_serialization_round_trip() {
    let entry = ModelEntry {
        name: "qwen3:8b".to_string(),
        family: Some("qwen2".to_string()),
        parameter_size: Some("8B".to_string()),
        quantization_level: Some("Q4_0".to_string()),
        size_gb: Some(4.5),
    };
    let json = serde_json::to_string(&entry).expect("ModelEntry should serialize");
    let parsed: ModelEntry = serde_json::from_str(&json).expect("ModelEntry should deserialize");
    assert_eq!(parsed.name, "qwen3:8b");
    assert_eq!(parsed.family, Some("qwen2".to_string()));
    assert_eq!(parsed.parameter_size, Some("8B".to_string()));
}

#[test]
fn model_list_response_carries_count() {
    let resp = ModelListResponse {
        models: vec![ModelEntry {
            name: "qwen3:8b".to_string(),
            family: None,
            parameter_size: None,
            quantization_level: None,
            size_gb: None,
        }],
        count: 1,
    };
    let json = serde_json::to_string(&resp).expect("ModelListResponse should serialize");
    let parsed: ModelListResponse =
        serde_json::from_str(&json).expect("ModelListResponse should deserialize");
    assert_eq!(parsed.count, 1);
    assert_eq!(parsed.models.len(), 1);
}

#[test]
fn model_list_response_empty_list() {
    let resp = ModelListResponse {
        models: vec![],
        count: 0,
    };
    let json = serde_json::to_string(&resp).expect("ModelListResponse should serialize");
    let parsed: ModelListResponse =
        serde_json::from_str(&json).expect("ModelListResponse should deserialize");
    assert_eq!(parsed.count, 0);
    assert!(parsed.models.is_empty());
}

// ── CNS Types ─────────────────────────────────────────────────────────────

#[test]
fn cns_health_response_serialization() {
    let resp = CnsHealthResponse {
        overall_deficit: 53,
        critical_count: 2,
        warning_count: 5,
        healthy: true,
    };
    let json = serde_json::to_string(&resp).expect("CnsHealthResponse should serialize");
    let parsed: CnsHealthResponse =
        serde_json::from_str(&json).expect("CnsHealthResponse should deserialize");
    assert_eq!(parsed.overall_deficit, 53);
    assert_eq!(parsed.critical_count, 2);
    assert!(parsed.healthy);
}

#[test]
fn cns_variety_response_serialization() {
    let mut counters = HashMap::new();
    counters.insert(
        "cns.tool".to_string(),
        VarietyCounterResponse {
            variety: 47,
            total: 200,
            entropy: 0.85,
        },
    );
    let resp = CnsVarietyResponse {
        domains: vec!["cns.tool".to_string()],
        total_deficit: 53,
        counters,
    };
    let json = serde_json::to_string(&resp).expect("CnsVarietyResponse should serialize");
    let parsed: CnsVarietyResponse =
        serde_json::from_str(&json).expect("CnsVarietyResponse should deserialize");
    assert_eq!(parsed.domains.len(), 1);
    assert_eq!(parsed.total_deficit, 53);
    assert!(parsed.counters.contains_key("cns.tool"));
}

// ── Variety Counter Types ─────────────────────────────────────────────────

#[test]
fn variety_counter_response_serialization() {
    let resp = VarietyCounterResponse {
        variety: 47,
        total: 200,
        entropy: 0.85,
    };
    let json = serde_json::to_string(&resp).expect("VarietyCounterResponse should serialize");
    let parsed: VarietyCounterResponse =
        serde_json::from_str(&json).expect("VarietyCounterResponse should deserialize");
    assert_eq!(parsed.variety, 47);
    assert_eq!(parsed.total, 200);
    assert!((parsed.entropy - 0.85).abs() < 0.001);
}

// ── Sovereignty Types ─────────────────────────────────────────────────────

#[test]
fn sovereignty_status_response_serialization() {
    let resp = SovereigntyStatusResponse {
        explicit_consent: true,
        requires_affirmative_consent: "required".to_string(),
        sovereign_data: vec!["episodic_memory".to_string()],
        shared_data: vec![],
        public_data: vec!["model_list".to_string()],
        granted_categories: vec!["memory".to_string()],
    };
    let json = serde_json::to_string(&resp).expect("SovereigntyStatusResponse should serialize");
    let parsed: SovereigntyStatusResponse =
        serde_json::from_str(&json).expect("SovereigntyStatusResponse should deserialize");
    assert!(parsed.explicit_consent);
    assert_eq!(parsed.requires_affirmative_consent, "required");
    assert_eq!(parsed.granted_categories.len(), 1);
}

// ── A2A Types ─────────────────────────────────────────────────────────────

#[test]
fn a2a_register_request_serialization() {
    let req = A2ARegisterRequest {
        webid: "agent-webid-123".to_string(),
        agent_type: "Bot".to_string(),
        capabilities: vec!["tool:execute".to_string(), "memory:read".to_string()],
    };
    let json = serde_json::to_string(&req).expect("A2ARegisterRequest should serialize");
    let parsed: A2ARegisterRequest =
        serde_json::from_str(&json).expect("A2ARegisterRequest should deserialize");
    assert_eq!(parsed.webid, "agent-webid-123");
    assert_eq!(parsed.agent_type, "Bot");
    assert_eq!(parsed.capabilities.len(), 2);
}

#[test]
fn a2a_register_response_serialization() {
    let resp = A2ARegisterResponse {
        token: "base64-encoded-token-data".to_string(),
        registered_at: 1700000000,
        webid: "agent-webid-123".to_string(),
    };
    let json = serde_json::to_string(&resp).expect("A2ARegisterResponse should serialize");
    let parsed: A2ARegisterResponse =
        serde_json::from_str(&json).expect("A2ARegisterResponse should deserialize");
    assert_eq!(parsed.token, "base64-encoded-token-data");
    assert_eq!(parsed.registered_at, 1700000000);
    assert_eq!(parsed.webid, "agent-webid-123");
}

// ── Wallet Types ──────────────────────────────────────────────────────────

#[test]
fn withdrawal_fee_estimate_response_serialization() {
    let resp = WithdrawalFeeEstimateResponse {
        chain: "hinkal".to_string(),
        rjoules: 7,
        native_units: 0.000010,
        usdc_equivalent: 0.0015,
    };
    let json =
        serde_json::to_string(&resp).expect("WithdrawalFeeEstimateResponse should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("WithdrawalFeeEstimateResponse JSON should parse");
    assert_eq!(parsed.get("chain").and_then(|v| v.as_str()), Some("hinkal"));
    assert_eq!(parsed.get("rjoules").and_then(|v| v.as_u64()), Some(7));
    assert!(
        parsed
            .get("native_units")
            .and_then(|v| v.as_f64())
            .is_some()
    );
    assert!(
        parsed
            .get("usdc_equivalent")
            .and_then(|v| v.as_f64())
            .is_some()
    );
}

// ── Spec Types ────────────────────────────────────────────────────────────

#[test]
fn spec_list_response_serialization() {
    let resp = SpecListResponse {
        spec_id: "spec-1".to_string(),
        name: "MDS Specification".to_string(),
        category: "core".to_string(),
        complete: true,
    };
    let json = serde_json::to_string(&resp).expect("SpecListResponse should serialize");
    let parsed: SpecListResponse =
        serde_json::from_str(&json).expect("SpecListResponse should deserialize");
    assert_eq!(parsed.spec_id, "spec-1");
    assert_eq!(parsed.name, "MDS Specification");
    assert!(parsed.complete);
}

#[test]
fn spec_coherence_response_serialization() {
    let resp = SpecCoherenceResponse {
        coherence_score: 0.95,
        violations: vec!["missing REQ tag".to_string()],
        suggestions: vec!["add REQ tag to handler".to_string()],
    };
    let json = serde_json::to_string(&resp).expect("SpecCoherenceResponse should serialize");
    let parsed: SpecCoherenceResponse =
        serde_json::from_str(&json).expect("SpecCoherenceResponse should deserialize");
    assert!((parsed.coherence_score - 0.95).abs() < 0.001);
    assert_eq!(parsed.violations.len(), 1);
    assert_eq!(parsed.suggestions.len(), 1);
}

#[test]
fn spec_writing_quality_response_serialization() {
    let resp = SpecWritingQualityResponse {
        dimensions_passing: 7,
        meets_publication_standard: true,
    };
    let json = serde_json::to_string(&resp).expect("SpecWritingQualityResponse should serialize");
    let parsed: SpecWritingQualityResponse =
        serde_json::from_str(&json).expect("SpecWritingQualityResponse should deserialize");
    assert_eq!(parsed.dimensions_passing, 7);
    assert!(parsed.meets_publication_standard);
}

// ── Template Types ────────────────────────────────────────────────────────

#[test]
fn template_response_serialization() {
    let resp = TemplateResponse {
        id: "tpl-greeting".to_string(),
        template_type: "Prompt".to_string(),
        name: "Greeting".to_string(),
        description: "A friendly greeting template".to_string(),
        source_path: "/templates/greeting.yaml".to_string(),
        lexicon_terms: vec!["greet".to_string(), "hello".to_string()],
    };
    let json = serde_json::to_string(&resp).expect("TemplateResponse should serialize");
    let parsed: TemplateResponse =
        serde_json::from_str(&json).expect("TemplateResponse should deserialize");
    assert_eq!(parsed.id, "tpl-greeting");
    assert_eq!(parsed.name, "Greeting");
    assert_eq!(parsed.template_type, "Prompt");
    assert_eq!(parsed.lexicon_terms.len(), 2);
}

// ── Capability Types ──────────────────────────────────────────────────────

#[test]
fn grant_capability_request_serialization() {
    let req = GrantCapabilityRequest {
        capability: "tool:execute".to_string(),
    };
    let json = serde_json::to_string(&req).expect("GrantCapabilityRequest should serialize");
    let parsed: GrantCapabilityRequest =
        serde_json::from_str(&json).expect("GrantCapabilityRequest should deserialize");
    assert_eq!(parsed.capability, "tool:execute");
}
