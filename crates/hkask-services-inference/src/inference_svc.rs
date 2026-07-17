use std::sync::Arc;

use hkask_inference::{InferenceConfig, InferenceRouter, ProviderId, RouterModelEntry};
use hkask_ports::InferencePort;

use hkask_services_core::ServiceError;

pub struct InferenceContext {
    pub shared_port: Option<Arc<dyn InferencePort>>,
    pub default_model: String,
    pub inference_config: InferenceConfig,
}

impl InferenceContext {
    #[must_use]
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

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub provider: ProviderId,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
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

pub struct InferenceService;

impl InferenceService {
    #[must_use = "result must be used"]
    pub fn resolve_port(
        ctx: &InferenceContext,
        model: &str,
    ) -> Result<Arc<dyn InferencePort>, ServiceError> {
        tracing::info!(target: "cns.inference_svc", operation = "resolve_port", model = %model, has_shared = ctx.shared_port.is_some(), "CNS");

        if let Some(ref port) = ctx.shared_port
            && model == ctx.default_model
        {
            return Ok(Arc::clone(port));
        }

        let router = InferenceRouter::new(ctx.inference_config.clone());
        Ok(Arc::new(router) as Arc<dyn InferencePort>)
    }

    #[must_use = "result must be used"]
    pub async fn list_models(ctx: &InferenceContext) -> Result<Vec<ModelInfo>, ServiceError> {
        tracing::info!(target: "cns.inference_svc", operation = "list_models", "CNS");
        // Lazy TTL cache: first call fetches live (the "start-up" update),
        // subsequent calls within the TTL return cached. See `model_cache`.
        crate::model_cache::ModelCache::list_models(ctx).await
    }

    #[must_use = "result must be used"]
    pub async fn search_models(
        ctx: &InferenceContext,
        query: &str,
    ) -> Result<Vec<ModelInfo>, ServiceError> {
        tracing::info!(target: "cns.inference_svc", operation = "search_models", query = %query, "CNS");
        // Search is a filter over the cached full list — one cache, filtered in-memory.
        let all = crate::model_cache::ModelCache::list_models(ctx).await?;
        if query.is_empty() {
            return Ok(all);
        }
        let lower = query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|m| m.name.to_lowercase().contains(&lower))
            .collect())
    }
}
