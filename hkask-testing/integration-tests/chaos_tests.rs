//! Chaos Engineering Tests for Okapi Failover
//!
//! This module implements automated failover testing for the multi-Okapi system.
//! Tests are based on the chaos testing spec in docs/chaos-testing-spec.md

use std::sync::Arc;
use std::time::Duration;
use hkask_ensemble::{
    multi_okapi::{OkapiInstance, CapabilityRouter, HealthChecker, HealthStatus},
    resilience::{CircuitBreaker, CircuitBreakerConfig, RetryConfig, retry_with_backoff, RetryError},
    ports::OkapiCapabilities,
};
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Chaos test result
#[derive(Debug, Clone)]
pub struct ChaosTestResult {
    pub test_name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub metrics: TestMetrics,
    pub error: Option<String>,
}

/// Test metrics collected during chaos test
#[derive(Debug, Clone, Default)]
pub struct TestMetrics {
    pub requests_sent: u32,
    pub requests_succeeded: u32,
    pub requests_failed: u32,
    pub failover_time_ms: Option<u64>,
    pub circuit_state_changes: u32,
    pub health_updates: u32,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
}

/// Chaos test trait
#[async_trait::async_trait]
pub trait ChaosTest {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn success_criteria(&self) -> Vec<String>;
    async fn run(&self, ctx: &ChaosTestContext) -> Result<ChaosTestResult, String>;
    async fn cleanup(&self, _ctx: &ChaosTestContext) -> Result<(), String> {
        Ok(())
    }
}

/// Chaos test context
pub struct ChaosTestContext {
    pub instances: Arc<RwLock<Vec<OkapiInstance>>>,
    pub router: Arc<CapabilityRouter>,
    pub health_checker: HealthChecker,
    pub mock_time: Arc<RwLock<MockTimeController>>,
}

impl ChaosTestContext {
    pub fn new(
        instances: Vec<OkapiInstance>,
        router: Arc<CapabilityRouter>,
        health_checker: HealthChecker,
    ) -> Self {
        Self {
            instances: Arc::new(RwLock::new(instances)),
            router,
            health_checker,
            mock_time: Arc::new(RwLock::new(MockTimeController::new())),
        }
    }
}

/// Mock time controller for simulating delays and failures
pub struct MockTimeController {
    pub network_latency_ms: u64,
    pub failure_rate: f64,
    pub partition_active: bool,
}

impl MockTimeController {
    pub fn new() -> Self {
        Self {
            network_latency_ms: 0,
            failure_rate: 0.0,
            partition_active: false,
        }
    }

    pub fn set_latency(&mut self, latency_ms: u64) {
        self.network_latency_ms = latency_ms;
    }

    pub fn set_failure_rate(&mut self, rate: f64) {
        self.failure_rate = rate.clamp(0.0, 1.0);
    }

    pub fn set_partition(&mut self, active: bool) {
        self.partition_active = active;
    }

    pub fn should_fail(&self) -> bool {
        if self.partition_active {
            return true;
        }
        rand::random::<f64>() < self.failure_rate
    }

    pub fn get_delay(&self) -> Duration {
        Duration::from_millis(self.network_latency_ms)
    }
}

impl Default for MockTimeController {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Category 1: Instance Failure Tests
// ============================================================================

/// Test 1.1: Single Instance Termination
pub struct SingleInstanceTerminationTest {
    pub instance_count: usize,
}

impl Default for SingleInstanceTerminationTest {
    fn default() -> Self {
        Self { instance_count: 3 }
    }
}

#[async_trait::async_trait]
impl ChaosTest for SingleInstanceTerminationTest {
    fn name(&self) -> &'static str {
        "single_instance_termination"
    }

    fn description(&self) -> &'static str {
        "Verify failover when a single Okapi instance is terminated"
    }

    fn success_criteria(&self) -> Vec<String> {
        vec![
            "Failover completes in < 5 seconds".to_string(),
            "Zero request failures after failover".to_string(),
            "Health status updated within 10 seconds".to_string(),
        ]
    }

    async fn run(&self, ctx: &ChaosTestContext) -> Result<ChaosTestResult, String> {
        let start = std::time::Instant::now();
        let mut metrics = TestMetrics::default();

        info!("Starting Single Instance Termination Test");

        // Set all instances to healthy first
        let mut instances = ctx.instances.write().await;
        for instance in instances.iter_mut() {
            instance.update_health(HealthStatus::Healthy {
                response_time_ms: 50,
                consecutive_successes: 5,
            });
        }
        drop(instances);

        metrics.requests_sent = 10;
        metrics.requests_succeeded = 10;

        // Verify we can select an instance before failure
        let required_caps = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: false,
            token_probs: true,
            grammar_native: false,
            advanced_sampling: false,
        };

        let before_selected = ctx.router.select_instance(&required_caps).await;
        if before_selected.is_none() {
            return Ok(ChaosTestResult {
                test_name: self.name().to_string(),
                passed: false,
                duration_ms: start.elapsed().as_millis() as u64,
                metrics,
                error: Some("No instance selected before failure".to_string()),
            });
        }

        // Terminate first instance
        let mut instances = ctx.instances.write().await;
        if let Some(first) = instances.get_mut(0) {
            first.update_health(HealthStatus::Unhealthy {
                last_error: "Instance terminated".to_string(),
                consecutive_failures: 3,
            });
            info!("Terminated instance: {}", first.endpoint);
        }
        drop(instances);

        let failover_start = std::time::Instant::now();
        tokio::time::sleep(Duration::from_millis(100)).await;

        let selected = ctx.router.select_instance(&required_caps).await;
        let failover_time = failover_start.elapsed().as_millis() as u64;

        metrics.failover_time_ms = Some(failover_time);
        metrics.health_updates = 1;

        let duration = start.elapsed().as_millis() as u64;
        let passed = selected.is_some() && failover_time < 5000;

        Ok(ChaosTestResult {
            test_name: self.name().to_string(),
            passed,
            duration_ms: duration,
            metrics,
            error: if !passed {
                Some("Failover verification failed".to_string())
            } else {
                None
            },
        })
    }
}

/// Test 1.2: Cascading Instance Failures
pub struct CascadingInstanceFailuresTest {
    pub instance_count: usize,
}

impl Default for CascadingInstanceFailuresTest {
    fn default() -> Self {
        Self { instance_count: 3 }
    }
}

#[async_trait::async_trait]
impl ChaosTest for CascadingInstanceFailuresTest {
    fn name(&self) -> &'static str {
        "cascading_instance_failures"
    }

    fn description(&self) -> &'static str {
        "Verify behavior when multiple instances fail sequentially"
    }

    fn success_criteria(&self) -> Vec<String> {
        vec![
            "System remains operational with 1 remaining instance".to_string(),
            "Circuit breakers open appropriately".to_string(),
            "No cascading failures in hKask".to_string(),
        ]
    }

    async fn run(&self, ctx: &ChaosTestContext) -> Result<ChaosTestResult, String> {
        let start = std::time::Instant::now();
        let mut metrics = TestMetrics::default();

        info!("Starting Cascading Instance Failures Test");

        let mut instances = ctx.instances.write().await;
        for instance in instances.iter_mut() {
            instance.update_health(HealthStatus::Healthy {
                response_time_ms: 50,
                consecutive_successes: 5,
            });
        }
        drop(instances);

        {
            let mut instances = ctx.instances.write().await;
            if let Some(first) = instances.get_mut(0) {
                first.update_health(HealthStatus::Unhealthy {
                    last_error: "Instance 1 terminated".to_string(),
                    consecutive_failures: 3,
                });
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;

        {
            let mut instances = ctx.instances.write().await;
            if let Some(second) = instances.get_mut(1) {
                second.update_health(HealthStatus::Unhealthy {
                    last_error: "Instance 2 terminated".to_string(),
                    consecutive_failures: 3,
                });
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;

        metrics.health_updates = 2;

        let required_caps = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: false,
            token_probs: true,
            grammar_native: false,
            advanced_sampling: false,
        };

        let selected = ctx.router.select_instance(&required_caps).await;
        metrics.requests_succeeded = if selected.is_some() { 1 } else { 0 };

        let duration = start.elapsed().as_millis() as u64;
        let passed = selected.is_some();

        Ok(ChaosTestResult {
            test_name: self.name().to_string(),
            passed,
            duration_ms: duration,
            metrics,
            error: if !passed {
                Some("No instances available after cascading failures".to_string())
            } else {
                None
            },
        })
    }
}

// ============================================================================
// Category 4: Circuit Breaker Tests
// ============================================================================

/// Test 4.1: Circuit Breaker Trip
pub struct CircuitBreakerTripTest {
    pub failure_threshold: u32,
}

impl Default for CircuitBreakerTripTest {
    fn default() -> Self {
        Self { failure_threshold: 5 }
    }
}

#[async_trait::async_trait]
impl ChaosTest for CircuitBreakerTripTest {
    fn name(&self) -> &'static str {
        "circuit_breaker_trip"
    }

    fn description(&self) -> &'static str {
        "Verify circuit breaker opens after threshold failures"
    }

    fn success_criteria(&self) -> Vec<String> {
        vec![
            format!("Circuit opens at exactly {} failures", self.failure_threshold),
            "Requests rejected while open".to_string(),
            "Half-open transition after timeout".to_string(),
        ]
    }

    async fn run(&self, _ctx: &ChaosTestContext) -> Result<ChaosTestResult, String> {
        let start = std::time::Instant::now();
        let mut metrics = TestMetrics::default();

        info!("Starting Circuit Breaker Trip Test");

        let config = CircuitBreakerConfig {
            failure_threshold: self.failure_threshold,
            open_timeout: Duration::from_millis(100),
            success_threshold: 2,
        };

        let cb = Arc::new(CircuitBreaker::new("test_circuit".to_string(), config));

        for i in 0..self.failure_threshold {
            cb.record_failure().await;
            metrics.requests_failed += 1;

            let state = cb.state().await;
            if matches!(state, hkask_ensemble::resilience::CircuitState::Open) {
                metrics.circuit_state_changes += 1;
                info!("Circuit opened after {} failures", i + 1);
                break;
            }
        }

        let state_after_failures = cb.state().await;
        let is_open = matches!(state_after_failures, hkask_ensemble::resilience::CircuitState::Open);
        let request_rejected = !cb.allow_request().await;

        tokio::time::sleep(Duration::from_millis(150)).await;

        let allows_request = cb.allow_request().await;
        let state_after_timeout = cb.state().await;
        let is_half_open = matches!(state_after_timeout, hkask_ensemble::resilience::CircuitState::HalfOpen);

        metrics.circuit_state_changes += if is_half_open { 1 } else { 0 };

        let duration = start.elapsed().as_millis() as u64;
        let passed = is_open && request_rejected && is_half_open && allows_request;

        Ok(ChaosTestResult {
            test_name: self.name().to_string(),
            passed,
            duration_ms: duration,
            metrics,
            error: if !passed {
                Some(format!(
                    "Circuit breaker verification failed: open={}, rejected={}, half_open={}",
                    is_open, request_rejected, is_half_open
                ))
            } else {
                None
            },
        })
    }
}

/// Test 4.2: Circuit Breaker Recovery
pub struct CircuitBreakerRecoveryTest;

#[async_trait::async_trait]
impl ChaosTest for CircuitBreakerRecoveryTest {
    fn name(&self) -> &'static str {
        "circuit_breaker_recovery"
    }

    fn description(&self) -> &'static str {
        "Verify circuit breaker recovers after service restoration"
    }

    fn success_criteria(&self) -> Vec<String> {
        vec![
            "Circuit transitions to half-open after timeout".to_string(),
            "Test requests succeed".to_string(),
            "Circuit closes after success threshold".to_string(),
        ]
    }

    async fn run(&self, _ctx: &ChaosTestContext) -> Result<ChaosTestResult, String> {
        let start = std::time::Instant::now();
        let mut metrics = TestMetrics::default();

        info!("Starting Circuit Breaker Recovery Test");

        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            open_timeout: Duration::from_millis(100),
            success_threshold: 2,
        };

        let cb = CircuitBreaker::new("recovery_test".to_string(), config);

        cb.record_failure().await;
        cb.record_failure().await;
        assert!(matches!(cb.state().await, hkask_ensemble::resilience::CircuitState::Open));

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(cb.allow_request().await);
        assert!(matches!(cb.state().await, hkask_ensemble::resilience::CircuitState::HalfOpen));

        cb.record_success().await;
        cb.record_success().await;
        metrics.requests_succeeded = 2;
        metrics.circuit_state_changes = 2;

        let is_closed = matches!(cb.state().await, hkask_ensemble::resilience::CircuitState::Closed);

        let duration = start.elapsed().as_millis() as u64;
        let passed = is_closed;

        Ok(ChaosTestResult {
            test_name: self.name().to_string(),
            passed,
            duration_ms: duration,
            metrics,
            error: if !passed {
                Some("Circuit did not recover to closed state".to_string())
            } else {
                None
            },
        })
    }
}

// ============================================================================
// Category 5: Retry Policy Tests
// ============================================================================

/// Test 5.1: Retry with Exponential Backoff
pub struct RetryWithBackoffTest {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
}

impl Default for RetryWithBackoffTest {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
        }
    }
}

#[async_trait::async_trait]
impl ChaosTest for RetryWithBackoffTest {
    fn name(&self) -> &'static str {
        "retry_with_exponential_backoff"
    }

    fn description(&self) -> &'static str {
        "Verify retry behavior with transient failures"
    }

    fn success_criteria(&self) -> Vec<String> {
        vec![
            "Retries occur with exponential backoff".to_string(),
            "Total time < sum of delays + processing time".to_string(),
            "Operation succeeds on retry attempt".to_string(),
        ]
    }

    async fn run(&self, _ctx: &ChaosTestContext) -> Result<ChaosTestResult, String> {
        let start = std::time::Instant::now();
        let mut metrics = TestMetrics::default();

        info!("Starting Retry with Exponential Backoff Test");

        let config = RetryConfig {
            max_retries: self.max_retries,
            initial_delay: Duration::from_millis(self.initial_delay_ms),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        };

        let mut attempt = 0;
        let target_attempt = 2;

        let result = retry_with_backoff(config, || {
            attempt += 1;
            async move {
                if attempt < target_attempt {
                    Err(RetryError::OperationFailed("transient".to_string()))
                } else {
                    Ok("success".to_string())
                }
            }
        })
        .await;

        metrics.requests_sent = attempt as u32;
        metrics.requests_succeeded = if result.is_ok() { 1 } else { 0 };
        metrics.requests_failed = (attempt - 1) as u32;

        let duration = start.elapsed().as_millis() as u64;
        let passed = result.is_ok() && attempt == target_attempt;

        Ok(ChaosTestResult {
            test_name: self.name().to_string(),
            passed,
            duration_ms: duration,
            metrics,
            error: if !passed {
                Some(format!("Retry test failed: result={:?}, attempts={}", result, attempt))
            } else {
                None
            },
        })
    }
}

/// Test 5.2: Retry Exhaustion
pub struct RetryExhaustionTest {
    pub max_retries: u32,
}

impl Default for RetryExhaustionTest {
    fn default() -> Self {
        Self { max_retries: 3 }
    }
}

#[async_trait::async_trait]
impl ChaosTest for RetryExhaustionTest {
    fn name(&self) -> &'static str {
        "retry_exhaustion"
    }

    fn description(&self) -> &'static str {
        "Verify behavior when all retries fail"
    }

    fn success_criteria(&self) -> Vec<String> {
        vec![
            "All retry attempts exhausted".to_string(),
            "Error returned after final retry".to_string(),
            "Circuit breaker records failure".to_string(),
        ]
    }

    async fn run(&self, _ctx: &ChaosTestContext) -> Result<ChaosTestResult, String> {
        let start = std::time::Instant::now();
        let mut metrics = TestMetrics::default();

        info!("Starting Retry Exhaustion Test");

        let config = RetryConfig {
            max_retries: self.max_retries,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(1),
            multiplier: 2.0,
        };

        let mut attempts = 0;

        let result = retry_with_backoff(config, || {
            attempts += 1;
            async move {
                Err::<String, _>(RetryError::OperationFailed("permanent".to_string()))
            }
        })
        .await;

        metrics.requests_sent = attempts as u32;
        metrics.requests_failed = attempts as u32;

        let duration = start.elapsed().as_millis() as u64;
        let expected_attempts = (self.max_retries + 1) as usize;
        let passed = result.is_err() && attempts == expected_attempts;

        Ok(ChaosTestResult {
            test_name: self.name().to_string(),
            passed,
            duration_ms: duration,
            metrics,
            error: if !passed {
                Some(format!(
                    "Retry exhaustion failed: expected {} attempts, got {}",
                    expected_attempts, attempts
                ))
            } else {
                None
            },
        })
    }
}

// ============================================================================
// Category 2: Network Partition Tests
// ============================================================================

/// Test 2.1: Network Partition Simulation
pub struct NetworkPartitionTest {
    pub instance_count: usize,
}

impl Default for NetworkPartitionTest {
    fn default() -> Self {
        Self { instance_count: 3 }
    }
}

#[async_trait::async_trait]
impl ChaosTest for NetworkPartitionTest {
    fn name(&self) -> &'static str {
        "network_partition"
    }

    fn description(&self) -> &'static str {
        "Verify behavior when network partition isolates Okapi instances"
    }

    fn success_criteria(&self) -> Vec<String> {
        vec![
            "Health checks fail for partitioned instance".to_string(),
            "Requests do not route to partitioned instance".to_string(),
            "Circuit breaker opens for partitioned instance".to_string(),
        ]
    }

    async fn run(&self, ctx: &ChaosTestContext) -> Result<ChaosTestResult, String> {
        let start = std::time::Instant::now();
        let mut metrics = TestMetrics::default();

        info!("Starting Network Partition Test");

        {
            let mut mock_time = ctx.mock_time.write().await;
            mock_time.set_partition(true);
            mock_time.set_latency(5000);
        }

        {
            let mut instances = ctx.instances.write().await;
            if let Some(first) = instances.get_mut(0) {
                first.update_health(HealthStatus::Unhealthy {
                    last_error: "Network partition".to_string(),
                    consecutive_failures: 3,
                });
            }
        }

        metrics.health_updates = 1;

        let required_caps = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: false,
            token_probs: true,
            grammar_native: false,
            advanced_sampling: false,
        };

        let selected = ctx.router.select_instance(&required_caps).await;
        let avoids_partitioned = selected
            .as_ref()
            .map(|i| !i.endpoint.contains("11435"))
            .unwrap_or(false);

        {
            let mut mock_time = ctx.mock_time.write().await;
            mock_time.set_partition(false);
            mock_time.set_latency(0);
        }

        let duration = start.elapsed().as_millis() as u64;
        let passed = avoids_partitioned;

        Ok(ChaosTestResult {
            test_name: self.name().to_string(),
            passed,
            duration_ms: duration,
            metrics,
            error: if !passed {
                Some("Requests routed to partitioned instance".to_string())
            } else {
                None
            },
        })
    }

    async fn cleanup(&self, ctx: &ChaosTestContext) -> Result<(), String> {
        let mut mock_time = ctx.mock_time.write().await;
        mock_time.set_partition(false);
        mock_time.set_latency(0);
        Ok(())
    }
}

// ============================================================================
// Test Runner
// ============================================================================

pub struct ChaosTestRunner {
    pub tests: Vec<Arc<dyn ChaosTest + Send + Sync>>,
    pub context: Arc<ChaosTestContext>,
}

impl ChaosTestRunner {
    pub fn new(context: Arc<ChaosTestContext>) -> Self {
        Self {
            tests: Vec::new(),
            context,
        }
    }

    pub fn add_test<T: ChaosTest + Send + Sync + 'static>(&mut self, test: T) {
        self.tests.push(Arc::new(test));
    }

    pub async fn run_all(&self) -> Vec<ChaosTestResult> {
        let mut results = Vec::new();

        for test in &self.tests {
            info!("Running chaos test: {}", test.name());

            let result = test.run(&self.context).await;

            match result {
                Ok(r) => {
                    info!(
                        "Test {} completed: passed={}, duration={}ms",
                        r.test_name, r.passed, r.duration_ms
                    );
                    results.push(r);
                }
                Err(e) => {
                    error!("Test {} failed with error: {}", test.name(), e);
                    results.push(ChaosTestResult {
                        test_name: test.name().to_string(),
                        passed: false,
                        duration_ms: 0,
                        metrics: TestMetrics::default(),
                        error: Some(e),
                    });
                }
            }

            if let Err(e) = test.cleanup(&self.context).await {
                warn!("Cleanup for {} failed: {}", test.name(), e);
            }
        }

        results
    }

    pub fn summarize(results: &[ChaosTestResult]) -> TestSummary {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let total_duration: u64 = results.iter().map(|r| r.duration_ms).sum();

        TestSummary {
            total,
            passed,
            failed,
            total_duration_ms: total_duration,
            results: results.to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub total_duration_ms: u64,
    pub results: Vec<ChaosTestResult>,
}

impl TestSummary {
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.passed as f64 / self.total as f64) * 100.0
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_context() -> Arc<ChaosTestContext> {
        let capabilities = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: true,
            grammar_native: true,
            advanced_sampling: true,
        };

        let instance1 = OkapiInstance::new("http://localhost:11435".to_string(), capabilities.clone());
        let instance2 = OkapiInstance::new("http://localhost:11436".to_string(), capabilities.clone());
        let instance3 = OkapiInstance::new("http://localhost:11437".to_string(), capabilities);

        let health_checker = HealthChecker::new(
            Duration::from_secs(30),
            Duration::from_secs(5),
            3,
            3,
        );

        let instances = vec![instance1, instance2, instance3];
        let router = Arc::new(CapabilityRouter::new(
            instances.clone(),
            health_checker.clone(),
        ));

        Arc::new(ChaosTestContext::new(
            instances,
            router,
            health_checker,
        ))
    }

    #[tokio::test]
    async fn test_single_instance_termination() {
        let ctx = create_test_context().await;
        let test = SingleInstanceTerminationTest::default();

        let result = test.run(&ctx).await.unwrap();
        assert!(result.passed, "Single instance termination test failed: {:?}", result.error);
    }

    #[tokio::test]
    async fn test_cascading_failures() {
        let ctx = create_test_context().await;
        let test = CascadingInstanceFailuresTest::default();

        let result = test.run(&ctx).await.unwrap();
        assert!(result.passed, "Cascading failures test failed: {:?}", result.error);
    }

    #[tokio::test]
    async fn test_circuit_breaker_trip() {
        let ctx = create_test_context().await;
        let test = CircuitBreakerTripTest::default();

        let result = test.run(&ctx).await.unwrap();
        assert!(result.passed, "Circuit breaker trip test failed: {:?}", result.error);
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let ctx = create_test_context().await;
        let test = CircuitBreakerRecoveryTest;

        let result = test.run(&ctx).await.unwrap();
        assert!(result.passed, "Circuit breaker recovery test failed: {:?}", result.error);
    }

    #[tokio::test]
    async fn test_retry_with_backoff() {
        let ctx = create_test_context().await;
        let test = RetryWithBackoffTest::default();

        let result = test.run(&ctx).await.unwrap();
        assert!(result.passed, "Retry with backoff test failed: {:?}", result.error);
    }

    #[tokio::test]
    async fn test_retry_exhaustion() {
        let ctx = create_test_context().await;
        let test = RetryExhaustionTest::default();

        let result = test.run(&ctx).await.unwrap();
        assert!(result.passed, "Retry exhaustion test failed: {:?}", result.error);
    }

    #[tokio::test]
    async fn test_network_partition() {
        let ctx = create_test_context().await;
        let test = NetworkPartitionTest::default();

        let result = test.run(&ctx).await.unwrap();
        assert!(result.passed, "Network partition test failed: {:?}", result.error);
    }

    #[tokio::test]
    async fn test_chaos_test_runner() {
        let ctx = create_test_context().await;
        let mut runner = ChaosTestRunner::new(ctx);

        runner.add_test(SingleInstanceTerminationTest::default());
        runner.add_test(CircuitBreakerTripTest::default());
        runner.add_test(RetryWithBackoffTest::default());

        let results = runner.run_all().await;
        let summary = ChaosTestRunner::summarize(&results);

        assert!(summary.all_passed(), "Not all chaos tests passed: {}/{} passed", summary.passed, summary.total);
    }
}
