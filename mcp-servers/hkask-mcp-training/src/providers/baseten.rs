//! Baseten managed training host.
//!
//! Uses the Baseten REST API to submit training jobs with a generated `train.py`
//! script that loads models from HuggingFace, applies LoRA via TRL/SFTTrainer,
//! and saves checkpoints for automatic deployment.
//!
//! **Model loading:** Base models are loaded from HuggingFace via Baseten's
//! weights mount system (`hf://` source). Requires `HF_TOKEN` in Baseten Secrets
//! or passed as an environment variable.
//!
//! Environment variables:
//! - `BASETEN_API_KEY` — Baseten API key
//! - `BASETEN_PROJECT_ID` — Baseten training project ID
//! - `HF_TOKEN` — HuggingFace access token (for gated model loading)
//! - `BASETEN_GPU` — GPU accelerator type (default: "H100")
//! - `BASETEN_GPU_COUNT` — Number of GPUs (default: 1)

use crate::providers::harness::HarnessAdapter;
use crate::providers::types::*;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Baseten managed training host — runs training code on their GPU infra.
pub struct BasetenHost {
    api_key: String,
    project_id: String,
    base_url: String,
    harness: Box<dyn HarnessAdapter>,
    client: reqwest::Client,
    /// job_id tracking for status/cancel
    jobs: Arc<Mutex<HashMap<String, String>>>,
}

impl BasetenHost {
    pub fn new(api_key: String, project_id: String, harness: Box<dyn HarnessAdapter>) -> Self {
        Self {
            api_key,
            project_id,
            base_url: "https://api.baseten.co".to_string(),
            harness,
            client: reqwest::Client::new(),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl TrainingHost for BasetenHost {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        // Resolve HuggingFace model ID via canonical resolver.
        let hf_model_id = crate::huggingface::resolve_model_id(&job.base_model);

        // Clone the job and inject the resolved HF model ID so the harness
        // can render a training script with the correct model identifier.
        let mut resolved_job = job.clone();
        resolved_job.base_model = hf_model_id.clone();

        // Generate train.py via harness using render_config
        let train_script = self.harness.render_config(&resolved_job)?;
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(train_script.as_bytes());

        let gpu = std::env::var("BASETEN_GPU").unwrap_or_else(|_| "H100".to_string());
        let gpu_count: u32 = std::env::var("BASETEN_GPU_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let hf_token = std::env::var("HF_TOKEN").unwrap_or_default();
        let dataset_url = std::env::var("HKASK_DATASET_URL").unwrap_or_default();

        let body = json!({
            "training_job": {
                "name": format!("hkask-training-{}", &job.id[..8]),
                "image": {
                    "base_image": "baseten/trt-llm-train:latest",
                },
                "compute": {
                    "node_count": 1,
                    "cpu_count": 8,
                    "memory": "32Gi",
                    "accelerator": {
                        "accelerator": gpu,
                        "count": gpu_count,
                    },
                },
                "runtime": {
                    "start_commands": [
                        "pip install peft trl datasets accelerate",
                        format!("python -c \"import base64; open('train.py','w').write(base64.b64decode('{}').decode())\"", encoded),
                        "python train.py",
                    ],
                    "environment_variables": {
                        "HKASK_JOB_ID": job.id,
                        "HKASK_BASE_MODEL": job.base_model,
                        "HKASK_DATASET_URL": dataset_url,
                        "HKASK_NUM_EPOCHS": job.params.num_epochs.to_string(),
                        "HKASK_LORA_R": job.params.lora.r.to_string(),
                        "HF_TOKEN": hf_token,
                    },
                    "checkpointing_config": {
                        "enabled": true,
                        "checkpoint_path": "/mnt/ckpts",
                        "volume_size_gib": 20,
                    },
                },
                "weights": [
                    {
                        "source": format!("hf://{}", hf_model_id),
                        "mount_location": format!("/app/models/{}", hf_model_id),
                    }
                ],
            }
        });

        let url = format!(
            "{}/v1/training_projects/{}/jobs",
            self.base_url, self.project_id
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten API request failed: {}", e)))?;

        let status_code = response.status();
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten API parse error: {}", e)))?;

        if !status_code.is_success() {
            return Err(ProviderError::Backend(format!(
                "Baseten error {}: {}",
                status_code,
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )));
        }

        let baseten_job_id = json["training_job"]["id"]
            .as_str()
            .or_else(|| json["id"].as_str())
            .unwrap_or("unknown")
            .to_string();

        // Store mapping for status/cancel
        if let Ok(mut map) = self.jobs.lock() {
            map.insert(job.id.clone(), baseten_job_id.clone());
        }

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job.id,
            baseten_job_id = %baseten_job_id,
            host = "baseten",
            "Training job submitted to Baseten"
        );

        Ok(job.id.clone())
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let baseten_job_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let baseten_job_id = match baseten_job_id {
            Some(id) => id,
            None => {
                return Err(ProviderError::JobFailed(format!(
                    "No Baseten job found for {}",
                    job_id
                )));
            }
        };

        let url = format!(
            "{}/v1/training_projects/{}/jobs/{}",
            self.base_url, self.project_id, baseten_job_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten status request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten status parse error: {}", e)))?;

        let status_str = json["training_job"]["status"]
            .as_str()
            .or_else(|| json["status"].as_str())
            .unwrap_or("unknown");

        match status_str {
            "PENDING" | "QUEUED" | "CREATING" => Ok(TrainingJobStatus::Queued),
            "RUNNING" | "TRAINING" => Ok(TrainingJobStatus::Running),
            "COMPLETED" | "SUCCEEDED" | "DONE" => Ok(TrainingJobStatus::Completed),
            "FAILED" | "ERROR" | "CANCELLED" => Ok(TrainingJobStatus::Failed),
            _ => Ok(TrainingJobStatus::Queued),
        }
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let baseten_job_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let baseten_job_id = match baseten_job_id {
            Some(id) => id,
            None => {
                tracing::warn!(
                    target: "cns.training.job.cancel",
                    job_id = %job_id,
                    "No Baseten job found"
                );
                return Ok(());
            }
        };

        let url = format!(
            "{}/v1/training_projects/{}/jobs/{}",
            self.base_url, self.project_id, baseten_job_id
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten cancel request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Backend(format!(
                "Baseten cancel error: {}",
                body
            )));
        }

        if let Ok(mut map) = self.jobs.lock() {
            map.remove(job_id);
        }

        tracing::info!(
            target: "cns.training.job.cancel",
            job_id = %job_id,
            host = "baseten",
            "Training job cancelled on Baseten"
        );
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        let map = self
            .jobs
            .lock()
            .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
        Ok(map.keys().cloned().collect())
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        let _ = self.cancel(adapter_id).await;
        Ok(())
    }

    async fn completion_metadata(
        &self,
        _job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        // Baseten checkpoints contain metrics; extraction requires checkpoint API.
        Ok(None)
    }

    async fn adapter_weight_path(
        &self,
        _adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        // Weights are on Baseten — download via checkpoint archive URL.
        Ok(None)
    }
}
