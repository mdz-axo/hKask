# hKask Deployment Guide

**Version:** 1.0.0  
**Last Updated:** 2026-05-23  
**Audience:** DevOps engineers, system administrators, deployment teams

---

## 1. Overview

hKask (ℏKask — "Planck's Constant of Agent Systems") is a minimal agent-native container platform. This guide covers production deployment of the `kask` binary and supporting infrastructure.

**Components:**
- `kask` CLI binary — User-facing command interface
- `hkask-api` — HTTP API server (optional, for programmatic access)
- SQLite database — Persistent storage for registry, goals, CNS state
- Okapi LLM — External dependency for inference (local or remote)

---

## 2. Prerequisites

### 2.1 System Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| **OS** | Linux (kernel 5.4+), macOS 12+, Windows 10+ | Linux (Ubuntu 22.04+, RHEL 9+) |
| **CPU** | 2 cores | 4+ cores |
| **RAM** | 4 GB | 8+ GB |
| **Disk** | 2 GB | 10+ GB SSD |
| **Rust** | 1.85+ | Latest stable |

### 2.2 External Dependencies

| Dependency | Purpose | Required |
|------------|---------|----------|
| **Okapi LLM** | LLM inference | Yes (for inference features) |
| **SQLite** | Database engine | Bundled (rusqlite) |
| **Git** | Template loading (optional) | Optional |

---

## 3. Quick Start

### 3.1 Install from Source

```bash
# Clone repository
git clone https://github.com/mdz-axolotl/hKask.git
cd hKask

# Build release binary
cargo build --release -p hkask-cli

# Install binary
cp target/release/kask /usr/local/bin/

# Verify installation
kask --version
```

### 3.2 Run Interactive Chat

```bash
# Start interactive chat session
kask chat --interactive

# Chat with template
kask chat --interactive --template prompt/selector

# Process single input
echo "What is 2+2?" | kask chat
```

---

## 4. Configuration

### 4.1 Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `OKAPI_BASE_URL` | Okapi API endpoint | `http://localhost:8080` | No |
| `OKAPI_API_KEY` | Okapi API key (Bearer auth) | — | No |
| `OKAPI_TIMEOUT_SECS` | Request timeout | `30` | No |
| `OKAPI_POOL_MAX_IDLE` | Connection pool size | `10` | No |
| `HKASK_DATABASE_URL` | SQLite database path | `./hkask.db` | No |
| `HKASK_LOG_LEVEL` | Logging verbosity | `info` | No |
| `RUST_LOG` | Rust tracing filter | — | No |

### 4.2 Example Configuration

```bash
# Production environment
export OKAPI_BASE_URL="https://okapi.example.com"
export OKAPI_API_KEY="your-api-key-here"
export OKAPI_TIMEOUT_SECS=60
export HKASK_DATABASE_URL="/var/lib/hkask/hkask.db"
export HKASK_LOG_LEVEL="warn"
export RUST_LOG="hkask=info,hyper=warn"
```

### 4.3 Database Location

Default database locations by platform:

| Platform | Default Path |
|----------|--------------|
| Linux | `~/.local/share/hkask/hkask.db` |
| macOS | `~/Library/Application Support/hkask/hkask.db` |
| Windows | `%APPDATA%\hkask\hkask.db` |

Create custom location:
```bash
mkdir -p /var/lib/hkask
export HKASK_DATABASE_URL="/var/lib/hkask/hkask.db"
```

---

## 5. Production Deployment

### 5.1 Systemd Service (Linux)

Create `/etc/systemd/system/hkask-api.service`:

```ini
[Unit]
Description=hKask API Server
After=network.target

[Service]
Type=simple
User=hkask
Group=hkask
ExecStart=/usr/local/bin/hkask-api serve --host 0.0.0.0 --port 8080
Environment=OKAPI_BASE_URL=https://okapi.example.com
Environment=HKASK_DATABASE_URL=/var/lib/hkask/hkask.db
Environment=RUST_LOG=hkask=info
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable hkask-api
sudo systemctl start hkask-api
sudo systemctl status hkask-api
```

### 5.2 Docker Deployment

```dockerfile
FROM rust:1.85-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release -p hkask-cli -p hkask-api

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kask /usr/local/bin/
COPY --from=builder /app/target/release/hkask-api /usr/local/bin/

RUN useradd -m hkask
USER hkask

ENV HKASK_DATABASE_URL=/home/hkask/hkask.db
ENV OKAPI_BASE_URL=http://host.docker.internal:8080

EXPOSE 8080

CMD ["hkask-api", "serve", "--host", "0.0.0.0", "--port", "8080"]
```

Build and run:
```bash
docker build -t hkask:latest .
docker run -d -p 8080:8080 --name hkask hkask:latest
```

### 5.3 Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: hkask-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: hkask-api
  template:
    metadata:
      labels:
        app: hkask-api
    spec:
      containers:
      - name: hkask-api
        image: hkask:latest
        ports:
        - containerPort: 8080
        env:
        - name: OKAPI_BASE_URL
          value: "http://okapi.default.svc.cluster.local:8080"
        - name: HKASK_DATABASE_URL
          value: "/data/hkask.db"
        volumeMounts:
        - name: data
          mountPath: /data
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: hkask-data-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: hkask-api
spec:
  selector:
    app: hkask-api
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
```

---

## 6. Health Checks

### 6.1 API Health Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/cns/health` | GET | CNS health status |
| `/api/sovereignty/status` | GET | User sovereignty status |
| `/api/templates` | GET | Template registry status |

### 6.2 Example Health Check

```bash
# Check CNS health
curl -s http://localhost:8080/api/cns/health | jq

# Expected response
{
  "overall_deficit": 0,
  "critical_count": 0,
  "warning_count": 0,
  "healthy": true
}
```

### 6.3 CLI Health Check

```bash
# Check CNS via CLI
kask cns health

# Check sovereignty status
kask sovereignty status
```

---

## 7. Monitoring

### 7.1 Key Metrics

| Metric | Alert Threshold | Action |
|--------|-----------------|--------|
| CNS variety deficit | >100 | Investigate tool usage patterns |
| Algedonic alerts | >5/hour | Escalate to on-call |
| API latency p99 | >500ms | Scale horizontally |
| Database size | >10GB | Archive old data |

### 7.2 Log Analysis

```bash
# View recent errors
journalctl -u hkask-api -p err --no-pager

# Search for CNS alerts
journalctl -u hkask-api | grep "ALGEDONIC ALERT"

# Monitor variety counters
journalctl -u hkask-api | grep "variety"
```

### 7.3 CNS Dashboard (Future)

CNS metrics can be exported to Prometheus via future integration. Track:
- `hkask_cns_variety_deficit` — Current variety deficit
- `hkask_cns_algedonic_alerts_total` — Total alerts triggered
- `hkask_inference_latency_ms` — Inference request latency

---

## 8. Backup & Recovery

### 8.1 Database Backup

```bash
# Backup SQLite database
cp /var/lib/hkask/hkask.db /backup/hkask-$(date +%Y%m%d).db

# Verify backup integrity
sqlite3 /backup/hkask-$(date +%Y%m%d).db "PRAGMA integrity_check;"
```

### 8.2 Database Restore

```bash
# Stop service
sudo systemctl stop hkask-api

# Restore from backup
cp /backup/hkask-20260523.db /var/lib/hkask/hkask.db

# Start service
sudo systemctl start hkask-api
```

### 8.3 Template Registry Backup

```bash
# Export registry to JSON
kask template list | jq > /backup/templates-$(date +%Y%m%d).json

# Re-import after restore
# (Manual re-registration required for v0.21.0)
```

---

## 9. Security Hardening

### 9.1 File Permissions

```bash
# Set secure permissions
chown -R hkask:hkask /var/lib/hkask
chmod 700 /var/lib/hkask
chmod 600 /var/lib/hkask/hkask.db
```

### 9.2 Network Security

- Run API behind reverse proxy (nginx, Traefik)
- Enable TLS termination at proxy
- Restrict Okapi API access to internal network
- Use firewall rules to limit API access

### 9.3 Capability Security

- Rotate `HKASK_CAPABILITY_SECRET` regularly
- Use short-lived capability tokens (default: 30 days)
- Audit capability grants with `kask bot list`

---

## 10. Troubleshooting

### 10.1 Common Issues

| Issue | Cause | Resolution |
|-------|-------|------------|
| `Failed to initialize Okapi` | Okapi not running | Start Okapi service or check `OKAPI_BASE_URL` |
| `Database locked` | Concurrent access | Ensure single writer; use WAL mode |
| `Template not found` | Registry empty | Register templates with `kask template register` |
| `Capability denied` | Missing/invalid token | Grant capability with `kask bot grant` |

### 10.2 Debug Mode

```bash
# Enable verbose logging
export RUST_LOG=debug
kask chat --interactive --verbose

# View detailed CNS spans
kask cns health --verbose
```

### 10.3 Support

- Documentation: `docs/` directory
- Issue tracker: https://github.com/mdz-axolotl/hKask/issues
- Architecture: `docs/architecture/hKask-architecture-master.md`

---

## 11. Upgrade Path

### 11.1 Version Compatibility

| From Version | To Version | Breaking Changes | Migration Required |
|--------------|------------|------------------|-------------------|
| v0.20.x | v0.21.0 | CNS namespace (`okh.*` → `cns.*`) | No |
| v0.21.0 | v1.0.0 | None expected | No |

### 11.2 Upgrade Procedure

```bash
# Backup database
cp /var/lib/hkask/hkask.db /backup/hkask-pre-upgrade.db

# Stop service
sudo systemctl stop hkask-api

# Install new binary
cp target/release/kask /usr/local/bin/

# Start service
sudo systemctl start hkask-api

# Verify
kask --version
curl http://localhost:8080/api/cns/health
```

---

*This deployment guide is part of hKask v0.21.0 documentation. For architecture details, see `docs/architecture/hKask-architecture-master.md`.*

**ℏKask — Planck's Constant of Agent Systems**
