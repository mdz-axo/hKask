# hKask Monitoring Stack - Manual Deployment

## Prerequisites

- Docker (installed)
- Docker Compose (optional, for easier management)

## Deployment Options

### Option A: With Docker Compose (Recommended)

```bash
cd monitoring

# Start all services
docker-compose up -d

# Or use the deployment script
./deploy.sh start

# View status
docker-compose ps

# View logs
docker-compose logs -f
```

### Option B: Manual Docker Commands

If Docker Compose is not available, use individual docker commands:

```bash
cd monitoring

# Create network
docker network create hkask-monitoring

# Start Prometheus
docker run -d \
  --name hkask_prometheus \
  -p 9090:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml:ro \
  -v $(pwd)/alerts:/etc/prometheus/alerts:ro \
  -v prometheus_data:/prometheus \
  --network hkask-monitoring \
  prom/prometheus:latest \
  --config.file=/etc/prometheus/prometheus.yml

# Start Grafana
docker run -d \
  --name hkask_grafana \
  -p 3000:3000 \
  -e GF_SECURITY_ADMIN_USER=admin \
  -e GF_SECURITY_ADMIN_PASSWORD=admin \
  -v grafana_data:/var/lib/grafana \
  -v $(pwd)/grafana/datasources:/etc/grafana/provisioning/datasources:ro \
  -v $(pwd)/grafana/dashboards:/etc/grafana/provisioning/dashboards:ro \
  --network hkask-monitoring \
  grafana/grafana:latest

# Start Alertmanager
docker run -d \
  --name hkask_alertmanager \
  -p 9093:9093 \
  -v $(pwd)/alertmanager.yml:/etc/alertmanager/alertmanager.yml:ro \
  -v alertmanager_data:/alertmanager \
  --network hkask-monitoring \
  prom/alertmanager:latest

# Start Node Exporter
docker run -d \
  --name hkask_node_exporter \
  -p 9100:9100 \
  -v /proc:/host/proc:ro \
  -v /sys:/host/sys:ro \
  -v /:/rootfs:ro \
  --network hkask-monitoring \
  prom/node-exporter:latest
```

## Access

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000 (admin/admin)
- **Alertmanager**: http://localhost:9093
- **Node Exporter**: http://localhost:9100/metrics

## Verification

```bash
# Check Prometheus health
curl http://localhost:9090/-/healthy

# Check Grafana health
curl http://localhost:3000/api/health

# Check Alertmanager health
curl http://localhost:9093/-/healthy

# Check Prometheus targets
curl http://localhost:9090/api/v1/targets | jq
```

## Cleanup

```bash
# Stop all services
docker stop hkask_prometheus hkask_grafana hkask_alertmanager hkask_node_exporter

# Remove containers
docker rm hkask_prometheus hkask_grafana hkask_alertmanager hkask_node_exporter

# Remove network
docker network rm hkask-monitoring

# Or with docker-compose
docker-compose down
```

## Production Configuration

For production deployment, you should:

1. **Configure Okapi targets** in `prometheus.yml`:
   ```yaml
   - job_name: 'okapi'
     static_configs:
       - targets:
         - 'okapi-1:11435'
         - 'okapi-2:11435'
         - 'okapi-3:11435'
   ```

2. **Configure hKask metrics endpoint**:
   - Ensure hK exposes `/metrics` endpoint
   - Add to prometheus.yml scrape config

3. **Configure alerting notifications**:
   - Edit `alertmanager.yml` to add Slack/Email webhook URLs
   - Configure notification routing

4. **Persistent storage**:
   - Use Docker volumes for long-term data retention
   - Configure backup strategy for Prometheus data

---

*ℏKask — Planck's Constant of Agent Systems — v1.2.0*