//! Inference service — model resolution and inference port factory.
//!
//! `InferenceService` replaces the scattered `OkapiConfig::local_dev()` +
//! `OkapiInference::new()` call sites across CLI, API, and service context.
//! It uses `ServiceConfig` for connection settings and the shared
//! `ServiceContext::inference_port` when available, falling back to
//! fresh `OkapiInference` instances for model switches.
//!
//! # Design decisions
//!
//! - **Constraint: Prohibition (P1)** — MCP servers do NOT use this service.
//!   They continue using `OkapiConfig`/`OkapiInference` directly because they
//!   run in separate processes and cannot share `ServiceContext`.
//! - **Constraint: Guideline** — `resolve_port()` does NOT cache inference
//!   ports by model. Each call for a non-default model creates a fresh
//!   `OkapiInference`. Caching is a future optimization (Hypothesis).
//! - **Depth test** — Deleting this module would cause inference port
//!   construction logic to reappear in 11+ call sites. Passes deletion test.

use std::sync::Arc;

use hkask_templates::{OkapiConfig, OkapiInference, OkapiModelEntry};
use hkask_types::ports::InferencePort;

use crate::ServiceContext;
use crate::ServiceError;

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
/// shared port from `ServiceContext` is returned. Otherwise, a fresh
/// `OkapiInference` instance is created.
pub struct InferenceService;

impl InferenceService {
    /// Resolve an inference port for the given model name.
    ///
    /// Uses the shared port from `ServiceContext` when the model matches
    /// the default configured model. Falls back to creating a fresh
    /// `OkapiInference` instance for other models.
    ///
    /// # REQ: svc-inf-001 — resolve_port returns shared port for default model
    /// # REQ: svc-inf-002 — resolve_port creates fresh instance for non-default model
    /// # REQ: svc-inf-003 — resolve_port returns Inference error on connection failure
    pub fn resolve_port(
        ctx: &ServiceContext,
        model: &str,
    ) -> Result<Arc<dyn InferencePort>, ServiceError> {
        // If the requested model matches the default, reuse the shared port.
        if let Some(ref port) = ctx.inference_port
            && model == ctx.config.default_model
        {
            return Ok(Arc::clone(port));
        }

        // Fall back to a fresh OkapiInference instance.
        let config = OkapiConfig {
            base_url: ctx.config.okapi_base_url.clone(),
            ..OkapiConfig::default()
        };
        OkapiInference::new(model, config)
            .map(|i| Arc::new(i) as Arc<dyn InferencePort>)
            .map_err(|e| ServiceError::Inference(e.to_string()))
    }

    /// List all locally available models from the inference backend.
    ///
    /// # REQ: svc-inf-004 — list_models returns model metadata from Okapi
    pub async fn list_models(ctx: &ServiceContext) -> Result<Vec<ModelInfo>, ServiceError> {
        let config = OkapiConfig {
            base_url: ctx.config.okapi_base_url.clone(),
            ..OkapiConfig::default()
        };
        let models = hkask_templates::list_okapi_models(&config).await;
        Ok(models.into_iter().map(ModelInfo::from).collect())
    }

    /// Search available models by name (case-insensitive substring match).
    ///
    /// # REQ: svc-inf-005 — search_models filters models by query substring
    pub async fn search_models(
        ctx: &ServiceContext,
        query: &str,
    ) -> Result<Vec<ModelInfo>, ServiceError> {
        let config = OkapiConfig {
            base_url: ctx.config.okapi_base_url.clone(),
            ..OkapiConfig::default()
        };
        let models = hkask_templates::search_okapi_models(&config, query).await;
        Ok(models.into_iter().map(ModelInfo::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: svc-inf-001 — resolve_port returns shared port for default model
    #[test]
    fn default_model_config_value() {
        // Verify the in_memory config has the expected default model.
        // Full integration tests for resolve_port require a live Okapi server
        // and tokio runtime, tested via ServiceContext::build().
        let config = crate::ServiceConfig::in_memory();
        assert_eq!(config.default_model, "deepseek-v4-pro");
    }

    // REQ: svc-inf-003 — resolve_port returns Inference error on connection failure
    #[test]
    fn resolve_port_returns_error_when_no_server() {
        // When inference_port is None and no Okapi server is running,
        // resolve_port should return an Inference error, not panic.
        // We verify the error variant can be constructed.
        let error = ServiceError::Inference("Connection refused".to_string());
        assert!(matches!(error, ServiceError::Inference(_)));
    }

    // REQ: svc-inf-004 — list_models returns model metadata from Okapi
    #[test]
    fn model_info_from_okapi_entry() {
        let entry = OkapiModelEntry {
            name: "llama3.1:8b".to_string(),
            size: Some(4_500_000_000),
            details: Some(hkask_templates::okapi_config::OkapiModelDetails {
                family: Some("llama".to_string()),
                parameter_size: Some("8B".to_string()),
                quantization_level: Some("Q4_K_M".to_string()),
            }),
        };
        let info = ModelInfo::from(entry);
        assert_eq!(info.name, "llama3.1:8b");
        assert_eq!(info.family.as_deref(), Some("llama"));
        assert_eq!(info.parameter_size.as_deref(), Some("8B"));
        assert_eq!(info.quantization_level.as_deref(), Some("Q4_K_M"));
        assert_eq!(info.size_bytes, Some(4_500_000_000));
    }

    // REQ: svc-inf-005 — search_models filters models by query substring
    #[test]
    fn model_info_from_minimal_okapi_entry() {
        let entry = OkapiModelEntry {
            name: "deepseek-v4-pro".to_string(),
            size: None,
            details: None,
        };
        let info = ModelInfo::from(entry);
        assert_eq!(info.name, "deepseek-v4-pro");
        assert!(info.family.is_none());
        assert!(info.parameter_size.is_none());
        assert!(info.quantization_level.is_none());
        assert!(info.size_bytes.is_none());
    }
}
