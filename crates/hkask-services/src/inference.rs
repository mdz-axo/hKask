//! Inference service — model resolution and inference port factory.
//!
//! `InferenceService` replaces the scattered `OkapiConfig::local_dev()` +
//! `OkapiInference::new()` call sites across CLI, API, and service context.
//! It uses `InferenceContext` (a lightweight struct with just the fields
//! needed for inference) instead of requiring a full `AgentService`.
//!
//! # Design decisions
//
//! - **Constraint: Prohibition (P1)** — MCP servers do NOT use this service.
//!   They continue using `OkapiConfig`/`OkapiInference` directly because they
//!   run in separate processes and cannot share `AgentService`.
//! - **Constraint: Guideline** — `resolve_port()` does NOT cache inference
//!   ports by model. Each call for a non-default model creates a fresh
//!   `OkapiInference`. Caching is a future optimization (Hypothesis).
//! - **Depth test** — Deleting this module would cause inference port
//!   construction logic to reappear in 11+ call sites. Passes deletion test.
//! - **Strangler fig** — `InferenceContext` is a lightweight alternative to
//!   `AgentService` that CLI and API surfaces construct from their own
//!   config/state. Full `AgentService` composition happens in Task 7b.

use std::sync::Arc;

use hkask_templates::{OkapiConfig, OkapiInference, OkapiModelEntry};
use hkask_types::ports::InferencePort;

use crate::ServiceError;

/// Lightweight context for `InferenceService` calls.
///
/// Contains only the fields needed for inference port resolution and model
/// listing. Construct from a `AgentService` (full assembly) or from parts
/// (CLI/API surfaces that don't yet compose a full `AgentService`).
///
/// This struct enables the strangler fig pattern: surfaces can route
/// inference through `InferenceService` without building a complete
/// `AgentService` (which opens databases, starts loops, etc.).
pub struct InferenceContext {
    /// Shared inference port for the default model. When the requested model
    /// matches `default_model`, this port is reused. `None` if no shared
    /// port is available (standalone commands, fallback paths).
    pub shared_port: Option<Arc<dyn InferencePort>>,
    /// Default model name (used to decide whether to reuse the shared port).
    pub default_model: String,
    /// Base URL for the Okapi inference server.
    pub okapi_base_url: String,
}

impl InferenceContext {
    /// Construct from individual parts (for CLI/API surfaces that don't
    /// have a full `AgentService`).
    ///
    /// `shared_port` is `None` for standalone commands that create fresh
    /// inference ports on every call. When available (e.g., from
    /// `ReplState::inference_port`), passing it here enables port reuse
    /// for the default model.
    pub fn from_parts(
        shared_port: Option<Arc<dyn InferencePort>>,
        default_model: impl Into<String>,
        okapi_base_url: impl Into<String>,
    ) -> Self {
        Self {
            shared_port,
            default_model: default_model.into(),
            okapi_base_url: okapi_base_url.into(),
        }
    }
}

impl From<&crate::AgentService> for InferenceContext {
    fn from(ctx: &crate::AgentService) -> Self {
        Self {
            shared_port: ctx.coordination().0.clone(),
            default_model: ctx.config().default_model.clone(),
            okapi_base_url: ctx.config().okapi_base_url.clone(),
        }
    }
}

/// Model metadata returned by the inference backend.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub size_bytes: Option<u64>,
}

impl From<OkapiModelEntry> for ModelInfo {
    fn from(entry: OkapiModelEntry) -> Self {
        Self {
            name: entry.name,
            family: entry.details.as_ref().and_then(|d| d.family.clone()),
            parameter_size: entry
                .details
                .as_ref()
                .and_then(|d| d.parameter_size.clone()),
            quantization_level: entry
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone()),
            size_bytes: entry.size,
        }
    }
}

/// Inference service — resolves inference ports and lists available models.
///
/// Use `InferenceService::resolve_port()` to get an inference port for a
/// specific model. If the model matches the default configured model, the
/// shared port from `AgentService` is returned. Otherwise, a fresh
/// `OkapiInference` instance is created.
pub struct InferenceService;

impl InferenceService {
    /// Resolve an inference port for the given model name.
    ///
    /// Uses the shared port from `AgentService` when the model matches
    /// the default configured model. Falls back to creating a fresh
    /// `OkapiInference` instance for other models.
    ///
    /// # REQ: svc-inf-001 — resolve_port returns shared port for default model
    /// # REQ: svc-inf-002 — resolve_port creates fresh instance for non-default model
    /// # REQ: svc-inf-003 — resolve_port returns Inference error on connection failure
    pub fn resolve_port(
        ctx: &InferenceContext,
        model: &str,
    ) -> Result<Arc<dyn InferencePort>, ServiceError> {
        // If the requested model matches the default, reuse the shared port.
        if let Some(ref port) = ctx.shared_port
            && model == ctx.default_model
        {
            return Ok(Arc::clone(port));
        }

        // Fall back to a fresh OkapiInference instance.
        let config = OkapiConfig {
            base_url: ctx.okapi_base_url.clone(),
            ..OkapiConfig::default()
        };
        OkapiInference::new(model, config)
            .map(|i| Arc::new(i) as Arc<dyn InferencePort>)
            .map_err(ServiceError::InferencePort)
    }

    /// List all locally available models from the inference backend.
    ///
    /// # REQ: svc-inf-004 — list_models returns model metadata from Okapi
    pub async fn list_models(ctx: &InferenceContext) -> Result<Vec<ModelInfo>, ServiceError> {
        let config = OkapiConfig {
            base_url: ctx.okapi_base_url.clone(),
            ..OkapiConfig::default()
        };
        let models = hkask_templates::list_okapi_models(&config).await;
        Ok(models.into_iter().map(ModelInfo::from).collect())
    }

    /// Search available models by name (case-insensitive substring match).
    ///
    /// # REQ: svc-inf-005 — search_models filters models by query substring
    pub async fn search_models(
        ctx: &InferenceContext,
        query: &str,
    ) -> Result<Vec<ModelInfo>, ServiceError> {
        let config = OkapiConfig {
            base_url: ctx.okapi_base_url.clone(),
            ..OkapiConfig::default()
        };
        let models = hkask_templates::search_okapi_models(&config, query).await;
        Ok(models.into_iter().map(ModelInfo::from).collect())
    }
}
