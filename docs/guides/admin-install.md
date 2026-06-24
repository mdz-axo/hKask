# hKask Admin Install Guide

**Audience:** Server operators deploying hKask for a team.  
**Last updated:** 2026-06-23  
**Version:** 0.30.0

---

## Overview

hKask deploys as a single binary with Matrix and TLS infrastructure. Two paths are supported:

| Path | Best for | TLS | Matrix |
|------|----------|-----|--------|
| **Kubernetes** | Production, federation, multi-node | nginx-ingress + cert-manager | Conduit (co-located in pod) |
| **Bare-metal** | Development, single-node testing | Caddy (docker-compose) | Conduit (docker-compose) |

**Kubernetes is the recommended path.** hKask is designed to evolve into a federated network of nodes communicating over Matrix — the k8s deployment model supports this from day one. Bare-metal is suitable for local development and single-node testing.

Users access hKask through a browser — no client install required.

---

## Common Prerequisites

| Requirement | Notes |
|-------------|-------|
| **Domain** | One FQDN, e.g. `hkask.example.com`. DNS A record must point to your server/ingress IP. |
| **GitHub OAuth App** | Create at https://github.com/settings/developers |

### GitHub OAuth App Setup

1. Go to https://github.com/settings/developers → "New OAuth App"
2. **Homepage URL:** `https://hkask.example.com`
3. **Authorization callback URL:** `https://hkask.example.com/api/v1/auth/callback?provider=github`
4. Copy the **Client ID** and generate a **Client Secret**

---

## Path A: Kubernetes (recommended)

### Prerequisites

| Requirement | Notes |
|-------------|-------|
| **Kubernetes cluster** | k3s recommended. Single-node is fine. |
| **kubectl** | Configured for your cluster |
| **nginx-ingress** | `kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/cloud/deploy.yaml` |
| **cert-manager** | `kubectl apply -f https://github.com/cert-manager/cert-manager/releases/latest/download/cert-manager.yaml` |
| **ClusterIssuer** | See below |
| **S3-compatible storage** | Hetzner Object Storage, Backblaze B2, Cloudflare R2, etc. For Litestream SQLite backup. |
| **ghcr.io access** | Pull the container image from GitHub Container Registry |

### Step A1: Set up cert-manager ClusterIssuer

```bash
cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: you@example.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
EOF
```

### Step A2: Create an S3 bucket

Create a bucket in your object storage provider. Example for Hetzner:

1. Hetzner Cloud Console → Object Storage → Create Bucket
2. Name: `hkask-backups`
3. Generate access key + secret key

### Step A3: Configure secrets and config

```bash
cd hKask

# Edit the k8s manifests with your values
vim deploy/k8s/secret.yaml
vim deploy/k8s/configmap.yaml
```

`deploy/k8s/secret.yaml` — fill in:

| Key | Value |
|-----|-------|
| `oauth-github-client-id` | Your GitHub OAuth App client ID |
| `oauth-github-client-secret` | Your GitHub OAuth App client secret |
| `litestream-access-key-id` | S3 access key |
| `litestream-secret-access-key` | S3 secret key |
| `master-passphrase` | Strong passphrase for database encryption |

`deploy/k8s/configmap.yaml` — fill in:

| Key | Value |
|-----|-------|
| `domain` | `hkask.example.com` |
| `conduit-server-name` | `hkask.example.com` |
| `litestream-bucket` | `hkask-backups` |
| `litestream-endpoint` | Your S3 endpoint URL |
| `litestream-region` | `auto` (or your region) |
| `litestream-force-path-style` | `true` for Hetzner OS/MinIO, `false` for AWS |

### Step A4: Deploy

```bash
kubectl apply -k deploy/k8s/
```

This creates: namespace, secret, configmap, PVC, deployment, service, and ingress.

### Step A5: Verify

```bash
# Wait for the pod to be ready
kubectl -n hkask get pods -w

# Check logs
kubectl -n hkask logs deployment/hkask

# Get the ingress external IP
kubectl -n hkask get ingress

# Ensure DNS points to the ingress IP
dig hkask.example.com
```

Once DNS resolves and TLS is provisioned (cert-manager may take 1-2 minutes), open `https://hkask.example.com` and sign in with GitHub.

### Step A6: Update

```bash
kubectl -n hkask set image deployment/hkask hkask=ghcr.io/mdz-axo/hkask:kask-main
kubectl -n hkask rollout status deployment/hkask
```

Litestream restores the database from S3 if the pod restarts on a new node.

---

## Path B: Bare-Metal (development)

### Prerequisites

| Requirement | Minimum | Notes |
|-------------|---------|-------|
| **OS** | Linux (amd64) | Debian/Ubuntu bookworm recommended |
| **RAM** | 2 GB | More if running inference locally |
| **Disk** | 10 GB | SQLite-backed, grows with user data |
| **Docker** | 24+ | For Caddy + Conduit sidecars |
| **Ports** | 80, 443 | Let's Encrypt HTTP challenge + HTTPS |
| **Rust** | 1.91+ | Only if building from source |

### Step B1: Build or Pull

```bash
# Option 1: Build from source
git clone https://github.com/mdz-axo/hKask.git
cd hKask
cargo build --release --bin kask
sudo cp target/release/kask /usr/local/bin/kask

# Option 2: Pre-built binary from CI
# Download from https://github.com/mdz-axo/hKask/releases
```

### Step B2: Initialize

```bash
kask init
```

Prompts for master passphrase, data directory (`/var/lib/hkask`), domain, and GitHub OAuth credentials. Secrets go to the OS keychain.

### Step B3: Deploy Sidecars

```bash
kask matrix deploy-sidecar --domain hkask.example.com
cd ~/.config/hkask/sidecar && docker compose up -d
sleep 30
kask matrix status-sidecar
```

### Step B4: Start the Server

```bash
kask serve
```

Or via systemd (unit generated by `kask init`):

```bash
sudo cp ~/.config/hkask/hkask.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now hkask
```

### Step B5: Verify

Open `https://hkask.example.com`, sign in with GitHub. You should land on the browser terminal.

---

## Post-Deployment (both paths)

### Verify first sign-in

1. Open `https://hkask.example.com` in a browser
2. Click **"Sign in with GitHub"**
3. Authorize the OAuth app
4. You should see the browser terminal with `kask repl`

### Sovereignty exports

```bash
kask export create --passphrase "user-chosen-passphrase"
```

Archives stored at `/var/lib/hkask/exports/{webid}/` (bare-metal) or `/data/exports/{webid}/` (k8s PVC).

### Additional users

Team members sign in the same way — visit the domain, click "Sign in with GitHub." Each gets their own WebID-scoped terminal, replicants, and data.

---

## Directory Layout

### Bare-metal

```
~/.config/hkask/
  ├── config.json
  ├── hkask.service
  └── sidecar/
      ├── docker-compose.yml
      ├── Caddyfile
      └── conduit.toml

/var/lib/hkask/
  ├── kask.db
  ├── agents/
  ├── exports/{webid}/
  └── registry/
```

### Kubernetes

```
PVC: /data
  ├── kask.db
  ├── agents/
  ├── exports/{webid}/
  └── registry/

Secrets: hkask-secrets (OAuth, S3, passphrase)
Config:  hkask-config (domain, endpoints, bucket)
```

---

## Troubleshooting

### "Connection refused" on port 443

**Bare-metal:** Check Caddy: `docker ps | grep caddy`. Check DNS: `dig hkask.example.com`. Firewall: ports 80 and 443 open.

**Kubernetes:** Check ingress: `kubectl -n hkask get ingress`. Check cert-manager: `kubectl get certificaterequests -A`. Ensure DNS points to ingress IP.

### "OAuth callback failed"

- Verify the callback URL in GitHub OAuth App settings matches exactly: `https://hkask.example.com/api/v1/auth/callback?provider=github`
- Bare-metal: check `HKASK_DOMAIN` is set
- Kubernetes: check `deploy/k8s/configmap.yaml` domain value

### Database errors

**Bare-metal:** Check `/var/lib/hkask/` exists and is writable.
**Kubernetes:** Check PVC is bound: `kubectl -n hkask get pvc`. Check pod logs for Litestream restore errors.

### Pod won't start (k8s)

```bash
kubectl -n hkask describe pod -l app=hkask
kubectl -n hkask logs -l app=hkask --tail=50
```

Common issues: image pull failure (check ghcr.io access), PVC not bound (check storage class), secret or configmap not found.

### Sidecar health check fails (bare-metal)

```bash
cd ~/.config/hkask/sidecar && docker compose logs
```

Conduit may take 15-30 seconds to initialize its database on first start.

---

## Related Documents

- [Deployment & Multi-User Plan](../plans/deployment-and-backup.md) — Full architecture and design decisions
- [K8s Deployment README](../../deploy/k8s/README.md) — Detailed k8s manifest reference
- [PRINCIPLES.md](../architecture/core/PRINCIPLES.md) — Magna Carta P1–P12
