//! Okapi Infrastructure Adapters
//!
//! Concrete implementations of port traits for Okapi HTTP infrastructure.

use crate::ports::{
    CapabilityProvider, GenerateRequest, GenerateResponse, InferenceClient, MetricsSource,
    OkapiCapabilities, OkapiMetrics,
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
}

#[async_trait]
impl CapabilityProvider for OkapiCapabilityFetcher {
    type Capabilities = OkapiCapabilities;
    type Error = OkapiAdapterError;

    async fn get_capabilities(&self) -> Result<Self::Capabilities, Self::Error> {
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

/// Mock capability provider for testing
pub struct MockCapabilityProvider {
    capabilities: OkapiCapabilities,
}

impl MockCapabilityProvider {
    pub fn new(capabilities: OkapiCapabilities) -> Self {
        Self { capabilities }
    }

    pub fn with_limited_capabilities() -> Self {
        Self {
            capabilities: OkapiCapabilities {
                runner_type: "llamarunner".to_string(),
                lora_hot_swap: false,
                token_probs: false,
                grammar_native: false,
                advanced_sampling: false,
            },
        }
    }

    pub fn with_full_capabilities() -> Self {
        Self {
            capabilities: OkapiCapabilities {
                runner_type: "ollamarunner".to_string(),
                lora_hot_swap: true,
                token_probs: true,
                grammar_native: true,
                advanced_sampling: true,
            },
        }
    }
}

#[async_trait]
impl CapabilityProvider for MockCapabilityProvider {
    type Capabilities = OkapiCapabilities;
    type Error = OkapiAdapterError;

    async fn get_capabilities(&self) -> Result<Self::Capabilities, Self::Error> {
        Ok(self.capabilities.clone())
    }
}
