//! Together AI cloud training host.
//!
//! Submits fine-tuning jobs via the Together AI REST API.
//! Uses the Together AI fine-tuning API (<https://api.together.xyz/v1/fine-tunes>)
//! to submit LoRA fine-tuning jobs, poll status, and manage adapters.
//! No local GPU or CLI required.

use crate::providers::harness::HarnessAdapter;
use crate::providers::types::*;

/// Together AI training host — submits fine-tuning jobs via REST API.
pub struct TogetherHost {
    api_key: String,
    base_url: String,
    #[allow(dead_code)]
    harness: Box<dyn HarnessAdapter>,
    client: reqwest::Client,
}

impl TogetherHost {
    pub fn new(api_key: String, harness: Box<dyn HarnessAdapter>) -> Self {
        Self {
            api_key,
            base_url: "https://api.together.ai".to_string(),
            harness,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl TrainingHost for TogetherHost {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        // Step 1: Upload the dataset file
        let file_name = job
            .dataset_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("dataset.jsonl");

        let file_bytes = tokio::fs::read(&job.dataset_path)
            .await
            .map_err(|e| ProviderError::Backend(format!("Failed to read dataset file: {}", e)))?;

        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name.to_string())
            .mime_str("application/jsonl")
            .map_err(|e| ProviderError::Backend(format!("Invalid MIME type: {}", e)))?;

        let form = reqwest::multipart::Form::new()
            .text("purpose", "fine-tune")
            .text("file_name", file_name.to_string())
            .part("file", file_part);

        let upload_response = self
            .client
            .post(format!("{}/v1/files/upload", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Together AI upload failed: {}", e)))?;

        let upload_status = upload_response.status();
        let upload_json: serde_json::Value = upload_response.json().await.map_err(|e| {
            ProviderError::Backend(format!("Together AI upload parse error: {}", e))
        })?;

        if !upload_status.is_success() {
            return Err(ProviderError::Backend(format!(
                "Together AI upload error {}: {}",
                upload_status,
                serde_json::to_string_pretty(&upload_json).unwrap_or_default()
            )));
        }

        let file_id = upload_json["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Backend("No file ID in upload response".to_string()))?
            .to_string();

        tracing::info!(
            target: "hkask.training.file.upload",
            file_id = %file_id,
            "Dataset uploaded to Together AI"
        );

        // Step 2: Submit the fine-tuning job
        let body = serde_json::json!({
            "model": job.base_model,
            "training_file": file_id,
            "n_epochs": job.params.num_epochs,
            "n_checkpoints": 5,
            "learning_rate": job.params.learning_rate,
            "lora": true,
            "lora_r": job.params.lora.r,
            "lora_alpha": job.params.lora.alpha,
            "batch_size": job.params.batch_size.max(8),
            "warmup_ratio": 0.0,
            "weight_decay": job.params.optimization.weight_decay,
            "max_grad_norm": job.params.optimization.max_grad_norm.unwrap_or(1.0),
            "train_on_inputs": "auto",
            "suffix": format!("hkask-{}", &job.id[..8]),
        });

        let response = self
            .client
            .post(format!("{}/v1/fine-tunes", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Together AI request failed: {}", e)))?;

        let status = response.status();
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Together AI parse error: {}", e)))?;

        if !status.is_success() {
            return Err(ProviderError::Backend(format!(
                "Together AI error {}: {}",
                status,
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )));
        }

        let job_id = json["id"].as_str().unwrap_or("unknown").to_string();

        tracing::info!(
            target: "hkask.training.job.submit",
            job_id = %job_id,
            host = "together",
            harness = ?job.harness,
            "Training job submitted to Together AI"
        );

        Ok(job_id)
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let response = self
            .client
            .get(format!("{}/v1/fine-tunes/{}", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI status request failed: {}", e))
            })?;

        let status_code = response.status();
        let json: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::Backend(format!("Together AI status parse error: {}", e))
        })?;

        if !status_code.is_success() {
            return Err(ProviderError::Backend(format!(
                "Together AI status error {}: {}",
                status_code,
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )));
        }

        let status_str = json["status"].as_str().unwrap_or("unknown");
        match status_str {
            "pending" | "queued" => Ok(TrainingJobStatus::Queued),
            "running" => Ok(TrainingJobStatus::Running),
            "completed" | "succeeded" => Ok(TrainingJobStatus::Completed),
            "failed" | "error" => Ok(TrainingJobStatus::Failed),
            "cancelled" => Ok(TrainingJobStatus::Cancelled),
            _ => Ok(TrainingJobStatus::Queued),
        }
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let response = self
            .client
            .post(format!("{}/v1/fine-tunes/{}/cancel", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI cancel request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Backend(format!(
                "Together AI cancel error: {}",
                body
            )));
        }

        tracing::info!(
            target: "hkask.training.job.cancel",
            job_id = %job_id,
            host = "together",
            "Training job cancelled"
        );
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .client
            .get(format!("{}/v1/fine-tunes", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI list request failed: {}", e))
            })?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Together AI list parse error: {}", e)))?;

        let adapters: Vec<String> = json["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|j| j["status"] == "completed" || j["status"] == "succeeded")
            .filter_map(|j| j["id"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(adapters)
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        let response = self
            .client
            .delete(format!("{}/v1/fine-tunes/{}", self.base_url, adapter_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI delete request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Backend(format!(
                "Together AI delete error: {}",
                body
            )));
        }

        tracing::info!(
            target: "hkask.training.adapter.deleted",
            adapter_id = %adapter_id,
            host = "together",
            "LoRA adapter deleted from Together AI"
        );
        Ok(())
    }

    async fn completion_metadata(
        &self,
        job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        let response = self
            .client
            .get(format!("{}/v1/fine-tunes/{}", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI metadata request failed: {}", e))
            })?;

        let json: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::Backend(format!("Together AI metadata parse error: {}", e))
        })?;

        let status_str = json["status"].as_str().unwrap_or("");
        if status_str != "completed" && status_str != "succeeded" {
            return Ok(None);
        }

        let base_model = json["model"].as_str().unwrap_or("unknown").to_string();
        let output_name = json["output_name"].as_str().map(|s| s.to_string());

        // Extract loss from the last training event
        let loss = json["events"].as_array().and_then(|events| {
            events.iter().rev().find_map(|e| {
                e.get("type")
                    .and_then(|t| t.as_str())
                    .filter(|t| *t == "training_loss" || *t == "checkpoint")
                    .and_then(|_| e.get("data").and_then(|d| d.get("loss")))
                    .and_then(|l| l.as_f64())
                    .map(|l| l as f32)
            })
        });

        let tokens_processed = json["events"].as_array().and_then(|events| {
            events.iter().rev().find_map(|e| {
                e.get("type")
                    .and_then(|t| t.as_str())
                    .filter(|t| *t == "training_loss" || *t == "checkpoint")
                    .and_then(|_| e.get("data").and_then(|d| d.get("tokens")))
                    .and_then(|t| t.as_u64())
            })
        });

        let training_duration_secs = json["events"].as_array().and_then(|events| {
            let created = events.first()?.get("created_at")?.as_i64()?;
            let finished = events.last()?.get("created_at")?.as_i64()?;
            Some((finished - created) as u64)
        });

        Ok(Some(CompletionMetadata {
            base_model,
            output_name,
            loss,
            training_duration_secs,
            tokens_processed,
        }))
    }
}
