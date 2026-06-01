//! Okapi Port Definitions
//!
//! Abstract interfaces for Okapi integration, following hexagonal architecture.
//! These traits define the boundaries between application logic and infrastructure.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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
