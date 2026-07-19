//! Thinking Machines Tinker adapter backend — inference via Tinker's
//! OpenAI-compatible API.
//!
//! After training with `TinkerHost`, the adapter weights are saved to
//! Tinker's checkpoint store. This backend provisions an inference endpoint
//! via Tinker's OpenAI-compatible API (`/tinker/compatible-apis/openai/`)
//! and routes `infer()` calls through it.
//!
//! Unlike RunPod (serverless endpoint from a template) or Together AI
//! (dedicated endpoint), Tinker manages the GPU infrastructure for both
//! training AND inference — no pod provisioning, no SSH, no torch/CUDA
//! management. The adapter is referenced by its Tinker checkpoint name.
//!
//! Environment variables:
//! - `TINKER_API_KEY` — Thinking Machines Tinker API key

use super::AdapterProviderBackend;
use super::openai::openai_compatible_infer;
use crate::adapter_config::AdapterConfig;
use crate::adapter_port::AdapterError;
use crate::adapter_store::TrainedLoRAAdapter;
use crate::provider_cost::{CostModel, ProviderCapability};
use hkask_ports::InferenceResult;
use hkask_types::template::LLMParameters;

// Scaffolded Tinker adapter backend — not yet registered in the router
// (see adapter_router/mod.rs). `dead_code` allowed until the wiring lands.
#[allow(dead_code)]
pub(super) struct TinkerAdapterBackend {
    cost_model: CostModel,
    capability: ProviderCapability,
    api_key: String,
    client: reqwest::Client,
}

impl TinkerAdapterBackend {
    #[allow(dead_code)]
    pub(super) fn new() -> Self {
        Self {
            cost_model: CostModel::tinker(),
            capability: ProviderCapability::tinker(),
            api_key: std::env::var("TINKER_API_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl AdapterProviderBackend for TinkerAdapterBackend {
    async fn provision_endpoint(
        &self,
        adapter: &TrainedLoRAAdapter,
    ) -> Result<String, AdapterError> {
        if self.api_key.is_empty() {
            return Err(AdapterError::ProviderUnavailable(
                "TINKER_API_KEY not set".into(),
            ));
        }

        // Tinker's OpenAI-compatible API base URL.
        // The adapter is referenced by its checkpoint name (stored in
        // adapter.source.repository). Tinker loads it on first request
        // (lazy provisioning) — no explicit endpoint creation needed.
        let model_name = adapter.source.repository_id();

        let endpoint_url = format!("https://api.tinker.ai/v1/openai/{}", model_name);

        tracing::info!(
            target: "cns.adapter",
            adapter_id = %adapter.id,
            model_name = %model_name,
            "Tinker inference endpoint ready (lazy-provisioned)"
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

    async fn teardown(&self, _endpoint_url: &str) -> Result<(), AdapterError> {
        // Tinker manages endpoint lifecycle — no explicit teardown needed.
        // The inference endpoint scales to zero when idle.
        tracing::info!(target: "cns.adapter", "Tinker endpoint torn down (automatic)");
        Ok(())
    }

    async fn upload_adapter(
        &self,
        _adapter: &TrainedLoRAAdapter,
        _config: &AdapterConfig,
    ) -> Result<String, AdapterError> {
        // Adapters trained via TinkerHost are already in Tinker's checkpoint
        // store. For adapters trained elsewhere (RunPod/Together), the user
        // must download the adapter and upload it to Tinker via the CLI:
        //   tinker checkpoint upload <path>
        Err(AdapterError::ProviderUnavailable(
            "Tinker adapters are stored in Tinker's checkpoint store after training. \
             For external adapters, use `tinker checkpoint upload <path>` to import them."
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
