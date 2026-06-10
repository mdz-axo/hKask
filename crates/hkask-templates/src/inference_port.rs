//! Okapi LLM inference port
//!
//! `InferencePort` trait lives in hkask-types (port membrane).
//! This module provides `OkapiInference` — the concrete implementation.

use crate::okapi_config::{OkapiConfig, validate_prompt};
use futures_util::StreamExt;
use hkask_types::LLMParameters;
use hkask_types::cns::RetryConfig;
use hkask_types::ports::{
    CircuitBreakerPort, InferenceError, InferencePort, InferenceResult, InferenceStreamChunk,
    InferenceUsage, StructuredToolCall, TokenProb, TokenProbability,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

pub struct OkapiInference {
    model: String,
    config: OkapiConfig,
    retry_config: RetryConfig,
    client: Arc<reqwest::Client>,
    circuit_breaker: Option<Arc<dyn CircuitBreakerPort>>,
}

// ── Private helpers ────────────────────────────────────────────────────────

/// Map Okapi raw tool calls to `StructuredToolCall`
fn map_tool_calls(calls: &[RawToolCall]) -> Vec<StructuredToolCall> {
    calls
        .iter()
        .map(|tc| {
            let (server, tool) = tc
                .function
                .name
                .split_once('/')
                .map(|(s, t)| (s.to_string(), t.to_string()))
                .unwrap_or_else(|| (String::new(), tc.function.name.clone()));
            StructuredToolCall {
                server,
                tool,
                args: tc.function.arguments.clone(),
                call_id: tc.id.clone(),
            }
        })
        .collect()
}

fn build_request(
    model: &str,
    prompt: &str,
    images: Option<Vec<String>>,
    params: &LLMParameters,
    stream: Option<bool>,
    n_probs: Option<i32>,
) -> OkapiRequest {
    OkapiRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
            images,
        }],
        temperature: params.temperature,
        top_p: params.top_p,
        top_k: params.top_k as i32,
        frequency_penalty: params.frequency_penalty,
        presence_penalty: params.presence_penalty,
        max_tokens: params.max_tokens as i32,
        seed: params.seed,
        n_probs,
        stream,
    }
}

// ── OkapiInference core ────────────────────────────────────────────────────

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
        })
    }

    async fn execute_request(
        &self,
        request: OkapiRequest,
    ) -> Result<InferenceResult, InferenceError> {
        if let Some(ref cb) = self.circuit_breaker
            && !cb.allow_request()
        {
            tracing::debug!(target: "cns.inference", model = %self.model, "Circuit breaker open");
            return Err(InferenceError::Connection("Circuit breaker is open".into()));
        }
        let mut req = self
            .client
            .post(format!("{}/api/generate", self.config().base_url))
            .json(&request);
        if let Some(auth_header) = self.config().get_authorization_header() {
            req = req.header("Authorization", auth_header);
        }
        let response = req
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if let Some(ref cb) = self.circuit_breaker {
                cb.record_failure();
            }
            return Err(InferenceError::Connection(format!(
                "{}Okapi status {}: {}",
                if self.retry_config.is_retryable_status(status.as_u16()) {
                    "Retryable "
                } else {
                    ""
                },
                status,
                error_text
            )));
        }
        let okapi_response: OkapiResponse = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("Okapi JSON parse: {}", e)))?;
        let choice = okapi_response
            .choices
            .first()
            .ok_or_else(|| InferenceError::Generation("Empty response from Okapi".to_string()))?;
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
        if let Some(ref cb) = self.circuit_breaker {
            cb.record_success();
        }
        let tool_calls = choice
            .tool_calls
            .as_ref()
            .map(|calls| map_tool_calls(calls))
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
                        warn!(target: "hkask.inference", %attempt, %delay_ms, error = ?last_error, "Retryable error, waiting");
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
        Err(last_error.expect("retry loop always records the last error"))
    }
}

// ── InferencePort impl ─────────────────────────────────────────────────────

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
            validate_prompt(&prompt).map_err(|e| InferenceError::Generation(e.to_string()))?;
            let result = self
                .execute_with_retry(build_request(
                    &self.model,
                    &prompt,
                    None,
                    &parameters,
                    None,
                    Some(5),
                ))
                .await?;
            info!(target: "hkask.inference", model = %result.model, tokens = result.usage.total_tokens, finish_reason = %result.finish_reason, "Inference completed");
            Ok(result)
        })
    }
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
            validate_prompt(&prompt).map_err(|e| InferenceError::Generation(e.to_string()))?;
            let model_id = model_override.unwrap_or_else(|| self.model.clone());
            let result = self
                .execute_with_retry(build_request(
                    &model_id,
                    &prompt,
                    None,
                    &parameters,
                    None,
                    Some(5),
                ))
                .await?;
            info!(target: "hkask.inference", model = %result.model, tokens = result.usage.total_tokens, finish_reason = %result.finish_reason, "Inference with model completed");
            Ok(result)
        })
    }
}

// ── Direct impl (not part of InferencePort trait) ──────────────────────────

impl OkapiInference {
    /// Stream inference with optional model override (SSE parsing).
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
        let base_url = self.config().base_url.clone();
        let auth_header = self.config().get_authorization_header();

        Box::pin(
            futures_util::stream::once(async move {
                let request =
                    build_request(&model_id, &prompt, None, &parameters, Some(true), None);

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
                        "Okapi streaming status {}: {}",
                        status, error_text
                    )))];
                }

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
                        .map(|calls| map_tool_calls(calls))
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

    /// Vision/multimodal inference via Okapi. Falls back to `fallback_model` on failure.
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
        let request = build_request(
            &model_id,
            prompt,
            Some(images.to_vec()),
            parameters,
            None,
            Some(5),
        );

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
                if let Some(fallback) = fallback_model
                    && fallback != model_id
                {
                    warn!(
                        target: "hkask.inference",
                        %model_id, fallback_model = %fallback, error = %primary_err,
                        "Primary vision model failed, failover"
                    );
                    let fb_req = build_request(
                        fallback,
                        prompt,
                        Some(images.to_vec()),
                        parameters,
                        None,
                        Some(5),
                    );
                    self.execute_with_retry(fb_req).await
                } else {
                    Err(primary_err)
                }
            }
        }
    }
}

// ── Okapi wire-format types (private) ──────────────────────────────────────

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
    n_probs: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OkapiResponse {
    model: String,
    choices: Vec<Choice>,
    usage: OkapiUsage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    finish_reason: String,
    #[serde(default, rename = "token_probs")]
    token_probs: Option<Vec<RawTokenProb>>,
    #[serde(default)]
    tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OkapiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

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

#[derive(Debug, Deserialize)]
struct RawToolCall {
    id: Option<String>,
    #[serde(rename = "function")]
    function: RawFunctionCall,
}

#[derive(Debug, Deserialize)]
struct RawFunctionCall {
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
    /// Base64-encoded images for multimodal/vision requests (Ollama chat API `images` field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

// ── SSE streaming response types ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
    model: String,
    #[serde(default)]
    usage: Option<OkapiUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
}
