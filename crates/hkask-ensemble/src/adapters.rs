//! Okapi Infrastructure Adapters
//!
//! Concrete implementations of port traits for Okapi HTTP infrastructure.

use crate::ports::{
    GenerateRequest, GenerateResponse, InferenceClient, MetricsSource, OkapiCapabilities,
    OkapiMetrics, TokenProb, TokenProbability,
};
use async_trait::async_trait;
use thiserror::Error;

/// Error type for Okapi HTTP adapters
#[derive(Debug, Error)]
pub enum OkapiAdapterError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    ParseError(String),

    #[error("SSE stream ended unexpectedly")]
    SseStreamEnded,

    #[error("Invalid SSE event format: {0}")]
    InvalidSseEvent(String),
}

/// Error type for improv-specific Okapi client
#[derive(Debug, Error)]
pub enum ImprovClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Okapi inference client for improv sessions
pub struct OkapiImprovClient {
    base_url: String,
}

impl OkapiImprovClient {
    pub fn new() -> Self {
        let base_url =
            std::env::var("OKAPI_BASE_URL").unwrap_or_else(|_| "http://localhost:8001".to_string());
        Self { base_url }
    }
}

impl Default for OkapiImprovClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceClient for OkapiImprovClient {
    type Error = ImprovClientError;

    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        let client = reqwest::Client::new();
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

        let resp = client
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ImprovClientError::Parse(format!("Failed to parse response: {}", e)))?;

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
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        let resp = client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        resp.json()
            .await
            .map_err(|e| ImprovClientError::Parse(format!("Failed to parse chat response: {}", e)))
    }
}

/// HTTP-based metrics source adapter (SSE stream)
pub struct OkapiSseAdapter {
    client: reqwest::Client,
    sse_url: String,
}

impl OkapiSseAdapter {
    pub fn new(okapi_base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            sse_url: format!("{}/api/metrics/stream?interval=5", okapi_base_url),
        }
    }
}

#[async_trait]
impl MetricsSource for OkapiSseAdapter {
    type Metrics = OkapiMetrics;
    type Error = OkapiAdapterError;

    async fn next_metrics(&self) -> Result<Self::Metrics, Self::Error> {
        let response = self.client.get(&self.sse_url).send().await?;
        let stream = response.text().await?;

        for line in stream.lines() {
            if line.starts_with("data: ") {
                let data = line.strip_prefix("data: ").ok_or_else(|| {
                    OkapiAdapterError::InvalidSseEvent("Missing data prefix".into())
                })?;

                let metrics: OkapiMetrics = serde_json::from_str(data)
                    .map_err(|e| OkapiAdapterError::ParseError(e.to_string()))?;

                return Ok(metrics);
            }
        }

        Err(OkapiAdapterError::SseStreamEnded)
    }
}

/// HTTP-based inference client adapter
pub struct OkapiHttpClient {
    client: reqwest::Client,
    base_url: String,
}

impl OkapiHttpClient {
    pub fn new(okapi_base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: okapi_base_url.to_string(),
        }
    }
}

#[async_trait]
impl InferenceClient for OkapiHttpClient {
    type Error = OkapiAdapterError;

    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(request)
            .send()
            .await?;

        let result: GenerateResponse = response
            .json()
            .await
            .map_err(|e| OkapiAdapterError::ParseError(e.to_string()))?;

        Ok(result)
    }

    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        model: String,
    ) -> Result<serde_json::Value, Self::Error> {
        let request = serde_json::json!({
            "model": model,
            "messages": messages,
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OkapiAdapterError::ParseError(e.to_string()))?;

        Ok(result)
    }
}

/// HTTP-based capability provider adapter
pub struct OkapiCapabilityFetcher {
    client: reqwest::Client,
    base_url: String,
}

impl OkapiCapabilityFetcher {
    pub fn new(okapi_base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: okapi_base_url.to_string(),
        }
    }

    pub async fn get_capabilities(&self) -> Result<OkapiCapabilities, OkapiAdapterError> {
        let response = self
            .client
            .get(format!("{}/api/engine/status", self.base_url))
            .send()
            .await?;

        let capabilities: OkapiCapabilities = response
            .json()
            .await
            .map_err(|e| OkapiAdapterError::ParseError(e.to_string()))?;

        Ok(capabilities)
    }
}

/// Mock metrics source for testing
pub struct MockMetricsSource {
    metrics: Vec<OkapiMetrics>,
    current_index: tokio::sync::Mutex<usize>,
}

impl MockMetricsSource {
    pub fn new(metrics: Vec<OkapiMetrics>) -> Self {
        Self {
            metrics,
            current_index: tokio::sync::Mutex::new(0),
        }
    }
}

#[async_trait]
impl MetricsSource for MockMetricsSource {
    type Metrics = OkapiMetrics;
    type Error = OkapiAdapterError;

    async fn next_metrics(&self) -> Result<Self::Metrics, Self::Error> {
        let mut index = self.current_index.lock().await;
        if *index >= self.metrics.len() {
            return Err(OkapiAdapterError::SseStreamEnded);
        }
        let metrics = self.metrics[*index].clone();
        *index += 1;
        Ok(metrics)
    }
}

/// Mock inference client for testing
pub struct MockInferenceClient {
    responses: tokio::sync::Mutex<Vec<Result<GenerateResponse, OkapiAdapterError>>>,
}

impl MockInferenceClient {
    pub fn new(responses: Vec<Result<GenerateResponse, OkapiAdapterError>>) -> Self {
        Self {
            responses: tokio::sync::Mutex::new(responses),
        }
    }
}

#[async_trait]
impl InferenceClient for MockInferenceClient {
    type Error = OkapiAdapterError;

    async fn generate(&self, _request: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        let mut responses = self.responses.lock().await;
        responses.pop().ok_or(OkapiAdapterError::SseStreamEnded)?
    }

    async fn chat(
        &self,
        _messages: Vec<serde_json::Value>,
        _model: String,
    ) -> Result<serde_json::Value, Self::Error> {
        Err(OkapiAdapterError::SseStreamEnded)
    }
}
