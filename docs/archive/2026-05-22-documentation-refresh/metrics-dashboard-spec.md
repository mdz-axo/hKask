# Okapi Metrics Dashboard Specification

**Version:** 1.0.0  
**Status:** Specification for Grafana/Prometheus integration

---

## Overview

This document specifies the metrics, dashboards, and alerting rules for monitoring Okapi instances and hKask resilience components.

---

## Metrics Export

### hKask Internal Metrics

The following metrics are exposed via CNS spans and should be exported to Prometheus:

#### Circuit Breaker Metrics

| Metric Name | Type | Labels | Description |
|-------------|------|--------|-------------|
| `hkask_circuit_breaker_state` | Gauge | `name`, `instance` | Current state (0=Closed, 1=Open, 2=HalfOpen) |
| `hkask_circuit_breaker_failures_total` | Counter | `name`, `instance` | Total failures recorded |
| `hkask_circuit_breaker_successes_total` | Counter | `name`, `instance` | Total successes recorded |
| `hkask_circuit_breaker_state_changes_total` | Counter | `name`, `from_state`, `to_state` | State transition count |
| `hkask_circuit_breaker_open_duration_seconds` | Histogram | `name`, `instance` | Duration circuit stayed open |

#### Retry Metrics

| Metric Name | Type | Labels | Description |
|-------------|------|--------|-------------|
| `hkask_retry_attempts_total` | Counter | `operation`, `outcome` | Total retry attempts |
| `hkask_retry_duration_seconds` | Histogram | `operation`, `outcome` | Retry operation duration |
| `hkask_retry_backoff_seconds` | Histogram | `operation`, `attempt` | Backoff delay per attempt |
| `hkask_retry_exhausted_total` | Counter | `operation` | Operations that exhausted retries |

#### Multi-Okapi Metrics

| Metric Name | Type | Labels | Description |
|-------------|------|--------|-------------|
| `hkask_okapi_instances_total` | Gauge | - | Total configured instances |
| `hkask_okapi_instances_healthy` | Gauge | - | Number of healthy instances |
| `hkask_okapi_instances_degraded` | Gauge | - | Number of degraded instances |
| `hkask_okapi_instances_unhealthy` | Gauge | - | Number of unhealthy instances |
| `hkask_okapi_health_check_duration_seconds` | Histogram | `instance` | Health check latency |
| `hkask_okapi_health_check_failures_total` | Counter | `instance`, `reason` | Health check failures |
| `hkask_okapi_requests_total` | Counter | `instance`, `status` | Requests routed to instance |
| `hkask_okapi_request_duration_seconds` | Histogram | `instance` | Request latency per instance |
| `hkask_okapi_load_factor` | Gauge | `instance` | Current load (0.0-1.0) |

#### Capability Router Metrics

| Metric Name | Type | Labels | Description |
|-------------|------|--------|-------------|
| `hkask_capability_routing_total` | Counter | `required_capability`, `selected_instance` | Routing decisions |
| `hkask_capability_routing_failures_total` | Counter | `required_capability` | Failed routing attempts |
| `hkask_capability_match_duration_seconds` | Histogram | - | Capability matching latency |

### Okapi Native Metrics

Okapi exposes these metrics that should be scraped:

| Metric Name | Type | Description |
|-------------|------|-------------|
| `okapi_engine_status` | Gauge | Engine health status |
| `okapi_inference_requests_total` | Counter | Total inference requests |
| `okapi_inference_duration_seconds` | Histogram | Inference latency |
| `okapi_context_tokens_used` | Gauge | Current context token usage |
| `okapi_context_tokens_available` | Gauge | Available context tokens |
| `okapi_memory_bytes` | Gauge | Memory usage |
| `okapi_gpu_utilization` | Gauge | GPU utilization percentage |
| `okapi_gpu_memory_bytes` | Gauge | GPU memory usage |

---

## Prometheus Configuration

### prometheus.yml

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # hKask application metrics
  - job_name: 'hkask'
    static_configs:
      - targets: ['localhost:8080']  # hKask metrics endpoint
    metrics_path: '/metrics'

  # Okapi instance metrics
  - job_name: 'okapi'
    static_configs:
      - targets:
        - 'localhost:11435'
        - 'localhost:11436'
        - 'localhost:11437'
    metrics_path: '/api/metrics'
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
        regex: '.*:(\\d+)'
        replacement: '${1}'

  # Node exporter for system metrics
  - job_name: 'node'
    static_configs:
      - targets: ['localhost:9100']
```

---

## Grafana Dashboard

### Dashboard JSON Structure

```json
{
  "dashboard": {
    "title": "hKask Okapi Cluster",
    "tags": ["hkask", "okapi", "llm"],
    "timezone": "browser",
    "panels": [
      {
        "title": "Circuit Breaker State",
        "type": "stat",
        "targets": [
          {
            "expr": "hkask_circuit_breaker_state",
            "legendFormat": "{{name}} - {{instance}}"
          }
        ],
        "mappings": [
          {"value": "0", "text": "Closed"},
          {"value": "1", "text": "Open"},
          {"value": "2", "text": "HalfOpen"}
        ]
      },
      {
        "title": "Okapi Instance Health",
        "type": "table",
        "targets": [
          {
            "expr": "hkask_okapi_instances_healthy",
            "legendFormat": "Healthy"
          },
          {
            "expr": "hkask_okapi_instances_degraded",
            "legendFormat": "Degraded"
          },
          {
            "expr": "hkask_okapi_instances_unhealthy",
            "legendFormat": "Unhealthy"
          }
        ]
      },
      {
        "title": "Request Rate by Instance",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(hkask_okapi_requests_total[1m])",
            "legendFormat": "{{instance}}"
          }
        ]
      },
      {
        "title": "Request Latency (P50, P95, P99)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(hkask_okapi_request_duration_seconds_bucket[5m]))",
            "legendFormat": "P50"
          },
          {
            "expr": "histogram_quantile(0.95, rate(hkask_okapi_request_duration_seconds_bucket[5m]))",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(hkask_okapi_request_duration_seconds_bucket[5m]))",
            "legendFormat": "P99"
          }
        ]
      },
      {
        "title": "Retry Attempts",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(hkask_retry_attempts_total[1m])",
            "legendFormat": "{{outcome}}"
          }
        ]
      },
      {
        "title": "Circuit Breaker State Changes",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(hkask_circuit_breaker_state_changes_total[5m])",
            "legendFormat": "{{from_state}} → {{to_state}}"
          }
        ]
      },
      {
        "title": "Okapi Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "okapi_memory_bytes",
            "legendFormat": "{{instance}}"
          }
        ]
      },
      {
        "title": "GPU Utilization",
        "type": "graph",
        "targets": [
          {
            "expr": "okapi_gpu_utilization",
            "legendFormat": "{{instance}}"
          }
        ]
      }
    ]
  }
}
```

---

## Alerting Rules

### Prometheus Alert Rules

```yaml
groups:
  - name: hkask_alerts
    interval: 30s
    rules:
      # Circuit Breaker Alerts
      - alert: OkapiCircuitBreakerOpen
        expr: hkask_circuit_breaker_state == 1
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Circuit breaker open for {{ $labels.name }}"
          description: "Circuit breaker {{ $labels.name }} for instance {{ $labels.instance }} has been open for more than 1 minute"

      - alert: OkapiCircuitBreakerFrequentTrips
        expr: rate(hkask_circuit_breaker_state_changes_total[1h]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Frequent circuit breaker trips"
          description: "Circuit breaker has tripped {{ $value }} times in the last hour"

      # Instance Health Alerts
      - alert: OkapiInstanceUnhealthy
        expr: hkask_okapi_instances_unhealthy > 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Okapi instance unhealthy"
          description: "{{ $value }} Okapi instance(s) have been unhealthy for more than 2 minutes"

      - alert: OkapiInstanceDegraded
        expr: hkask_okapi_instances_degraded > 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Okapi instance degraded"
          description: "{{ $value }} Okapi instance(s) are in degraded state"

      - alert: OkapiAllInstancesDown
        expr: hkask_okapi_instances_healthy == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "All Okapi instances down"
          description: "No healthy Okapi instances available"

      # Retry Alerts
      - alert: OkapiRetryRateHigh
        expr: rate(hkask_retry_attempts_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High retry rate"
          description: "Retry rate is {{ $value }} per second"

      - alert: OkapiRetryExhausted
        expr: rate(hkask_retry_exhausted_total[5m]) > 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Retry exhausted"
          description: "Operations are exhausting retry attempts"

      # Latency Alerts
      - alert: OkapiLatencyHigh
        expr: histogram_quantile(0.99, rate(hkask_okapi_request_duration_seconds_bucket[5m])) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High request latency"
          description: "P99 latency is {{ $value }} seconds"

      # Resource Alerts
      - alert: OkapiMemoryHigh
        expr: okapi_memory_bytes / (1024 * 1024 * 1024) > 8
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Okapi memory usage high"
          description: "Memory usage is {{ $value }} GB"

      - alert: OkapiGPUUtilizationHigh
        expr: okapi_gpu_utilization > 90
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "GPU utilization high"
          description: "GPU utilization is {{ $value }}%"
```

---

## Implementation Checklist

- [ ] Add metrics export to hkask-ensemble (Prometheus format)
- [ ] Create metrics middleware for Okapi HTTP client
- [ ] Implement CNS span → Prometheus metrics bridge
- [ ] Create prometheus.yml configuration
- [ ] Create Grafana dashboard JSON
- [ ] Create alerting rules YAML
- [ ] Set up Prometheus server
- [ ] Set up Grafana with dashboard provisioning
- [ ] Configure alertmanager for notifications
- [ ] Test alerts with chaos injection

---

## Quick Start

```bash
# Start Prometheus
docker run -d \
  -p 9090:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus

# Start Grafana
docker run -d \
  -p 3000:3000 \
  -v $(pwd)/grafana/dashboards:/etc/grafana/provisioning/dashboards \
  -v $(pwd)/grafana/datasources:/etc/grafana/provisioning/datasources \
  grafana/grafana

# Access Grafana at http://localhost:3000 (admin/admin)
```

---

*ℏKask — Planck's Constant of Agent Systems — v1.1.0*
