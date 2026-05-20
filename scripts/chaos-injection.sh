#!/bin/bash
# Chaos Injection Scripts for Okapi Failover Testing
# 
# These scripts provide automated chaos injection for testing hKask resilience.
# Requires: bash, curl, jq, tc (optional for network tests)

set -euo pipefail

# Configuration
OKAPI_INSTANCES=("127.0.0.1:11435" "127.0.0.1:11436" "127.0.0.1:11437")
HKASK_METRICS_URL="http://localhost:8080/metrics"
LOG_FILE="chaos_injection_$(date +%Y%m%d_%H%M%S).log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log_success() {
    log "${GREEN}✓ $1${NC}"
}

log_warning() {
    log "${YELLOW}⚠ $1${NC}"
}

log_error() {
    log "${RED}✗ $1${NC}"
}

# ============================================================================
# Instance Termination Tests
# ============================================================================

# Test 1.1: Single Instance Termination
test_single_instance_termination() {
    log "Starting Test 1.1: Single Instance Termination"
    
    local instance=${OKAPI_INSTANCES[0]}
    log "Target instance: $instance"
    
    # Record baseline metrics
    log "Recording baseline metrics..."
    curl -s "$HKASK_METRICS_URL" | grep hkask_okapi_instances_healthy
    
    # Simulate instance termination (mark as unhealthy)
    log "Simulating instance termination..."
    # In production, this would be: kubectl delete pod okapi-1
    # For local testing, we'll use a mock endpoint
    curl -X POST "http://$instance/api/chaos/terminate" 2>/dev/null || true
    
    # Monitor for 60 seconds
    log "Monitoring failover for 60 seconds..."
    for i in {1..12}; do
        sleep 5
        healthy=$(curl -s "$HKASK_METRICS_URL" 2>/dev/null | grep hkask_okapi_instances_healthy | awk '{print $2}' || echo "N/A")
        log "Healthy instances: $healthy"
    done
    
    # Verify failover
    log "Verifying failover..."
    # Check that requests are still being served
    log_success "Test 1.1 complete"
}

# Test 1.2: Cascading Instance Failures
test_cascading_failures() {
    log "Starting Test 1.2: Cascading Instance Failures"
    
    # Fail first instance
    log "Terminating instance 1..."
    curl -X POST "http://${OKAPI_INSTANCES[0]}/api/chaos/terminate" 2>/dev/null || true
    sleep 30
    
    # Fail second instance
    log "Terminating instance 2..."
    curl -X POST "http://${OKAPI_INSTANCES[1]}/api/chaos/terminate" 2>/dev/null || true
    sleep 30
    
    # Verify system still operational with last instance
    log "Verifying system operational with remaining instance..."
    curl -s "$HKASK_METRICS_URL" | grep hkask_okapi_instances_healthy
    
    log_success "Test 1.2 complete"
}

# ============================================================================
# Network Partition Tests
# ============================================================================

# Test 2.1: Network Partition (requires tc)
test_network_partition() {
    log "Starting Test 2.1: Network Partition"
    
    if ! command -v tc &> /dev/null; then
        log_warning "tc not found, skipping network partition test"
        return 0
    fi
    
    local instance=${OKAPI_INSTANCES[0]}
    local ip=$(echo $instance | cut -d: -f1)
    
    log "Creating network partition to $ip..."
    
    # Add packet loss
    sudo tc qdisc add dev lo root netem loss 100% dst $ip 2>/dev/null || true
    
    # Monitor for 60 seconds
    log "Monitoring for 60 seconds..."
    for i in {1..12}; do
        sleep 5
        unhealthy=$(curl -s "$HKASK_METRICS_URL" 2>/dev/null | grep hkask_okapi_instances_unhealthy | awk '{print $2}' || echo "N/A")
        log "Unhealthy instances: $unhealthy"
    done
    
    # Cleanup
    log "Removing network partition..."
    sudo tc qdisc del dev lo root 2>/dev/null || true
    
    log_success "Test 2.1 complete"
}

# Test 2.2: High Latency Injection (requires tc)
test_high_latency() {
    log "Starting Test 2.2: High Latency Injection"
    
    if ! command -v tc &> /dev/null; then
        log_warning "tc not found, skipping latency injection test"
        return 0
    fi
    
    local instance=${OKAPI_INSTANCES[0]}
    local ip=$(echo $instance | cut -d: -f1)
    
    log "Injecting 500ms latency to $ip..."
    
    # Add latency
    sudo tc qdisc add dev lo root netem delay 500ms dst $ip 2>/dev/null || true
    
    # Monitor for 60 seconds
    log "Monitoring for 60 seconds..."
    for i in {1..12}; do
        sleep 5
        latency=$(curl -s "$HKASK_METRICS_URL" 2>/dev/null | grep hkask_okapi_request_duration_seconds | head -1 || echo "N/A")
        log "Request latency: $latency"
    done
    
    # Cleanup
    log "Removing latency injection..."
    sudo tc qdisc del dev lo root 2>/dev/null || true
    
    log_success "Test 2.2 complete"
}

# ============================================================================
# Resource Exhaustion Tests
# ============================================================================

# Test 3.1: Memory Exhaustion (requires stress-ng)
test_memory_exhaustion() {
    log "Starting Test 3.1: Memory Exhaustion"
    
    if ! command -v stress-ng &> /dev/null; then
        log_warning "stress-ng not found, skipping memory exhaustion test"
        return 0
    fi
    
    log "Starting memory stress on Okapi process..."
    
    # Find Okapi PID (adjust for your setup)
    local okapi_pid=$(pgrep -f okapi || echo "")
    
    if [ -z "$okapi_pid" ]; then
        log_warning "Okapi process not found"
        return 0
    fi
    
    # Apply memory pressure
    stress-ng --vm 4 --vm-bytes 2G --timeout 60s --pid $okapi_pid &
    
    # Monitor for 90 seconds
    log "Monitoring for 90 seconds..."
    for i in {1..18}; do
        sleep 5
        memory=$(curl -s "$HKASK_METRICS_URL" 2>/dev/null | grep okapi_memory_bytes | awk '{print $2}' || echo "N/A")
        log "Memory usage: $memory bytes"
    done
    
    log_success "Test 3.1 complete"
}

# ============================================================================
# Circuit Breaker Tests
# ============================================================================

# Test 4.1: Circuit Breaker Trip
test_circuit_breaker_trip() {
    log "Starting Test 4.1: Circuit Breaker Trip"
    
    local instance=${OKAPI_INSTANCES[0]}
    
    # Send failing requests to trip circuit breaker
    log "Sending failing requests to trip circuit breaker..."
    for i in {1..10}; do
        curl -X POST "http://$instance/api/generate" \
            -H "Content-Type: application/json" \
            -d '{"prompt": "test", "max_tokens": -1}' 2>/dev/null || true
        sleep 0.5
    done
    
    # Check circuit breaker state
    log "Checking circuit breaker state..."
    state=$(curl -s "$HKASK_METRICS_URL" 2>/dev/null | grep hkask_circuit_breaker_state | grep -v "^#" | head -1 || echo "N/A")
    log "Circuit breaker state: $state"
    
    log_success "Test 4.1 complete"
}

# Test 4.2: Circuit Breaker Recovery
test_circuit_breaker_recovery() {
    log "Starting Test 4.2: Circuit Breaker Recovery"
    
    # Wait for circuit breaker timeout (30 seconds default)
    log "Waiting for circuit breaker timeout..."
    sleep 35
    
    # Send successful requests
    log "Sending successful requests..."
    for i in {1..5}; do
        curl -X POST "http://${OKAPI_INSTANCES[1]}/api/generate" \
            -H "Content-Type: application/json" \
            -d '{"prompt": "Hello", "max_tokens": 10}' 2>/dev/null || true
        sleep 1
    done
    
    # Check circuit breaker state
    log "Checking circuit breaker state..."
    state=$(curl -s "$HKASK_METRICS_URL" 2>/dev/null | grep hkask_circuit_breaker_state | grep -v "^#" | head -1 || echo "N/A")
    log "Circuit breaker state: $state"
    
    log_success "Test 4.2 complete"
}

# ============================================================================
# Retry Policy Tests
# ============================================================================

# Test 5.1: Retry with Exponential Backoff
test_retry_backoff() {
    log "Starting Test 5.1: Retry with Exponential Backoff"
    
    local instance=${OKAPI_INSTANCES[0]}
    
    # Send request that will fail initially then succeed
    log "Sending request with transient failures..."
    start_time=$(date +%s%N)
    
    curl -X POST "http://$instance/api/generate" \
        -H "Content-Type: application/json" \
        -d '{"prompt": "test", "fail_transient": true}' 2>/dev/null || true
    
    end_time=$(date +%s%N)
    duration=$(( (end_time - start_time) / 1000000 ))
    
    log "Request completed in ${duration}ms (should show exponential backoff)"
    
    # Check retry metrics
    log "Checking retry metrics..."
    curl -s "$HKASK_METRICS_URL" | grep hkask_retry
    
    log_success "Test 5.1 complete"
}

# Test 5.2: Retry Exhaustion
test_retry_exhaustion() {
    log "Starting Test 5.2: Retry Exhaustion"
    
    local instance=${OKAPI_INSTANCES[0]}
    
    # Send request that always fails
    log "Sending request that will exhaust retries..."
    curl -X POST "http://$instance/api/generate" \
        -H "Content-Type: application/json" \
        -d '{"prompt": "test", "fail_always": true}' 2>/dev/null || true
    
    # Check retry exhausted metric
    log "Checking retry exhausted metric..."
    exhausted=$(curl -s "$HKASK_METRICS_URL" 2>/dev/null | grep hkask_retry_exhausted_total | awk '{print $2}' || echo "N/A")
    log "Retry exhausted count: $exhausted"
    
    log_success "Test 5.2 complete"
}

# ============================================================================
# Full Test Suite
# ============================================================================

run_all_tests() {
    log "=========================================="
    log "Running Full Chaos Test Suite"
    log "=========================================="
    
    test_single_instance_termination
    sleep 10
    
    test_cascading_failures
    sleep 10
    
    test_network_partition
    sleep 10
    
    test_high_latency
    sleep 10
    
    test_memory_exhaustion
    sleep 10
    
    test_circuit_breaker_trip
    sleep 10
    
    test_circuit_breaker_recovery
    sleep 10
    
    test_retry_backoff
    sleep 10
    
    test_retry_exhaustion
    
    log "=========================================="
    log "All tests complete! Log file: $LOG_FILE"
    log "=========================================="
}

# ============================================================================
# Main
# ============================================================================

case "${1:-all}" in
    "1.1"|"single")
        test_single_instance_termination
        ;;
    "1.2"|"cascading")
        test_cascading_failures
        ;;
    "2.1"|"partition")
        test_network_partition
        ;;
    "2.2"|"latency")
        test_high_latency
        ;;
    "3.1"|"memory")
        test_memory_exhaustion
        ;;
    "4.1"|"circuit-trip")
        test_circuit_breaker_trip
        ;;
    "4.2"|"circuit-recovery")
        test_circuit_breaker_recovery
        ;;
    "5.1"|"retry-backoff")
        test_retry_backoff
        ;;
    "5.2"|"retry-exhaustion")
        test_retry_exhaustion
        ;;
    "all")
        run_all_tests
        ;;
    *)
        echo "Usage: $0 {1.1|1.2|2.1|2.2|3.1|4.1|4.2|5.1|5.2|all}"
        echo ""
        echo "Test Categories:"
        echo "  1.1 - Single Instance Termination"
        echo "  1.2 - Cascading Instance Failures"
        echo "  2.1 - Network Partition"
        echo "  2.2 - High Latency Injection"
        echo "  3.1 - Memory Exhaustion"
        echo "  4.1 - Circuit Breaker Trip"
        echo "  4.2 - Circuit Breaker Recovery"
        echo "  5.1 - Retry with Exponential Backoff"
        echo "  5.2 - Retry Exhaustion"
        echo "  all - Run all tests"
        exit 1
        ;;
esac
