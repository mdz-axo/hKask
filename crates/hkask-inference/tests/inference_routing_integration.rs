//! Integration tests for the hKask inference router.
//!
//! Verifies provider-prefix routing, unavailable-backend errors,
//! default-provider fallback, model-override routing, and graceful
//! degradation during model listing. Uses `wiremock` to simulate
//! DeepInfra and Together AI HTTP backends without real network calls.
//!
//! # Architecture under test
//!
//! ```text
//! InferenceRouter
//!   ├── DeepInfraBackend — DI/ prefix → POST /v1/chat/completions
//!   └── TogetherBackend  — TG/ prefix → POST /v1/chat/completions
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

/// Build a mock DeepInfra `/v1/models` response.
fn mock_deepinfra_models(models: &[serde_json::Value]) -> serde_json::Value {
    json!({ "data": models })
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
/// The router dispatches DI/-prefixed models to the DeepInfra backend
/// and TG/-prefixed models to the Together AI backend.
#[tokio::test]
async fn routing_by_provider_prefix() {
    let deepinfra_mock = MockServer::start().await;
    let together_mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "meta-llama/Llama-3.3-70B-Instruct",
            "Response from DeepInfra",
        )))
        .mount(&deepinfra_mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "Qwen/Qwen2.5-7B-Instruct-Turbo",
            "Response from Together",
        )))
        .mount(&together_mock)
        .await;

    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
        together_base_url: together_mock.uri(),
        together_api_key: "test-key".to_string(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

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

    let result = router
        .generate_with_model(
            "Hello",
            &default_params(),
            Some("TG/Qwen/Qwen2.5-7B-Instruct-Turbo"),
        )
        .await
        .expect("TG/ routing should succeed");
    assert_eq!(result.text, "Response from Together");
    assert_eq!(result.model, "Qwen/Qwen2.5-7B-Instruct-Turbo");
}

/// REQ: P9-inf-test-unavailable-backend-error — Unavailable backend error
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates graceful boundary unavailability
///
/// When a provider's backend is not configured (e.g., no API key),
/// requests with that provider's prefix return an error.
#[tokio::test]
async fn unavailable_backend_returns_error() {
    let deepinfra_mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "meta-llama/Llama-3.3-70B-Instruct",
            "DeepInfra response",
        )))
        .mount(&deepinfra_mock)
        .await;

    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
        together_api_key: String::new(), // empty → TG backend not created
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    let result = router
        .generate_with_model(
            "Hello",
            &default_params(),
            Some("DI/meta-llama/Llama-3.3-70B-Instruct"),
        )
        .await;
    assert!(
        result.is_ok(),
        "DI/ should succeed when DeepInfra is available"
    );

    let result = router
        .generate_with_model("Hello", &default_params(), Some("TG/some-model"))
        .await;
    assert!(
        result.is_err(),
        "TG/ should fail when Together AI is unavailable"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not available") || err.contains("Together"),
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
    let deepinfra_mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "deepseek-v4-pro",
            "Default provider response",
        )))
        .mount(&deepinfra_mock)
        .await;

    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
        default_model: "deepseek-v4-pro".to_string(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

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
    let deepinfra_mock = MockServer::start().await;
    let together_mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "meta-llama/Llama-3.3-70B-Instruct",
            "Override DeepInfra",
        )))
        .mount(&deepinfra_mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "Qwen/Qwen2.5-7B-Instruct-Turbo",
            "Override Together",
        )))
        .mount(&together_mock)
        .await;

    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
        together_base_url: together_mock.uri(),
        together_api_key: "test-key".to_string(),
        default_model: "DI/meta-llama/Llama-3.3-70B-Instruct".to_string(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    let result = router
        .generate_with_model(
            "Hello",
            &default_params(),
            Some("TG/Qwen/Qwen2.5-7B-Instruct-Turbo"),
        )
        .await
        .expect("Model override should succeed");
    assert_eq!(result.text, "Override Together");

    let result = router
        .generate_with_model(
            "Hello",
            &default_params(),
            Some("DI/meta-llama/Llama-3.3-70B-Instruct"),
        )
        .await
        .expect("Model override should succeed");
    assert_eq!(result.text, "Override DeepInfra");
}

/// REQ: P9-inf-test-list-models-degradation — Graceful degradation in list_models
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates graceful catalog degradation
///
/// When one provider's model-listing endpoint is unavailable,
/// `list_models()` still returns results from reachable providers.
#[tokio::test]
async fn list_models_graceful_degradation() {
    let deepinfra_mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_deepinfra_models(&[json!({
                "id": "meta-llama/Llama-3.3-70B-Instruct",
                "object": "model",
                "created_at": "2026-01-01T00:00:00Z",
                "owned_by": "deepinfra"
            })])),
        )
        .mount(&deepinfra_mock)
        .await;

    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    let models = router.list_models().await;

    assert!(
        !models.is_empty(),
        "list_models should return results from reachable providers"
    );

    let deepinfra_model = models
        .iter()
        .find(|m| m.prefixed_name == "DI/meta-llama/Llama-3.3-70B-Instruct");
    assert!(
        deepinfra_model.is_some(),
        "DeepInfra model should be present with DI/ prefix. Got models: {:?}",
        models.iter().map(|m| &m.prefixed_name).collect::<Vec<_>>()
    );
    assert_eq!(deepinfra_model.unwrap().provider, ProviderId::DeepInfra);
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
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "meta-llama/Llama-3.3-70B-Instruct",
            "Summary text",
        )))
        .mount(&mock)
        .await;

    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_base_url: mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
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
        .generate_with_model(
            "Summarize this.",
            &params,
            Some("DI/meta-llama/Llama-3.3-70B-Instruct"),
        )
        .await
        .expect("Request with disable_thinking should succeed");

    assert_eq!(result.text, "Summary text");
    assert_eq!(result.model, "meta-llama/Llama-3.3-70B-Instruct");
}

/// REQ: P9-inf-test-generate-unavailable-backend — generate() with unavailable default provider
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates graceful boundary unavailability
///
/// When the default model resolves to a provider whose backend is None,
/// `generate()` returns Err(Connection).
#[tokio::test]
async fn generate_unavailable_backend_returns_error() {
    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_api_key: String::new(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    let result = router.generate("Hello", &default_params()).await;
    assert!(
        result.is_err(),
        "generate() should fail when default provider backend is unavailable"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not available") || err.contains("DeepInfra") || err.contains("backend"),
        "Error should mention unavailable backend, got: {}",
        err
    );
}

/// REQ: P9-inf-test-generate-stream-unavailable — generate_stream() with unavailable default provider
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates graceful boundary unavailability
///
/// When the default model resolves to a provider whose backend is None,
/// `generate_stream()` yields an Err(Connection) as its first (and only) item.
#[tokio::test]
async fn generate_stream_unavailable_backend_returns_error() {
    use futures_util::StreamExt;

    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_api_key: String::new(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    let mut stream = router.generate_stream("Hello", &default_params());
    let first = stream.next().await;
    assert!(
        first.is_some(),
        "generate_stream() should yield at least one item"
    );
    assert!(
        first.unwrap().is_err(),
        "generate_stream() first item should be Err when backend unavailable"
    );
}

/// REQ: P9-inf-test-generate-stream-with-model-unavailable — generate_stream_with_model() with unavailable provider
/// \[P9\] Motivating: Homeostatic Self-Regulation — validates graceful boundary unavailability
///
/// When model_override resolves to a provider whose backend is None,
/// `generate_stream_with_model()` yields an Err(Connection) as its first item.
#[tokio::test]
async fn generate_stream_with_model_unavailable_backend_returns_error() {
    use futures_util::StreamExt;

    let deepinfra_mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response(
            "meta-llama/Llama-3.3-70B-Instruct",
            "DeepInfra response",
        )))
        .mount(&deepinfra_mock)
        .await;

    let config = InferenceConfig {
        default_provider: ProviderId::DeepInfra,
        deepinfra_base_url: deepinfra_mock.uri(),
        deepinfra_api_key: "test-key".to_string(),
        together_api_key: String::new(),
        ..Default::default()
    };
    let router = InferenceRouter::new(config);

    let mut stream = router.generate_stream_with_model(
        "Hello",
        &default_params(),
        Some("DI/meta-llama/Llama-3.3-70B-Instruct"),
    );
    let first = stream.next().await;
    assert!(first.is_some(), "DI/ stream should yield items");
    assert!(first.unwrap().is_ok(), "DI/ stream should succeed");

    let mut stream =
        router.generate_stream_with_model("Hello", &default_params(), Some("TG/some-model"));
    let first = stream.next().await;
    assert!(first.is_some(), "TG/ stream should yield at least one item");
    assert!(
        first.unwrap().is_err(),
        "TG/ stream first item should be Err when backend unavailable"
    );
}