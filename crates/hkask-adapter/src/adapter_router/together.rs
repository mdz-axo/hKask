//! Together AI adapter backend — upload, provision, infer, teardown.
//!
//! Docs: <https://docs.together.ai/docs/dedicated-endpoints/adapter>

use super::AdapterProviderBackend;
use super::openai::openai_compatible_infer;
use crate::adapter_config::AdapterConfig;
use crate::adapter_port::AdapterError;
use crate::adapter_store::AdapterSource;
use crate::adapter_store::TrainedLoRAAdapter;
use crate::provider_cost::{CostModel, ProviderCapability};
use hkask_ports::InferenceResult;
use hkask_types::template::LLMParameters;

pub(super) struct TogetherAdapterBackend {
    cost_model: CostModel,
    capability: ProviderCapability,
    api_key: String,
    client: reqwest::Client,
}

impl TogetherAdapterBackend {
    pub(super) fn new() -> Self {
        let api_key = std::env::var("TG_API_KEY").unwrap_or_default();
        Self {
            cost_model: CostModel::together(),
            capability: ProviderCapability::together(),
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Poll Together AI fine-tune job until completed, then return model_name.
    async fn poll_until_complete(&self, job_id: &str) -> Result<String, AdapterError> {
        let max_attempts = 30;
        for attempt in 1..=max_attempts {
            let response = self
                .client
                .get(format!("https://api.together.ai/v1/jobs/{}", job_id))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await
                .map_err(|e| {
                    AdapterError::Internal(format!("Together AI poll request failed: {e}"))
                })?;

            let status_code = response.status();
            if !status_code.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(AdapterError::Internal(format!(
                    "Together AI poll returned {status_code}: {error_body}"
                )));
            }

            let json: serde_json::Value = response.json().await.map_err(|e| {
                AdapterError::Internal(format!("Failed to parse poll response: {e}"))
            })?;

            let status = json["status"].as_str().unwrap_or("unknown");
            match status {
                "completed" | "succeeded" => {
                    return Ok(json["model_name"]
                        .as_str()
                        .or_else(|| json["output_name"].as_str())
                        .unwrap_or("unknown")
                        .to_string());
                }
                "failed" | "error" | "cancelled" => {
                    return Err(AdapterError::Internal(format!(
                        "Together AI fine-tune job {job_id} {status}"
                    )));
                }
                _ => {
                    tracing::debug!(
                        target: "cns.adapter",
                        job_id = %job_id,
                        status = %status,
                        attempt = attempt,
                        "Together AI fine-tune job still pending"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            }
        }
        Err(AdapterError::Internal(format!(
            "Together AI fine-tune job {job_id} did not complete within {max_attempts} attempts"
        )))
    }
}

#[async_trait::async_trait]
impl AdapterProviderBackend for TogetherAdapterBackend {
    async fn provision_endpoint(
        &self,
        _adapter: &TrainedLoRAAdapter,
    ) -> Result<String, AdapterError> {
        Ok("https://api.together.ai/v1".to_string())
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
        let _ = endpoint_url;
        tracing::debug!(
            target: "cns.adapter",
            endpoint_url = %endpoint_url,
            "Together AI teardown — adapter auto-expires, no explicit deletion needed"
        );
        Ok(())
    }

    async fn upload_adapter(
        &self,
        adapter: &TrainedLoRAAdapter,
        config: &AdapterConfig,
    ) -> Result<String, AdapterError> {
        let AdapterSource::HuggingFace { ref repo } = adapter.source;
        let hf_repo = repo.clone();

        if self.api_key.is_empty() {
            return Err(AdapterError::Internal(
                "TG_API_KEY not set — cannot upload adapter. \
                 Set the environment variable and retry."
                    .into(),
            ));
        }

        let body = if let Ok(hf_token) = std::env::var("HF_TOKEN") {
            serde_json::json!({
                "model_source": hf_repo,
                "model_type": "adapter",
                "base_model": config.base_model_name_or_path,
                "hf_token": hf_token,
            })
        } else {
            serde_json::json!({
                "model_source": hf_repo,
                "model_type": "adapter",
                "base_model": config.base_model_name_or_path,
            })
        };

        tracing::info!(
            target: "cns.adapter",
            adapter_id = %adapter.id,
            hf_repo = %hf_repo,
            base_model = %config.base_model_name_or_path,
            "Calling Together AI adapter upload API"
        );

        let response = self
            .client
            .post("https://api.together.ai/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AdapterError::Internal(format!("Together AI upload request failed: {e}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AdapterError::Internal(format!(
                "Together AI upload returned {status}: {error_body}"
            )));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            AdapterError::Internal(format!("Failed to parse Together AI upload response: {e}"))
        })?;

        let job_id = response_json["id"]
            .as_str()
            .or_else(|| response_json["job_id"].as_str());

        let model_name = if let Some(jid) = job_id {
            tracing::info!(
                target: "cns.adapter",
                job_id = %jid,
                "Together AI upload async — polling for completion"
            );
            let model = self.poll_until_complete(jid).await?;
            tracing::info!(
                target: "cns.adapter",
                job_id = %jid,
                model_name = %model,
                "Together AI adapter upload completed"
            );
            model
        } else {
            response_json["model_name"]
                .as_str()
                .unwrap_or(response_json["output_name"].as_str().unwrap_or("unknown"))
                .to_string()
        };

        tracing::info!(
            target: "cns.adapter",
            adapter_id = %adapter.id,
            model_name = %model_name,
            "Adapter uploaded to Together AI"
        );

        Ok(model_name)
    }

    fn capability(&self) -> ProviderCapability {
        self.capability.clone()
    }

    fn cost_model(&self) -> CostModel {
        self.cost_model.clone()
    }
}
