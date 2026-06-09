//! Okapi LLM inference port for high-temperature templates
//
//! `InferencePort` trait lives in hkask-types (port membrane).
//! This module provides `OkapiInference` — the concrete implementation.

use crate::okapi_config::{OkapiConfig, validate_prompt};
use futures_util::StreamExt;
use hkask_types::LLMParameters;
use hkask_types::cns::RetryConfig;
use hkask_types::ports::{
    CircuitBreakerPort, InferenceError, InferencePort, InferenceResult, InferenceStreamChunk,
    InferenceUsage, TokenProb, TokenProbability,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Okapi-backed inference implementation
pub struct OkapiInference {
    model: String,
    config: OkapiConfig,
    retry_config: RetryConfig,
    client: Arc<reqwest::Client>,
    /// Circuit breaker for resilience
    circuit_breaker: Option<Arc<dyn CircuitBreakerPort>>,
    /// Prompt cache for skipping redundant LLM calls
    prompt_cache: Option<Arc<crate::prompt_cache::PromptCache>>,
}

impl OkapiInference {
    pub fn new(model: &str, config: OkapiConfig) -> Result<Self, InferenceError> {
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        Ok(Self {
            model: model.to_string(),
            retry_config: RetryConfig::default(),
            config,
            client,
            circuit_breaker: None,
            prompt_cache: None,
        })
    }

    /// Execute HTTP request to Okapi API
    async fn execute_request(
        &self,
        request: OkapiRequest,
    ) -> Result<InferenceResult, InferenceError> {
        // Check circuit breaker before request
        if let Some(ref cb) = self.circuit_breaker
            && !cb.allow_request()
        {
            tracing::debug!(
                target: "cns.inference",
                model = %self.model,
                "Circuit breaker open, rejecting request"
            );
            return Err(InferenceError::Connection(
                "Circuit breaker is open".to_string(),
            ));
        }

        let mut req = self
            .client
            .post(format!("{}/api/generate", self.config.base_url))
            .json(&request);

        // Add authorization header if configured
        if let Some(auth_header) = self.config.get_authorization_header() {
            req = req.header("Authorization", auth_header);
        }

        let response = req
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Record failure in circuit breaker
            if let Some(ref cb) = self.circuit_breaker {
                cb.record_failure();
            }

            // Check if retryable
            if self.retry_config.is_retryable_status(status.as_u16()) {
                return Err(InferenceError::Connection(format!(
                    "Retryable status {}: {}",
                    status, error_text
                )));
            }

            return Err(InferenceError::Connection(format!(
                "Okapi API returned status {}: {}",
                status, error_text
            )));
        }

        let okapi_response: OkapiResponse = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("Okapi JSON parse error: {}", e)))?;

        let choice = okapi_response
            .choices
            .first()
            .ok_or_else(|| InferenceError::Generation("Empty response from Okapi".to_string()))?;

        // Extract token probabilities if available
        let token_probabilities = choice.token_probs.as_ref().map(|probs| {
            probs
                .iter()
                .map(|p| TokenProbability {
                    token: p.token.clone(),
                    prob: p.prob,
                    top_k: p
                        .top_k
                        .iter()
                        .map(|tk| TokenProb {
                            token: tk.token.clone(),
                            prob: tk.prob,
                        })
                        .collect(),
                })
                .collect()
        });

        // Record success in circuit breaker
        if let Some(ref cb) = self.circuit_breaker {
            cb.record_success();
        }

        // Extract structured tool calls if present (native function calling)
        let tool_calls = choice
            .tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .map(|tc| hkask_types::ports::StructuredToolCall {
                        server: tc.function.name.split('/').next().unwrap_or("").to_string(),
                        tool: tc
                            .function
                            .name
                            .split('/')
                            .nth(1)
                            .unwrap_or(&tc.function.name)
                            .to_string(),
                        args: tc.function.arguments.clone(),
                        call_id: tc.id.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(InferenceResult {
            text: choice.message.content.clone(),
            model: okapi_response.model.clone(),
            usage: InferenceUsage {
                prompt_tokens: okapi_response.usage.prompt_tokens,
                completion_tokens: okapi_response.usage.completion_tokens,
                total_tokens: okapi_response.usage.total_tokens,
            },
            finish_reason: choice.finish_reason.clone(),
            token_probabilities,
            tool_calls,
        })
    }

    /// Execute request with retry logic
    async fn execute_with_retry(
        &self,
        request: OkapiRequest,
    ) -> Result<InferenceResult, InferenceError> {
        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            match self.execute_request(request.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.retry_config.max_retries {
                        let delay_ms = self.retry_config.delay_for_attempt(attempt);
                        warn!(
                            target: "hkask.inference",
                            attempt = %attempt,
                            delay_ms = %delay_ms,
                            error = ?last_error,
                            "Retryable error, waiting before retry"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(last_error.expect("retry loop always records the last error"))
    }
}

impl InferencePort for OkapiInference {
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        let prompt = prompt.to_string();
        let parameters = parameters.clone();
        Box::pin(async move {
            // Validate input
            validate_prompt(&prompt).map_err(|e| InferenceError::Generation(e.to_string()))?;

            // Check prompt cache before API call
            if let Some(ref cache) = self.prompt_cache {
                let cache_key = crate::prompt_cache::PromptCache::generate_key(
                    &prompt,
                    &self.model,
                    &parameters,
                );
                match cache.get(&cache_key) {
                    Ok(result) => {
                        info!(
                            target: "hkask.inference",
                            model = %result.model,
                            cache_key = %cache_key,
                            "Cache hit - returning cached result"
                        );
                        return Ok(result);
                    }
                    Err(crate::prompt_cache::CacheError::Miss) => {
                        // Cache miss, proceed with API call
                    }
                    Err(e) => {
                        warn!(
                            target: "hkask.inference",
                            error = %e,
                            "Cache lookup error, proceeding with API call"
                        );
                    }
                }
            }

            let request = OkapiRequest {
                model: self.model.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: prompt.clone(),
                    images: None,
                }],
                temperature: parameters.temperature,
                top_p: parameters.top_p,
                top_k: parameters.top_k as i32,
                frequency_penalty: parameters.frequency_penalty,
                presence_penalty: parameters.presence_penalty,
                max_tokens: parameters.max_tokens as i32,
                seed: parameters.seed,
                n_probs: Some(5),
                stream: None,
            };

            let result = self.execute_with_retry(request).await?;

            // Cache the successful result
            if let Some(ref cache) = self.prompt_cache {
                let cache_key = crate::prompt_cache::PromptCache::generate_key(
                    &prompt,
                    &self.model,
                    &parameters,
                );
                if let Err(e) = cache.put(&cache_key, &prompt, &self.model, &result) {
                    warn!(
                        target: "hkask.inference",
                        error = %e,
                        "Failed to cache inference result"
                    );
                }
            }

            info!(
                target: "hkask.inference",
                model = %result.model,
                tokens = result.usage.total_tokens,
                finish_reason = %result.finish_reason,
                "Inference completed"
            );

            Ok(result)
        })
    }

    /// Stream inference output from Okapi via SSE.
    ///
    /// Sends the request with `stream: true` and parses Server-Sent Events
    /// (OpenAI-compatible format) into `InferenceStreamChunk` items.
    fn generate_stream(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> std::pin::Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        self.generate_stream_with_model(prompt, parameters, None)
    }

    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        let prompt = prompt.to_string();
        let parameters = parameters.clone();
        let model_override = model_override.map(|s| s.to_string());
        Box::pin(async move {
            // Validate input
            validate_prompt(&prompt).map_err(|e| InferenceError::Generation(e.to_string()))?;

            let model_id = model_override.unwrap_or_else(|| self.model.clone());

            let request = OkapiRequest {
                model: model_id.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: prompt.clone(),
                    images: None,
                }],
                temperature: parameters.temperature,
                top_p: parameters.top_p,
                top_k: parameters.top_k as i32,
                frequency_penalty: parameters.frequency_penalty,
                presence_penalty: parameters.presence_penalty,
                max_tokens: parameters.max_tokens as i32,
                seed: parameters.seed,
                n_probs: Some(5),
                stream: None,
            };

            let result = self.execute_with_retry(request).await?;

            info!(
                target: "hkask.inference",
                model = %result.model,
                tokens = result.usage.total_tokens,
                finish_reason = %result.finish_reason,
                "Inference with model completed"
            );

            Ok(result)
        })
    }
}

// Direct impl (not part of InferencePort trait)
impl OkapiInference {
    /// Stream inference output from Okapi via SSE, with optional model override.
    ///
    /// Sends the request with `stream: true` and parses Server-Sent Events
    /// (OpenAI-compatible format) into `InferenceStreamChunk` items.
    pub fn generate_stream_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> std::pin::Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        let prompt = prompt.to_string();
        let parameters = parameters.clone();
        let model_id = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.model.clone());
        let client = Arc::clone(&self.client);
        let base_url = self.config.base_url.clone();
        let auth_header = self.config.get_authorization_header();

        Box::pin(
            futures_util::stream::once(async move {
                // Build and send the streaming request
                let request = OkapiRequest {
                    model: model_id.clone(),
                    messages: vec![Message {
                        role: "user".to_string(),
                        content: prompt.clone(),
                        images: None,
                    }],
                    temperature: parameters.temperature,
                    top_p: parameters.top_p,
                    top_k: parameters.top_k as i32,
                    frequency_penalty: parameters.frequency_penalty,
                    presence_penalty: parameters.presence_penalty,
                    max_tokens: parameters.max_tokens as i32,
                    seed: parameters.seed,
                    n_probs: None,
                    stream: Some(true),
                };

                let mut req = client
                    .post(format!("{}/api/generate", base_url))
                    .json(&request);

                if let Some(ref header) = auth_header {
                    req = req.header("Authorization", header);
                }

                let response = match req
                    .send()
                    .await
                    .map_err(|e| InferenceError::Connection(e.to_string()))
                {
                    Ok(r) => r,
                    Err(e) => return vec![Err(e)],
                };

                let status = response.status();
                if !status.is_success() {
                    let error_text = response.text().await.unwrap_or_default();
                    return vec![Err(InferenceError::Connection(format!(
                        "Okapi streaming returned status {}: {}",
                        status, error_text
                    )))];
                }

                // Read the full SSE/NDJSON response and parse into chunks.
                let body = match response
                    .text()
                    .await
                    .map_err(|e| InferenceError::Connection(e.to_string()))
                {
                    Ok(b) => b,
                    Err(e) => return vec![Err(e)],
                };

                let mut chunks: Vec<Result<InferenceStreamChunk, InferenceError>> = Vec::new();
                for line in body.lines() {
                    let line = line.trim();
                    if line.is_empty() || line == "data: [DONE]" {
                        continue;
                    }
                    // Strip "data: " prefix if present (SSE format)
                    let json_str = line.strip_prefix("data: ").unwrap_or(line);
                    let chunk: StreamChunk = match serde_json::from_str(json_str) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let choice = match chunk.choices.first() {
                        Some(c) => c,
                        None => continue,
                    };

                    let text_delta = choice.delta.content.clone().unwrap_or_default();
                    let finish_reason = choice.finish_reason.clone();

                    let tool_calls = choice
                        .tool_calls
                        .as_ref()
                        .map(|calls| {
                            calls
                                .iter()
                                .map(|tc| hkask_types::ports::StructuredToolCall {
                                    server: tc
                                        .function
                                        .name
                                        .split('/')
                                        .next()
                                        .unwrap_or("")
                                        .to_string(),
                                    tool: tc
                                        .function
                                        .name
                                        .split('/')
                                        .nth(1)
                                        .unwrap_or(&tc.function.name)
                                        .to_string(),
                                    args: tc.function.arguments.clone(),
                                    call_id: tc.id.clone(),
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let usage = chunk.usage.map(|u| InferenceUsage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                    });

                    chunks.push(Ok(InferenceStreamChunk {
                        text_delta,
                        model: chunk.model.clone(),
                        finish_reason: finish_reason.clone(),
                        usage: if finish_reason.is_some() { usage } else { None },
                        tool_calls: if finish_reason.is_some() {
                            tool_calls
                        } else {
                            vec![]
                        },
                    }));
                }

                // If no chunks parsed, return empty final chunk
                if chunks.is_empty() {
                    chunks.push(Ok(InferenceStreamChunk {
                        text_delta: String::new(),
                        model: model_id,
                        finish_reason: Some("stop".to_string()),
                        usage: None,
                        tool_calls: vec![],
                    }));
                }

                chunks
            })
            .map(futures_util::stream::iter)
            .flatten(),
        )
    }

    /// Generate text from images (vision/multimodal) via Okapi.
    ///
    /// Sends base64-encoded images along with a text prompt to a vision-capable
    /// model. Not part of the `InferencePort` trait — this is a direct method
    /// for multimodal inference that doesn't fit the text-only trait signature.
    ///
    /// Falls back to `fallback_model` if the primary model fails.
    pub async fn generate_vision(
        &self,
        prompt: &str,
        images: &[String],
        model_override: Option<&str>,
        fallback_model: Option<&str>,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt).map_err(|e| InferenceError::Generation(e.to_string()))?;

        if images.is_empty() {
            return Err(InferenceError::Generation(
                "No images provided for vision inference".to_string(),
            ));
        }

        let model_id = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.model.clone());

        let request = OkapiRequest {
            model: model_id.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
                images: Some(images.to_vec()),
            }],
            temperature: parameters.temperature,
            top_p: parameters.top_p,
            top_k: parameters.top_k as i32,
            frequency_penalty: parameters.frequency_penalty,
            presence_penalty: parameters.presence_penalty,
            max_tokens: parameters.max_tokens as i32,
            seed: parameters.seed,
            n_probs: Some(5),
            stream: None,
        };

        match self.execute_with_retry(request).await {
            Ok(result) => {
                info!(
                    target: "hkask.inference",
                    model = %result.model,
                    tokens = result.usage.total_tokens,
                    finish_reason = %result.finish_reason,
                    "Vision inference completed"
                );
                Ok(result)
            }
            Err(primary_err) => {
                if let Some(fallback) = fallback_model {
                    if fallback != model_id {
                        warn!(
                            target: "hkask.inference",
                            primary_model = %model_id,
                            fallback_model = %fallback,
                            error = %primary_err,
                            "Primary vision model failed, attempting failover"
                        );

                        let fallback_request = OkapiRequest {
                            model: fallback.to_string(),
                            messages: vec![Message {
                                role: "user".to_string(),
                                content: prompt.to_string(),
                                images: Some(images.to_vec()),
                            }],
                            temperature: parameters.temperature,
                            top_p: parameters.top_p,
                            top_k: parameters.top_k as i32,
                            frequency_penalty: parameters.frequency_penalty,
                            presence_penalty: parameters.presence_penalty,
                            max_tokens: parameters.max_tokens as i32,
                            seed: parameters.seed,
                            n_probs: Some(5),
                            stream: None,
                        };

                        self.execute_with_retry(fallback_request).await
                    } else {
                        Err(primary_err)
                    }
                } else {
                    Err(primary_err)
                }
            }
        }
    }
}

// Okapi wire-format types (private)

/// Okapi API request structure
#[derive(Debug, Clone, Serialize)]
struct OkapiRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    top_p: f32,
    top_k: i32,
    frequency_penalty: f32,
    presence_penalty: f32,
    max_tokens: i32,
    seed: Option<u64>,
    /// Number of top token probabilities to return
    n_probs: Option<i32>,
    /// Enable streaming (SSE) response from the server
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

/// Okapi API response structure
#[derive(Debug, Deserialize)]
struct OkapiResponse {
    model: String,
    choices: Vec<Choice>,
    usage: OkapiUsage,
}

/// Okapi API choice structure
#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    finish_reason: String,
    /// Token probabilities if requested
    #[serde(default, rename = "token_probs")]
    token_probs: Option<Vec<RawTokenProb>>,
    /// Structured tool calls from native function calling
    #[serde(default)]
    tool_calls: Option<Vec<RawToolCall>>,
}

/// Wire-format token usage from Okapi API
#[derive(Debug, Deserialize)]
struct OkapiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Raw token probability from Okapi API
#[derive(Debug, Deserialize)]
struct RawTokenProb {
    token: String,
    prob: f64,
    #[serde(default)]
    top_k: Vec<RawTokenProbTopK>,
}

#[derive(Debug, Deserialize)]
struct RawTokenProbTopK {
    token: String,
    prob: f64,
}

/// Wire-format tool call from Okapi API (OpenAI-compatible function calling)
#[derive(Debug, Deserialize)]
struct RawToolCall {
    /// Unique identifier for the tool call (e.g., "call_abc123")
    id: Option<String>,
    /// The function call details
    #[serde(rename = "function")]
    function: RawFunctionCall,
}

/// Wire-format function call within a tool call
#[derive(Debug, Deserialize)]
struct RawFunctionCall {
    /// The function name, which may be "server/tool" or just "tool"
    name: String,
    /// The JSON arguments for the function call
    #[serde(default)]
    arguments: serde_json::Value,
}

/// Okapi API message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
    /// Base64-encoded images for multimodal/vision requests.
    /// Ollama's chat API accepts `images` per-message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

// ── SSE streaming response types ──────────────────────────────────────────

/// SSE chunk from Okapi streaming endpoint (OpenAI-compatible format).
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
    model: String,
    #[serde(default)]
    usage: Option<OkapiUsage>,
}

/// A single choice within a streaming chunk.
#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<RawToolCall>>,
}

/// Content delta within a streaming chunk.
#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
}
