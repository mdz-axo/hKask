# Okapi Cluster Monitoring with Grafana/Prometheus

**Version:** 1.0.0 (v1.1+)  
**Status:** Future Work Specification

---

## Overview

This document specifies the metrics dashboard integration for monitoring multi-Okapi clusters with Grafana and Prometheus.

---

## Metrics Export

### Okapi Instance Metrics

Each Okapi instance should expose the following Prometheus metrics:

```prometheus
# Okapi engine metrics
okapi_engine_info{instance="okapi-1", runner_type="ollamarunner"} 1
okapi_context_length{instance="okapi-1"} 8192
okapi_context_used{instance="okapi-1"} 1024

# Request metrics
okapi_requests_total{instance="okapi-1", endpoint="/api/generate", status="success"} 1234
okapi_request_duration_seconds{instance="okapi-1", endpoint="/api/generate", quantile="0.5"} 0.15
okapi_request_duration_seconds{instance="okapi-1", endpoint="/api/generate", quantile="0.95"} 0.45
okapi_request_duration_seconds{instance="okapi-1", endpoint="/api/generate", quantile="0.99"} 0.89

# Token metrics
okapi_tokens_generated_total{instance="okapi-1"} 567890
okapi_tokens_per_second{instance="okapi-1"} 45.2

# Cache metrics
okapi_kv_cache_tokens{instance="okapi-1"} 2048
okapi_kv_cache_utilization{instance="okapi-1"} 0.25

# GPU metrics (if applicable)
okapi_gpu_memory_used_bytes{instance="okapi-1"} 4294967296
okapi_gpu_utilization{instance="okapi-1"} 0.78

# Health metrics
okapi_health_status{instance="okapi-1"} 1  # 1=healthy, 0=unhealthy
okapi_health_check_duration_seconds{instance="okapi-1"} 0.05
```

### hKask CNS Metrics

hKask should export CNS metrics for Prometheus scraping:

```prometheus
# CNS span metrics
cns_spans_total{category="connector", action="llm.tokens"} 12345
cns_spans_total{category="tool", action="adapter_swap"} 56

# Variety counter metrics
cns_variety_count{domain="llm", state="active"} 15
cns_variety_count{domain="tool", state="active"} 8

# Algedonic alert metrics
cns_algedonic_alerts_total{severity="critical"} 3
cns_algedonic_alerts_total{severity="warning"} 12

# Capability validation metrics
cns_capability_validation_total{result="success"} 456
cns_capability_validation_total{result="failure"} 12

# Circuit breaker metrics
cns_circuit_breaker_state{name="okapi-1", state="closed"} 1
cns_circuit_breaker_failures_total{name="okapi-1"} 5
cns_circuit_breaker_successes_total{name="okapi-1"} 1234

# Retry metrics
cns_retry_attempts_total{operation="generate", result="success"} 89
cns_retry_attempts_total{operation="generate", result="failure"} 3
```

---

## Prometheus Configuration

### prometheus.yml

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # Okapi instances
  - job_name: 'okapi'
    static_configs:
      - targets:
          - 'okapi-1:11435'
          - 'okapi-2:11435'
          - 'okapi-3:11435'
    metrics_path: '/metrics'
    scrape_interval: 10s

  # hKask CNS
  - job_name: 'hkask-cns'
    static_configs:
      - targets:
          - 'hkask:8080'
    metrics_path: '/api/metrics'
    scrape_interval: 15s

  # Node exporters (for system metrics)
  - job_name: 'node'
    static_configs:
      - targets:
          - 'okapi-1:9100'
          - 'okapi-2:9100'
          - 'okapi-3:9100'
          - 'hkask:9100'
```

---

## Grafana Dashboards

### Dashboard 1: Okapi Cluster Overview

**Panel 1: Cluster Health**
- Gauge: Overall cluster health (% healthy instances)
- Graph: Health status over time (per instance)
- Stat: Total instances, healthy, unhealthy

**Panel 2: Request Metrics**
- Graph: Requests per second (per instance)
- Graph: Request latency p50, p95, p99
- Table: Current request rates

**Panel 3: Token Throughput**
- Graph: Tokens generated per second
- Stat: Total tokens generated (24h)
- Graph: Token generation by instance

**Panel 4: Resource Utilization**
- Gauge: Context utilization (per instance)
- Gauge: GPU memory utilization (per instance)
- Graph: KV cache usage over time

### Dashboard 2: hKask CNS Monitoring

**Panel 1: CNS Span Activity**
- Graph: Spans per minute by category
- Pie chart: Span distribution by category
- Table: Recent CNS events

**Panel 2: Capability Validation**
- Graph: Validation success/failure rate
- Stat: Validation success rate (%)
- Table: Recent validation failures

**Panel 3: Circuit Breakers**
- State: Circuit breaker status per Okapi instance
- Graph: Failure count over time
- Graph: Retry attempts over time

**Panel 4: Variety Counters**
- Graph: Variety count by domain
- Alert: Variety deficit > 100
- Stat: Current variety deficit

### Dashboard 3: Multi-Okapi Failover

**Panel 1: Instance Selection**
- Graph: Requests routed per instance
- Pie chart: Request distribution
- Stat: Current active instance

**Panel 2: Failover Events**
- Table: Recent failover events
- Graph: Failover frequency
- Stat: Total failovers (24h)

**Panel 3: Health Check Latency**
- Graph: Health check response time
- Graph: Consecutive failures per instance
- Alert: Health check timeout

---

## Alert Rules

### Prometheus Alert Rules

```yaml
groups:
  - name: okapi_alerts
    rules:
      - alert: OkapiInstanceDown
        expr: okapi_health_status == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Okapi instance {{ $labels.instance }} is down"
          description: "Okapi instance {{ $labels.instance }} has been unhealthy for more than 1 minute."

      - alert: OkapiHighLatency
        expr: okapi_request_duration_seconds{quantile="0.95"} > 1.0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency on Okapi instance {{ $labels.instance }}"
          description: "95th percentile latency is {{ $value }}s for instance {{ $labels.instance }}."

      - alert: OkapiHighContextUtilization
        expr: okapi_kv_cache_utilization > 0.9
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High context utilization on {{ $labels.instance }}"
          description: "KV cache utilization is {{ $value | humanizePercentage }} on instance {{ $labels.instance }}."

  - name: hkask_cns_alerts
    rules:
      - alert: CNSVarietyDeficit
        expr: cns_variety_deficit > 100
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "CNS variety deficit exceeds threshold"
          description: "Variety deficit is {{ $value }}, exceeding threshold of 100."

      - alert: CNSCircuitBreakerOpen
        expr: cns_circuit_breaker_state{state="open"} == 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Circuit breaker open for {{ $labels.name }}"
          description: "Circuit breaker for {{ $labels.name }} has been open for more than 5 minutes."

      - alert: CNSHighValidationFailureRate
        expr: rate(cns_capability_validation_total{result="failure"}[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High capability validation failure rate"
          description: "Validation failure rate is {{ $value }} failures/second."
```

---

## Implementation Checklist

### Phase 1: Metrics Export

- [ ] Add Prometheus metrics endpoint to Okapi instances
- [ ] Implement hKask CNS metrics exporter
- [ ] Add circuit breaker metrics
- [ ] Add retry metrics
- [ ] Add capability validation metrics

### Phase 2: Dashboard Setup

- [ ] Create Okapi Cluster Overview dashboard
- [ ] Create hKask CNS Monitoring dashboard
- [ ] Create Multi-Okapi Failover dashboard
- [ ] Configure dashboard refresh intervals
- [ ] Set up dashboard variables for filtering

### Phase 3: Alerting

- [ ] Configure Prometheus alert rules
- [ ] Set up Alertmanager routing
- [ ] Configure notification channels (Slack, email, PagerDuty)
- [ ] Test alert delivery
- [ ] Document alert runbooks

### Phase 4: Production Hardening

- [ ] Configure Prometheus retention policies
- [ ] Set up Prometheus federation for multi-region
- [ ] Configure Grafana authentication
- [ ] Set up dashboard backups
- [ ] Document operational procedures

---

## Example Grafana JSON Dashboard

See `dashboards/okapi-cluster-overview.json` for a complete example dashboard definition.

---

## References

- Prometheus documentation: https://prometheus.io/docs/
- Grafana documentation: https://grafana.com/docs/
- Prometheus metric types: https://prometheus.io/docs/concepts/metric_types/
- Alertmanager configuration: https://prometheus.io/docs/alerting/latest/configuration/

---

*ℏKask — Planck's Constant of Agent Systems — v1.1+ (Future Work)*
