---
title: "Kubernetes Primer for hKask — K3s on Hetzner"
audience: [developers]
last_updated: 2026-06-20
version: "0.30.0"
status: "Reference Guide"
domain: "Deployment"
mds_categories: [lifecycle]
---

# Kubernetes Primer for hKask — K3s on Hetzner

**Purpose:** A practical guide for hKask developers who need to understand enough Kubernetes to deploy and operate hKask pods on Hetzner Cloud using K3s. Assumes zero prior Kubernetes knowledge.

**Related:** [Cloud Deployment Research Report](../plans/deployment-and-backup.md#14-related-research-and-past-plans), [Cloud Implementation Plans](../plans/deployment-and-backup.md#14-related-research-and-past-plans), `crates/hkask-services-cloud/src/hetzner.rs`, `crates/hkask-cli/src/commands/pod.rs::export_k8s`

---

## 1. Why Kubernetes for hKask?

The hKask architecture requires the Curator pod to directly open other pods' SQLCipher database files for semantic sync. This is specified in `MULTI_POD_ARCHITECTURE.md`:

> "Curator opens source pod's DB with deterministic passphrase, queries triples since cursor"

On PaaS platforms (fly.io, Railway, Render), each pod is an isolated container with no shared filesystem. The Curator cannot reach another pod's files. This is why we moved to Kubernetes.

Kubernetes runs all pods inside a shared cluster. Pods are isolated by **Namespace + NetworkPolicy**, not by separate VMs. This gives us:

- **Curator file access.** The Curator pod can open `{data_dir}/pods/{pod_id}.db` because all pods share the cluster's volume infrastructure.
- **Per-pod isolation via NetworkPolicy.** Despite sharing the cluster, pods cannot talk to each other unless explicitly allowed.
- **Declarative deployment.** Every pod is described as a YAML manifest. `kubectl apply` creates or updates it.

---

## 2. What Is Kubernetes? (The 30-Second Version)

Kubernetes (K8s) is a container orchestrator. You tell it what you want ("run 3 copies of this container, here's its config, give it 10GB of storage") and it makes it happen. It monitors the actual state and reconciles it with your declared desired state.

**The cluster:** A set of machines (nodes). One or more control plane nodes manage the cluster. Worker nodes run your containers.

**The control plane components:**
- **API server** — the front door. Every `kubectl` command goes here.
- **Scheduler** — decides which worker node runs each pod.
- **Controller manager** — watches for drift between desired and actual state.

**K3s** is a lightweight Kubernetes distribution. It runs the control plane as a single binary (not separate daemons), uses SQLite instead of etcd by default, and strips out cloud-provider-specific code. Perfect for smaller clusters like ours.

---

## 3. Core Concepts — Mapped to hKask

### 3.1 Pod

A pod is the smallest deployable unit. It's one or more containers that share a network namespace and storage volumes. In hKask, each StatefulSet creates one pod with three containers:

```
+-----------------------------------------+
|  Pod: hkask-pod-alice                   |
|                                         |
|  +----------+ +----------+ +---------+  |
|  | kask     | | litestream| | conduit |  |
|  | binary   | | sidecar  | | Matrix  |  |
|  | :3000    | | :9090    | | :8008   |  |
|  +----------+ +----------+ +---------+  |
|       |            |            |       |
|       +------------+------------+       |
|                    |                    |
|            +-------v-------+            |
|            |  PVC: data    |            |
|            |  /data/       |            |
|            |  kask.db      |            |
|            |  conduit.db   |            |
|            +---------------+            |
+-----------------------------------------+
```

All three containers in this pod can talk to each other via `localhost`. kask connects to Conduit at `http://localhost:8008`. Litestream watches `/data/kask.db` for WAL changes.

### 3.2 Namespace

A namespace is a virtual cluster — a way to partition resources. hKask gives each pod its own namespace:

```
kubectl get namespaces
NAME                    STATUS   AGE
hkask-pod-curator       Active   5d
hkask-pod-alice         Active   2d
hkask-pod-bob           Active   1d
```

Namespaces are the primary isolation boundary. NetworkPolicy rules are scoped to namespaces. Secrets and ConfigMaps are namespace-scoped. Deleting a namespace deletes everything in it (the "destroy pod" operation).

### 3.3 StatefulSet

A StatefulSet manages pods that need stable identities and persistent storage. Unlike a Deployment (stateless, pod names are random like `app-7f8d9c-abc12`), a StatefulSet gives pods predictable names (`kask-0`).

hKask uses StatefulSets because:
- Each pod needs its own persistent volume (not shared)
- Pod identity matters (pod_id is used for Matrix identity, database paths)
- Pods need ordered startup (litestream-restore init container runs before kask)

### 3.4 PersistentVolumeClaim (PVC)

A PVC is a request for storage. Think of it as a disk you can attach to your pod. In hKask, the StatefulSet includes a `volumeClaimTemplate` that auto-creates a PVC for each pod:

```yaml
volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      storageClassName: hcloud-volumes    # Hetzner CSI driver
      accessModes: [ReadWriteOnce]        # One pod at a time
      resources:
        requests:
          storage: 10Gi                   # 10 GB
```

On Hetzner, this triggers the Hetzner CSI driver to create a block volume (EUR 0.044/GB/month) and attach it to the worker node running your pod.

**Critical:** If you delete the pod, the PVC survives. If you delete the StatefulSet, PVCs survive unless you explicitly delete them. This is why "deactivate" (= scale to zero) doesn't lose data — the volume is still there.

### 3.5 ConfigMap and Secret

Both store configuration data that containers can access without rebuilding images.

- **ConfigMap** — non-sensitive config (litestream.yml, conduit.toml). Plain text YAML.
- **Secret** — sensitive data (API keys, object storage credentials, keystore passphrase). Base64-encoded at rest (not encrypted — use SQLCipher for actual encryption).

In hKask's manifests:
- `litestream-config` ConfigMap holds the litestream.yml template
- `conduit-config` ConfigMap holds the conduit.toml
- `litestream-replica` Secret holds object storage credentials
- `kask-secrets` Secret holds POD_ID, keystore passphrase, base URL

### 3.6 NetworkPolicy

NetworkPolicy is a firewall rule for pods. hKask uses it to enforce per-pod isolation:

```yaml
spec:
  podSelector: {}          # Applies to all pods in this namespace
  policyTypes: [Ingress, Egress]
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: hkask-ingress   # Only allow from the ingress namespace
      ports:
        - port: 3000
          protocol: TCP
  egress:
    - to:
        - ipBlock:
            cidr: 0.0.0.0/0        # Allow outbound to internet
            except: [10.0.0.0/8]   # Except internal cluster traffic
      ports:
        - port: 443                # HTTPS for inference API calls
        - port: 80
```

This means: pods in this namespace can receive traffic on port 3000 only from the ingress controller. They can make outbound HTTPS calls to external APIs. They cannot talk to other pods in the cluster (except through the shared Conduit Matrix server).

### 3.7 Init Containers

Init containers run to completion before the main containers start. hKask uses two:

1. **litestream-restore** — if no local database exists but a Litestream replica does, restore from object storage
2. **kask-migrate** — run database migrations (idempotent)

If the init container fails, the pod does not start. This prevents `kask serve` from running on a corrupted or missing database.

### 3.8 HorizontalPodAutoscaler (HPA)

HPA automatically scales the number of pod replicas based on metrics. hKask uses CPU utilization as the primary metric:

```yaml
spec:
  minReplicas: 1
  maxReplicas: 3
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

When the kask container's CPU averages above 70%, HPA scales up. When it drops, it scales down (with a 5-minute stabilization window to prevent flutter).

---

## 4. K3s on Hetzner — How It Works

### 4.1 The hetzner-k3s Tool

[`hetzner-k3s`](https://github.com/vitobotta/hetzner-k3s) is a CLI tool that provisions Hetzner Cloud servers and installs K3s on them:

```
hetzner-k3s create \
  --name hkask-prod \
  --location nbg1 \
  --masters 3 --master-type cx33 \
  --workers 3 --worker-type cx43
```

It automatically:
- Creates a private network (10.0.0.0/16) between all nodes
- Installs K3s on each node
- Deploys the **Hetzner Cloud Controller Manager** (CCM) — enables Load Balancer creation
- Deploys the **Hetzner CSI driver** — enables dynamic volume provisioning
- Deploys the Cluster Autoscaler — adds/removes worker nodes as needed
- Outputs a `kubeconfig` file

The whole process takes 2-3 minutes.

### 4.2 Cloudfleet (Managed Alternative)

[Cloudfleet](https://cloudfleet.ai/) provides a managed K8s control plane on top of your Hetzner servers. You provide the API token; Cloudfleet handles the rest. Free tier covers up to 24 vCPUs. This reduces the operational burden but adds cost.

### 4.3 Provisioning Flow

```
+-----------------------------------------------------------------+
|  Admin machine                                                   |
|  +-----------+     +-----------+     +-----------------------+  |
|  | .env      |---->| kask pod  |---->| kubectl apply         |  |
|  | HCLOUD_   |     | export-k8s|     | -f k8s-manifests/    |  |
|  | TOKEN=..  |     +-----------+     +-----------+-----------+  |
|  +-----------+                                   |              |
+--------------------------------------------------+--------------+
                                                   |
                                                   v
+--------------------------------------------------+--------------+
|  Hetzner Cloud                                                  |
|                                                                 |
|  +----------------------------------------------------------+  |
|  |  K3s Cluster (10.0.0.0/16)                               |  |
|  |                                                           |  |
|  |  +----------+  +----------+  +----------+  +----------+  |  |
|  |  | Master 1 |  | Master 2 |  | Master 3 |  | Worker 1 |  |  |
|  |  | CX33     |  | CX33     |  | CX33     |  | CX43     |  |  |
|  |  +----------+  +----------+  +----------+  +----------+  |  |
|  |                                                           |  |
|  |  +----------------------------------------------------+  |  |
|  |  |  Namespace: hkask-pod-{id}                         |  |  |
|  |  |  StatefulSet: kask                                  |  |  |
|  |  |  Pod: kask-0                                        |  |  |
|  |  |  +- kask container     (ghcr.io/.../hkask)          |  |  |
|  |  |  +- litestream sidecar (litestream:0.5.0)           |  |  |
|  |  |  +- conduit sidecar    (ghcr.io/.../hkask)          |  |  |
|  |  |  +- PVC: data-0       (10Gi, hcloud-volumes)        |  |  |
|  |  |                                                     |  |  |
|  |  |  NetworkPolicy: ingress from LB only                |  |  |
|  |  |  HPA: scale 1-3 pods based on CPU                   |  |  |
|  |  +----------------------------------------------------+  |  |
|  +----------------------------------------------------------+  |
|                                                                 |
|  +--------------------------+  +-----------------------------+  |
|  |  Load Balancer           |  |  Object Storage (S3)        |  |
|  |  -> Ingress -> cert-mgr  |  |  pods/{id}/kask.db          |  |
|  |  -> hkask-pod-*.dev      |  |  pods/{id}/conduit.db       |  |
|  +--------------------------+  +-----------------------------+  |
+-----------------------------------------------------------------+
```

### 4.4 Key Hetzner-Specific Details

**Hetzner CSI driver** creates block volumes (`hcloud-volumes` storage class). These are NVMe SSD volumes (not network-attached) so performance is excellent. A volume is pinned to its server's physical location — you can't move it between Falkenstein and Nuremberg without a snapshot/restore cycle.

**Hetzner CCM** enables Load Balancer creation. When you create a K8s Service of type LoadBalancer, Hetzner provisions a cloud load balancer (EUR 5.89/month) and configures it to route traffic to your pods.

**Hetzner Object Storage** is S3-compatible. Litestream treats it like any S3 endpoint. Uses path-style addressing: `https://nbg1.your-objectstorage.com/bucket/object`. EUR 5/TB/month with 1TB free egress.

**Network traffic:** 20TB free egress per server per month. This is a major cost advantage — hKask pods make many outbound API calls (inference, search, financial data).

---

## 5. hKask's Deployment Model in K8s Terms

### 5.1 What `export_k8s` Generates

The `kask pod export-k8s <pod-id>` command generates 6 YAML files:

| File | Kind | Purpose |
|------|------|---------|
| `namespace.yaml` | Namespace | Creates `hkask-pod-{pod_id}` namespace |
| `networkpolicy.yaml` | NetworkPolicy | Restricts pod traffic (ingress-only from LB, egress to internet) |
| `statefulset.yaml` | StatefulSet | Defines the pod with all 3 containers + init containers |
| `configmap.yaml` | ConfigMap | Litestream configuration + Conduit configuration |
| `secrets.yaml` | Secret | Object storage credentials + keystore passphrase |
| `hpa.yaml` | HorizontalPodAutoscaler | CPU-based scaling (1-3 replicas) |

### 5.2 Pod Lifecycle Commands

```
Create:   kask pod create <name>    -> creates pod DB locally
          kask pod export-k8s <id>  -> generates manifests
          kubectl apply -f k8s-manifests/  -> deploys to cluster

Activate: kask pod activate <id>    -> kubectl apply (best-effort)

Deactivate: kask pod deactivate <id> -> kubectl scale statefulset kask --replicas=0

Destroy:  kubectl delete namespace hkask-pod-<id>  -> removes everything
```

### 5.3 Useful kubectl Commands

```bash
# See all hKask pods
kubectl get namespaces -l app=hkask

# See pods in a specific namespace
kubectl get pods -n hkask-pod-alice

# Watch pod startup (init containers -> main containers)
kubectl get pods -n hkask-pod-alice -w

# See logs for the kask container
kubectl logs -n hkask-pod-alice statefulset/kask -c kask

# See logs for the litestream sidecar
kubectl logs -n hkask-pod-alice statefulset/kask -c litestream

# See logs for the conduit sidecar
kubectl logs -n hkask-pod-alice statefulset/kask -c conduit

# Exec into the kask container
kubectl exec -n hkask-pod-alice -it statefulset/kask -c kask -- /bin/sh

# Check Litestream replication
kubectl exec -n hkask-pod-alice statefulset/kask -c litestream -- \
  litestream generations /data/kask.db

# Describe a pod (useful for debugging why it won't start)
kubectl describe pod -n hkask-pod-alice kask-0

# Check HPA status
kubectl get hpa -n hkask-pod-alice

# Get events (what's happening in the namespace)
kubectl get events -n hkask-pod-alice --sort-by='.lastTimestamp'

# Port-forward for local testing (bypass network policy)
kubectl port-forward -n hkask-pod-alice statefulset/kask 3000:3000
```

---

## 6. Cluster Setup Steps

### 6.1 Prerequisites

```bash
# Install CLI tools
# kubectl: https://kubernetes.io/docs/tasks/tools/
# hetzner-k3s: https://github.com/vitobotta/hetzner-k3s

# Set your Hetzner API token
export HCLOUD_TOKEN=your-read-write-token
```

### 6.2 Create K3s Cluster

```bash
hetzner-k3s create \
  --name hkask-prod \
  --location nbg1 \
  --masters 3 --master-type cx33 \
  --workers 3 --worker-type cx43 \
  --network-zone eu-central \
  --autoscaling-enabled

# This outputs kubeconfig to ./kubeconfig
export KUBECONFIG=$(pwd)/kubeconfig
```

### 6.3 Verify

```bash
kubectl get nodes
# NAME                  STATUS   ROLES                  AGE
# hkask-prod-master-1   Ready    control-plane,master   2m
# hkask-prod-master-2   Ready    control-plane,master   2m
# hkask-prod-master-3   Ready    control-plane,master   2m
# hkask-prod-worker-1   Ready    <none>                 2m
# hkask-prod-worker-2   Ready    <none>                 2m
# hkask-prod-worker-3   Ready    <none>                 2m

kubectl get storageclass
# NAME             PROVISIONER
# hcloud-volumes   csi.hetzner.cloud
```

### 6.4 Install cert-manager and NGINX Ingress

```bash
# cert-manager for automatic TLS via Let's Encrypt
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/latest/download/cert-manager.yaml

# NGINX Ingress Controller
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.10.0/deploy/static/provider/cloud/deploy.yaml
```

### 6.5 Deploy hKask

```bash
# Create a pod
kask pod create alice

# Generate manifests
kask pod export-k8s alice

# Deploy
kubectl apply -f k8s-manifests/

# Watch it come up
kubectl get pods -n hkask-pod-alice -w

# Check health
kubectl port-forward -n hkask-pod-alice statefulset/kask 3000:3000 &
curl http://localhost:3000/health
```

---

## 7. Key Differences: K8s vs Deprecated fly.io

| Concept | fly.io (deprecated) | Kubernetes (Hetzner K3s) |
|---------|---------------------|--------------------------|
| Pod isolation | Separate VMs | Namespace + NetworkPolicy |
| Storage | Per-app volume, isolated | PVC in shared cluster, CSI-backed |
| Curator access to pod DBs | Impossible | Direct file access in cluster |
| Networking | Internal WireGuard DNS | Cluster network + Services |
| Start/stop | Machine start/stop API | `kubectl scale --replicas=N` |
| Configuration | fly.toml + fly secrets | YAML manifests + K8s Secrets |
| Container image | Push to registry, fly deploys | Push to registry, kubectl apply |
| TLS | Auto-provisioned | cert-manager + Let's Encrypt |
| Health checks | fly.io monitoring | K8s liveness/readiness probes |

---

## 8. Other K8s/K3s Providers

Hetzner is our primary recommendation, but hKask's K8s manifests are provider-agnostic. Any cluster with CSI volume support, NetworkPolicy enforcement, and S3-compatible object storage will work. Here are the leading alternatives and when they make sense.

### 8.1 Managed Kubernetes Services

| Provider | Managed? | Block Storage | Object Storage | Starting Price | Best For |
|----------|----------|--------------|----------------|---------------|----------|
| **DigitalOcean DOKS** | Yes | CSI (DO Block, $0.10/GB) | Spaces (S3, $5/250GB) | $12/mo/node | US-focused, simple UX |
| **Akamai LKE** | Yes | CSI (Linode Block, $0.10/GB) | Object Storage (S3, $5/250GB) | $12/mo/node | US/EU/APAC, good docs |
| **Civo** | Yes, K3s-based | CSI (Civo Block, $0.10/GB) | None native (use B2/R2) | ~$5/mo/node | Lightweight, K3s-native |
| **OVHcloud** | Yes | CSI (OVH Block, EUR 0.06/GB) | Object Storage (S3, EUR 0.01/GB) | ~EUR 10/mo/node | EU-regulated, B2B |
| **Scaleway** | Yes | CSI (Scaleway Block, EUR 0.08/GB) | Object Storage (S3, EUR 0.015/GB) | EUR 10/mo/node | EU, green energy |
| **Vultr** | Self-managed K3s only | CSI (Vultr Block, $0.10/GB) | Object Storage (S3, $5/250GB) | $6/mo/node | Global, 32 locations |

All of these support NetworkPolicy (Calico or Cilium), required for hKask's P4.1 per-pod isolation.

### 8.2 Self-Managed K3s on Any VPS

K3s can run on any Linux VPS. The `hetzner-k3s` tool is Hetzner-specific, but plain K3s installs in under a minute anywhere:

```bash
curl -sfL https://get.k3s.io | sh -s - --write-kubeconfig-mode 644 --disable traefik
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config
```

Providers suited to self-managed K3s:

| Provider | Cheapest Instance | Block Storage | Object Storage | Notes |
|----------|------------------|---------------|----------------|-------|
| **Hetzner** | CX23: EUR 3.99 (2 vCPU, 4GB) | EUR 0.044/GB | Yes (EUR 5/TB) | Cost leader, primary |
| **Netcup** | RS 1000 G11: ~EUR 5 (4 vCPU, 8GB) | None native | Use B2/R2 | Cheapest raw compute, no CSI |
| **Akamai/Linode** | Nanode: $5 (1 vCPU, 1GB) | $0.10/GB | Yes | Broad regions |
| **Vultr** | Regular: $6 (1 vCPU, 1GB) | $0.10/GB | Yes | 32 regions, hourly billing |

### 8.3 Decision Guide

| If you need... | Choose |
|---------------|--------|
| Lowest cost per pod at scale | **Hetzner + K3s** (EUR 4-6/pod/month) |
| Fully managed, zero K8s ops | **Civo** (K3s-native, 90s cluster creation) |
| EU data residency + managed | **OVHcloud** or **Scaleway** |
| Global edge presence (30+ regions) | **Vultr** + self-managed K3s |
| GDPR + BSI C5 compliance | **Hetzner** (BSI C5 certified data centers) |
| Cheapest raw compute possible | **Netcup** VPS + Backblaze B2 (no CSI, caveat below) |

### 8.4 Minimum Provider Requirements

For any K8s provider to run hKask pods:

1. **CSI block storage driver.** StatefulSet `volumeClaimTemplates` require dynamic provisioning. Without CSI, pods cannot get persistent volumes.
2. **NetworkPolicy support.** Required for P4.1 (OCAP per-pod isolation). Calico, Cilium, and most managed K8s CNIs support this.
3. **S3-compatible object storage.** Litestream requires this. If the provider lacks native object storage, use Backblaze B2 or Cloudflare R2 (work identically, provider-agnostic).
4. **LoadBalancer service type.** Required for ingress traffic. Most providers offer this via their cloud controller manager.
5. **Outbound internet.** Pods must reach inference APIs, search APIs, and object storage endpoints.

### 8.5 Provider That Does Not Work

**Netcup** has no CSI block storage driver. K3s runs on Netcup VPS instances, but StatefulSet PVCs cannot provision. This is a hard blocker for hKask's per-pod SQLite volumes. Workarounds exist (hostPath volumes, s3fs mounts) but break pod portability and are not recommended for production.

---

## 9. Further Reading

- [Kubernetes Concepts](https://kubernetes.io/docs/concepts/) — Official K8s docs
- [K3s Documentation](https://docs.k3s.io/) — Lightweight K8s distribution
- [hetzner-k3s](https://github.com/vitobotta/hetzner-k3s) — Tool we use to provision clusters
- [Hetzner Cloud API](https://docs.hetzner.cloud/) — Our IaaS layer
- [Hetzner CSI Driver](https://github.com/hetznercloud/csi-driver) — Volume provisioning
- [Litestream on Kubernetes](https://litestream.io/guides/kubernetes/) — Sidecar pattern
- [cert-manager](https://cert-manager.io/docs/) — Automatic TLS certificate management
