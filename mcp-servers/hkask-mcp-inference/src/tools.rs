//! MCP tools for Okapi-backed LLM inference
//!
//! Three tools exposed via MCP protocol:
//! - `inference:generate` — Generate text via Okapi LLM (with failover + rate limiting)
//! - `inference:metrics` — Get current inference metrics
//! - `inference:models` — List available model tiers

use hkask_templates::{InferencePort, OkapiConfig, OkapiInference};
use hkask_types::LLMParameters;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};

const FALLBACK_MODEL: &str = "ollama/llama-3.1-8b-instruct";
const RATE_LIMIT_MAX_TOKENS: f64 = 10.0;
const RATE_LIMIT_REFILL_RATE: f64 = 1.0;

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
    "ollama/llama-3.1-8b-instruct".to_string()
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
struct RateBucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateBucket {
    fn new() -> Self {
        Self {
            tokens: RATE_LIMIT_MAX_TOKENS,
            last_refill: Instant::now(),
        }
    }

    fn consume(&mut self, amount: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * RATE_LIMIT_REFILL_RATE).min(RATE_LIMIT_MAX_TOKENS);
        self.last_refill = now;

        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct InferenceMetrics {
    pub total_requests: AtomicU64,
    pub total_tokens_generated: AtomicU64,
    pub total_errors: AtomicU64,
    pub total_failovers: AtomicU64,
    pub total_rate_limited: AtomicU64,
}

impl Default for InferenceMetrics {
    fn default() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_tokens_generated: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_failovers: AtomicU64::new(0),
            total_rate_limited: AtomicU64::new(0),
        }
    }
}

#[derive(Debug, Default)]
pub struct InferenceServer {
    metrics: Arc<InferenceMetrics>,
    active_models: Arc<RwLock<Vec<String>>>,
    rate_buckets: Arc<RwLock<HashMap<String, RateBucket>>>,
}

impl InferenceServer {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(InferenceMetrics::default()),
            active_models: Arc::new(RwLock::new(vec![
                "ollama/llama-3.1-8b-instruct".to_string(),
                "ollama/llama-3.1-70b-instruct".to_string(),
                "ollama/codellama-34b".to_string(),
            ])),
            rate_buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn check_rate_limit(&self, caller_id: &str) -> bool {
        let mut buckets = self.rate_buckets.write().await;
        let bucket = buckets
            .entry(caller_id.to_string())
            .or_insert_with(RateBucket::new);
        bucket.consume(1.0)
    }

    async fn try_generate(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> Result<hkask_templates::InferenceResult, hkask_templates::InferenceError> {
        let config = OkapiConfig::default();
        let inference = OkapiInference::new(model, config)?;
        inference.generate(prompt, params).await
    }
}

#[tool_router(server_handler)]
impl InferenceServer {
    #[tool(
        description = "Generate text using Okapi-backed LLM inference. Supports model selection with automatic failover, temperature control, token limits, and per-caller rate limiting."
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
        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        if !self.check_rate_limit(&caller_id).await {
            self.metrics
                .total_rate_limited
                .fetch_add(1, Ordering::Relaxed);
            warn!(
                target: "hkask.mcp.inference",
                caller_id = %caller_id,
                "Rate limit exceeded"
            );
            return format!(
                r#"{{"error":"Rate limit exceeded for caller: {}"}}"#,
                caller_id
            );
        }

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
                            return format!(
                                r#"{{"error":"All models failed. Primary: {}, Fallback: {}"}}"#,
                                primary_err, fallback_err
                            );
                        }
                    }
                } else {
                    self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
                    return format!(r#"{{"error":"Generation failed: {}"}}"#, primary_err);
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

        format!(
            r#"{{"text":{},"model":{},"usage":{{"prompt_tokens":{},"completion_tokens":{},"total_tokens":{}}},"finish_reason":{}}}"#,
            serde_json::to_string(&result.text).unwrap_or_default(),
            serde_json::to_string(&result.model).unwrap_or_default(),
            result.usage.prompt_tokens,
            result.usage.completion_tokens,
            result.usage.total_tokens,
            serde_json::to_string(&result.finish_reason).unwrap_or_default(),
        )
    }

    #[tool(
        description = "Get current inference metrics including total requests, tokens generated, error counts, failover count, and rate-limited requests."
    )]
    async fn inference_metrics(
        &self,
        Parameters(MetricsRequest { reset }): Parameters<MetricsRequest>,
    ) -> String {
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
        let total_rate_limited = load_or_swap(&self.metrics.total_rate_limited);

        format!(
            r#"{{"total_requests":{},"total_tokens_generated":{},"total_errors":{},"total_failovers":{},"total_rate_limited":{},"reset":{}}}"#,
            total_requests, total_tokens, total_errors, total_failovers, total_rate_limited, reset
        )
    }

    #[tool(description = "List available model tiers and their configurations.")]
    async fn inference_models(
        &self,
        Parameters(ModelsRequest { filter }): Parameters<ModelsRequest>,
    ) -> String {
        let models = self.active_models.read().await;
        let filtered: Vec<&String> = if filter.is_empty() {
            models.iter().collect()
        } else {
            models
                .iter()
                .filter(|m| m.to_lowercase().contains(&filter.to_lowercase()))
                .collect()
        };

        let model_list: Vec<String> = filtered
            .iter()
            .map(|m| {
                let tier = if m.contains("70b") {
                    "high"
                } else if m.contains("34b") || m.contains("13b") {
                    "balanced"
                } else {
                    "fast_local"
                };
                format!(
                    r#"{{"name":{},"tier":"{}"}}"#,
                    serde_json::to_string(m).unwrap_or_default(),
                    tier
                )
            })
            .collect();

        format!(
            r#"{{"models":[{}],"count":{}}}"#,
            model_list.join(","),
            model_list.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_default() {
        let metrics = InferenceMetrics::default();
        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.total_tokens_generated.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.total_errors.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.total_failovers.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.total_rate_limited.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_server_default_models() {
        let server = InferenceServer::new();
        let models = server.active_models.blocking_read();
        assert_eq!(models.len(), 3);
    }

    #[tokio::test]
    async fn test_rate_limit_allows_initial_requests() {
        let server = InferenceServer::new();
        for _ in 0..10 {
            assert!(server.check_rate_limit("test-caller").await);
        }
    }

    #[tokio::test]
    async fn test_rate_limit_blocks_after_exhaustion() {
        let server = InferenceServer::new();
        for _ in 0..10 {
            server.check_rate_limit("test-caller").await;
        }
        assert!(!server.check_rate_limit("test-caller").await);
    }

    #[tokio::test]
    async fn test_rate_limit_independent_per_caller() {
        let server = InferenceServer::new();
        for _ in 0..10 {
            server.check_rate_limit("caller-a").await;
        }
        assert!(!server.check_rate_limit("caller-a").await);
        assert!(server.check_rate_limit("caller-b").await);
    }

    #[test]
    fn test_rate_bucket_refill() {
        let mut bucket = RateBucket::new();
        for _ in 0..10 {
            assert!(bucket.consume(1.0));
        }
        assert!(!bucket.consume(1.0));
    }
}
