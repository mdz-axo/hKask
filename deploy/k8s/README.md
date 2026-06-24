# hKask Kubernetes Deployment

Single-container pod running kask + Conduit + Litestream via supervisord.

## Prerequisites

- Kubernetes cluster (tested on k3s)
- `nginx-ingress` controller installed
- `cert-manager` installed (for Let's Encrypt TLS)
- A `ClusterIssuer` named `letsencrypt-prod`
- S3-compatible object storage (Hetzner Object Storage, Backblaze B2, Cloudflare R2, etc.)
- Container registry access to `ghcr.io/mdz-axo/hkask`

## Quick Start

```bash
# Edit the secrets and config first
vim deploy/k8s/secret.yaml    # OAuth credentials, S3 keys, passphrase
vim deploy/k8s/configmap.yaml  # Domain, S3 endpoint, bucket name

# Deploy
kubectl apply -k deploy/k8s/

# Verify
kubectl -n hkask get pods
kubectl -n hkask logs deployment/hkask
```

## Configuration

### Required secrets (`secret.yaml`)

| Key | Purpose |
|-----|---------|
| `oauth-github-client-id` | GitHub OAuth App client ID |
| `oauth-github-client-secret` | GitHub OAuth App client secret |
| `litestream-access-key-id` | S3 access key for Litestream backups |
| `litestream-secret-access-key` | S3 secret key for Litestream backups |
| `master-passphrase` | SQLCipher database encryption passphrase |

### Required config (`configmap.yaml`)

| Key | Example | Purpose |
|-----|---------|---------|
| `domain` | `hkask.example.com` | Public domain for OAuth redirects and TLS |
| `conduit-server-name` | `hkask.example.com` | Matrix homeserver name (usually same as domain) |
| `litestream-bucket` | `hkask-backups` | S3 bucket for Litestream WAL backup |
| `litestream-endpoint` | `https://s3.example.com` | S3-compatible endpoint |
| `litestream-region` | `auto` | S3 region |
| `litestream-force-path-style` | `true` | Use path-style addressing (required for Hetzner OS, MinIO) |

### Ingress

The Ingress assumes:
- `nginx-ingress` as the ingress controller (`ingressClassName: nginx`)
- `cert-manager` with a `ClusterIssuer` named `letsencrypt-prod`
- DNS A record for your domain pointing to the ingress controller's external IP

If you don't have cert-manager, remove the `cert-manager.io/cluster-issuer` annotation
and the `tls` section. You can add TLS later with `kubectl create secret tls`.

### Litestream backup

Litestream continuously replicates the SQLite WAL to S3. On pod restart with no
local database, it restores from the latest replica. This provides:

- Disaster recovery (database survives node loss)
- Pod migration (new pod restores from S3)

The `POD_ID` env var (from downward API) scopes backups per-pod:
`s3://bucket/pods/<pod-name>/kask.db`

## Architecture

```
                    ┌──────────────────────────────┐
                    │         Ingress (nginx)       │
                    │    TLS via cert-manager        │
                    │    / → kask:3000              │
                    │    /_matrix → kask:8008       │
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │       Service (ClusterIP)     │
                    │    port 3000, 8008            │
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │       Pod (single container)  │
                    │                               │
                    │  supervisord                  │
                    │  ├── kask serve  (port 3000)  │
                    │  ├── conduit     (port 8008)  │
                    │  └── litestream  (WAL → S3)   │
                    │                               │
                    │  /data (PVC)                  │
                    │  ├── kask.db                  │
                    │  └── conduit.db               │
                    └───────────────────────────────┘
```
