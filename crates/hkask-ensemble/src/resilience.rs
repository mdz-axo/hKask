//! Production Hardening for Okapi Integration
//!
//! Provides circuit breaker, retry policies, and exponential backoff
//! for resilient Okapi communication in production environments.
//!
//! CircuitBreaker is re-exported from hkask-templates (canonical implementation).

use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

// Re-export canonical CircuitBreaker types from hkask-templates
pub use hkask_templates::resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitState,
};

/// Retry configuration — alias for the canonical RetryConfig
pub type EnsembleEnsembleRetryConfig = hkask_types::cns::RetryConfig;

/// Retry with exponential backoff
pub async fn retry_with_backoff<F, Fut, T>(
    config: EnsembleEnsembleRetryConfig,
    mut operation: F,
) -> Result<T, RetryError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, RetryError>>,
{
    let mut delay_ms = config.initial_delay_ms;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt >= config.max_retries {
                    return Err(e);
                }

                warn!(
                    target: "hkask.retry",
                    attempt = %attempt,
                    max_retries = %config.max_retries,
                    delay_ms = %delay_ms,
                    error = %e,
                    "Retry attempt failed, backing off"
                );

                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

                // Exponential backoff with max cap
                delay_ms = std::cmp::min(
                    (delay_ms as f64 * config.multiplier) as u64,
                    config.max_delay_ms,
                );
            }
        }
    }

    unreachable!()
}

/// Retry error
#[derive(Debug, thiserror::Error)]
pub enum RetryError {
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Circuit breaker open")]
    CircuitOpen,

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Resilient Okapi client wrapper
pub struct ResilientOkapiClient<C> {
    inner: C,
    circuit_breaker: Arc<CircuitBreaker>,
    retry_config: EnsembleEnsembleRetryConfig,
}

impl<C> ResilientOkapiClient<C>
where
    C: Clone,
{
    /// Create new resilient client
    pub fn new(
        inner: C,
        name: String,
        circuit_config: CircuitBreakerConfig,
        retry_config: EnsembleEnsembleRetryConfig,
    ) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(name, circuit_config));

        Self {
            inner,
            circuit_breaker,
            retry_config,
        }
    }

    /// Execute operation with circuit breaker and retry
    pub async fn execute<F, Fut, T>(&self, mut operation: F) -> Result<T, RetryError>
    where
        F: FnMut(C) -> Fut,
        Fut: std::future::Future<Output = Result<T, RetryError>>,
    {
        let cb = Arc::clone(&self.circuit_breaker);
        let retry_config = self.retry_config.clone();
        let inner = self.inner.clone();

        retry_with_backoff(retry_config, || {
            let cb = Arc::clone(&cb);
            let inner = inner.clone();
            let op = operation(inner);

            async move {
                if !cb.allow_request() {
                    return Err(RetryError::CircuitOpen);
                }

                match op.await {
                    Ok(result) => {
                        cb.record_success();
                        Ok(result)
                    }
                    Err(e) => {
                        cb.record_failure();
                        Err(e)
                    }
                }
            }
        })
        .await
    }

    /// Get circuit breaker stats
    pub async fn circuit_stats(&self) -> CircuitBreakerStats {
        self.circuit_breaker.stats()
    }
}
