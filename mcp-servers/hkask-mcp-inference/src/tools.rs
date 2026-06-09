//! MCP tools for Okapi-backed LLM inference
//!
//! Three tools exposed via MCP protocol:
//! - `inference_generate` — Generate text via Okapi LLM (with failover)
//! - `inference_metrics` — Get current inference metrics
//! - `inference_models` — List available model tiers
//!
//! **Throttling is not handled here.** Per-agent rate limiting is a CNS concern
//! (Loop 6 regulation) owned by `GovernedTool` energy budget accounting. The
//! `McpDispatcher` routes all invocations through the `GovernedTool` membrane
//! which handles OCAP, energy budget, and CNS observability. This server runs as
//! a separate process; placing throttling here would duplicate the canonical
//! implementation and violate the authority DAG (Cybernetics governs Communication).

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_templates::{InferencePort, OkapiConfig, OkapiInference};
use hkask_types::{LLMParameters, McpErrorKind, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tracing::{info, warn};

const FALLBACK_MODEL: &str = "qwen3.5";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateRequest {
    pub prompt: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub fallback_model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_caller_id")]
    pub caller_id: String,
}

fn default_model() -> String {
    "qwen3.5".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    1024
}

fn default_caller_id() -> String {
    "anonymous".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MetricsRequest {
    #[serde(default)]
    pub reset: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModelsRequest {
    #[serde(default)]
    pub filter: String,
}

#[derive(Debug)]
pub struct InferenceMetrics {
    pub total_requests: AtomicU64,
    pub total_tokens_generated: AtomicU64,
    pub total_errors: AtomicU64,
    pub total_failovers: AtomicU64,
}

impl Default for InferenceMetrics {
    fn default() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_tokens_generated: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_failovers: AtomicU64::new(0),
        }
    }
}

#[derive(Debug)]
pub struct InferenceServer {
    webid: WebID,
    metrics: Arc<InferenceMetrics>,
    active_models: Arc<RwLock<Vec<String>>>,
}

impl InferenceServer {
    pub fn new(webid: WebID) -> anyhow::Result<Self> {
        Ok(Self {
            webid,
            metrics: Arc::new(InferenceMetrics::default()),
            active_models: Arc::new(RwLock::new(vec!["qwen3.5".to_string()])),
        })
    }

    async fn try_generate(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> Result<hkask_types::ports::InferenceResult, hkask_types::ports::InferenceError> {
        let config = OkapiConfig::default();
        let inference = OkapiInference::new(model, config)?;
        inference.generate(prompt, params).await
    }
}

#[tool_router(server_handler)]
impl InferenceServer {
    #[tool(
        description = "Generate text using Okapi-backed LLM inference. Supports model selection with automatic failover, temperature control, and token limits. Per-agent rate limiting is handled by the CNS throttle at the MCP dispatch layer."
    )]
    async fn inference_generate(
        &self,
        Parameters(GenerateRequest {
            prompt,
            model,
            fallback_model,
            temperature,
            max_tokens,
            caller_id,
        }): Parameters<GenerateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("inference_generate", &self.webid);

        // Validate identifiers
        validate_field!(span, "model", &model, 128);
        validate_field!(span, "caller_id", &caller_id, 128);

        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        info!(
            target: "hkask.mcp.inference",
            model = %model,
            caller_id = %caller_id,
            prompt_len = prompt.len(),
            temperature = temperature,
            max_tokens = max_tokens,
            "Generating inference"
        );

        let params = LLMParameters {
            temperature,
            max_tokens,
            ..Default::default()
        };

        let result = match self.try_generate(&model, &prompt, &params).await {
            Ok(r) => r,
            Err(primary_err) => {
                let fallback = if fallback_model.is_empty() {
                    FALLBACK_MODEL
                } else {
                    &fallback_model
                };

                if fallback != model {
                    warn!(
                        target: "hkask.mcp.inference",
                        primary_model = %model,
                        fallback_model = %fallback,
                        error = %primary_err,
                        "Primary model failed, attempting failover"
                    );

                    self.metrics.total_failovers.fetch_add(1, Ordering::Relaxed);

                    match self.try_generate(fallback, &prompt, &params).await {
                        Ok(r) => r,
                        Err(fallback_err) => {
                            self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
                            return span.error(
                                McpErrorKind::Unavailable,
                                McpToolError::unavailable(format!(
                                    "All models failed. Primary: {}, Fallback: {}",
                                    primary_err, fallback_err
                                ))
                                .to_json_string(),
                            );
                        }
                    }
                } else {
                    self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
                    return span.error(
                        McpErrorKind::Unavailable,
                        McpToolError::unavailable(format!("Generation failed: {}", primary_err))
                            .to_json_string(),
                    );
                }
            }
        };

        self.metrics
            .total_tokens_generated
            .fetch_add(result.usage.total_tokens as u64, Ordering::Relaxed);

        let mut models = self.active_models.write().await;
        if !models.contains(&result.model) {
            models.push(result.model.clone());
        }

        span.ok_json(serde_json::json!({
            "text": result.text,
            "model": result.model,
            "usage": {
                "prompt_tokens": result.usage.prompt_tokens,
                "completion_tokens": result.usage.completion_tokens,
                "total_tokens": result.usage.total_tokens,
            },
            "finish_reason": result.finish_reason,
        }))
    }

    #[tool(
        description = "Get current inference metrics including total requests, tokens generated, error counts, and failover count."
    )]
    async fn inference_metrics(
        &self,
        Parameters(MetricsRequest { reset }): Parameters<MetricsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("inference_metrics", &self.webid);

        let load_or_swap = |counter: &AtomicU64| -> u64 {
            if reset {
                counter.swap(0, Ordering::Relaxed)
            } else {
                counter.load(Ordering::Relaxed)
            }
        };

        let total_requests = load_or_swap(&self.metrics.total_requests);
        let total_tokens = load_or_swap(&self.metrics.total_tokens_generated);
        let total_errors = load_or_swap(&self.metrics.total_errors);
        let total_failovers = load_or_swap(&self.metrics.total_failovers);
        span.ok_json(serde_json::json!({
            "total_requests": total_requests,
            "total_tokens_generated": total_tokens,
            "total_errors": total_errors,
            "total_failovers": total_failovers,
            "reset": reset,
        }))
    }

    #[tool(description = "List available model tiers and their configurations.")]
    async fn inference_models(
        &self,
        Parameters(ModelsRequest { filter }): Parameters<ModelsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("inference_models", &self.webid);

        let models = self.active_models.read().await;
        let filtered: Vec<&String> = if filter.is_empty() {
            models.iter().collect()
        } else {
            models
                .iter()
                .filter(|m| m.to_lowercase().contains(&filter.to_lowercase()))
                .collect()
        };

        let model_entries: Vec<serde_json::Value> = filtered
            .iter()
            .map(|m| {
                let tier = if m.contains("70b") {
                    "high"
                } else if m.contains("34b") || m.contains("13b") {
                    "balanced"
                } else {
                    "fast_local"
                };
                serde_json::json!({
                    "name": m,
                    "tier": tier,
                })
            })
            .collect();

        span.ok_json(serde_json::json!({
            "models": model_entries,
            "count": model_entries.len(),
        }))
    }
}
