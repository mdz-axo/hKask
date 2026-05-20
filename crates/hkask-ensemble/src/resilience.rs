//! Production Hardening for Okapi Integration
//!
//! Provides circuit breaker, retry policies, and exponential backoff
//! for resilient Okapi communication in production environments.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected immediately
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Duration circuit stays open before half-open
    pub open_timeout: Duration,
    /// Number of successes in half-open to close circuit
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            open_timeout: Duration::from_secs(30),
            success_threshold: 2,
        }
    }
}

/// Circuit breaker for Okapi calls
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: RwLock<u32>,
    success_count: RwLock<u32>,
    last_failure_time: RwLock<Option<std::time::Instant>>,
    config: CircuitBreakerConfig,
    name: String,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: RwLock::new(0),
            success_count: RwLock::new(0),
            last_failure_time: RwLock::new(None),
            config,
            name,
        }
    }

    /// Check if request should be allowed
    pub async fn allow_request(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() >= self.config.open_timeout {
                        // Transition to half-open
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                        info!(
                            target: "hkask.circuit_breaker",
                            name = %self.name,
                            "Circuit transitioned to Half-Open"
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record successful call
    pub async fn record_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    *success_count = 0;
                    info!(
                        target: "hkask.circuit_breaker",
                        name = %self.name,
                        "Circuit transitioned to Closed"
                    );
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Record failed call
    pub async fn record_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;
        *self.last_failure_time.write().await = Some(std::time::Instant::now());

        let state = *self.state.read().await;

        if state == CircuitState::HalfOpen || *failure_count >= self.config.failure_threshold {
            *self.state.write().await = CircuitState::Open;
            *failure_count = 0;
            error!(
                target: "hkask.circuit_breaker",
                name = %self.name,
                "Circuit transitioned to Open (failures: {})",
                self.config.failure_threshold
            );
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get statistics
    pub async fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            state: self.state().await,
            failure_count: *self.failure_count.read().await,
            success_count: *self.success_count.read().await,
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

/// Retry with exponential backoff
pub async fn retry_with_backoff<F, Fut, T>(
    config: RetryConfig,
    mut operation: F,
) -> Result<T, RetryError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, RetryError>>,
{
    let mut delay = config.initial_delay;

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
                    delay_ms = %delay.as_millis(),
                    error = %e,
                    "Retry attempt failed, backing off"
                );

                tokio::time::sleep(delay).await;

                // Exponential backoff with max cap
                delay = std::cmp::min(
                    Duration::from_millis((delay.as_millis() as f64 * config.multiplier) as u64),
                    config.max_delay,
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
    retry_config: RetryConfig,
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
        retry_config: RetryConfig,
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
                if !cb.allow_request().await {
                    return Err(RetryError::CircuitOpen);
                }

                match op.await {
                    Ok(result) => {
                        cb.record_success().await;
                        Ok(result)
                    }
                    Err(e) => {
                        cb.record_failure().await;
                        Err(e)
                    }
                }
            }
        })
        .await
    }

    /// Get circuit breaker stats
    pub async fn circuit_stats(&self) -> CircuitBreakerStats {
        self.circuit_breaker.stats().await
    }
}
