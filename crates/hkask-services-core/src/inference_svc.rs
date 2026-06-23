use std::sync::Arc;

use hkask_inference::{InferenceConfig, InferenceRouter, ProviderId, RouterModelEntry};
use hkask_ports::InferencePort;

use crate::ServiceError;

pub struct InferenceContext {
    pub shared_port: Option<Arc<dyn InferencePort>>,
    pub default_model: String,
    pub inference_config: InferenceConfig,
}

impl InferenceContext {
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

    pub async fn list_models(ctx: &InferenceContext) -> Result<Vec<ModelInfo>, ServiceError> {
        tracing::info!(target: "cns.inference_svc", operation = "list_models", "CNS");

        let router = InferenceRouter::new(ctx.inference_config.clone());
        let models = router.list_models().await;
        Ok(models.into_iter().map(ModelInfo::from).collect())
    }

    pub async fn search_models(
        ctx: &InferenceContext,
        query: &str,
    ) -> Result<Vec<ModelInfo>, ServiceError> {
        tracing::info!(target: "cns.inference_svc", operation = "search_models", query = %query, "CNS");

        let router = InferenceRouter::new(ctx.inference_config.clone());
        let models = router.search_models(query).await;
        Ok(models.into_iter().map(ModelInfo::from).collect())
    }
}
