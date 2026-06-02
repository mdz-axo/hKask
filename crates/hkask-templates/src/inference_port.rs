//! Okapi LLM inference port for high-temperature templates
//!
//! This module provides the InferencePort trait for LLM invocations
//! with temperature-controlled parameters for anti-normative generation.
//!
//! # Example
//!
//! ```rust,no_run
//! use hkask_templates::{OkapiInference, OkapiConfig, InferencePort};
//! use hkask_types::LLMParameters;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create Okapi inference client
//! let config = OkapiConfig::local_dev();
//! let inference = OkapiInference::new("ollama/llama-3.1-8b-instruct", config)?;
//!
//! // Generate text
//! let params = LLMParameters::default();
//! let result = inference.generate("What is the meaning of life?", &params).await?;
//!
//! println!("Response: {}", result.text);
//! # Ok(())
//! # }
//! ```

use crate::manifest::ModelRequirements;
use crate::okapi_config::{OkapiConfig, validate_prompt};
use crate::resilience::CircuitBreaker;
use async_trait::async_trait;
use hkask_types::LLMParameters;
use hkask_types::cns::RetryConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

/// Inference error types
///
/// Errors returned by Okapi inference operations.
#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Okapi connection error: {0}")]
    Connection(String),
    #[error("Model error: {0}")]
    Model(String),
    #[error("Generation error: {0}")]
    Generation(String),
    #[error("JSON error: {0}")]
    Json(String),
}

/// Inference result from Okapi
///
/// Contains the generated text, model used, token usage, and optional token probabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub text: String,
    pub model: String,
    pub usage: Usage,
    pub finish_reason: String,
    /// Token-level probabilities for confidence scoring
    pub token_probabilities: Option<Vec<TokenProbability>>,
}

/// Token probability from Okapi response
/// Contains the token and its probability, plus top-k alternatives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenProbability {
    pub token: String,
    pub prob: f64,
    pub top_k: Vec<TokenProb>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenProb {
    pub token: String,
    pub prob: f64,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Okapi inference port
/// Trait for LLM backends. Okapi is the default implementation.
#[async_trait]
pub trait InferencePort: Send + Sync {
    /// Generate text with parameters
    async fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError>;

    /// Generate text with model requirements
    async fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        _model_requirements: Option<&ModelRequirements>,
    ) -> Result<InferenceResult, InferenceError> {
        // Default implementation ignores model_requirements
        self.generate(prompt, parameters).await
    }

    /// Generate multiple outputs for template selection
    async fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> Result<Vec<InferenceResult>, InferenceError> {
        use futures_util::future::join_all;

        // Create n concurrent generate futures
        let futures: Vec<_> = (0..n).map(|_| self.generate(prompt, parameters)).collect();

        // Execute all futures concurrently
        let results = join_all(futures).await;

        // Collect results, returning first error if any
        results.into_iter().collect()
    }
}

/// Okapi-backed inference implementation
pub struct OkapiInference {
    model: String,
    config: OkapiConfig,
    retry_config: RetryConfig,
    client: Arc<reqwest::Client>,
    /// CNS span emitter
    /// Circuit breaker for resilience
    circuit_breaker: Option<Arc<CircuitBreaker>>,
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

    /// Create OkapiInference with a shared HTTP client
    pub fn with_shared_client(
        model: &str,
        config: OkapiConfig,
        client: Arc<reqwest::Client>,
    ) -> Self {
        Self {
            model: model.to_string(),
            retry_config: RetryConfig::default(),
            config,
            client,
            circuit_breaker: None,
            prompt_cache: None,
        }
    }

    pub fn with_retry_config(
        model: &str,
        config: OkapiConfig,
        retry_config: RetryConfig,
    ) -> Result<Self, InferenceError> {
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        Ok(Self {
            model: model.to_string(),
            retry_config,
            config,
            client,
            circuit_breaker: None,
            prompt_cache: None,
        })
    }

    pub fn with_circuit_breaker(
        model: &str,
        config: OkapiConfig,
        retry_config: RetryConfig,
        circuit_breaker: CircuitBreaker,
    ) -> Result<Self, InferenceError> {
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        Ok(Self {
            model: model.to_string(),
            retry_config,
            config,
            client,
            circuit_breaker: Some(Arc::new(circuit_breaker)),
            prompt_cache: None,
        })
    }

    /// Default local Okapi endpoint (no auth)
    pub fn local(model: &str) -> Result<Self, InferenceError> {
        Self::new(model, OkapiConfig::local_dev())
    }

    /// Fast local model preset
    pub fn fast_local() -> Result<Self, InferenceError> {
        Self::local("fast-local-model")
    }

    /// Attach a prompt cache to skip redundant LLM calls
    pub fn with_cache(mut self, cache: crate::prompt_cache::PromptCache) -> Self {
        self.prompt_cache = Some(Arc::new(cache));
        self
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

        Ok(InferenceResult {
            text: choice.message.content.clone(),
            model: okapi_response.model.clone(),
            usage: Usage {
                prompt_tokens: okapi_response.usage.prompt_tokens,
                completion_tokens: okapi_response.usage.completion_tokens,
                total_tokens: okapi_response.usage.total_tokens,
            },
            finish_reason: choice.finish_reason.clone(),
            token_probabilities,
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

#[async_trait]
impl InferencePort for OkapiInference {
    async fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        // Validate input
        validate_prompt(prompt).map_err(|e| InferenceError::Generation(e.to_string()))?;

        // Check prompt cache before API call
        if let Some(ref cache) = self.prompt_cache {
            let cache_key =
                crate::prompt_cache::PromptCache::generate_key(prompt, &self.model, parameters);
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
                content: prompt.to_string(),
            }],
            temperature: parameters.temperature,
            top_p: parameters.top_p,
            top_k: parameters.top_k as i32,
            frequency_penalty: parameters.frequency_penalty,
            presence_penalty: parameters.presence_penalty,
            max_tokens: parameters.max_tokens as i32,
            seed: parameters.seed,
            n_probs: Some(5),
        };

        let result = self.execute_with_retry(request).await?;

        // Cache the successful result
        if let Some(ref cache) = self.prompt_cache {
            let cache_key =
                crate::prompt_cache::PromptCache::generate_key(prompt, &self.model, parameters);
            if let Err(e) = cache.put(&cache_key, prompt, &self.model, &result) {
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
    }

    async fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_requirements: Option<&ModelRequirements>,
    ) -> Result<InferenceResult, InferenceError> {
        // Validate input
        validate_prompt(prompt).map_err(|e| InferenceError::Generation(e.to_string()))?;

        let model_id = model_requirements
            .map(|r| r.required.clone())
            .unwrap_or_else(|| self.model.clone());

        let request = OkapiRequest {
            model: model_id.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: parameters.temperature,
            top_p: parameters.top_p,
            top_k: parameters.top_k as i32,
            frequency_penalty: parameters.frequency_penalty,
            presence_penalty: parameters.presence_penalty,
            max_tokens: parameters.max_tokens as i32,
            seed: parameters.seed,
            n_probs: Some(5),
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
    }
}

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
}

/// Okapi API response structure
#[derive(Debug, Deserialize)]
struct OkapiResponse {
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

/// Okapi API choice structure
#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    finish_reason: String,
    /// Token probabilities if requested
    #[serde(default, rename = "token_probs")]
    token_probs: Option<Vec<RawTokenProb>>,
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

/// Okapi API message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}
