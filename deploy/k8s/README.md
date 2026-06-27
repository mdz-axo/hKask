# hKask Kubernetes Deployment

Two Deployments: **kask** (with Litestream sidecar) and **conduit** (Matrix homeserver).

## Prerequisites

- Kubernetes cluster (tested on k3s)
- `nginx-ingress` controller installed
- `cert-manager` installed (for Let's Encrypt TLS)
- A `ClusterIssuer` named `letsencrypt-prod`
- S3-compatible object storage (Hetzner Object Storage, Backblaze B2, Cloudflare R2, etc.)
- Container registry with your hKask image pushed

## Quick Start

```bash
# Edit secrets and config with your real values
vim deploy/k8s/secret.yaml      # OAuth credentials, S3 keys, passphrase
vim deploy/k8s/configmap.yaml    # Domain, S3 endpoint, bucket name
vim deploy/k8s/ingress.yaml      # Your domain name

# Deploy Conduit first (kask depends on it)
kubectl apply -f deploy/k8s/conduit/

# Wait for Conduit
kubectl -n hkask-conduit wait --for=condition=ready pod --selector=app=conduit --timeout=120s

# Deploy kask
kubectl apply -f deploy/k8s/

# Verify
kubectl -n hkask get pods
kubectl -n hkask logs deploy/hkask -c kask
kubectl -n hkask logs deploy/hkask -c litestream
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
local database, the entrypoint script restores from the latest replica. This provides:

- Disaster recovery (database survives node loss)
- Pod migration (new pod restores from S3)

The kask Deployment includes Litestream as a **sidecar container** — it shares
the `/data` volume with kask so it can replicate the SQLite WAL in real time.
This is the legitimate multi-container use case per the Kubernetes maintainers'
guidance: containers that must share a lifecycle and volume.

## Architecture

```
                    ┌──────────────────────────────┐
                    │         Ingress (nginx)       │
                    │    TLS via cert-manager        │
                    │    / → kask:3000              │
                    │    /_matrix → conduit:8008    │
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────┼───────────────┐
                    │              │               │
                    ▼              ▼               │
          ┌──────────────┐  ┌──────────────┐      │
          │  kask Service │  │conduit Service│     │
          │  (port 3000)  │  │ (port 8008)  │      │
          └──────┬───────┘  └──────┬───────┘      │
                 │                 │               │
                 ▼                 ▼               │
    ┌─────────────────┐  ┌──────────────┐         │
    │   kask Pod       │  │conduit Pod   │         │
    │                  │  │              │         │
    │ [kask container] │  │[conduit]     │         │
    │ [litestream     ]│  │              │         │
    │  sidecar        ]│  │              │         │
    │                  │  │              │         │
    │ /data (PVC)      │  │/data (PVC)   │         │
    │  └── kask.db     │  │ └──conduit.db│         │
    └────────┬────────┘  └──────────────┘         │
             │                                     │
             ▼                                     │
    ┌────────────────────┐                        │
    │  S3 Object Storage │                        │
    │  (Litestream WAL)  │                        │
    └────────────────────┘                        │
                                                  │
    Namespace: hkask       Namespace: hkask-conduit
```

### Design Decisions

**Why separate Deployments?** Kubernetes co-creators Hightower, Burns, and Beda advise one container per pod by default. kask and Conduit have independent lifecycles, scaling needs, and failure modes. Coupling them in one pod would mean a Conduit crash restarts kask too.

**Why is Litestream a sidecar?** Litestream needs to share the `/data` volume with kask to replicate the SQLite WAL to S3. The sidecar pattern is the legitimate multi-container use case — containers that share a lifecycle and storage.

**Why separate namespaces?** `hkask` for kask, `hkask-conduit` for Conduit. Provides NetworkPolicy isolation and independent ResourceQuotas. A compromised Conduit can't access kask's secrets.

**Why no Helm chart?** The deployment is intentionally simple — 15 YAML files with no templating. Helm adds complexity for a single-service deployment. The `kask curator init` command handles dynamic configuration (signing keys, domain) at deploy time.
