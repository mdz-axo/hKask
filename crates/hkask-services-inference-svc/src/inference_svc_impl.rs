//! Inference service — model resolution and inference port factory.
//!
//! `InferenceService` constructs an `InferenceRouter` from `InferenceContext`
//! and provides model listing/search across all configured providers.
//!
//! # Design decisions
//!
//! - **Constraint: Prohibition (P1)** — MCP servers do NOT use this service.
//!   They continue using inference primitives directly because they
//!   run in separate processes and cannot share `AgentService`.
//! - **Constraint: Guideline** — `resolve_port()` does NOT cache inference
//!   ports by model. Each call for a non-default model creates a fresh
//!   router. Caching is a future optimization (Hypothesis).
//! - **Depth test** — Deleting this module would cause inference port
//!   construction logic to reappear in 11+ call sites. Passes deletion test.

use hkask_rsolidity::contract;

use std::sync::Arc;

use hkask_inference::{InferenceConfig, InferenceRouter, ProviderId, RouterModelEntry};
use hkask_types::ports::InferencePort;

use hkask_services_core::ServiceError;

/// Lightweight context for `InferenceService` calls.
///
/// Contains only the fields needed for inference port resolution and model
/// listing. Construct from a `AgentService` (full assembly) or from parts
/// (CLI/API surfaces that don't yet compose a full `AgentService`).
pub struct InferenceContext {
    /// Shared inference port for the default model. When the requested model
    /// matches `default_model`, this port is reused. `None` if no shared
    /// port is available (standalone commands, fallback paths).
    pub shared_port: Option<Arc<dyn InferencePort>>,
    /// Default model name (used to decide whether to reuse the shared port).
    pub default_model: String,
    /// Inference configuration for the router.
    pub inference_config: InferenceConfig,
}

impl InferenceContext {
    /// Construct from individual parts (for CLI/API surfaces that don't
    /// have a full `AgentService`).
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  default_model must be non-empty; inference_config must be valid
    /// post: returns InferenceContext with provided parts; shared_port may be None
    #[contract(id = "P9-svc-inference-228", principle = "P9")]
    pub fn from_parts(
        shared_port: Option<Arc<dyn InferencePort>>,
        default_model: impl Into<String>,
        inference_config: InferenceConfig,
    ) -> Self {
        Self {
            shared_port,
            default_model: default_model.into(),
            inference_config,
        }
    }
}

/// Model metadata returned by the inference backend.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Full model name with provider prefix (e.g., "OM/qwen3:8b")
    pub name: String,
    /// Provider this model belongs to
    pub provider: ProviderId,
    /// Model family (e.g., "llama", "qwen2")
    pub family: Option<String>,
    /// Parameter count (e.g., "8B", "70B")
    pub parameter_size: Option<String>,
    /// Quantization level (e.g., "Q4_0")
    pub quantization_level: Option<String>,
    /// Model size in bytes (if available)
    pub size_bytes: Option<u64>,
}

impl From<RouterModelEntry> for ModelInfo {
    fn from(entry: RouterModelEntry) -> Self {
        Self {
            name: entry.prefixed_name,
            provider: entry.provider,
            family: entry.family,
            parameter_size: entry.parameter_size,
            quantization_level: entry.quantization_level,
            size_bytes: entry.size_bytes,
        }
    }
}

/// Inference service — resolves inference ports and lists available models.
pub struct InferenceService;

impl InferenceService {
    /// Resolve an inference port for the given model name.
    ///
    /// Uses the shared port from `AgentService` when the model matches
    /// the default configured model. Falls back to creating a fresh
    /// `InferenceRouter` instance for other models.
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx must have valid inference_config; model must be non-empty
    /// post: returns Arc<dyn InferencePort> — shared port if model matches default, else fresh InferenceRouter; Err on connection failure
    /// # REQ: P9-svc-inference-svc-inf-001 — resolve_port returns shared port for default model
    /// # expect: "The service layer provides CNS health and regulation queries" [P9]
    /// # REQ: P9-svc-inference-svc-inf-002 — resolve_port creates fresh instance for non-default model
    /// # expect: "The service layer provides CNS health and regulation queries" [P9]
    /// # REQ: P9-svc-inference-svc-inf-003 — resolve_port returns Inference error on connection failure
    /// # expect: "The service layer provides CNS health and regulation queries" [P9]
    #[contract(id = "P9-svc-inference-229", principle = "P9")]
    pub fn resolve_port(
        ctx: &InferenceContext,
        model: &str,
    ) -> Result<Arc<dyn InferencePort>, ServiceError> {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.inference_svc", operation = "resolve_port", model = %model, has_shared = ctx.shared_port.is_some(), "CNS");

        // If the requested model matches the default, reuse the shared port.
        if let Some(ref port) = ctx.shared_port
            && model == ctx.default_model
        {
            return Ok(Arc::clone(port));
        }

        // Fall back to a fresh InferenceRouter instance.
        let router = InferenceRouter::new(ctx.inference_config.clone());
        Ok(Arc::new(router) as Arc<dyn InferencePort>)
    }

    /// List all locally available models from all configured providers.
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx must have valid inference_config
    /// post: returns Vec<ModelInfo> from all configured providers; empty Vec if none
    /// # REQ: P9-svc-inference-svc-inf-004 — list_models returns model metadata from all providers
    /// # expect: "The service layer provides CNS health and regulation queries" [P9]
    #[contract(id = "P9-svc-inference-230", principle = "P9")]
    pub async fn list_models(ctx: &InferenceContext) -> Result<Vec<ModelInfo>, ServiceError> {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.inference_svc", operation = "list_models", "CNS");

        let router = InferenceRouter::new(ctx.inference_config.clone());
        let models = router.list_models().await;
        Ok(models.into_iter().map(ModelInfo::from).collect())
    }

    /// Search available models by name (case-insensitive substring match).
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx must have valid inference_config; query must be non-empty
    /// post: returns Vec<ModelInfo> matching query; empty Vec if no matches
    /// # REQ: P9-svc-inference-svc-inf-005 — search_models filters models by query substring
    /// # expect: "The service layer provides CNS health and regulation queries" [P9]
    #[contract(id = "P9-svc-inference-231", principle = "P9")]
    pub async fn search_models(
        ctx: &InferenceContext,
        query: &str,
    ) -> Result<Vec<ModelInfo>, ServiceError> {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.inference_svc", operation = "search_models", query = %query, "CNS");

        let router = InferenceRouter::new(ctx.inference_config.clone());
        let models = router.search_models(query).await;
        Ok(models.into_iter().map(ModelInfo::from).collect())
    }
}
