//! Okapi Port Definitions
//!
//! Abstract interfaces for Okapi integration, following hexagonal architecture.
//! These traits define the boundaries between application logic and infrastructure.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Okapi metrics data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OkapiMetrics {
    pub tokens_generated_total: i64,
    pub kv_cache_tokens: i64,
    pub context_length: i64,
    pub adapter_swap_latency_ms: i64,
    pub gpu_memory_used_bytes: u64,
    pub prompt_cache_hit_ratio: Option<f64>,
}

/// Okapi capabilities data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiCapabilities {
    pub runner_type: String,
    pub lora_hot_swap: bool,
    pub token_probs: bool,
    pub grammar_native: bool,
    pub advanced_sampling: bool,
}

/// Token probability from Okapi response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenProbability {
    pub token: String,
    pub prob: f64,
    pub top_k: Vec<TokenProb>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenProb {
    pub token: String,
    pub prob: f64,
}

/// Generate request for Okapi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    pub options: Option<GenerateOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOptions {
    pub n_probs: Option<i32>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
}

/// Generate response from Okapi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub response: String,
    pub model: String,
    pub completion_probabilities: Option<Vec<TokenProbability>>,
}

/// Port for receiving Okapi metrics (e.g., from SSE stream)
#[async_trait]
pub trait MetricsSource: Send + Sync {
    type Metrics: Clone + Debug;
    type Error: std::error::Error + Send + Sync;

    async fn next_metrics(&self) -> Result<Self::Metrics, Self::Error>;
}

/// Port for Okapi inference operations
#[async_trait]
pub trait InferenceClient: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, Self::Error>;

    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        model: String,
    ) -> Result<serde_json::Value, Self::Error>;
}

/// Port for Okapi capability discovery
#[async_trait]
pub trait CapabilityProvider: Send + Sync {
    type Capabilities: Clone + Debug;
    type Error: std::error::Error + Send + Sync;

    async fn get_capabilities(&self) -> Result<Self::Capabilities, Self::Error>;
}
