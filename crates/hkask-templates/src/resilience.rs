//! Resilience Patterns for Okapi Integration
//!
//! Provides circuit breaker and retry logic for resilient inference.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{error, info};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub open_timeout: Duration,
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            open_timeout: Duration::from_secs(60),
            success_threshold: 2,
        }
    }
}

/// Circuit breaker for Okapi calls
pub struct CircuitBreaker {
    state: AtomicU32,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64,
    config: CircuitBreakerConfig,
    name: String,
}

impl CircuitBreaker {
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU32::new(CircuitState::Closed as u32),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            config,
            name,
        }
    }

    pub fn allow_request(&self) -> bool {
        let state = self.state();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                if last_failure == 0 {
                    return false;
                }

                let elapsed = Instant::now().duration_since(
                    Instant::now() - Duration::from_secs_f64(last_failure as f64 / 1_000_000_000.0),
                );

                if elapsed >= self.config.open_timeout {
                    self.set_state(CircuitState::HalfOpen);
                    self.success_count.store(0, Ordering::Relaxed);
                    info!(
                        target: "hkask.circuit_breaker",
                        name = %self.name,
                        "Circuit transitioned to Half-Open"
                    );
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&self) {
        let state = self.state();

        match state {
            CircuitState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

                if success_count >= self.config.success_threshold {
                    self.set_state(CircuitState::Closed);
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    info!(
                        target: "hkask.circuit_breaker",
                        name = %self.name,
                        "Circuit transitioned to Closed"
                    );
                }
            }
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&self) {
        let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        let now = Instant::now();
        let now_secs = now.duration_since(Instant::now()).as_nanos() as u64;
        self.last_failure_time.store(now_secs, Ordering::Relaxed);

        let state = self.state();

        if state == CircuitState::HalfOpen || failure_count >= self.config.failure_threshold {
            self.set_state(CircuitState::Open);
            self.failure_count.store(0, Ordering::Relaxed);
            error!(
                target: "hkask.circuit_breaker",
                name = %self.name,
                "Circuit transitioned to Open (failures: {})",
                self.config.failure_threshold
            );
        }
    }

    pub fn state(&self) -> CircuitState {
        match self.state.load(Ordering::Relaxed) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    fn set_state(&self, state: CircuitState) {
        self.state.store(state as u32, Ordering::Relaxed);
    }

    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            state: self.state(),
            failure_count: self.failure_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
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

/// Retry error
#[derive(Debug, Error)]
pub enum RetryError {
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Circuit breaker open")]
    CircuitOpen,

    #[error("Timeout: {0}")]
    Timeout(String),
}
