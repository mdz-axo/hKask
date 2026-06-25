//! Baseten adapter backend — REST endpoint provisioning + OpenAI-compatible inference.
//!
//! Docs: https://docs.baseten.co/api-reference

use super::AdapterProviderBackend;
use super::openai::openai_compatible_infer;
use crate::adapter_config::AdapterConfig;
use crate::adapter_port::AdapterError;
use crate::adapter_store::TrainedLoRAAdapter;
use crate::provider_cost::{CostModel, ProviderCapability};
use hkask_ports::InferenceResult;
use hkask_types::template::LLMParameters;

pub(super) struct BasetenAdapterBackend {
    cost_model: CostModel,
    capability: ProviderCapability,
    api_key: String,
    client: reqwest::Client,
}

impl BasetenAdapterBackend {
    pub(super) fn new() -> Self {
        Self {
            cost_model: CostModel::baseten(),
            capability: ProviderCapability::baseten(),
            api_key: std::env::var("BASETEN_API_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl AdapterProviderBackend for BasetenAdapterBackend {
    async fn provision_endpoint(
        &self,
        adapter: &TrainedLoRAAdapter,
    ) -> Result<String, AdapterError> {
        if self.api_key.is_empty() {
            return Err(AdapterError::ProviderUnavailable(
                "BASETEN_API_KEY not set".into(),
            ));
        }

        tracing::info!(
            target: "cns.adapter",
            adapter_id = %adapter.id,
            "Provisioning Baseten endpoint"
        );

        let body = serde_json::json!({
            "model_id": adapter.base_model_family,
            "name": format!("hkask-adapter-{}", adapter.id),
        });

        let response = self
            .client
            .post("https://api.baseten.co/v1/models/deploy")
            .header("Authorization", format!("Api-Key {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AdapterError::Internal(format!("Baseten provision request failed: {e}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AdapterError::Internal(format!(
                "Baseten provision returned {status}: {error_body}"
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            AdapterError::Internal(format!("Failed to parse Baseten provision response: {e}"))
        })?;

        let endpoint = json["endpoint_url"]
            .as_str()
            .unwrap_or("https://api.baseten.co/v1")
            .to_string();

        tracing::info!(
            target: "cns.adapter",
            endpoint_url = %endpoint,
            "Baseten endpoint provisioned"
        );
        Ok(endpoint)
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
                "BASETEN_API_KEY not set — cannot teardown endpoint. \
                 The endpoint may still be active and billing at Baseten."
                    .into(),
            ));
        }
        self.client
            .delete(endpoint_url)
            .header("Authorization", format!("Api-Key {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                AdapterError::Internal(format!(
                    "Baseten teardown HTTP request failed: {e}. \
                     Endpoint may need manual deletion via Baseten console."
                ))
            })?;
        tracing::info!(target: "cns.adapter", endpoint_url = %endpoint_url, "Baseten endpoint torn down");
        Ok(())
    }

    async fn upload_adapter(
        &self,
        _adapter: &TrainedLoRAAdapter,
        _config: &AdapterConfig,
    ) -> Result<String, AdapterError> {
        Err(AdapterError::ProviderUnavailable(
            "Baseten does not support direct LoRA adapter uploads. Adapters must be pre-built into the serving image."
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
