//! Okapi Infrastructure Adapters
//
//! Concrete implementations of port traits for Okapi HTTP infrastructure.

use crate::ports::{
    GenerateRequest, GenerateResponse, InferenceClient, TokenProb, TokenProbability,
};
use async_trait::async_trait;
use thiserror::Error;

/// Error type for Okapi client operations
#[derive(Debug, Error)]
pub enum OkapiClientError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    ParseError(String),

    #[error("SSE stream ended unexpectedly")]
    SseStreamEnded,

    #[error("Invalid SSE event format: {0}")]
    InvalidSseEvent(String),
}

/// Unified Okapi inference client with detailed response parsing
///
/// Collapses the former `OkapiHttpClient` and `OkapiImprovClient` into a
/// single client that carries a reusable `reqwest::Client` and provides
/// detailed `TokenProbability` response parsing.
pub struct OkapiClient {
    client: reqwest::Client,
    base_url: String,
}

impl OkapiClient {
    /// Create a new OkapiClient pointing at the given base URL
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Create an OkapiClient reading the base URL from the `OKAPI_BASE_URL`
    /// environment variable, defaulting to `http://localhost:8001`.
    pub fn from_env() -> Self {
        let base_url =
            std::env::var("OKAPI_BASE_URL").unwrap_or_else(|_| "http://localhost:8001".to_string());
        Self::new(&base_url)
    }
}

impl Default for OkapiClient {
    fn default() -> Self {
        Self::from_env()
    }
}

#[async_trait]
impl InferenceClient for OkapiClient {
    type Error = OkapiClientError;

    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        let body = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "stream": false,
            "options": request.options.as_ref().map(|opts| serde_json::json!({
                "num_predict": opts.max_tokens,
                "temperature": opts.temperature,
                "num_probs": opts.n_probs,
            })),
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await.map_err(|e| {
            OkapiClientError::ParseError(format!("Failed to parse response: {}", e))
        })?;

        let response_text = json
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let model = json
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&request.model)
            .to_string();

        let completion_probs = json
            .get("completion_probabilities")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let token = item.get("token")?.as_str()?.to_string();
                        let prob = item.get("prob")?.as_f64()?;
                        let top_k = item
                            .get("top_k")
                            .and_then(|v| v.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|t| {
                                        Some(TokenProb {
                                            token: t.get("token")?.as_str()?.to_string(),
                                            prob: t.get("prob")?.as_f64()?,
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        Some(TokenProbability { token, prob, top_k })
                    })
                    .collect::<Vec<_>>()
            });

        Ok(GenerateResponse {
            response: response_text,
            model,
            completion_probabilities: completion_probs,
        })
    }

    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        model: String,
    ) -> Result<serde_json::Value, Self::Error> {
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        resp.json().await.map_err(|e| {
            OkapiClientError::ParseError(format!("Failed to parse chat response: {}", e))
        })
    }
}
