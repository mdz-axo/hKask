//! Circuit Breaker — Cybernetics Regulation Function
//
//! Circuit breaking is a CNS regulation mechanism: it enforces homeostatic
//! control over external service calls (e.g. Okapi inference) by preventing
//! cascading failures when downstream systems degrade. This is a Cybernetics
//! concern, not a templates concern — the CNS governs when the system must
//! shed load to preserve stability (Ashby's Law of Requisite Variety).

use hkask_types::cns::CircuitState;
use hkask_types::ports::CircuitBreakerPort;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{error, info};

/// Default number of consecutive failures before opening the circuit.
pub(crate) const DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 5;

/// Default duration (in seconds) to keep the circuit open before attempting half-open.
pub(crate) const DEFAULT_CIRCUIT_BREAKER_OPEN_TIMEOUT_SECS: u64 = 60;

/// Default number of consecutive successes in half-open state to close the circuit.
pub(crate) const DEFAULT_CIRCUIT_BREAKER_SUCCESS_THRESHOLD: u32 = 2;

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub(crate) struct CircuitBreakerConfig {
    pub(crate) failure_threshold: u32,
    pub(crate) open_timeout: Duration,
    pub(crate) success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD,
            open_timeout: Duration::from_secs(DEFAULT_CIRCUIT_BREAKER_OPEN_TIMEOUT_SECS),
            success_threshold: DEFAULT_CIRCUIT_BREAKER_SUCCESS_THRESHOLD,
        }
    }
}

/// Circuit breaker for Okapi calls
pub struct CircuitBreaker {
    state: AtomicU32,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64,
    /// Reference Instant for computing elapsed time (stored as nanos since creation).
    created_at: Instant,
    config: CircuitBreakerConfig,
    name: String,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given name and configuration.
    pub(crate) fn new(name: String, config: CircuitBreakerConfig) -> Self {
        let created_at = Instant::now();
        Self {
            state: AtomicU32::new(CircuitState::Closed as u32),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            created_at,
            config,
            name,
        }
    }

    /// Create a circuit breaker with inference-appropriate defaults.
    ///
    /// Suitable for wrapping Okapi inference calls: 5 failures to open,
    /// 60s open timeout, 2 successes to close from half-open.
    pub fn default_for_inference(name: &str) -> Self {
        Self::new(name.to_string(), CircuitBreakerConfig::default())
    }

    pub fn allow_request(&self) -> bool {
        let state = self.state();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure_nanos = self.last_failure_time.load(Ordering::Relaxed);
                if last_failure_nanos == 0 {
                    return false;
                }

                // Reconstruct the last-failure Instant from nanos since creation
                let last_failure_instant =
                    self.created_at + Duration::from_nanos(last_failure_nanos);
                let elapsed = Instant::now().duration_since(last_failure_instant);

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

        // Store nanoseconds since creation as the failure timestamp.
        // `as_nanos()` returns u128; the `as u64` truncation is safe because
        // u64 overflow at 2^64 nanos ≈ 584 years after creation.
        let elapsed_nanos = Instant::now().duration_since(self.created_at).as_nanos() as u64;
        self.last_failure_time
            .store(elapsed_nanos, Ordering::Relaxed);

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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_timeout: Duration::from_millis(100),
            success_threshold: 2,
        }
    }

    #[test]
    fn circuit_starts_closed() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn closed_allows_requests() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        assert!(cb.allow_request());
    }

    #[test]
    fn failures_open_circuit_at_threshold() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn open_rejects_requests() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        // Force open
        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn success_in_closed_resets_failure_count() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        cb.record_failure();
        cb.record_failure(); // 2 failures
        cb.record_success(); // Resets failure count
        cb.record_failure(); // Only 1 failure now
        assert_eq!(cb.state(), CircuitState::Closed); // Not 3 failures yet
    }

    #[test]
    fn failure_in_halfopen_opens_circuit() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        // Open the circuit
        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        // Wait for open_timeout to transition to HalfOpen
        std::thread::sleep(Duration::from_millis(150));
        assert!(cb.allow_request()); // Transitions to HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        // A single failure in HalfOpen opens immediately
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn successes_in_halfopen_close_circuit() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        // Open the circuit
        for _ in 0..3 {
            cb.record_failure();
        }
        // Wait for timeout → HalfOpen
        std::thread::sleep(Duration::from_millis(150));
        cb.allow_request(); // Triggers HalfOpen transition
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        // Two successes close the circuit (success_threshold = 2)
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen); // Not yet
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn default_for_inference_uses_default_config() {
        let cb = CircuitBreaker::default_for_inference("okapi");
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn halfopen_allows_requests() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        for _ in 0..3 {
            cb.record_failure();
        }
        std::thread::sleep(Duration::from_millis(150));
        cb.allow_request(); // Transitions to HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.allow_request()); // HalfOpen allows
    }
}

impl CircuitBreakerPort for CircuitBreaker {
    fn allow_request(&self) -> bool {
        CircuitBreaker::allow_request(self)
    }

    fn record_success(&self) {
        CircuitBreaker::record_success(self)
    }

    fn record_failure(&self) {
        CircuitBreaker::record_failure(self)
    }

    fn state(&self) -> CircuitState {
        CircuitBreaker::state(self)
    }
}
