//! MCP tools for Okapi-backed LLM inference
//!
//! Three tools exposed via MCP protocol:
//! - `inference:generate` — Generate text via Okapi LLM (with failover + rate limiting)
//! - `inference:metrics` — Get current inference metrics
//! - `inference:models` — List available model tiers

use hkask_mcp::server::{McpToolError, McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_templates::{InferencePort, OkapiConfig, OkapiInference};
use hkask_types::{LLMParameters, McpErrorKind, WebID};
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

/// Per-caller rate-limit bucket (token-bucket algorithm).
///
// TODO: Migrate to `hkask_cns::RateLimiter` (which uses `WebID` keys)
//       via `SecurityGateway` in a future PR. The local `RateBucket` is
//       retained for now because the shared rate limiter operates on
//       `WebID` rather than `String` keys.
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
    /// External API boundary rate limit counter (per-caller token bucket).
    /// Distinct from internal energy budget tracking.
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

#[derive(Debug)]
pub struct InferenceServer {
    webid: WebID,
    metrics: Arc<InferenceMetrics>,
    active_models: Arc<RwLock<Vec<String>>>,
    /// Per-caller token bucket rate limiter (external API boundary protection).
    /// Distinct from internal energy budget tracking.
    rate_buckets: Arc<RwLock<HashMap<String, RateBucket>>>,
}

impl InferenceServer {
    pub fn new(webid: WebID) -> anyhow::Result<Self> {
        Ok(Self {
            webid,
            metrics: Arc::new(InferenceMetrics::default()),
            active_models: Arc::new(RwLock::new(vec![
                "ollama/llama-3.1-8b-instruct".to_string(),
                "ollama/llama-3.1-70b-instruct".to_string(),
                "ollama/codellama-34b".to_string(),
            ])),
            rate_buckets: Arc::new(RwLock::new(HashMap::new())),
        })
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
    ) -> Result<hkask_types::ports::InferenceResult, hkask_types::ports::InferenceError> {
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
        let span = ToolSpanGuard::new("inference:generate", &self.webid);

        // Validate identifiers
        if let Err(e) = validate_identifier("model", &model, 128) {
            return span.error(e.kind, e.to_json_string());
        }
        if let Err(e) = validate_identifier("caller_id", &caller_id, 128) {
            return span.error(e.kind, e.to_json_string());
        }

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
            return span.error(
                McpErrorKind::RateLimited,
                McpToolError::rate_limited(format!(
                    "Rate limit exceeded for caller: {}",
                    caller_id
                ))
                .to_json_string(),
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

        span.ok(McpToolOutput::new(serde_json::json!({
            "text": result.text,
            "model": result.model,
            "usage": {
                "prompt_tokens": result.usage.prompt_tokens,
                "completion_tokens": result.usage.completion_tokens,
                "total_tokens": result.usage.total_tokens,
            },
            "finish_reason": result.finish_reason,
        }))
        .to_json_string())
    }

    #[tool(
        description = "Get current inference metrics including total requests, tokens generated, error counts, failover count, and rate-limited requests."
    )]
    async fn inference_metrics(
        &self,
        Parameters(MetricsRequest { reset }): Parameters<MetricsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("inference:metrics", &self.webid);

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

        span.ok(McpToolOutput::new(serde_json::json!({
            "total_requests": total_requests,
            "total_tokens_generated": total_tokens,
            "total_errors": total_errors,
            "total_failovers": total_failovers,
            "total_rate_limited": total_rate_limited,
            "reset": reset,
        }))
        .to_json_string())
    }

    #[tool(description = "List available model tiers and their configurations.")]
    async fn inference_models(
        &self,
        Parameters(ModelsRequest { filter }): Parameters<ModelsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("inference:models", &self.webid);

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

        span.ok(McpToolOutput::new(serde_json::json!({
            "models": model_entries,
            "count": model_entries.len(),
        }))
        .to_json_string())
    }
}
