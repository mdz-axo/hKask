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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use hkask_types::cns::CircuitState;
    use hkask_types::ports::InferenceUsage;
    use hkask_types::ports::{CircuitBreakerPort, InferenceError, InferencePort, InferenceResult};
    use hkask_types::template::LLMParameters;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

    struct MockInferencePort {
        should_fail: AtomicBool,
        call_count: AtomicU64,
    }

    impl MockInferencePort {
        fn new() -> Self {
            Self {
                should_fail: AtomicBool::new(false),
                call_count: AtomicU64::new(0),
            }
        }
        fn succeeding() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl InferencePort for MockInferencePort {
        async fn generate(
            &self,
            _prompt: &str,
            _params: &LLMParameters,
        ) -> Result<InferenceResult, InferenceError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            if self.should_fail.load(Ordering::Relaxed) {
                Err(InferenceError::Connection("mock failure".to_string()))
            } else {
                Ok(InferenceResult {
                    text: "mock response".to_string(),
                    model: "mock-model".to_string(),
                    usage: InferenceUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                    finish_reason: "stop".to_string(),
                    token_probabilities: None,
                })
            }
        }

        async fn generate_with_model(
            &self,
            prompt: &str,
            params: &LLMParameters,
            _model: Option<&str>,
        ) -> Result<InferenceResult, InferenceError> {
            self.generate(prompt, params).await
        }
    }

    struct MockCircuitBreaker {
        allow: AtomicBool,
        success_count: AtomicU64,
        failure_count: AtomicU64,
        state_val: AtomicU32, // 0=Closed, 1=Open, 2=HalfOpen
    }

    impl MockCircuitBreaker {
        fn closed() -> Self {
            Self {
                allow: AtomicBool::new(true),
                success_count: AtomicU64::new(0),
                failure_count: AtomicU64::new(0),
                state_val: AtomicU32::new(0),
            }
        }
        fn open() -> Self {
            Self {
                allow: AtomicBool::new(false),
                success_count: AtomicU64::new(0),
                failure_count: AtomicU64::new(0),
                state_val: AtomicU32::new(1),
            }
        }
    }

    impl CircuitBreakerPort for MockCircuitBreaker {
        fn allow_request(&self) -> bool {
            self.allow.load(Ordering::Relaxed)
        }
        fn record_success(&self) {
            self.success_count.fetch_add(1, Ordering::Relaxed);
        }
        fn record_failure(&self) {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
        }
        fn state(&self) -> CircuitState {
            match self.state_val.load(Ordering::Relaxed) {
                1 => CircuitState::Open,
                2 => CircuitState::HalfOpen,
                _ => CircuitState::Closed,
            }
        }
    }

    #[tokio::test]
    async fn circuit_breaker_adapter_allows_when_closed() {
        let mock_inference = Arc::new(MockInferencePort::succeeding());
        let mock_breaker = Arc::new(MockCircuitBreaker::closed());
        let adapter = InferencePortAdapter::new(mock_inference.clone());
        let cb_adapter = CircuitBreakerInferenceAdapter::new(adapter, mock_breaker.clone());

        let request = GenerateRequest {
            model: "mock-model".to_string(),
            prompt: "hello".to_string(),
            options: None,
        };
        let result = cb_adapter.generate(&request).await;
        assert!(result.is_ok());
        assert_eq!(mock_breaker.success_count.load(Ordering::Relaxed), 1);
        assert_eq!(mock_breaker.failure_count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn circuit_breaker_adapter_rejects_when_open() {
        let mock_inference = Arc::new(MockInferencePort::succeeding());
        let mock_breaker = Arc::new(MockCircuitBreaker::open());
        let adapter = InferencePortAdapter::new(mock_inference.clone());
        let cb_adapter = CircuitBreakerInferenceAdapter::new(adapter, mock_breaker.clone());

        let request = GenerateRequest {
            model: "mock-model".to_string(),
            prompt: "hello".to_string(),
            options: None,
        };
        let result = cb_adapter.generate(&request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            InferenceError::CircuitOpen(_) => {}
            other => panic!("expected CircuitOpen, got {:?}", other),
        }
        // Inference should not have been called
        assert_eq!(mock_inference.call_count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn circuit_breaker_adapter_records_failure_on_error() {
        let mock_inference = Arc::new(MockInferencePort::succeeding());
        mock_inference.should_fail.store(true, Ordering::Relaxed);
        let mock_breaker = Arc::new(MockCircuitBreaker::closed());
        let adapter = InferencePortAdapter::new(mock_inference.clone());
        let cb_adapter = CircuitBreakerInferenceAdapter::new(adapter, mock_breaker.clone());

        let request = GenerateRequest {
            model: "mock-model".to_string(),
            prompt: "hello".to_string(),
            options: None,
        };
        let result = cb_adapter.generate(&request).await;
        assert!(result.is_err());
        assert_eq!(mock_breaker.failure_count.load(Ordering::Relaxed), 1);
        assert_eq!(mock_breaker.success_count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn inference_port_adapter_clone() {
        let mock_inference = Arc::new(MockInferencePort::succeeding());
        let adapter = InferencePortAdapter::new(mock_inference.clone());
        let cloned = adapter.clone();

        let request = GenerateRequest {
            model: "mock-model".to_string(),
            prompt: "hello".to_string(),
            options: None,
        };

        let _ = adapter.generate(&request).await;
        let _ = cloned.generate(&request).await;

        // Both adapters share the same underlying port via Arc
        assert_eq!(mock_inference.call_count.load(Ordering::Relaxed), 2);
    }
}
