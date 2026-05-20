# Automated Failover Testing — Chaos Engineering for Okapi

**Version:** 1.1.0  
**Status:** Core resilience components implemented, unit tests passing

---

## Implementation Status

### Implemented (v1.1)

The following chaos testing components have been implemented in `hkask-ensemble`:

1. **Circuit Breaker** (`hkask-ensemble/src/resilience.rs`)
   - Closed → Open → HalfOpen state machine
   - Configurable failure threshold (default: 5)
   - Configurable open timeout (default: 30s)
   - Configurable success threshold (default: 2)
   - Unit tests: 4 passing

2. **Retry with Exponential Backoff** (`hkask-ensemble/src/resilience.rs`)
   - Configurable max retries (default: 3)
   - Configurable initial delay (default: 100ms)
   - Configurable max delay (default: 10s)
   - Configurable multiplier (default: 2.0)
   - Unit tests: 2 passing

3. **Multi-Okapi Failover** (`hkask-ensemble/src/multi_okapi.rs`)
   - OkapiInstance with health tracking
   - HealthChecker for instance monitoring
   - CapabilityRouter for instance selection
   - HealthStatus: Healthy, Degraded, Unhealthy, Unknown
   - Unit tests: 4 passing

4. **ResilientOkapiClient** (`hkask-ensemble/src/resilience.rs`)
   - Combines circuit breaker + retry policies
   - Automatic failure recording
   - Unit tests: 1 passing

5. **Unit Test Coverage**
   - `test_circuit_breaker_transitions`: Validates state machine
   - `test_retry_with_backoff_success/failure`: Validates retry logic
   - `test_resilient_client`: Validates combined resilience
   - `test_health_status_transitions`: Validates health tracking
   - `test_instance_availability`: Validates instance selection
   - `test_capability_router`: Validates routing logic

### In Progress

1. **Integration Tests** - Full end-to-end chaos testing framework requiring live Okapi instances

### Not Yet Implemented

1. **Grafana/Prometheus Metrics Dashboard**
2. **Automated Test Scheduling**
3. **Network Partition Injection (tc/chaos-mesh)**
4. **Resource Exhaustion Tests (stress-ng)**
5. **CI/CD Pipeline Integration**

---

## Overview

This document specifies chaos engineering tests for validating Okapi failover behavior under adverse conditions.

---

## Chaos Test Categories

### Category 1: Instance Failure Tests

#### Test 1.1: Single Instance Termination

**Objective:** Verify failover when a single Okapi instance is terminated.

**Procedure:**
1. Start 3 Okapi instances (okapi-1, okapi-2, okapi-3)
2. Route all requests through hKask capability router
3. After 60 seconds, terminate okapi-1
4. Observe:
   - Requests should be rerouted to okapi-2 or okapi-3 within 5 seconds
   - No requests should fail after failover
   - Health checker should detect failure within 10 seconds

**Success Criteria:**
- Failover completes in < 5 seconds
- Zero request failures after failover
- Health status updated within 10 seconds

**Chaos Tool:** `chaos-mesh` or custom script
```bash
# Terminate okapi-1
kubectl delete pod okapi-1

# Or with chaos-mesh
kubectl apply -f pod-kill-okapi-1.yaml
```

#### Test 1.2: Cascading Instance Failures

**Objective:** Verify behavior when multiple instances fail sequentially.

**Procedure:**
1. Start 3 Okapi instances
2. After 60 seconds, terminate okapi-1
3. After 30 more seconds, terminate okapi-2
4. Observe:
   - All requests should route to okapi-3
   - Circuit breakers should open for failed instances
   - System should remain operational

**Success Criteria:**
- System remains operational with 1 remaining instance
- Circuit breakers open appropriately
- No cascading failures in hKask

---

### Category 2: Network Partition Tests

#### Test 2.1: Network Partition Between hKask and Okapi

**Objective:** Verify behavior when network partition isolates Okapi instances.

**Procedure:**
1. Start hKask and 3 Okapi instances
2. After 60 seconds, introduce network partition between hKask and okapi-1
3. Observe:
   - Health checks should fail for okapi-1
   - Requests should not route to okapi-1
   - Circuit breaker should open for okapi-1

**Chaos Tool:** `tc` (traffic control) or `chaos-mesh`
```bash
# Drop packets to okapi-1
tc qdisc add dev eth0 root netem loss 100% dst 10.0.0.1

# Or with chaos-mesh
kubectl apply -f network-partition-okapi-1.yaml
```

#### Test 2.2: High Latency Injection

**Objective:** Verify behavior under high network latency.

**Procedure:**
1. Start hKask and 3 Okapi instances
2. After 60 seconds, inject 500ms latency to okapi-1
3. Observe:
   - Requests should prefer low-latency instances
   - okapi-1 should be marked as degraded
   - Load balancer should reduce traffic to okapi-1

**Success Criteria:**
- Latency detected within 30 seconds
- Traffic shifted away from high-latency instance
- Circuit breaker does not open (instance still functional)

---

### Category 3: Resource Exhaustion Tests

#### Test 3.1: Okapi Memory Exhaustion

**Objective:** Verify behavior when Okapi instance runs out of memory.

**Procedure:**
1. Start 3 Okapi instances with memory limits
2. Generate load to cause memory pressure on okapi-1
3. Observe:
   - Memory usage increases
   - Instance becomes unresponsive
   - Health checks fail
   - Failover occurs

**Chaos Tool:** `stress-ng`
```bash
# Stress memory on okapi-1
kubectl exec okapi-1 -- stress-ng --vm 4 --vm-bytes 2G --timeout 60s
```

#### Test 3.2: Context Window Exhaustion

**Objective:** Verify behavior when Okapi context windows are full.

**Procedure:**
1. Start Okapi with limited context window
2. Send requests with large prompts to fill context
3. Observe:
   - Context utilization increases
   - New requests may fail or be routed elsewhere
   - System recovers when context is freed

---

### Category 4: Circuit Breaker Tests

#### Test 4.1: Circuit Breaker Trip

**Objective:** Verify circuit breaker opens after threshold failures.

**Procedure:**
1. Configure circuit breaker with failure_threshold=5
2. Send requests that will fail (e.g., to partitioned instance)
3. Observe:
   - Circuit opens after 5 failures
   - Subsequent requests rejected immediately
   - Circuit transitions to half-open after timeout

**Success Criteria:**
- Circuit opens at exactly 5 failures
- Requests rejected while open
- Half-open transition after timeout

#### Test 4.2: Circuit Breaker Recovery

**Objective:** Verify circuit breaker recovers after service restoration.

**Procedure:**
1. Trip circuit breaker
2. Restore failed instance
3. Observe:
   - Circuit transitions to half-open
   - Test requests succeed
   - Circuit closes after success threshold

---

### Category 5: Retry Policy Tests

#### Test 5.1: Retry with Exponential Backoff

**Objective:** Verify retry behavior with transient failures.

**Procedure:**
1. Configure retry with max_retries=3, initial_delay=100ms
2. Send request that fails twice, succeeds on third attempt
3. Observe:
   - First attempt fails
   - Retry after 100ms
   - Second attempt fails
   - Retry after 200ms
   - Third attempt succeeds

**Success Criteria:**
- Retries occur with exponential backoff
- Total time < sum of delays + processing time
- Operation succeeds on third attempt

#### Test 5.2: Retry Exhaustion

**Objective:** Verify behavior when all retries fail.

**Procedure:**
1. Configure retry with max_retries=3
2. Send request that always fails
3. Observe:
   - All 3 retries attempted
   - Error returned after final retry
   - Circuit breaker records failure

---

## Chaos Test Infrastructure

### Test Runner

```rust
// chaos_tests/runner.rs
pub struct ChaosTestRunner {
    okapi_cluster: OkapiCluster,
    hkask_client: HkaskClient,
    metrics_collector: MetricsCollector,
}

impl ChaosTestRunner {
    pub async fn run_test(&self, test: ChaosTest) -> TestResult {
        // 1. Establish baseline
        let baseline = self.collect_metrics().await;

        // 2. Inject chaos
        test.inject_chaos(&self.okapi_cluster).await?;

        // 3. Monitor during chaos
        let during = self.collect_metrics_during(test.duration).await;

        // 4. Verify success criteria
        let result = test.verify(&baseline, &during);

        // 5. Cleanup
        test.cleanup(&self.okapi_cluster).await?;

        result
    }
}
```

### Metrics Collection

```rust
// chaos_tests/metrics.rs
pub struct MetricsCollector {
    prometheus_client: PrometheusClient,
}

impl MetricsCollector {
    pub async fn collect(&self) -> ClusterMetrics {
        ClusterMetrics {
            request_rate: self.get_metric("okapi_requests_total").await,
            latency_p99: self.get_metric("okapi_request_duration_seconds{quantile=\"0.99\"}").await,
            error_rate: self.get_metric("okapi_requests_total{status=\"error\"}").await,
            circuit_breaker_state: self.get_metric("cns_circuit_breaker_state").await,
            health_status: self.get_metric("okapi_health_status").await,
        }
    }
}
```

---

## Test Execution Schedule

### Daily Tests (Automated)
- Test 1.1: Single Instance Termination
- Test 4.1: Circuit Breaker Trip
- Test 5.1: Retry with Exponential Backoff

### Weekly Tests (Automated)
- Test 1.2: Cascading Instance Failures
- Test 2.1: Network Partition
- Test 3.1: Memory Exhaustion

### Monthly Tests (Manual)
- Test 2.2: High Latency Injection
- Test 3.2: Context Window Exhaustion
- Test 4.2: Circuit Breaker Recovery
- Test 5.2: Retry Exhaustion

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Failover Time | < 5 seconds | Time from failure to first successful request on new instance |
| Request Success Rate | > 99.9% | Percentage of successful requests during chaos |
| Circuit Breaker Accuracy | 100% | Circuit opens/closes at correct thresholds |
| Recovery Time | < 30 seconds | Time from restoration to full capacity |
| False Positive Rate | < 1% | Percentage of incorrect failover decisions |

---

## Implementation Checklist

- [x] Create chaos test runner framework (unit test level)
- [x] Implement circuit breaker with state machine
- [x] Implement retry with exponential backoff
- [x] Implement multi-Okapi failover system
- [x] Implement health checker for instance monitoring
- [x] Create unit tests for all resilience components (11 tests passing)
- [ ] Complete integration test framework for live Okapi instances
- [ ] Implement metrics collection (Prometheus/Grafana)
- [ ] Implement chaos injection tools (tc, chaos-mesh integration)
- [ ] Create test definitions for all categories (3, 4, 5 complete; 1, 2 need integration)
- [ ] Set up automated test scheduling
- [ ] Configure alerting for test failures
- [ ] Document runbooks for each test
- [ ] Integrate with CI/CD pipeline

---

## Running Tests

```bash
# Run all hkask-ensemble tests (includes resilience tests)
cargo test --package hkask-ensemble

# Run specific resilience tests
cargo test --package hkask-ensemble resilience

# Run multi-Okapi tests
cargo test --package hkask-ensemble multi_okapi

# Run E2E tests (requires Okapi instance on localhost:11435)
OKAPI_E2E_TEST=1 cargo test --package hkask-ensemble --test e2e_okapi_integration
```

---

## References

- Chaos Engineering Principles: https://principlesofchaos.org/
- Chaos Mesh: https://chaos-mesh.org/
- AWS Fault Injection Simulator: https://aws.amazon.com/fis/
- Gremlin Chaos Engineering: https://www.gremlin.com/

---

*ℏKask — Planck's Constant of Agent Systems — v1.1.0*
