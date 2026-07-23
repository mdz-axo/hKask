use crate::TrainingServer;
use crate::adapters::AdapterMetrics;
use crate::huggingface::CompletionManifest;
use crate::providers::TrainingJobStatus;
use crate::types::TrainStatusRequest;
use hkask_mcp_server::server::{McpToolError, execute_tool};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::tool;
use serde_json::json;

impl TrainingServer {
    #[tool(
        description = "Check the status of a training job. Polls the training host for pod status and checks for a completion manifest on HuggingFace. When training completes, automatically registers the adapter in the adapter store with metadata from the completion manifest."
    )]
    pub async fn training_status(
        &self,
        Parameters(TrainStatusRequest { job_id }): Parameters<TrainStatusRequest>,
    ) -> String {
        execute_tool(self, "training_status", async {
            match self.host.status(&job_id).await {
                Ok(pod_status) => {
                    // The pod stays RUNNING (exec sleep infinity for SSH
                    // debugging), so RunPod's desiredStatus alone cannot signal
                    // completion. Check for a completion manifest on HuggingFace.
                    let (status, manifest) = if pod_status == TrainingJobStatus::Running {
                        self.check_completion_manifest(&job_id)
                            .await
                            .unwrap_or((TrainingJobStatus::Running, None))
                    } else {
                        (pod_status, None)
                    };

                    let mut result = json!({
                        "job_id": job_id,
                        "status": serde_json::to_value(status).unwrap_or_default(),
                    });

                    // Persist status update
                    if let Some(ref job_store) = self.job_store {
                        let status_str = format!("{:?}", status).to_lowercase();
                        if let Err(e) = job_store.update_status(&job_id, &status_str) {
                            tracing::warn!(target: "hkask.training.job.persist", job_id = %job_id, error = %e, "Failed to update job status");
                        }
                    }

                    // Auto-register adapter on completion
                    if status == TrainingJobStatus::Completed {
                        let adapter: crate::adapter::TrainedLoRAAdapter = match self
                            .adapter_store
                            .get_by_id(uuid::Uuid::parse_str(&job_id).unwrap_or_default())
                            .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))?
                        {
                            Some(existing) => {
                                result["adapter_registered"] = json!(true);
                                result["adapter_note"] = json!("Already registered (pre-registered by retrain)");
                                existing
                            }
                            None => {
                                // Fresh auto-registration from the completion manifest.
                                if let Some(ref manifest) = manifest {
                                    let base_model = manifest.base_model.clone().unwrap_or_default();
                                    let adapter_name = format!("adapter-{}", &job_id[..8]);
                                    let weight_path = manifest.adapter.repository.clone();
                                    let adapter = Self::build_trained_adapter(
                                        job_id.clone(),
                                        adapter_name,
                                        base_model.clone(),
                                        String::new(),
                                        job_id.clone(),
                                        chrono::Utc::now().timestamp(),
                                        0,
                                        String::new(),
                                        1,
                                        Some(AdapterMetrics {
                                            loss: manifest.loss.map(|v| v as f32),
                                            perplexity: None,
                                            training_duration_secs: manifest.training_duration_secs,
                                            tokens_processed: None,
                                        }),
                                        Some(std::path::Path::new(&weight_path)),
                                    );
                                    match self
                                        .adapter_store
                                        .store(&adapter)
                                        .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))
                                    {
                                        Ok(()) => {
                                            result["adapter_registered"] = json!(true);
                                            result["adapter_name"] = json!(adapter.expertise.name);
                                            result["base_model"] = json!(base_model);
                                            result["adapter_repository"] = json!(manifest.adapter.repository);
                                            result["adapter_path"] = json!(manifest.adapter.path);
                                            tracing::info!(target: "hkask.training.adapter.created", adapter_id = %job_id, "Adapter auto-registered from completion manifest");
                                            adapter
                                        }
                                        Err(e) => {
                                            result["adapter_registered"] = json!(false);
                                            result["adapter_error"] = json!(e.to_string());
                                            return Ok(result);
                                        }
                                    }
                                } else {
                                    result["adapter_registered"] = json!(false);
                                    result["adapter_note"] = json!("No completion manifest available");
                                    return Ok(result);
                                }
                            }
                        };

                        // A/B comparison (skill retraining only)
                        let adapter_skill = adapter.skill_name.clone().unwrap_or_default();
                        if !adapter_skill.is_empty() {
                            let current_loss = Self::metrics_from_trained(&adapter).and_then(|m| m.loss);
                            if let Some(prev) = self
                                .adapter_store
                                .get_previous_by_skill_name(&adapter_skill, adapter.id)
                                .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))?
                                && let (Some(new_loss), Some(prev_loss)) = (
                                    current_loss,
                                    Self::metrics_from_trained(&prev).and_then(|m| m.loss),
                                ) {
                                    let improved = new_loss < prev_loss;
                                    result["ab_comparison"] = json!({
                                        "skill_name": adapter_skill,
                                        "previous_version": prev.version,
                                        "previous_adapter_name": prev.expertise.name,
                                        "previous_loss": prev_loss,
                                        "new_version": adapter.version,
                                        "new_loss": new_loss,
                                        "loss_improved": improved,
                                        "auto_promoted": improved,
                                    });
                                    tracing::info!(target: "hkask.training.retrain.ab", skill = %adapter_skill, prev_loss = %prev_loss, new_loss = %new_loss, improved = improved, "A/B comparison completed");
                                }
                        }
                    }

                    Ok(result)
                }
                Err(e) => Err(McpToolError::internal(format!("Status query failed: {}", e))),
            }
        })
        .await
    }
}
