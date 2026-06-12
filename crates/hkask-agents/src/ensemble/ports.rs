//! Inference Port Definitions
//
//! Abstract interfaces for inference integration, following hexagonal architecture.
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

/// Generate request for inference
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

/// Generate response from inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub response: String,
    pub model: String,
    pub completion_probabilities: Option<Vec<TokenProbability>>,
}

/// Port for inference operations
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

// Gas Governance Port — CNS observability bridge

/// Port for CNS gas governance observability.
///
/// Allows the ensemble to report gas usage to the CNS CyberneticsLoop
/// so the CNS can sense ensemble gas consumption. The ensemble's internal
/// gas counter drives degradation levels; this port provides CNS visibility.
///
/// Implementations must be safe to call from synchronous contexts.
///
/// Implementations:
/// - `CyberneticsLoopGasAdapter` — Production adapter (in CLI/API wiring layer)
pub trait GasGovernancePort: Send + Sync {
    /// Check if a gas-consuming operation may proceed according to CNS governance.
    /// Returns `true` if the CNS allows the operation.
    fn can_proceed(&self, gas: u64) -> bool;

    /// Report gas consumption to the CNS governance layer.
    /// This is best-effort — fire-and-forget semantics.
    fn acquire(&self, gas: u64);
}
