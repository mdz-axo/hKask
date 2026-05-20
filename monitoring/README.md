# hKask Monitoring Stack

This directory contains configuration for monitoring hKask and Okapi instances using Prometheus and Grafana.

## Quick Start

### Prerequisites

- Docker and Docker Compose
- hKask application running with metrics endpoint enabled
- Okapi instances running

### Start Monitoring Stack

```bash
cd monitoring
docker-compose up -d
```

### Access Dashboards

- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090
- **Alertmanager**: http://localhost:9093

## Configuration Files

| File | Purpose |
|------|---------|
| `docker-compose.yml` | Container orchestration |
| `prometheus.yml` | Prometheus scrape configuration |
| `alerts/hkask_alerts.yml` | Alerting rules |
| `grafana/datasources/` | Prometheus datasource config |
| `grafana/dashboards/` | Dashboard provisioning |

## Metrics Endpoints

### hKask Application

Configure hKask to expose metrics at `/metrics`:

```rust
// In your hKask application
use prometheus::{Registry, Counter, Histogram};

let registry = Registry::new();
// Register metrics...
// Expose at HTTP endpoint /metrics
```

### Okapi Instances

Okapi should expose metrics at `/api/metrics`:

```bash
curl http://localhost:11435/api/metrics
```

## Alerting

### Configure Notifications

Edit `alertmanager.yml` to configure notification channels:

```yaml
receivers:
  - name: 'slack'
    slack_configs:
      - api_url: 'YOUR_SLACK_WEBHOOK_URL'
        channel: '#alerts'
  
  - name: 'email'
    email_configs:
      - to: 'team@example.com'
        from: 'alertmanager@example.com'
        smarthost: 'smtp.example.com:587'
```

### Alert Routing

```yaml
route:
  receiver: 'slack'
  routes:
    - match:
        severity: critical
      receiver: 'slack'
    - match:
        severity: warning
      receiver: 'email'
```

## Dashboards

### Importing Dashboards

1. Access Grafana at http://localhost:3000
2. Go to Dashboards > Import
3. Upload dashboard JSON or use dashboard ID

### Pre-configured Dashboards

The following dashboards are auto-provisioned:

- **hKask Okapi Cluster**: Main operational dashboard
- **Circuit Breaker Status**: Detailed CB metrics
- **Instance Health**: Health status overview
- **Retry Metrics**: Retry policy performance

## Troubleshooting

### Prometheus Not Scraping Targets

```bash
# Check Prometheus targets
curl http://localhost:9090/api/v1/targets

# Check Prometheus logs
docker logs monitoring_prometheus_1
```

### Grafana Not Showing Data

1. Verify datasource is configured (Configuration > Data Sources)
2. Check Prometheus is scraping successfully
3. Verify metric names match dashboard queries

### Alerts Not Firing

```bash
# Check alert rules
curl http://localhost:9090/api/v1/rules

# Check pending/firing alerts
curl http://localhost:9090/api/v1/alerts

# Check Alertmanager
curl http://localhost:9093/api/v2/alerts
```

## Custom Metrics

### Adding New Metrics

1. Add metric to hKask code:
```rust
let custom_counter = Counter::new("hkask_custom_metric", "Description")?;
registry.register(Box::new(custom_counter))?;
```

2. Add to `prometheus.yml` scrape config if needed

3. Create dashboard panel or alert rule

4. Test with:
```bash
curl http://localhost:9090/api/v1/query?query=hkask_custom_metric
```

## Production Deployment

### Kubernetes

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: hkask
spec:
  selector:
    matchLabels:
      app: hkask
  endpoints:
    - port: metrics
      path: /metrics
      interval: 15s
```

### Docker Swarm

```yaml
version: '3.8'
services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    networks:
      - hkask-network
    deploy:
      placement:
        constraints:
          - node.role == manager
```

## Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [Alertmanager Documentation](https://prometheus.io/docs/alerting/alertmanager/)
- [hKask Metrics Spec](../docs/specifications/metrics-dashboard-spec.md)

---

*ℏKask — Planck's Constant of Agent Systems — v1.2.0*
