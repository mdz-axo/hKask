# Automated Failover Testing — Chaos Engineering for Okapi

**Version:** 1.0.0 (v1.1+)  
**Status:** Future Work Specification

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

- [ ] Create chaos test runner framework
- [ ] Implement metrics collection
- [ ] Implement chaos injection tools
- [ ] Create test definitions for all categories
- [ ] Set up automated test scheduling
- [ ] Configure alerting for test failures
- [ ] Document runbooks for each test
- [ ] Integrate with CI/CD pipeline

---

## References

- Chaos Engineering Principles: https://principlesofchaos.org/
- Chaos Mesh: https://chaos-mesh.org/
- AWS Fault Injection Simulator: https://aws.amazon.com/fis/
- Gremlin Chaos Engineering: https://www.gremlin.com/

---

*ℏKask — Planck's Constant of Agent Systems — v1.1+ (Future Work)*
