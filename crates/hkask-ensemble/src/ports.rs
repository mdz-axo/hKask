//! Okapi Port Definitions
//
//! Abstract interfaces for Okapi integration, following hexagonal architecture.
//! These traits define the boundaries between application logic and infrastructure.
//
//! Token probability types are re-exported from hkask-types (canonical definitions).
//! The `InferenceClient` trait is specific to ensemble — it differs from
//! `hkask_types::ports::InferencePort` by accepting ensemble-specific request/response types.
//
//! Sovereignty and registry port traits are re-exported from hkask-types so that
//! ensemble depends only on the types crate, not on hkask-agents or hkask-templates.

// Re-export canonical token probability types from hkask-types
pub use hkask_types::ports::{TokenProb, TokenProbability};

use async_trait::async_trait;

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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
