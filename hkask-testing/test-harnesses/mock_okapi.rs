//! Mock Okapi Server for Integration Tests
//!
//! Simulates Okapi API responses for testing without real API calls.
//! Supports configurable responses (success, error, latency).

use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Mock server state
#[derive(Debug, Clone)]
pub struct MockOkapiState {
    pub response_delay_ms: u64,
    pub should_fail: bool,
    pub failure_status: u16,
    pub custom_response: Option<Value>,
    pub request_count: Arc<RwLock<u64>>,
}

impl Default for MockOkapiState {
    fn default() -> Self {
        Self {
            response_delay_ms: 0,
            should_fail: false,
            failure_status: 500,
            custom_response: None,
            request_count: Arc::new(RwLock::new(0)),
        }
    }
}

/// Okapi generate request
#[derive(Debug, Deserialize)]
pub struct OkapiGenerateRequest {
    pub model: String,
    pub messages: Vec<OkapiMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub n_probs: Option<i32>,
}

/// Okapi message
#[derive(Debug, Deserialize)]
pub struct OkapiMessage {
    pub role: String,
    pub content: String,
}

/// Okapi generate response
#[derive(Debug, Serialize)]
pub struct OkapiGenerateResponse {
    pub model: String,
    pub choices: Vec<OkapiChoice>,
    pub usage: OkapiUsage,
}

/// Okapi choice
#[derive(Debug, Serialize)]
pub struct OkapiChoice {
    pub message: OkapiMessage,
    pub finish_reason: String,
    pub token_probs: Option<Vec<TokenProb>>,
}

/// Token probability
#[derive(Debug, Serialize)]
pub struct TokenProb {
    pub token: String,
    pub prob: f64,
    pub top_k: Vec<TopKProb>,
}

/// Top-K probability
#[derive(Debug, Serialize)]
pub struct TopKProb {
    pub token: String,
    pub prob: f64,
}

/// Token usage
#[derive(Debug, Serialize)]
pub struct OkapiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Create mock Okapi router
pub fn create_mock_okapi_router(state: MockOkapiState) -> Router {
    Router::new()
        .route("/api/generate", post(handle_generate))
        .route("/api/chat", post(handle_chat))
        .route("/api/metrics/stream", post(handle_metrics))
        .with_state(state)
}

async fn handle_generate(
    State(state): State<MockOkapiState>,
    Json(request): Json<OkapiGenerateRequest>,
) -> Result<Json<OkapiGenerateResponse>, StatusCode> {
    let mut count = state.request_count.write().await;
    *count += 1;

    if state.response_delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(state.response_delay_ms)).await;
    }

    if state.should_fail {
        return Err(StatusCode::from_u16(state.failure_status).unwrap());
    }

    let response_text = if let Some(custom) = &state.custom_response {
        custom["text"].as_str().unwrap_or("Mock response").to_string()
    } else {
        format!("Mock response to: {}", request.messages.first().map(|m| m.content.as_str()).unwrap_or(""))
    };

    let token_probs = request.n_probs.map(|n| {
        (0..n as usize)
            .map(|i| TokenProb {
                token: format!("token_{}", i),
                prob: 0.9 - (i as f64 * 0.1),
                top_k: vec![
                    TopKProb {
                        token: format!("alt_token_{}_1", i),
                        prob: 0.05,
                    },
                    TopKProb {
                        token: format!("alt_token_{}_2", i),
                        prob: 0.03,
                    },
                ],
            })
            .collect()
    });

    let response = OkapiGenerateResponse {
        model: request.model,
        choices: vec![OkapiChoice {
            message: OkapiMessage {
                role: "assistant".to_string(),
                content: response_text,
            },
            finish_reason: "stop".to_string(),
            token_probs,
        }],
        usage: OkapiUsage {
            prompt_tokens: request.messages.iter().map(|m| m.content.len() as u32 / 4).sum(),
            completion_tokens: response_text.len() as u32 / 4,
            total_tokens: 0,
        },
    };

    response.usage.total_tokens = response.usage.prompt_tokens + response.usage.completion_tokens;

    Ok(Json(response))
}

async fn handle_chat(
    State(state): State<MockOkapiState>,
    Json(_request): Json<Value>,
) -> Result<Json<OkapiGenerateResponse>, StatusCode> {
    if state.should_fail {
        return Err(StatusCode::from_u16(state.failure_status).unwrap());
    }

    Ok(Json(OkapiGenerateResponse {
        model: "mock-chat".to_string(),
        choices: vec![OkapiChoice {
            message: OkapiMessage {
                role: "assistant".to_string(),
                content: "Mock chat response".to_string(),
            },
            finish_reason: "stop".to_string(),
            token_probs: None,
        }],
        usage: OkapiUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
    }))
}

async fn handle_metrics(
    State(_state): State<MockOkapiState>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "tokens_generated_total": 1000,
        "kv_cache_tokens": 500,
        "context_length": 8192,
        "gpu_memory_used_bytes": 1024 * 1024 * 1024,
    })))
}

/// Start mock server
pub async fn start_mock_okapi(
    port: u16,
    state: MockOkapiState,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let router = create_mock_okapi_router(state);
    let addr = format!("127.0.0.1:{}", port);

    let handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, router).await.unwrap();
    });

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_mock_generate() {
        let state = MockOkapiState::default();
        let router = create_mock_okapi_router(state);

        let request = serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "temperature": 0.7,
            "max_tokens": 100,
        });

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/api/generate")
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from_json(&request).unwrap())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: OkapiGenerateResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.model, "test-model");
        assert_eq!(json.choices.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_generate_failure() {
        let state = MockOkapiState {
            should_fail: true,
            failure_status: 503,
            ..Default::default()
        };
        let router = create_mock_okapi_router(state);

        let request = serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "Hello"}],
        });

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/api/generate")
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from_json(&request).unwrap())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
