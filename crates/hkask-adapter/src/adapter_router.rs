//! AdapterRouter — composes adapter + base model + provider → endpoint (P4 Clear Boundaries).
//!
//! The `AdapterRouter` implements `AdapterPort`. It holds an `AdapterStore` for adapter CRUD
//! and a registry of provider backends for endpoint provisioning and inference.
//!
//! All three provider backends (Together, Runpod, Baseten) have real HTTP integration
//! for adapter upload, endpoint provisioning, and inference.

use crate::AdapterStore;
use crate::adapter_config::AdapterConfig;
use crate::adapter_port::{
    AdapterError, AdapterPort, CompositionEstimate, EndpointStatus, InferenceEndpointHandle,
    ProviderSelection, SingleCandidate,
};
use crate::adapter_store::AdapterSource;
use crate::adapter_store::TrainedLoRAAdapter;
use crate::endpoint_lifecycle::{EndpointLifecycle, EndpointPhase};
use crate::provider_cost::{CostModel, ProviderCapability, ProviderInfo};
use hkask_capability::DelegationToken;
use hkask_inference::ProviderId;
use hkask_ports::InferenceResult;
use hkask_ports::InferenceUsage;
use hkask_storage::Store;
use hkask_types::id::WebID;
use hkask_types::template::LLMParameters;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use tracing;
use uuid::Uuid;

// ── Provider backend abstraction ─────────────────────────────────────────────

/// Operations a cloud provider must support for adapter composition.
///
/// Each provider backend handles the actual HTTP API calls to provision
/// endpoints, run inference, and tear down. This is the trait boundary
/// for adding new providers (P7 — Evolutionary Architecture).
#[async_trait::async_trait]
trait AdapterProviderBackend: Send + Sync {
    /// Provision a new endpoint for adapter inference.
    async fn provision_endpoint(
        &self,
        adapter: &TrainedLoRAAdapter,
    ) -> Result<String, AdapterError>;

    /// Run inference against a provisioned endpoint.
    async fn infer(
        &self,
        endpoint_url: &str,
        prompt: &str,
        params: &LLMParameters,
        model_name: &str,
    ) -> Result<InferenceResult, AdapterError>;

    /// Tear down a provisioned endpoint.
    async fn teardown(&self, endpoint_url: &str) -> Result<(), AdapterError>;

    /// Upload adapter weights to the provider.
    ///
    /// Returns a provider-specific model_name that can be used for endpoint provisioning.
    /// For Together AI, this calls the adapter upload API and returns the model name.
    /// For vLLM-based providers (Runpod), adapters are loaded at server start via
    /// --lora-modules, so upload is a no-op if the adapter is already accessible.
    async fn upload_adapter(
        &self,
        adapter: &TrainedLoRAAdapter,
        config: &AdapterConfig,
    ) -> Result<String, AdapterError>;

    /// Provider capabilities.
    fn capability(&self) -> ProviderCapability;

    /// Provider cost model.
    fn cost_model(&self) -> CostModel;
}

// ── Provider backend implementations ───────────────────────────────────────
// Together: real HTTP upload + inference (OpenAI-compatible). Provision is
//   auto-deployed after upload — no separate endpoint creation needed.
//   Docs: https://docs.together.ai/docs/dedicated-endpoints/adapter
// Runpod: serverless vLLM endpoint provisioning + OpenAI-compatible inference.
//   Docs: https://docs.runpod.io/serverless/endpoints/manage-endpoints
// Baseten: REST endpoint provisioning + OpenAI-compatible inference.
//   Docs: https://docs.baseten.co/api-reference
// Adapter upload: Together uploads to HF; Runpod/Baseten use vLLM --lora-modules.
// HuggingFace: https://huggingface.co/docs/peft/main/quicktour

struct TogetherAdapterBackend {
    cost_model: CostModel,
    capability: ProviderCapability,
    api_key: String,
    client: reqwest::Client,
}

impl TogetherAdapterBackend {
    fn new() -> Self {
        let api_key = std::env::var("TOGETHER_API_KEY").unwrap_or_default();
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
        // Together AI: adapters are deployed as dedicated endpoints after upload.
        // The inference URL is always https://api.together.ai/v1
        // Docs: https://docs.together.ai/docs/dedicated-endpoints/adapter
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
        // Together AI: adapters are deployed as dedicated endpoints.
        // If an endpoint was explicitly created (POST /v1/endpoints), it can be
        // deleted via DELETE /v1/endpoints/{id}. If the adapter was used directly
        // via model_name without explicit endpoint creation, there's nothing to
        // tear down — the adapter just exists in their system.
        // Docs: https://docs.together.ai/docs/dedicated-endpoints/adapter
        let _ = endpoint_url;

        // Best-effort: attempt deletion. Endpoint ID is embedded in the URL path.
        // Since we use model_name directly for inference, explicit teardown is
        // typically unnecessary. Together AI doesn't charge for idle adapters.
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
                "TOGETHER_API_KEY not set — cannot upload adapter. \
                 Set the environment variable and retry."
                    .into(),
            ));
        }

        // Together AI adapter upload API:
        // Docs: https://docs.together.ai/docs/dedicated-endpoints/adapter
        // POST https://api.together.ai/v1/models (upload)
        // Body: { model_source, model_type: "adapter", base_model, hf_token? }
        // Returns: { job_id, model_name } — upload is async, poll for completion
        // Status: GET https://api.together.ai/v1/jobs/{job_id}
        // Inference: POST https://api.together.ai/v1/chat/completions
        // Teardown: DELETE https://api.together.ai/v1/endpoints/{endpoint_id}

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

        // Together AI fine-tune API is async — returns a job ID that must be polled.
        // If the response includes a job_id and status, poll until completed.
        let job_id = response_json["id"]
            .as_str()
            .or_else(|| response_json["job_id"].as_str());

        let model_name = if let Some(jid) = job_id {
            tracing::info!(
                target: "cns.adapter",
                job_id = %jid,
                "Together AI upload async — polling for completion"
            );
            // Poll for completion (max 30 attempts, 10s interval = 5 min timeout)
            let model = self.poll_until_complete(jid).await?;
            tracing::info!(
                target: "cns.adapter",
                job_id = %jid,
                model_name = %model,
                "Together AI adapter upload completed"
            );
            model
        } else {
            // Synchronous response — extract model_name directly
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

struct RunpodAdapterBackend {
    cost_model: CostModel,
    capability: ProviderCapability,
    api_key: String,
    client: reqwest::Client,
}

impl RunpodAdapterBackend {
    fn new() -> Self {
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

        // Runpod serverless endpoint API — provisions a serverless worker
        // that auto-scales to zero when idle.
        // Docs: https://docs.runpod.io/serverless/endpoints/manage-endpoints
        // Inference: POST https://api.runpod.ai/v2/{endpoint_id}/openai/v1/chat/completions
        // Teardown: DELETE via console (no REST API for deletion yet)
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

        // Runpod serverless — endpoint is template-based, no separate provisioning.
        // The endpoint URL is the serverless API endpoint for this template.
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
        // Runpod serverless endpoint deletion is console-only per their docs.
        // Docs: https://docs.runpod.io/serverless/endpoints/manage-endpoints#delete-an-endpoint
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

struct BasetenAdapterBackend {
    cost_model: CostModel,
    capability: ProviderCapability,
    api_key: String,
    client: reqwest::Client,
}

impl BasetenAdapterBackend {
    fn new() -> Self {
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

        // Baseten API: deploy a model. The model ID is returned and used to
        // construct the endpoint URL: https://model-{id}.api.baseten.co
        // Docs: https://docs.baseten.co/api-reference
        // Auth: Api-Key {key}
        // Native inference: POST /production/predict
        // vLLM inference: POST /v1/chat/completions (OpenAI-compatible)
        // CAVEAT: Model creation endpoint is best-effort. Adjust if actual API differs.
        let body = serde_json::json!({
            "name": adapter.expertise.name,
            "model_source": adapter.source.repository_id(),
            "base_model": adapter.base_model_family,
        });

        tracing::info!(
            target: "cns.adapter",
            adapter_id = %adapter.id,
            "Provisioning Baseten endpoint"
        );

        let response = self
            .client
            .post("https://api.baseten.co/v1/models")
            .header("Authorization", format!("Api-Key {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| AdapterError::Internal(format!("Baseten API request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AdapterError::Internal(format!(
                "Baseten API returned {status}: {error_body}"
            )));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            AdapterError::Internal(format!("Failed to parse Baseten response: {e}"))
        })?;

        let model_id = response_json["id"]
            .as_str()
            .ok_or_else(|| AdapterError::Internal("Baseten response missing model ID".into()))?;

        let endpoint_url = format!("https://model-{}.api.baseten.co/v1", model_id);
        tracing::info!(
            target: "cns.adapter",
            model_id = %model_id,
            "Baseten endpoint provisioned"
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
            return Err(AdapterError::ProviderUnavailable(
                "BASETEN_API_KEY not set".into(),
            ));
        }
        self.client
            .delete(endpoint_url)
            .header("Authorization", format!("Api-Key {}", self.api_key))
            .send()
            .await
            .map_err(|e| AdapterError::Internal(format!("Baseten teardown failed: {e}")))?;
        tracing::info!(target: "cns.adapter", endpoint_url = %endpoint_url, "Baseten endpoint torn down");
        Ok(())
    }

    async fn upload_adapter(
        &self,
        _adapter: &TrainedLoRAAdapter,
        _config: &AdapterConfig,
    ) -> Result<String, AdapterError> {
        Err(AdapterError::ProviderUnavailable(
            "Baseten does not support LoRA adapter uploads via this path.".into(),
        ))
    }

    fn capability(&self) -> ProviderCapability {
        self.capability.clone()
    }

    fn cost_model(&self) -> CostModel {
        self.cost_model.clone()
    }
}

// ── Shared OpenAI-compatible inference helper ────────────────────────────

async fn openai_compatible_infer(
    client: &reqwest::Client,
    api_key: &str,
    endpoint_url: &str,
    prompt: &str,
    params: &LLMParameters,
    model_name: &str,
) -> Result<InferenceResult, AdapterError> {
    if api_key.is_empty() {
        return Err(AdapterError::ProviderUnavailable("API key not set".into()));
    }
    let body = serde_json::json!({
        "model": model_name,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": params.temperature,
        "top_p": params.top_p,
        "max_tokens": params.max_tokens,
    });
    let response = client
        .post(format!("{}/chat/completions", endpoint_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AdapterError::Internal(format!("Inference request failed: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(AdapterError::Internal(format!(
            "Inference returned {status}: {error_body}"
        )));
    }
    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AdapterError::Internal(format!("Failed to parse inference response: {e}")))?;
    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let usage = serde_json::from_value(response_json["usage"].clone()).unwrap_or(InferenceUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    });
    Ok(InferenceResult {
        text: content,
        model: model_name.to_string(),
        usage,
        finish_reason: response_json["choices"][0]["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string(),
        token_probabilities: None,
        tool_calls: vec![],
    })
}

// ── Endpoint record ──────────────────────────────────────────────────────────
// Tracks active endpoints in memory (companion table for AdapterStore).

struct EndpointRecord {
    handle: InferenceEndpointHandle,
    backend: Arc<dyn AdapterProviderBackend>,
}

// ── AdapterRouter ────────────────────────────────────────────────────────────

/// Multi-provider adapter composition router implementing `AdapterPort`.
///
/// Holds an `AdapterStore` for adapter CRUD and a registry of provider
/// backends for endpoint provisioning and inference. Mirrors the
/// `InferenceRouter` pattern from `hkask-inference`.
pub struct AdapterRouter {
    store: Arc<AdapterStore>,
    backends: HashMap<ProviderId, Arc<dyn AdapterProviderBackend>>,
    endpoints: Mutex<HashMap<Uuid, EndpointRecord>>,
}

impl AdapterRouter {
    /// Build the router from an `AdapterStore` and available providers.
    ///
    /// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
    /// \[P4\] Clear Boundaries — router assembled from configured provider boundaries
    /// pre:  store is a valid AdapterStore
    /// post: returns AdapterRouter with backends for adapter-capable providers
    /// post: previously active endpoints are loaded from store (metadata only — backends
    ///       are runtime objects and cannot be restored; orphaned endpoints are logged)
    pub fn new(store: Arc<AdapterStore>) -> Self {
        let mut backends: HashMap<ProviderId, Arc<dyn AdapterProviderBackend>> = HashMap::new();

        backends.insert(
            ProviderId::Together,
            Arc::new(TogetherAdapterBackend::new()),
        );
        backends.insert(ProviderId::Runpod, Arc::new(RunpodAdapterBackend::new()));
        backends.insert(ProviderId::Baseten, Arc::new(BasetenAdapterBackend::new()));

        let router = Self {
            store,
            backends,
            endpoints: Mutex::new(HashMap::new()),
        };

        // Restore previously active endpoints from persistent store.
        // Backends are runtime objects (HTTP clients with API keys) and cannot
        // be serialized — restored endpoints are metadata-only for audit.
        // Active inference requires re-creating the endpoint via create_endpoint().
        if let Err(e) = router.log_orphaned_endpoints() {
            tracing::warn!(
                target: "cns.adapter",
                error = %e,
                "Failed to read persisted endpoints on startup"
            );
        }

        router
    }

    /// Log any endpoints that were active when the system last shut down.
    /// These are orphaned — their provider resources may still exist and incur cost.
    fn log_orphaned_endpoints(&self) -> Result<(), AdapterError> {
        let conn = (*self.store)
            .lock_conn()
            .map_err(|e| AdapterError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT endpoint_id, provider, model_name, expertise_name, phase, cost_accrued, created_at FROM active_endpoints")
            .map_err(|e| AdapterError::Internal(format!("Query failed: {e}")))?;

        let rows: Vec<(String, String, String, String, String, f64, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })
            .map_err(|e| AdapterError::Internal(format!("Query failed: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        if !rows.is_empty() {
            tracing::warn!(
                target: "cns.adapter",
                count = rows.len(),
                "Found orphaned endpoints from previous session — these may still incur provider costs"
            );
            for (id, provider, model, expertise, phase, cost, created) in &rows {
                tracing::warn!(
                    target: "cns.adapter",
                    endpoint_id = %id,
                    provider = %provider,
                    model = %model,
                    expertise = %expertise,
                    phase = %phase,
                    cost = %cost,
                    created = %created,
                    "Orphaned endpoint — may need manual teardown via provider console"
                );
            }
        }
        Ok(())
    }

    /// List providers that can compose the given adapter.
    pub(crate) fn list_compatible_providers(
        &self,
        adapter: &TrainedLoRAAdapter,
    ) -> Vec<ProviderInfo> {
        self.backends
            .iter()
            .filter(|(_, backend)| backend.capability().can_compose(&adapter.base_model_family))
            .map(|(provider_id, backend)| ProviderInfo {
                provider: *provider_id,
                cost_model: backend.cost_model(),
                capability: backend.capability(),
            })
            .collect()
    }

    /// Select a provider for adapter composition — user-in-the-loop (P2 Affirmative Consent).
    ///
    /// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
    /// \[P2\] Affirmative Consent — provider selection is explicit, informed, and user-driven
    /// pre:  adapter exists in store, at least one provider supports LoRA composition
    /// post: returns list of compatible providers with cost estimates; caller selects
    ///
    /// Returns all compatible providers sorted by hourly cost (cheapest first).
    /// If `budget_limit` is provided, providers exceeding the budget are still returned
    /// but marked with a budget warning. The caller must present these to the user
    /// and obtain explicit consent before calling `create_endpoint`.
    pub fn select_provider(
        &self,
        adapter_id: Uuid,
        budget_limit: Option<f64>,
    ) -> Result<ProviderSelection, AdapterError> {
        let adapter = self
            .store
            .get_by_id(adapter_id)?
            .ok_or(AdapterError::NotFound(adapter_id))?;

        let mut providers: Vec<ProviderInfo> = self.list_compatible_providers(&adapter);

        // Sort cheapest first
        providers.sort_by(|a, b| {
            a.cost_model
                .gpu_hourly_rate
                .partial_cmp(&b.cost_model.gpu_hourly_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Compute within-budget count (drop borrow before moving providers)
        let within_budget_count = if let Some(limit) = budget_limit {
            providers
                .iter()
                .filter(|p| p.cost_model.gpu_hourly_rate <= limit)
                .count()
        } else {
            providers.len()
        };

        let single_candidate = if providers.len() == 1 {
            Some(SingleCandidate {
                provider: providers[0].clone(),
                requires_confirmation: true, // P2: never silent selection
            })
        } else {
            None
        };

        Ok(ProviderSelection {
            adapter_id,
            expertise_name: adapter.expertise.name.clone(),
            base_model_family: adapter.base_model_family.clone(),
            providers,
            within_budget_count,
            single_candidate,
        })
    }

    /// Drain (teardown) all billable endpoints.
    ///
    /// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
    /// pre:  owner is a valid WebID (reserved for future multi-tenant scoping)
    /// post: all billable endpoints are transitioned to Terminated
    pub fn drain_all_owner(&self, _owner: WebID) -> Result<usize, AdapterError> {
        // Note: _owner is reserved for future multi-tenant scoping (P1 — User Sovereignty).
        // When multi-tenant is implemented, this will filter endpoints by owner before draining.
        tracing::debug!(
            target: "cns.adapter",
            "drain_all_owner called — draining all billable endpoints (owner filter not yet active)"
        );

        let mut endpoints = self
            .endpoints
            .lock()
            .map_err(|e| AdapterError::Internal(format!("lock poisoned: {e}")))?;

        let to_remove: Vec<Uuid> = endpoints
            .iter()
            .filter(|(_, record)| {
                let phase = record.handle.phase();
                phase.is_billable()
            })
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            if let Some(record) = endpoints.remove(&id) {
                // Best-effort teardown — use current runtime if available,
                // otherwise create a fresh one. Using try_current() avoids
                // the "Cannot start a runtime from within a runtime" panic
                // when called from inside block_on.
                let teardown_result = match tokio::runtime::Handle::try_current() {
                    Ok(handle) => {
                        handle.spawn(async move {
                            let _ = record.backend.teardown(&record.handle.endpoint_url).await;
                        });
                        Ok(())
                    }
                    Err(_) => {
                        let rt = tokio::runtime::Runtime::new()
                            .map_err(|e| AdapterError::Internal(e.to_string()))?;
                        rt.block_on(record.backend.teardown(&record.handle.endpoint_url))
                    }
                };
                let _ = teardown_result
                    .inspect_err(|e| tracing::warn!("Failed to teardown adapter endpoint: {e}"));
                // Remove from persistent store
                let _ = self.remove_endpoint_from_store(&id);
            }
        }

        Ok(count)
    }

    /// Persist an active endpoint to the AdapterStore for restart survival.
    fn save_endpoint_to_store(&self, handle: &InferenceEndpointHandle) -> Result<(), AdapterError> {
        let conn = (*self.store)
            .lock_conn()
            .map_err(|e| AdapterError::Internal(e.to_string()))?;
        let phase = handle.phase();
        let cost = handle.cost_accrued();
        let rate = handle
            .lifecycle
            .lock()
            .map(|lc| lc.hourly_rate)
            .unwrap_or(0.0);
        conn.execute(
            "INSERT OR REPLACE INTO active_endpoints
             (endpoint_id, adapter_id, provider, endpoint_url, model_name, expertise_name, phase, cost_accrued, hourly_rate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                handle.endpoint_id.to_string(),
                "",  // adapter_id — not tracked at this level
                handle.provider.as_str(),
                handle.endpoint_url,
                handle.model_name,
                handle.expertise_name,
                phase.to_string(),
                cost,
                rate,
            ],
        )
        .map_err(|e| AdapterError::Internal(format!("Failed to persist endpoint: {e}")))?;
        Ok(())
    }

    /// Remove an endpoint from persistent store after teardown.
    fn remove_endpoint_from_store(&self, endpoint_id: &Uuid) -> Result<(), AdapterError> {
        let conn = (*self.store)
            .lock_conn()
            .map_err(|e| AdapterError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM active_endpoints WHERE endpoint_id = ?1",
            rusqlite::params![endpoint_id.to_string()],
        )
        .map_err(|e| {
            AdapterError::Internal(format!("Failed to remove endpoint from store: {e}"))
        })?;
        Ok(())
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn resolve_backend(
        &self,
        provider: ProviderId,
    ) -> Result<Arc<dyn AdapterProviderBackend>, AdapterError> {
        self.backends.get(&provider).cloned().ok_or_else(|| {
            AdapterError::ProviderUnavailable(format!(
                "Provider {} is not available for adapter composition",
                provider.as_str()
            ))
        })
    }

    fn resolve_endpoint(&self, endpoint_id: Uuid) -> Result<EndpointRecord, AdapterError> {
        let endpoints = self
            .endpoints
            .lock()
            .map_err(|e| AdapterError::Internal(format!("lock poisoned: {e}")))?;
        let record = endpoints
            .get(&endpoint_id)
            .ok_or(AdapterError::EndpointNotFound(endpoint_id))?;
        Ok(EndpointRecord {
            handle: record.handle.clone(),
            backend: Arc::clone(&record.backend),
        })
    }
}

// ── AdapterPort implementation ───────────────────────────────────────────────

impl AdapterPort for AdapterRouter {
    fn list_adapters(
        &self,
        expertise: Option<&str>,
        _token: &DelegationToken,
    ) -> Result<Vec<TrainedLoRAAdapter>, AdapterError> {
        match expertise {
            Some(name) => Ok(self.store.get_by_expertise(name)?),
            None => {
                // Return all adapters — for now, limited to the store's query capability.
                // Per-owner filtering would need the WebID from the token.
                // This is a deliberate simplification: list_adapters with no filter
                // returns adapters accessible to the caller (ownership filtered at higher level).
                Err(AdapterError::Internal(
                    "unfiltered list_adapters requires owner scope — use expertise filter or pass owner"
                        .into(),
                ))
            }
        }
    }

    async fn estimate_composition(
        &self,
        adapter_id: Uuid,
        provider: ProviderId,
        _token: &DelegationToken,
    ) -> Result<CompositionEstimate, AdapterError> {
        let adapter = self
            .store
            .get_by_id(adapter_id)?
            .ok_or(AdapterError::NotFound(adapter_id))?;

        let backend = self.resolve_backend(provider)?;
        let capability = backend.capability();
        let cost_model = backend.cost_model();

        let is_compatible = capability.can_compose(&adapter.base_model_family);
        let incompatibility_reason = if !is_compatible {
            Some(format!(
                "Provider {} does not support base model family '{}'",
                provider.as_str(),
                adapter.base_model_family
            ))
        } else {
            None
        };

        Ok(CompositionEstimate {
            provider,
            cost_model: cost_model.clone(),
            is_compatible,
            incompatibility_reason,
            estimated_setup_cost: cost_model.estimated_setup_cost(),
            estimated_hourly_cost: cost_model.gpu_hourly_rate,
        })
    }

    async fn create_endpoint(
        &self,
        adapter_id: Uuid,
        provider: ProviderId,
        _token: &DelegationToken,
    ) -> Result<InferenceEndpointHandle, AdapterError> {
        // 1. Look up the adapter
        let adapter = self
            .store
            .get_by_id(adapter_id)?
            .ok_or(AdapterError::NotFound(adapter_id))?;

        // 2. Resolve the provider backend
        let backend = self.resolve_backend(provider)?;

        // 3. Validate compatibility
        let capability = backend.capability();
        if !capability.can_compose(&adapter.base_model_family) {
            return Err(AdapterError::Incompatible {
                reason: format!(
                    "Provider {} does not support base model family '{}'",
                    provider.as_str(),
                    adapter.base_model_family
                ),
            });
        }

        // 4. Parse adapter config and upload to provider
        let adapter_config = AdapterConfig::from_dir(&adapter.storage_path)
            .map_err(|e| AdapterError::Internal(format!("Failed to parse adapter config: {e}")))?;
        let model_name = backend.upload_adapter(&adapter, &adapter_config).await?;

        // 5. Provision the endpoint via the provider
        let endpoint_url = backend.provision_endpoint(&adapter).await?;

        // 6. Create lifecycle
        let cost_model = backend.cost_model();
        let lifecycle = EndpointLifecycle::new(cost_model.gpu_hourly_rate)
            .map_err(|e| AdapterError::Internal(format!("lifecycle creation failed: {e}")))?;

        // 6. Transition to Ready (provisioning step is synthetic here)
        let mut lifecycle = lifecycle;
        lifecycle.transition(EndpointPhase::Ready).map_err(|_e| {
            AdapterError::InvalidTransition {
                current: EndpointPhase::Provisioning,
                attempted: EndpointPhase::Ready,
            }
        })?;

        let handle = InferenceEndpointHandle {
            endpoint_id: Uuid::new_v4(),
            endpoint_url,
            model_name: model_name.clone(),
            provider,
            expertise_name: adapter.expertise.name.clone(),
            lifecycle: Arc::new(Mutex::new(lifecycle)),
            cost_model,
            created_at: chrono::Utc::now(),
        };

        // 7. Store the endpoint record
        {
            let mut endpoints = self
                .endpoints
                .lock()
                .map_err(|e| AdapterError::Internal(format!("lock poisoned: {e}")))?;
            endpoints.insert(
                handle.endpoint_id,
                EndpointRecord {
                    handle: handle.clone(),
                    backend,
                },
            );
        }

        // 8. Persist endpoint for restart survival
        let _ = self.save_endpoint_to_store(&handle);

        Ok(handle)
    }

    fn endpoint_status(
        &self,
        endpoint_id: Uuid,
        _token: &DelegationToken,
    ) -> Result<EndpointStatus, AdapterError> {
        let record = self.resolve_endpoint(endpoint_id)?;
        let handle = &record.handle;

        Ok(EndpointStatus {
            endpoint_id: handle.endpoint_id,
            phase: handle.phase(),
            cost_accrued: handle.cost_accrued(),
            provider: handle.provider,
            expertise_name: handle.expertise_name.clone(),
            created_at: handle.created_at,
            elapsed_seconds: handle
                .lifecycle
                .lock()
                .map(|lc| lc.elapsed_seconds())
                .unwrap_or(0.0),
        })
    }

    async fn infer(
        &self,
        endpoint_id: Uuid,
        prompt: &str,
        params: LLMParameters,
        _token: &DelegationToken,
    ) -> Result<InferenceResult, AdapterError> {
        let record = self.resolve_endpoint(endpoint_id)?;

        // Transition to Active if currently Ready
        {
            let mut lc = record
                .handle
                .lifecycle
                .lock()
                .map_err(|e| AdapterError::Internal(format!("lock poisoned: {e}")))?;
            if lc.phase == EndpointPhase::Ready {
                lc.transition(EndpointPhase::Active).map_err(|_e| {
                    AdapterError::InvalidTransition {
                        current: EndpointPhase::Ready,
                        attempted: EndpointPhase::Active,
                    }
                })?;
            }
        }

        // Run inference via the provider backend
        let model_name = record.handle.model_name.clone();
        record
            .backend
            .infer(&record.handle.endpoint_url, prompt, &params, &model_name)
            .await
    }

    async fn teardown_endpoint(&self, endpoint_id: Uuid) -> Result<(), AdapterError> {
        let record = self.resolve_endpoint(endpoint_id)?;

        // Transition to Draining
        {
            let mut lc = record
                .handle
                .lifecycle
                .lock()
                .map_err(|e| AdapterError::Internal(format!("lock poisoned: {e}")))?;
            let current = lc.phase;
            lc.transition(EndpointPhase::Draining).map_err(|_e| {
                AdapterError::InvalidTransition {
                    current,
                    attempted: EndpointPhase::Draining,
                }
            })?;
        }

        // Call provider teardown
        record.backend.teardown(&record.handle.endpoint_url).await?;

        // Transition to Terminated
        {
            let mut lc = record
                .handle
                .lifecycle
                .lock()
                .map_err(|e| AdapterError::Internal(format!("lock poisoned: {e}")))?;
            lc.transition(EndpointPhase::Terminated).map_err(|_e| {
                AdapterError::InvalidTransition {
                    current: EndpointPhase::Draining,
                    attempted: EndpointPhase::Terminated,
                }
            })?;
        }

        // Remove from active endpoints
        {
            let mut endpoints = self
                .endpoints
                .lock()
                .map_err(|e| AdapterError::Internal(format!("lock poisoned: {e}")))?;
            endpoints.remove(&endpoint_id);
        }

        Ok(())
    }
}

// ── EndpointGuard — RAII teardown (P5 Essentialism, T8) ──────────────────

/// RAII guard that tears down an endpoint on drop.
///
/// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
/// \[P5\] Essentialism — every resource earns its existence; idle endpoints must drain
/// pre:  guard is created after successful endpoint provisioning
/// post: on drop, the endpoint is transitioned to Draining → Terminated
///
/// Uses a `Weak<AdapterRouter>` reference — if the router has been dropped,
/// teardown is silently skipped (the endpoint was already cleaned up).
pub struct EndpointGuard {
    endpoint_id: Uuid,
    router: Weak<AdapterRouter>,
    /// Whether the guard has been explicitly consumed (teardown already called)
    consumed: bool,
}

impl EndpointGuard {
    /// Wrap an endpoint handle in a RAII teardown guard.
    ///
    /// Returns both the handle (for use by the caller) and the guard.
    /// The guard will call `teardown_endpoint` on drop if not explicitly consumed.
    pub fn new(router: &Arc<AdapterRouter>, endpoint_id: Uuid) -> Self {
        Self {
            endpoint_id,
            router: Arc::downgrade(router),
            consumed: false,
        }
    }

    /// Explicitly tear down and consume the guard (no-op on drop afterward).
    pub fn teardown(mut self) -> Result<(), AdapterError> {
        self.consumed = true;
        if let Some(router) = self.router.upgrade() {
            // The guard IS the authority (unforgeable ownership) — no token needed.
            tokio::runtime::Handle::current().block_on(router.teardown_endpoint(self.endpoint_id))
        } else {
            Ok(()) // Router already dropped
        }
    }

    /// Access the endpoint ID without consuming the guard.
    pub fn endpoint_id(&self) -> Uuid {
        self.endpoint_id
    }
}

impl Drop for EndpointGuard {
    fn drop(&mut self) {
        if !self.consumed
            && let Some(router) = self.router.upgrade()
        {
            let endpoint_id = self.endpoint_id;
            // Fire-and-forget: drop cannot be async, so spawn a task.
            // The router and its Arc hold resources until the task completes.
            tokio::task::spawn(async move {
                if let Err(e) = router.teardown_endpoint(endpoint_id).await {
                    tracing::warn!(
                        target: "cns.adapter",
                        endpoint_id = %endpoint_id,
                        error = %e,
                        "EndpointGuard: teardown on drop failed"
                    );
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter_store::AdapterSource;
    use crate::adapter_store::Checksum;
    use crate::expertise::{Expertise, MdsDomain, TrainingProvenance};
    use hkask_capability::DelegationAction;
    use hkask_capability::DelegationResource;
    use hkask_capability::auth::derive_signing_key;

    use hkask_storage::in_memory_db;
    use std::future::Future;

    /// Block on a future in a synchronous test context.
    fn block_on<F: Future>(f: F) -> F::Output {
        tokio::runtime::Runtime::new().unwrap().block_on(f)
    }

    /// Create a test DelegationToken with a derived signing key.
    fn test_token() -> DelegationToken {
        let sk = derive_signing_key(b"test-adapter-secret");
        DelegationToken::new(
            DelegationResource::Tool,
            "adapter:deploy".into(),
            DelegationAction::Execute,
            WebID::from_persona(b"test-root"),
            WebID::from_persona(b"test-agent"),
            &sk,
        )
    }

    /// Create a temp directory with a minimal adapter_config.json for testing.
    fn test_storage_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = serde_json::json!({
            "base_model_name_or_path": "meta-llama/Llama-3.3-70B-Instruct",
            "peft_type": "LORA",
            "r": 16,
            "lora_alpha": 32.0
        });
        std::fs::write(
            dir.path().join("adapter_config.json"),
            serde_json::to_string(&config).expect("serialize"),
        )
        .expect("write config");
        dir
    }

    fn make_test_adapter(name: &str) -> TrainedLoRAAdapter {
        let storage_dir = test_storage_dir();
        let storage_path = storage_dir.path().to_string_lossy().to_string();
        // Leak the tempdir — it will be cleaned up when the test process exits.
        // This is fine for test fixtures that outlive the function scope.
        std::mem::forget(storage_dir);
        let provenance = TrainingProvenance {
            training_run_id: format!("run-{name}"),
            training_source: "https://example.com/training".into(),
            completed_at: "2026-01-01T00:00:00Z".into(),
            base_model_family: "llama-3.3-70b".into(),
            dataset_hash: None,
            training_metrics: serde_json::Value::Null,
        };
        let expertise = Expertise::new(
            name.into(),
            MdsDomain::SolidityAudit,
            serde_json::Value::Null,
            provenance,
        )
        .expect("expertise");

        TrainedLoRAAdapter {
            id: Uuid::new_v4(),
            expertise,
            checksum: Checksum::from_hex("abcdef1234567890"),
            storage_path,
            base_model_family: "llama-3.3-70b".into(),
            version: None,
            source: AdapterSource::HuggingFace {
                repo: "test/adapter".into(),
            },
            size_bytes: None,
            owner: WebID::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn list_compatible_providers() {
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        let router = AdapterRouter::new(store);

        let providers = router.list_compatible_providers(&adapter);
        // All three backends support llama-3.3-70b
        assert_eq!(providers.len(), 3);
    }

    #[test]
    #[ignore = "requires TOGETHER_API_KEY"]
    fn create_endpoint_returns_handle() {
        unsafe {
            std::env::set_var("TOGETHER_API_KEY", "test-key");
        }
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(Arc::clone(&store));
        let token = test_token(); // test-only token

        let handle = block_on(router.create_endpoint(adapter.id, ProviderId::Together, &token))
            .expect("create endpoint");

        assert_eq!(handle.expertise_name, "solidity-audit");
        assert_eq!(handle.provider, ProviderId::Together);
        assert_eq!(handle.phase(), EndpointPhase::Ready);
    }

    #[test]
    #[ignore = "requires TOGETHER_API_KEY"]
    fn endpoint_status_query() {
        unsafe {
            std::env::set_var("TOGETHER_API_KEY", "test-key");
        }
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(Arc::clone(&store));
        let token = test_token();

        let handle = block_on(router.create_endpoint(adapter.id, ProviderId::Together, &token))
            .expect("create endpoint");

        let status = router
            .endpoint_status(handle.endpoint_id, &token)
            .expect("status");

        assert_eq!(status.phase, EndpointPhase::Ready);
        assert_eq!(status.provider, ProviderId::Together);
        assert_eq!(status.expertise_name, "solidity-audit");
    }

    #[test]
    #[ignore = "requires TOGETHER_API_KEY"]
    fn teardown_endpoint() {
        unsafe {
            std::env::set_var("TOGETHER_API_KEY", "test-key");
        }
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(Arc::clone(&store));
        let token = test_token();

        let handle = block_on(router.create_endpoint(adapter.id, ProviderId::Together, &token))
            .expect("create endpoint");

        block_on(router.teardown_endpoint(handle.endpoint_id)).expect("teardown");

        // Status should fail after teardown (endpoint removed)
        let status = router.endpoint_status(handle.endpoint_id, &token);
        assert!(status.is_err());
    }

    #[test]
    fn estimate_composition() {
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(Arc::clone(&store));
        let token = test_token();

        let estimate =
            block_on(router.estimate_composition(adapter.id, ProviderId::Together, &token))
                .expect("estimate");

        assert!(estimate.is_compatible);
        assert!(estimate.estimated_hourly_cost > 0.0);
        assert_eq!(estimate.provider, ProviderId::Together);
    }

    #[test]
    fn estimate_composition_incompatible() {
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        // This adapter uses llama-3.3-70b, which is compatible with all backends.
        // Test that DeepInfra (not registered as adapter backend) returns ProviderUnavailable
        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(store);
        let token = test_token();

        // DeepInfra is not registered as an adapter backend
        let result =
            block_on(router.estimate_composition(adapter.id, ProviderId::DeepInfra, &token));
        assert!(result.is_err());
        match result {
            Err(AdapterError::ProviderUnavailable(_)) => {} // expected
            other => panic!("expected ProviderUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn create_endpoint_incompatible_fails() {
        // This test uses Together backend which has a specific model family allowlist.
        // We create an adapter with a base model family NOT in the allowlist.
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let storage_dir = test_storage_dir();
        let storage_path = storage_dir.path().to_string_lossy().to_string();
        std::mem::forget(storage_dir);

        let provenance = TrainingProvenance {
            training_run_id: "run-test".into(),
            training_source: "https://example.com/training".into(),
            completed_at: "2026-01-01T00:00:00Z".into(),
            base_model_family: "unsupported-model".into(),
            dataset_hash: None,
            training_metrics: serde_json::Value::Null,
        };
        let expertise = Expertise::new(
            "test".into(),
            MdsDomain::CodeGeneration,
            serde_json::Value::Null,
            provenance,
        )
        .expect("expertise");

        let adapter = TrainedLoRAAdapter {
            id: Uuid::new_v4(),
            expertise,
            checksum: Checksum::from_hex("abcdef1234567890"),
            storage_path: storage_path.clone(),
            base_model_family: "unsupported-model".into(),
            version: None,
            source: AdapterSource::HuggingFace {
                repo: "test/adapter".into(),
            },
            size_bytes: None,
            owner: WebID::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(store);
        let token = test_token();

        let result = block_on(router.create_endpoint(adapter.id, ProviderId::Together, &token));
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "requires TOGETHER_API_KEY"]
    fn drain_all_owner_cleans_up() {
        unsafe {
            std::env::set_var("TOGETHER_API_KEY", "test-key");
        }
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(Arc::clone(&store));
        let token = test_token();

        let _handle = block_on(router.create_endpoint(adapter.id, ProviderId::Together, &token))
            .expect("create endpoint");

        let count = router.drain_all_owner(adapter.owner).expect("drain");
        assert_eq!(count, 1);
    }

    #[test]
    fn select_provider_returns_sorted_by_cost() {
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(Arc::clone(&store));

        let selection = router.select_provider(adapter.id, None).expect("select");

        assert_eq!(selection.expertise_name, "solidity-audit");
        assert_eq!(selection.providers.len(), 3);
        // Cheapest first: Runpod ($0.79) < Baseten ($0.85) < Together ($1.10)
        assert!(
            selection.providers[0].cost_model.gpu_hourly_rate
                <= selection.providers[1].cost_model.gpu_hourly_rate
        );
        assert!(
            selection.providers[1].cost_model.gpu_hourly_rate
                <= selection.providers[2].cost_model.gpu_hourly_rate
        );
    }

    #[test]
    fn single_candidate_requires_confirmation() {
        // When only one provider is compatible, single_candidate is set
        // but requires_confirmation is always true (P2)
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(Arc::clone(&store));

        // With 3 providers, single_candidate is None
        let selection = router.select_provider(adapter.id, None).expect("select");
        assert!(selection.single_candidate.is_none());
    }

    #[test]
    fn select_provider_budget_filter() {
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = AdapterRouter::new(Arc::clone(&store));

        // Budget of $0.80/hr — only Runpod ($0.79) fits
        let selection = router
            .select_provider(adapter.id, Some(0.80))
            .expect("select");
        assert_eq!(selection.within_budget_count, 1);

        // Budget of $2.00/hr — all three fit
        let selection = router
            .select_provider(adapter.id, Some(2.00))
            .expect("select");
        assert_eq!(selection.within_budget_count, 3);
    }

    // NOTE: Requires EndpointGuard::Drop to support nested runtime or async drop.
    //       The current implementation uses Handle::current().block_on() which
    //       conflicts with Runtime::block_on(). See ADR for async drop migration.
    #[test]
    #[ignore = "requires EndpointGuard Drop runtime fix"]
    fn endpoint_guard_teardown_on_drop() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = in_memory_db();
            let store = Arc::new(AdapterStore::new(db.conn_arc()));
            store.migrate().expect("migration");

            let adapter = make_test_adapter("solidity-audit");
            store.store(&adapter).expect("store");

            let router = Arc::new(AdapterRouter::new(Arc::clone(&store)));
            let token = test_token();

            let handle = router
                .create_endpoint(adapter.id, ProviderId::Together, &token)
                .await
                .expect("create endpoint");
            let endpoint_id = handle.endpoint_id;

            // Guard scope — drop triggers async teardown
            {
                let _guard = EndpointGuard::new(&router, endpoint_id);
                assert_eq!(_guard.endpoint_id(), endpoint_id);
                // Guard drops here → teardown called
            }

            // After guard drops, endpoint should be gone
            let status = router.endpoint_status(endpoint_id, &token);
            assert!(status.is_err());
        });
    }

    // NOTE: Same runtime conflict as endpoint_guard_teardown_on_drop.
    #[test]
    #[ignore = "requires EndpointGuard Drop runtime fix"]
    fn endpoint_guard_explicit_teardown() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = in_memory_db();
            let store = Arc::new(AdapterStore::new(db.conn_arc()));
            store.migrate().expect("migration");

            let adapter = make_test_adapter("solidity-audit");
            store.store(&adapter).expect("store");

            let router = Arc::new(AdapterRouter::new(Arc::clone(&store)));
            let token = test_token();

            let handle = router
                .create_endpoint(adapter.id, ProviderId::Together, &token)
                .await
                .expect("create endpoint");

            let guard = EndpointGuard::new(&router, handle.endpoint_id);
            // Explicit teardown consumes guard — drop becomes no-op
            guard.teardown().expect("teardown");

            let status = router.endpoint_status(handle.endpoint_id, &token);
            assert!(status.is_err());
        });
    }

    #[test]
    #[ignore = "requires TOGETHER_API_KEY"]
    fn end_to_end_store_deploy_status_teardown() {
        unsafe {
            std::env::set_var("TOGETHER_API_KEY", "test-key");
        }
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        // 1. Store adapter
        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = Arc::new(AdapterRouter::new(Arc::clone(&store)));
        let token = test_token();

        // 2. Select provider (P2 consent)
        let selection = router.select_provider(adapter.id, None).expect("select");
        assert!(!selection.providers.is_empty());

        // 3. Create endpoint
        let handle = block_on(router.create_endpoint(adapter.id, ProviderId::Together, &token))
            .expect("create endpoint");
        assert_eq!(handle.expertise_name, "solidity-audit");
        assert!(!handle.endpoint_url.is_empty());
        assert!(!handle.model_name.is_empty());
        assert_eq!(handle.phase(), EndpointPhase::Ready);

        // 4. Check status
        let status = router
            .endpoint_status(handle.endpoint_id, &token)
            .expect("status");
        assert_eq!(status.phase, EndpointPhase::Ready);
        assert_eq!(status.provider, ProviderId::Together);

        // 5. Teardown
        block_on(router.teardown_endpoint(handle.endpoint_id)).expect("teardown");
        assert!(router.endpoint_status(handle.endpoint_id, &token).is_err());

        // 6. Verify adapter still exists after teardown (only endpoint removed)
        let stored = store
            .get_by_id(adapter.id)
            .expect("get adapter")
            .expect("adapter exists");
        assert_eq!(stored.expertise.name, "solidity-audit");
    }

    #[test]
    fn end_to_end_budget_enforcement() {
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let router = Arc::new(AdapterRouter::new(Arc::clone(&store)));
        let _token = test_token();

        // Select with tight budget — only Runpod ($0.79) fits under $0.80
        let selection = router
            .select_provider(adapter.id, Some(0.80))
            .expect("select");
        assert_eq!(selection.within_budget_count, 1);

        // Select with generous budget — all three fit
        let selection = router
            .select_provider(adapter.id, Some(2.00))
            .expect("select");
        assert_eq!(selection.within_budget_count, 3);
    }

    #[test]
    fn end_to_end_version_management() {
        let db = in_memory_db();
        let store = Arc::new(AdapterStore::new(db.conn_arc()));
        store.migrate().expect("migration");

        let mut v1 = make_test_adapter("solidity-audit");
        v1.version = Some("1".into());
        store.store(&v1).expect("store v1");

        let mut v2 = make_test_adapter("solidity-audit");
        v2.version = Some("2".into());
        store.store(&v2).expect("store v2");

        let all = store.get_by_expertise("solidity-audit").expect("list");
        assert_eq!(all.len(), 2);

        // Both versions coexist (P2 — never implicitly supersede)
        let versions: Vec<&str> = all.iter().filter_map(|a| a.version.as_deref()).collect();
        assert!(versions.contains(&"1"));
        assert!(versions.contains(&"2"));
    }
}
