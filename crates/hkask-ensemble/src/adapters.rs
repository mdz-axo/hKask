//! Okapi Infrastructure Adapters
//
//! Adapter that wraps `InferencePort` (from hkask-types) to implement the
//! ensemble-specific `InferenceClient` trait. This replaces the former
//! `OkapiClient` which duplicated HTTP logic already in `OkapiInference`.
//
//! Circuit-breaker decorated adapter wraps `InferencePortAdapter` with
//! a `CircuitBreakerPort` membrane for fail-fast inference protection.

use crate::ports::{GenerateRequest, GenerateResponse, InferenceClient};
use async_trait::async_trait;
use hkask_types::ports::{CircuitBreakerPort, InferenceError, InferencePort};
use hkask_types::template::LLMParameters;
use std::sync::Arc;

/// Adapter wrapping an `InferencePort` to satisfy the ensemble's `InferenceClient` trait.
///
/// This is the recommended way to obtain an `InferenceClient` for ensemble
/// improv sessions. Use `InferencePortAdapter::new(port)` with any
/// `InferencePort` implementation (e.g. `OkapiInference` from hkask-templates).
#[derive(Clone)]
pub struct InferencePortAdapter {
    port: Arc<dyn InferencePort>,
}

impl InferencePortAdapter {
    /// Create an adapter wrapping an existing `InferencePort`.
    pub fn new(port: Arc<dyn InferencePort>) -> Self {
        Self { port }
    }

    /// Access the underlying inference port (for loop system wiring).
    pub fn port(&self) -> &Arc<dyn InferencePort> {
        &self.port
    }
}

#[async_trait]
impl InferenceClient for InferencePortAdapter {
    type Error = InferenceError;

    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        let params = LLMParameters {
            max_tokens: request
                .options
                .as_ref()
                .and_then(|o| o.max_tokens)
                .unwrap_or(512) as u32,
            temperature: request
                .options
                .as_ref()
                .and_then(|o| o.temperature)
                .unwrap_or(0.7) as f32,
            ..LLMParameters::default()
        };

        let result = self
            .port
            .generate_with_model(&request.prompt, &params, Some(&request.model))
            .await?;

        Ok(GenerateResponse {
            response: result.text,
            model: result.model,
            completion_probabilities: result.token_probabilities,
        })
    }

    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        model: String,
    ) -> Result<serde_json::Value, Self::Error> {
        // Build a prompt from messages for the generate endpoint
        let prompt = messages
            .iter()
            .filter_map(|m| {
                let role = m.get("role")?.as_str().unwrap_or("user");
                let content = m.get("content")?.as_str().unwrap_or("");
                Some(format!("[{}]: {}", role, content))
            })
            .collect::<Vec<_>>()
            .join("\n");

        let params = LLMParameters::default();
        let result = self
            .port
            .generate_with_model(&prompt, &params, Some(&model))
            .await?;

        Ok(serde_json::json!({
            "response": result.text,
            "model": result.model,
        }))
    }
}

/// Circuit-breaker decorated inference adapter.
///
/// Wraps `InferencePortAdapter` with a `CircuitBreakerPort` membrane.
/// Before each inference call, checks `allow_request()`. If the circuit
/// is open, the call is rejected immediately (fail-fast). On success,
/// records success; on failure, records failure.
pub struct CircuitBreakerInferenceAdapter {
    inner: InferencePortAdapter,
    breaker: Arc<dyn CircuitBreakerPort>,
}

impl CircuitBreakerInferenceAdapter {
    /// Create a new circuit-breaker decorated adapter.
    pub fn new(inner: InferencePortAdapter, breaker: Arc<dyn CircuitBreakerPort>) -> Self {
        Self { inner, breaker }
    }

    /// Access the underlying inference adapter.
    pub fn inner(&self) -> &InferencePortAdapter {
        &self.inner
    }

    /// Access the circuit breaker port.
    pub fn breaker(&self) -> &Arc<dyn CircuitBreakerPort> {
        &self.breaker
    }
}

#[async_trait]
impl InferenceClient for CircuitBreakerInferenceAdapter {
    type Error = InferenceError;

    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        if !self.breaker.allow_request() {
            return Err(InferenceError::CircuitOpen(format!(
                "Circuit is {:?}, rejecting generate request for model {}",
                self.breaker.state(),
                request.model,
            )));
        }

        match self.inner.generate(request).await {
            Ok(response) => {
                self.breaker.record_success();
                Ok(response)
            }
            Err(e) => {
                self.breaker.record_failure();
                Err(e)
            }
        }
    }

    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        model: String,
    ) -> Result<serde_json::Value, Self::Error> {
        if !self.breaker.allow_request() {
            return Err(InferenceError::CircuitOpen(format!(
                "Circuit is {:?}, rejecting chat request for model {}",
                self.breaker.state(),
                model,
            )));
        }

        match self.inner.chat(messages, model).await {
            Ok(response) => {
                self.breaker.record_success();
                Ok(response)
            }
            Err(e) => {
                self.breaker.record_failure();
                Err(e)
            }
        }
    }
}
