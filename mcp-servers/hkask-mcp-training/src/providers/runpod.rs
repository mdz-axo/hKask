//! Runpod GPU cloud training host.
//!
//! Dispatches training jobs to GPU pods via the Runpod GraphQL API.
//! Uses a pre-built template with axolotl installed; training is dispatched
//! via environment variables injected into the pod.

use crate::providers::harness::HarnessAdapter;
use crate::providers::types::*;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Runpod GPU cloud training host — dispatches training to GPU pods.
///
/// Uses the Runpod GraphQL API to create GPU pods from a pre-built template
/// (with axolotl installed), execute training, and retrieve LoRA adapters.
/// This is the "cloud dispatch" path for Axolotl — instead of running locally,
/// training runs on Runpod's GPU infrastructure.
///
/// **Template requirements:** The pod template must include a startup script
/// that reads `HKASK_*` environment variables, downloads the dataset from
/// `HKASK_DATASET_URL`, runs axolotl training, and uploads the resulting
/// adapter weights to a storage location.
///
/// Environment variables:
/// - `RUNPOD_API_KEY` — Runpod API key
/// - `RUNPOD_TEMPLATE_ID` — GPU pod template ID with axolotl pre-installed
/// - `RUNPOD_GPU_TYPE_ID` — GPU type ID (default: "NVIDIA RTX 4090")
/// - `RUNPOD_CONTAINER_DISK_GB` — Container disk in GB (default: 50)
/// - `RUNPOD_MIN_MEMORY_GB` — Minimum memory in GB (default: 24)
/// - `RUNPOD_MIN_VCPU_COUNT` — Minimum vCPU count (default: 8)
/// - `HKASK_DATASET_URL` — Public URL where the pod can download the dataset
///   (set this before calling training_submit with runpod provider)
pub struct RunpodHost {
    api_key: String,
    template_id: String,
    graphql_url: String,
    #[allow(dead_code)]
    harness: Box<dyn HarnessAdapter>,
    client: reqwest::Client,
    /// job_id → pod_id mapping for status/cancel
    jobs: Arc<Mutex<HashMap<String, String>>>,
}

impl RunpodHost {
    pub fn new(api_key: String, template_id: String, harness: Box<dyn HarnessAdapter>) -> Self {
        Self {
            api_key,
            template_id,
            graphql_url: "https://api.runpod.io/graphql".to_string(),
            harness,
            client: reqwest::Client::new(),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn graphql_query(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        let response = self
            .client
            .post(&self.graphql_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Runpod API request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Runpod API parse error: {}", e)))?;

        if let Some(errors) = json.get("errors") {
            return Err(ProviderError::Backend(format!(
                "Runpod GraphQL errors: {}",
                serde_json::to_string_pretty(errors).unwrap_or_default()
            )));
        }

        Ok(json)
    }
}

#[async_trait::async_trait]
impl TrainingHost for RunpodHost {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        // Create a GPU pod from the template
        let mutation = r#"
            mutation CreatePod($input: PodCreateAndDeployInput!) {
                podCreateAndDeploy(input: $input) {
                    id
                    name
                    desiredStatus
                    runtime { uptimeInSeconds }
                }
            }
        "#;

        let gpu_type_id =
            std::env::var("RUNPOD_GPU_TYPE_ID").unwrap_or_else(|_| "NVIDIA RTX 4090".to_string());
        let container_disk_gb: u32 = std::env::var("RUNPOD_CONTAINER_DISK_GB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50);
        let min_memory_gb: u32 = std::env::var("RUNPOD_MIN_MEMORY_GB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24);
        let min_vcpu: u32 = std::env::var("RUNPOD_MIN_VCPU_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8);
        let dataset_url = std::env::var("HKASK_DATASET_URL").unwrap_or_default();

        let variables = json!({
            "input": {
                "name": format!("hkask-training-{}", &job.id[..8]),
                "templateId": self.template_id,
                "gpuTypeId": gpu_type_id,
                "containerDiskInGb": container_disk_gb,
                "minMemoryInGb": min_memory_gb,
                "minVcpuCount": min_vcpu,
                "env": [
                    { "key": "HKASK_JOB_ID", "value": job.id },
                    { "key": "HKASK_BASE_MODEL", "value": job.base_model },
                    { "key": "HKASK_DATASET_URL", "value": dataset_url },
                    { "key": "HKASK_HARNESS", "value": format!("{:?}", job.harness).to_lowercase() },
                    { "key": "HKASK_NUM_EPOCHS", "value": job.params.num_epochs.to_string() },
                    { "key": "HKASK_LORA_R", "value": job.params.lora.r.to_string() },
                    { "key": "HKASK_LORA_ALPHA", "value": job.params.lora.alpha.to_string() },
                    { "key": "HKASK_LEARNING_RATE", "value": job.params.learning_rate.to_string() },
                    { "key": "HKASK_BATCH_SIZE", "value": job.params.batch_size.to_string() },
                ],
            }
        });

        let result = self.graphql_query(mutation, variables).await?;

        let pod_id = result["data"]["podCreateAndDeploy"]["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Backend("No pod ID in Runpod response".to_string()))?
            .to_string();

        // Store pod_id for status/cancel
        if let Ok(mut map) = self.jobs.lock() {
            map.insert(job.id.clone(), pod_id.clone());
        }

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job.id,
            pod_id = %pod_id,
            host = "runpod",
            harness = ?job.harness,
            "Training pod created on Runpod"
        );

        Ok(job.id.clone())
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let pod_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let pod_id = match pod_id {
            Some(id) => id,
            None => {
                return Err(ProviderError::JobFailed(format!(
                    "No pod found for job {}",
                    job_id
                )));
            }
        };

        let query = r#"
            query GetPod($id: String!) {
                pod(input: { podId: $id }) {
                    id
                    desiredStatus
                    runtime { uptimeInSeconds }
                    machine { gpuType }
                }
            }
        "#;

        let result = self.graphql_query(query, json!({ "id": pod_id })).await?;

        let status_str = result["data"]["pod"]["desiredStatus"]
            .as_str()
            .unwrap_or("UNKNOWN");

        match status_str {
            "CREATING" | "PENDING" => Ok(TrainingJobStatus::Queued),
            "RUNNING" => Ok(TrainingJobStatus::Running),
            "STOPPED" | "TERMINATED" => {
                // Check if training output exists — if pod stopped with output, it completed
                Ok(TrainingJobStatus::Completed)
            }
            "FAILED" | "ERROR" => Ok(TrainingJobStatus::Failed),
            _ => Ok(TrainingJobStatus::Queued),
        }
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let pod_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let pod_id = match pod_id {
            Some(id) => id,
            None => {
                tracing::warn!(
                    target: "cns.training.job.cancel",
                    job_id = %job_id,
                    "No pod found for job"
                );
                return Ok(());
            }
        };

        let mutation = r#"
            mutation TerminatePod($id: String!) {
                podTerminate(input: { podId: $id })
            }
        "#;

        self.graphql_query(mutation, json!({ "id": pod_id }))
            .await?;

        if let Ok(mut map) = self.jobs.lock() {
            map.remove(job_id);
        }

        tracing::info!(
            target: "cns.training.job.cancel",
            job_id = %job_id,
            pod_id = %pod_id,
            host = "runpod",
            "Training pod terminated"
        );
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        // List completed pods — adapters are identified by job_id
        let map = self
            .jobs
            .lock()
            .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
        Ok(map.keys().cloned().collect())
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        // Terminate the pod if still running
        let _ = self.cancel(adapter_id).await;
        Ok(())
    }

    async fn completion_metadata(
        &self,
        _job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        // Runpod doesn't provide structured training metrics via API.
        Ok(None)
    }

    async fn adapter_weight_path(
        &self,
        _adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        // Weights are on the Runpod pod — need to be downloaded separately.
        Ok(None)
    }
}
