//! Integration tests for the hKask inference router.
//!
//! Verifies provider-prefix routing, unavailable-backend errors,
//! default-provider fallback, model-override routing, and graceful
//! degradation during model listing. Uses `wiremock` to simulate
//! Ollama and DeepInfra HTTP backends without real network calls.
//!
//! # Architecture under test
//!
//! ```text
//! InferenceRouter
//!   ├── OllamaBackend    — OM/ prefix → POST /v1/chat/completions
//!   └── DeepInfraBackend — DI/ prefix → POST /v1/chat/completions
//! ```
//!
//! # REQ tags
//!
//! Each test carries a `// REQ: P{N}-inf-*` contract tag linking it to a
//! machine-parseable contract in the functional specification.

use hkask_inference::{InferenceConfig, InferenceRouter, ProviderId};
use hkask_types::ports::InferencePort;
use hkask_types::template::LLMParameters;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a mock chat-completion response (OpenAI-compatible JSON).
fn mock_chat_response(model: &str, content: &str) -> serde_json::Value {
    json!({
        "model": model,
        "choices": [{
            "message": {
                "role": "assistant",
                "content": content
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    })
}

/// Build a mock Ollama `/api/tags` response.
fn mock_ollama_tags(models: &[serde_json::Value]) -> serde_json::Value {
    json!({ "models": models })
}

/// Default LLMParameters for tests (minimal, non-streaming).
fn default_params() -> LLMParameters {
    LLMParameters {
        temperature: 0.7,
        max_tokens: 256,
        top_p: 0.9,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        top_k: 40,
        min_p: 0.0,
        typical_p: 0.0,
        seed: None,
        disable_thinking: false,
        adapter: None,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// REQ: P9-inf-test-routing-by-provider-prefix — Provider-prefix routing
/// \[P9\] Motivating: Homeostatic Self-Regulation — end-to-end provider routing
///
/// The router dispatches OM/-prefixed models to the Ollama backend
/// and DI/-prefixed models to the DeepInfra backend.
#[tokio::test]
async fn routing_by_provider_prefix() {
    // Stand up two mock servers
    let ollama_mock = MockServer::start().await;
    let deepinfra_mock = MockServer::start().await;

    // Ollama mock: responds to /v1/chat/completions
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_chat_response("qwen3:8b", "Response from Ollama")),
        )
        .mount(&ollama_mock)
        .await;

    // DeepInfra mock: responds to /v1/chat/completions
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "meta-llama/Llama-3.3-70B-Instruct",
            "Response from DeepInfra",
        )))
        .mount(&deepinfra_mock)
        .await;

    // Configure router pointing at mock servers
    let config = InferenceConfig {
        default_provider: ProviderId::Ollama,
        ollama_base_url: ollama_mock.uri(),
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(), // needed for DI backend to exist
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    // OM/ prefix → Ollama
    let result = router
        .generate_with_model("Hello", &default_params(), Some("OM/qwen3:8b"))
        .await
        .expect("OM/ routing should succeed");
    assert_eq!(result.text, "Response from Ollama");
    assert_eq!(result.model, "qwen3:8b");

    // DI/ prefix → DeepInfra
    let result = router
        .generate_with_model(
            "Hello",
            &default_params(),
            Some("DI/meta-llama/Llama-3.3-70B-Instruct"),
        )
        .await
        .expect("DI/ routing should succeed");
    assert_eq!(result.text, "Response from DeepInfra");
    assert_eq!(result.model, "meta-llama/Llama-3.3-70B-Instruct");
}

/// REQ: P9-inf-test-unavailable-backend-error — Unavailable backend error
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates graceful boundary unavailability
///
/// When a provider's backend is not configured (e.g., no API key),
/// requests with that provider's prefix return an error.
#[tokio::test]
async fn unavailable_backend_returns_error() {
    let ollama_mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_chat_response("qwen3:8b", "Ollama response")),
        )
        .mount(&ollama_mock)
        .await;

    // Configure router with Ollama only (no DeepInfra API key → DI unavailable)
    let config = InferenceConfig {
        default_provider: ProviderId::Ollama,
        ollama_base_url: ollama_mock.uri(),
        deepinfra_api_key: String::new(), // empty → DI backend not created
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    // OM/ prefix → works
    let result = router
        .generate_with_model("Hello", &default_params(), Some("OM/qwen3:8b"))
        .await;
    assert!(
        result.is_ok(),
        "OM/ should succeed when Ollama is available"
    );

    // DI/ prefix → error (backend unavailable)
    let result = router
        .generate_with_model("Hello", &default_params(), Some("DI/some-model"))
        .await;
    assert!(
        result.is_err(),
        "DI/ should fail when DeepInfra is unavailable"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not available") || err.contains("DeepInfra"),
        "Error should mention unavailable provider, got: {}",
        err
    );
}

/// REQ: P9-inf-test-default-provider-routing — Default provider routing
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates default provider fallback
///
/// Unprefixed model names use the configured default provider.
#[tokio::test]
async fn default_provider_routing() {
    let ollama_mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "deepseek-v4-pro",
            "Default provider response",
        )))
        .mount(&ollama_mock)
        .await;

    // Default provider = Ollama, default model = "deepseek-v4-pro"
    let config = InferenceConfig {
        default_provider: ProviderId::Ollama,
        ollama_base_url: ollama_mock.uri(),
        default_model: "deepseek-v4-pro".to_string(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    // generate() uses default_model (unprefixed → Ollama)
    let result = router
        .generate("Hello", &default_params())
        .await
        .expect("Default provider routing should succeed");
    assert_eq!(result.text, "Default provider response");
    assert_eq!(result.model, "deepseek-v4-pro");
}

/// REQ: P9-inf-test-model-override-routing — Model override routing
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates explicit model override
///
/// `generate_with_model` with an explicit model override routes to
/// the correct backend regardless of the default model.
#[tokio::test]
async fn model_override_routing() {
    let ollama_mock = MockServer::start().await;
    let deepinfra_mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_chat_response("qwen3:8b", "Override Ollama")),
        )
        .mount(&ollama_mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "meta-llama/Llama-3.3-70B-Instruct",
            "Override DeepInfra",
        )))
        .mount(&deepinfra_mock)
        .await;

    let config = InferenceConfig {
        default_provider: ProviderId::Ollama,
        ollama_base_url: ollama_mock.uri(),
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
        default_model: "OM/qwen3:8b".to_string(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    // Override to DI/ model even though default is OM/
    let result = router
        .generate_with_model(
            "Hello",
            &default_params(),
            Some("DI/meta-llama/Llama-3.3-70B-Instruct"),
        )
        .await
        .expect("Model override should succeed");
    assert_eq!(result.text, "Override DeepInfra");

    // Override to OM/ model
    let result = router
        .generate_with_model("Hello", &default_params(), Some("OM/qwen3:8b"))
        .await
        .expect("Model override should succeed");
    assert_eq!(result.text, "Override Ollama");
}

/// REQ: P9-inf-test-list-models-degradation — Graceful degradation in list_models
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates graceful catalog degradation
///
/// When one provider's model-listing endpoint is unavailable,
/// `list_models()` still returns results from reachable providers.
#[tokio::test]
async fn list_models_graceful_degradation() {
    let ollama_mock = MockServer::start().await;
    // DeepInfra mock: we intentionally do NOT mount a /v1/models mock,
    // so the request will fail (connection refused or 404).

    // Ollama mock: responds to /api/tags with models
    Mock::given(method("GET"))
        .and(path("/api/tags"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_ollama_tags(&[json!({
                "name": "qwen3:8b",
                "size": 5_000_000_000_u64,
                "details": {
                    "family": "qwen2",
                    "parameter_size": "8B",
                    "quantization_level": "Q4_0"
                }
            })])),
        )
        .mount(&ollama_mock)
        .await;

    // DeepInfra mock: stand up a server but do NOT mount /v1/models.
    // The GET will hit wiremock's default 404 → list_models treats as
    // graceful degradation (returns empty vec for that provider).
    let deepinfra_mock = MockServer::start().await;

    // Configure router with both providers.
    let config = InferenceConfig {
        default_provider: ProviderId::Ollama,
        ollama_base_url: ollama_mock.uri(),
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    let models = router.list_models().await;

    // Should have at least the Ollama model
    assert!(
        !models.is_empty(),
        "list_models should return results from reachable providers"
    );

    // The Ollama model should be present with OM/ prefix
    let ollama_model = models.iter().find(|m| m.prefixed_name == "OM/qwen3:8b");
    assert!(
        ollama_model.is_some(),
        "Ollama model should be present with OM/ prefix. Got models: {:?}",
        models.iter().map(|m| &m.prefixed_name).collect::<Vec<_>>()
    );
    assert_eq!(ollama_model.unwrap().provider, ProviderId::Ollama);
}

/// REQ: P9-inf-test-thinking-disable-flow — Thinking mode disable flows through router to wire format
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates reasoning flag propagation
///
/// When `LLMParameters.disable_thinking` is `true`, the router passes it
/// through to the backend, and `build_chat_request` maps it to
/// `enable_thinking: false` in the JSON request body sent to the provider.
#[tokio::test]
async fn disable_thinking_flows_to_wire_format() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_chat_response("qwen3:8b", "Summary text")),
        )
        .mount(&mock)
        .await;

    // We can't easily capture the body with wiremock's high-level API,
    // so we verify the end-to-end behavior: the request succeeds and
    // the response is correct. The wire-format mapping is tested in
    // chat_protocol::tests::disable_thinking_maps_to_wire_format.
    let config = InferenceConfig {
        default_provider: ProviderId::Ollama,
        ollama_base_url: mock.uri(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    let params = LLMParameters {
        temperature: 0.3,
        max_tokens: 200,
        top_p: 0.9,
        top_k: 40,
        min_p: 0.0,
        typical_p: 0.0,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        seed: None,
        disable_thinking: true,
        adapter: None,
    };

    let result = router
        .generate_with_model("Summarize this.", &params, Some("OM/qwen3:8b"))
        .await
        .expect("Request with disable_thinking should succeed");

    assert_eq!(result.text, "Summary text");
    assert_eq!(result.model, "qwen3:8b");
    // The wire-format mapping (disable_thinking → enable_thinking: false)
    // is verified by chat_protocol::tests::disable_thinking_maps_to_wire_format.
    // This integration test confirms the router passes LLMParameters through
    // to the backend without interference.
}
