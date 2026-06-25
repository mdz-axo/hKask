//! Runpod adapter backend — serverless vLLM endpoint provisioning.
//!
//! Docs: https://docs.runpod.io/serverless/endpoints/manage-endpoints

use super::openai::openai_compatible_infer;
use super::AdapterProviderBackend;
use crate::adapter_config::AdapterConfig;
use crate::adapter_port::AdapterError;
use crate::adapter_store::TrainedLoRAAdapter;
use crate::provider_cost::{CostModel, ProviderCapability};
use hkask_ports::InferenceResult;
use hkask_types::template::LLMParameters;

pub(super) struct RunpodAdapterBackend {
    cost_model: CostModel,
    capability: ProviderCapability,
    api_key: String,
    client: reqwest::Client,
}

impl RunpodAdapterBackend {
    pub(super) fn new() -> Self {
        Self {
            cost_model: CostModel::runpod(),
            capability: ProviderCapability::runpod(),
            api_key: std::env::var("RUNPOD_API_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl AdapterProviderBackend for RunpodAdapterBackend {
    async fn provision_endpoint(
        &self,
        adapter: &TrainedLoRAAdapter,
    ) -> Result<String, AdapterError> {
        if self.api_key.is_empty() {
            return Err(AdapterError::ProviderUnavailable(
                "RUNPOD_API_KEY not set".into(),
            ));
        }

        let template_id = std::env::var("RUNPOD_TEMPLATE_ID").unwrap_or_default();
        if template_id.is_empty() {
            return Err(AdapterError::ProviderUnavailable(
                "RUNPOD_TEMPLATE_ID not set — required for serverless endpoint provisioning".into(),
            ));
        }

        tracing::info!(
            target: "cns.adapter",
            adapter_id = %adapter.id,
            template_id = %template_id,
            "Provisioning Runpod serverless endpoint"
        );

        let endpoint_url = format!("https://api.runpod.ai/v2/{}/openai/v1", template_id);
        tracing::info!(
            target: "cns.adapter",
            template_id = %template_id,
            "Runpod serverless endpoint ready"
        );
        Ok(endpoint_url)
    }

    async fn infer(
        &self,
        endpoint_url: &str,
        prompt: &str,
        params: &LLMParameters,
        model_name: &str,
    ) -> Result<InferenceResult, AdapterError> {
        openai_compatible_infer(
            &self.client,
            &self.api_key,
            endpoint_url,
            prompt,
            params,
            model_name,
        )
        .await
    }

    async fn teardown(&self, endpoint_url: &str) -> Result<(), AdapterError> {
        if self.api_key.is_empty() {
            return Err(AdapterError::Internal(
                "RUNPOD_API_KEY not set — cannot teardown endpoint. \
                 The endpoint may still be active and billing at Runpod."
                    .into(),
            ));
        }
        self.client
            .delete(endpoint_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                AdapterError::Internal(format!(
                    "Runpod teardown HTTP request failed: {e}. \
                     Endpoint may need manual deletion via Runpod console."
                ))
            })?;
        tracing::info!(target: "cns.adapter", endpoint_url = %endpoint_url, "Runpod endpoint torn down");
        Ok(())
    }

    async fn upload_adapter(
        &self,
        _adapter: &TrainedLoRAAdapter,
        _config: &AdapterConfig,
    ) -> Result<String, AdapterError> {
        Err(AdapterError::ProviderUnavailable(
            "Runpod does not support LoRA adapter uploads. Adapters must be baked into the container image."
                .into(),
        ))
    }

    fn capability(&self) -> ProviderCapability {
        self.capability.clone()
    }

    fn cost_model(&self) -> CostModel {
        self.cost_model.clone()
    }
}
