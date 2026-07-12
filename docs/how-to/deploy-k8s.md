---
title: "How to Deploy on Kubernetes — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Deploy on Kubernetes

This guide covers deploying hKask on Kubernetes with the Conduit Matrix homeserver sidecar. For the full step-by-step walkthrough (including Hetzner setup, K3s installation, DNS, and TLS), see `docs/plans/k8s-admin-guide.md`. This document focuses on the architecture and key operational commands.

## Architecture Overview

The deployment consists of two Pods in separate namespaces:

```
Your Domain (hkask.yourdomain.com)
        │
        ▼
┌──────────────────────────────────────────┐
│  Ingress (nginx)                          │
│  /         → kask (port 3000)            │
│  /_matrix  → conduit (port 8008)         │
└──────────┬───────────────────────────────┘
           │
    ┌──────┴──────┐
    ▼             ▼
┌─────────┐  ┌──────────┐
│  kask   │  │ conduit  │
│  Pod    │  │  Pod     │
│ [kask]  │  │[conduit] │
│[litestr]│  │          │
│ /data   │  │ /data    │
│  PVC    │  │  PVC     │
└────┬────┘  └──────────┘
     │
     ▼
┌────────────────────────────┐
│  S3 Object Storage          │
│  Litestream streams WAL     │
│  Restores on pod restart    │
└────────────────────────────┘

Namespace: hkask       Namespace: hkask + -conduit
```

## The Conduit Sidecar

Conduit is a lightweight Matrix homeserver deployed as a separate Pod in the namespace named `hkask` + `-conduit`. It provides:

- **Agent-to-agent (A2A) communication**: Replicants register as Matrix users and communicate through rooms.
- **7R7 listener integration**: The `SevenR7Listener` polls Matrix rooms and emits CNS observation spans.
- **Thread-based attention**: Agents monitor threads via watchlists; the Curator decides escalation.

Conduit runs as its own Deployment because it has a different lifecycle from kask — a Conduit crash should not restart the main application.

## Litestream Sidecar

Litestream runs as a sidecar container in the kask Pod, sharing the `/data` PersistentVolume. It continuously streams SQLite WAL (write-ahead log) changes to S3-compatible object storage. On pod restart, an **init container** runs `litestream restore` before kask starts, ensuring the database is fully restored.

Configure these in `deploy/k8s/configmap.yaml`:
- `litestream-bucket`: S3 bucket name
- `litestream-endpoint`: S3 endpoint URL
- `litestream-region`: S3 region
- `litestream-force-path-style`: `"true"` or `"false"`

And in `deploy/k8s/secret.yaml`:
- `litestream-access-key-id`: S3 access key
- `litestream-secret-access-key`: S3 secret key

## Namespace Isolation

Two namespaces provide security boundaries:
- **`hkask`**: kask Deployment, ConfigMap, Secret, PVC, Service, Ingress
- **`hkask` + `-conduit`**: Conduit Deployment, Service, NetworkPolicy

NetworkPolicies restrict cross-namespace traffic. If Conduit is compromised, it cannot access kask's Secrets.

## Key Operational Commands

```bash
# View pods
kubectl -n hkask get pods
kubectl -n hkask-"conduit" get pods

# View logs
kubectl -n hkask logs deploy/kask
kubectl -n hkask logs deploy/kask -c litestream

# Restart
kubectl -n hkask rollout restart deploy/kask

# Check resource usage
kubectl -n hkask top pods

# Shell into the container
kubectl -n hkask exec -it deploy/kask -- /bin/sh

# Verify backups
kubectl -n hkask exec deploy/kask -c litestream -- litestream snapshots /data/kask.db
```

## Understanding Pod Startup

1. Init container runs `litestream restore` to pull the latest database from S3
2. Litestream sidecar starts streaming WAL changes to S3
3. kask container starts, opens the restored database, and begins serving

## Deployment Files (18 YAML files)

The full deployment in `deploy/k8s/` includes: `namespace.yaml`, `secret.yaml`, `configmap.yaml`, `pvc.yaml`, `deployment.yaml`, `service.yaml`, `ingress.yaml`, `entrypoint.sh`, `conduit/*`, `conduit-external-service.yaml`, `networkpolicy.yaml` (both namespaces), and `pdb.yaml`.
