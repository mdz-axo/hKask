---
title: "hKask Kubernetes Admin Guide"
audience: ["operators", "new admins"]
last_updated: 2026-06-26
version: "0.31.0"
status: "Active"
domain: "Deployment"
mds_categories: [lifecycle, composition]
anchored_on: [PRINCIPLES.md P5, P9]
---

# hKask Kubernetes Admin Guide

**Purpose:** Step-by-step guide for deploying hKask on Kubernetes. Written for someone who has never used K8s before. Every command is explained. Every concept is defined before it's used.

**You need:** A Linux or macOS terminal, a domain name you control, and about €5/month for a Hetzner server.

**You will learn:** What a container is, what K8s does, how to deploy a real application, how to debug it when things go wrong.

---

## 0. What You're Building

Before any commands, understand the pieces. This diagram shows everything:

```
Your Domain (hkask.yourdomain.com)
        │
        ▼
┌──────────────────────────────────────────┐
│  Ingress (nginx)                          │
│  Handles HTTPS via Let's Encrypt          │
│  /         → kask (port 3000)            │
│  /_matrix  → conduit (port 8008)         │
└──────────┬───────────────────────────────┘
           │
    ┌──────┴──────┐
    ▼             ▼
┌─────────┐  ┌──────────┐
│  kask   │  │ conduit  │
│  Pod    │  │  Pod     │
│         │  │          │
│ [kask]  │  │[conduit] │    ← One container per pod
│[litestr]│  │          │      (litestream is a sidecar
│         │  │          │       sharing /data volume)
│ /data   │  │ /data    │
│  PVC    │  │  PVC     │
└────┬────┘  └──────────┘
     │
     ▼
┌────────────────────────────┐
│  S3 Object Storage          │
│  Litestream continuously    │
│  streams DB changes here    │
│  Restores on pod restart    │
└────────────────────────────┘

Namespace: hkask       Namespace: hkask-conduit
```

**What is a container?** A container is a packaged program with its dependencies. Think of it like a `.exe` file that includes everything it needs to run — the code, libraries, and configuration — all bundled together. Unlike a virtual machine, it doesn't include a full operating system; it shares the host's kernel. This makes containers small (tens of megabytes, not gigabytes) and fast to start (seconds, not minutes).

**What is Kubernetes?** Kubernetes (K8s for short) is a program that manages containers across multiple computers. You tell it "I want 1 copy of this container running at all times" and it makes that happen. If the container crashes, K8s restarts it. If the computer dies, K8s moves the container to another computer. K8s doesn't run your program itself — it tells other programs (container runtimes) to run it, and then watches to make sure they stay running.

**Why not just run `./kask serve` on a VPS?** You can. A single VPS with systemd works fine for personal use. K8s gives you three things a VPS doesn't: (1) automatic restart when your program crashes, (2) automatic TLS certificates via cert-manager, (3) declarative configuration — your entire deployment is 18 YAML files you can version-control and recreate from scratch in 5 minutes.

---

## 1. Vocabulary

Learn these 12 terms before you start. They appear in every K8s command and error message.

| Term | What It Is | Plain English |
|------|-----------|---------------|
| **Pod** | The smallest unit K8s manages. Contains one or more containers that share a network and disk. | "A running program." If your program crashes, the Pod dies and gets replaced. |
| **Container** | A packaged program with its dependencies. Runs inside a Pod. | "The program itself." kask is a container. litestream is a container. |
| **Deployment** | Manages Pods. Says "I want N copies of this Pod." | "The babysitter." If a Pod dies, the Deployment creates a new one. |
| **Service** | A stable network address for Pods. Pods come and go (IPs change). | "The phone number." Always the same, even when Pods are replaced. |
| **ConfigMap** | Non-secret configuration. Key-value pairs your program reads at startup. | "The settings file." Domain name, bucket name, feature flags. |
| **Secret** | Like ConfigMap but for passwords and API keys. Base64-encoded in K8s storage. | "The password vault." OAuth secrets, S3 keys, encryption passphrases. |
| **PVC** | PersistentVolumeClaim. A request for disk space. | "The hard drive." "I need 20 gigabytes." K8s provisions it. |
| **Ingress** | Routes external HTTP traffic to Services. Also manages TLS certificates. | "The front door." Decides which Service gets which URL path. |
| **Namespace** | A way to group K8s resources. Like folders. | "The folder." `hkask` namespace has kask things. `hkask-conduit` has Conduit things. |
| **Sidecar** | A helper container in the same Pod as your main container. | "The assistant." Litestream runs alongside kask, sharing the `/data` disk. |
| **Init Container** | A container that runs to completion before the main containers start. | "The setup step." Restores the database from backup before kask starts. |
| **kubectl** | The command-line tool for talking to K8s. Pronounced "cube-cuttle." | "The remote control." Every command in this guide starts with `kubectl`. |

---

## 2. Architecture Decisions

These are the *why* behind the deployment design. You don't need to memorize them, but they explain choices that might look unusual.

**Why two separate Pods (kask and conduit) instead of one?** Kubernetes was designed by Kelsey Hightower, Brendan Burns, and Joe Beda at Google. Their guidance: one container per Pod by default. Add a second container only when it must share the Pod's network or storage. kask and Conduit have different lifecycles (a Conduit crash shouldn't restart kask), different scaling needs, and no reason to share a network namespace. Running them as separate Deployments follows the creators' intent.

**Why is Litestream a sidecar?** Litestream needs to read the SQLite WAL (write-ahead log) that kask writes to `/data/kask.db`. The only way to share a filesystem between containers is to put them in the same Pod. This is the legitimate multi-container use case: the sidecar pattern. Litestream streams changes to S3 continuously, and restores from S3 when a new Pod starts with no local database.

**Why an init container for the restore?** Pods start all their main containers simultaneously. If kask starts before Litestream has restored the database from S3, kask might open a partial or missing database. The init container runs `litestream restore` to completion before either main container starts — guaranteeing the database is on disk when kask opens it.

**Why two namespaces?** Namespaces provide isolation boundaries. NetworkPolicies can restrict traffic between namespaces. ResourceQuotas can limit how much CPU/memory each namespace uses. If Conduit is compromised, it can't access kask's Secrets because they're in a different namespace.

---

## 3. Prerequisites

### 3.1 Accounts You Need

| Account | Why | Cost |
|---------|-----|------|
| **Hetzner Cloud** | Runs the K3s server | ~€5/month for a CX22 (2 vCPU, 4GB RAM) |
| **GitHub** | OAuth sign-in for hKask users, container registry for images | Free |
| **Domain name** | Your hKask instance needs a URL | ~€10/year from any registrar |

### 3.2 Software on Your Local Machine

```bash
# Docker — builds container images
# macOS: brew install docker (or Docker Desktop)
# Ubuntu: sudo apt install docker.io
docker --version   # Should show 24.0 or later

# kubectl — talks to K8s
# macOS: brew install kubectl
# Ubuntu: sudo snap install kubectl --classic
kubectl version --client   # Should show v1.30 or later

# hcloud CLI — manages Hetzner servers
# macOS: brew install hcloud
# Ubuntu: see install instructions below
```

Install hcloud on Ubuntu:
```bash
curl -L https://github.com/hetznercloud/cli/releases/latest/download/hcloud-linux-amd64.tar.gz | tar xz
sudo mv hcloud /usr/local/bin/
hcloud version
```

### 3.3 Understanding Check

Before proceeding, you should be able to answer these in your own words:

1. What's the difference between a Pod and a Container?
2. Why does a Deployment exist? What problem does it solve?
3. If you delete a Pod managed by a Deployment, what happens?

If you can't answer these, re-read Section 1. These concepts appear in every step.

---

## 4. Step-by-Step Deployment

Each step has four parts: **What** (what you're doing), **Why** (why it's needed), **How** (the commands), **Success** (what you should see).

### Step 1: Create a Hetzner Server

**What:** Create a virtual machine on Hetzner Cloud. This is the computer that will run K3s and your hKask instance.

**Why:** You need a computer with a public IP address that's always on. Hetzner is the cheapest option for this (~€5/month).

**How:**
```bash
# Login to Hetzner
hcloud context create hkask
# Paste your API token when prompted
# Get a token at: https://console.hetzner.cloud/projects → Security → API Tokens

# Create the server
hcloud server create \
  --name k3s-controller \
  --type cx22 \
  --image ubuntu-24.04 \
  --ssh-key your-ssh-key-name

# Get the IP address — write this down
hcloud server ip k3s-controller
```

If you don't have an SSH key uploaded to Hetzner:
```bash
# Generate one locally
ssh-keygen -t ed25519 -f ~/.ssh/hetzner -C "hkask"

# Upload to Hetzner
hcloud ssh-key create --name hetzner-key --public-key-from-file ~/.ssh/hetzner.pub
```

**Success:** `hcloud server list` shows your server with status `running` and a public IPv4 address.

---

### Step 2: Install K3s on the Server

**What:** Install K3s, a lightweight Kubernetes distribution. K3s is made by Rancher Labs and bundles the entire K8s control plane into a single ~50MB binary — perfect for single-server deployments.

**Why:** You need a Kubernetes cluster. K3s gives you one with a single command instead of the multi-hour setup of a full K8s cluster. For a single-server deployment, K3s is the standard choice.

**How:**
```bash
# SSH into your server
ssh root@YOUR_SERVER_IP -i ~/.ssh/hetzner

# Install K3s (takes ~30 seconds)
curl -sfL https://get.k3s.io | sh -

# Verify it's running
kubectl get nodes
```

**Success:**
```
NAME         STATUS   ROLES                  AGE   VERSION
k3s-controller    Ready    control-plane,master   30s   v1.30.2+k3s1
```

The node shows `Ready`. This means the K8s control plane is running and the server is ready to accept workloads.

**Copy the kubeconfig to your local machine:**

On the server:
```bash
cat /etc/rancher/k3s/k3s.yaml
```

On your local machine, create `~/.kube/config` and paste the output. Change the line:
```yaml
server: https://127.0.0.1:6443
```
to:
```yaml
server: https://YOUR_SERVER_IP:6443
```

**Success:** On your local machine, `kubectl get nodes` shows the same Ready node.

---

### Step 3: Install nginx-ingress and cert-manager

**What:** Install two K8s add-ons. nginx-ingress is a traffic router — it receives external HTTP requests and forwards them to the right Service inside the cluster. cert-manager automatically obtains and renews TLS certificates from Let's Encrypt.

**Why:** Without an Ingress controller, your Services are only reachable from inside the cluster — nobody on the internet can reach them. Without cert-manager, you'd need to manually create TLS certificates and renew them every 90 days. These two add-ons together give you HTTPS with zero ongoing maintenance.

**How:**
```bash
# Install nginx-ingress
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/cloud/deploy.yaml

# Wait for it to be ready
kubectl wait --namespace ingress-nginx \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/component=controller \
  --timeout=120s

# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.16.0/cert-manager.yaml

# Wait for all three cert-manager pods
kubectl wait --namespace cert-manager \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/instance=cert-manager \
  --timeout=180s

# Create a staging ClusterIssuer (test first, no rate limits)
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-staging
spec:
  acme:
    server: https://acme-staging-v02.api.letsencrypt.org/directory
    email: your-email@example.com
    privateKeySecretRef:
      name: letsencrypt-staging
    solvers:
      - http01:
          ingress:
            class: nginx
EOF
```

**Success:**
```bash
$ kubectl get pods -n ingress-nginx
NAME                                       READY   STATUS    RESTARTS   AGE
ingress-nginx-controller-xxxxx             1/1     Running   0          30s

$ kubectl get pods -n cert-manager
NAME                                       READY   STATUS    RESTARTS   AGE
cert-manager-xxxxx                         1/1     Running   0          30s
cert-manager-cainjector-xxxxx              1/1     Running   0          30s
cert-manager-webhook-xxxxx                 1/1     Running   0          30s
```

All pods show `1/1 Ready`. The ClusterIssuer shows `READY: True` when you run `kubectl get clusterissuer`.

**Note on staging vs production:** The staging issuer uses Let's Encrypt's test environment. Certificates are issued instantly but browsers will show a warning. This is intentional — test with staging first. Once everything works, we'll switch to production in Step 7.

---

### Step 4: Set Up DNS

**What:** Create an A record pointing your domain to your server's IP address.

**Why:** cert-manager needs to prove you own the domain before issuing a TLS certificate. It does this via the HTTP-01 challenge: Let's Encrypt asks cert-manager to place a specific file at `http://yourdomain.com/.well-known/acme-challenge/<token>`. Let's Encrypt then visits that URL and verifies the file matches. This only works if DNS points to your server.

**How:** Go to your DNS provider's control panel (Cloudflare, Namecheap, GoDaddy, etc.) and add:

```
Type:  A
Name:  hkask              (or @ for the root domain)
Value: YOUR_SERVER_IP
TTL:   300                (5 minutes — short while testing)
```

**Success:** `dig hkask.yourdomain.com` returns YOUR_SERVER_IP. Note: DNS propagation can take anywhere from 30 seconds to several hours depending on your provider and TTL settings. Cloudflare is usually instant. If `dig` returns nothing, wait 5 minutes and try again.

---

### Step 5: Create Object Storage and GitHub OAuth App

**What:** Create two external resources your hKask instance needs: an S3-compatible bucket for database backups, and a GitHub OAuth application for user sign-in.

**Why:** Litestream streams database changes to S3 in real time. If your server dies, the database survives in object storage. GitHub OAuth lets users sign into your hKask instance with their existing GitHub account — no separate username/password to manage.

#### 5a: Object Storage Bucket

**How:** In the Hetzner Cloud Console:
1. Navigate to **Object Storage**
2. Click **Create Bucket**
3. Name: `backups-bucket`
4. Location: same region as your server (fsn1, nbg1, or hel1)
5. Generate access keys: **Object Storage → Access Keys → Generate access key**
6. Write down the **Access Key ID** and **Secret Access Key** — you'll need them in Step 6

**Success:** You see the bucket in the console. You have an Access Key ID (looks like `HCxxxx...`) and Secret Access Key.

#### 5b: GitHub OAuth App

**How:**
1. Go to https://github.com/settings/developers
2. Click **New OAuth App**
3. Fill in:
   - Application name: `hKask`
   - Homepage URL: `https://hkask.yourdomain.com`
   - Authorization callback URL: `https://hkask.yourdomain.com/auth/github/callback`
4. Click **Register application**
5. Click **Generate a new client secret**
6. Write down the **Client ID** and **Client Secret**

**Success:** Your OAuth app page shows a Client ID and a Client Secret. The callback URL is correct — this is where GitHub redirects users after they authorize.

---

### Step 6: Configure the Deployment Files

**What:** Edit the YAML files in `deploy/k8s/` to use your real values instead of the placeholders.

**Why:** The files contain example values like `hkask.example.com` and `your-github-client-id`. K8s will deploy with whatever's in these files — if you don't change them, nothing will work.

#### 6a: ConfigMap (`deploy/k8s/configmap.yaml`)

```yaml
data:
  domain: "hkask.yourdomain.com"              # Your actual domain
  conduit-server-name: "hkask.yourdomain.com"  # Same as domain
  litestream-bucket: "backups-bucket"           # Your bucket name
  litestream-endpoint: "https://fsn1.your-objectstorage.com"  # From Hetzner Console
  litestream-region: "auto"                    # Keep as is
  litestream-force-path-style: "true"          # Keep as is (required for Hetzner)
```

How to find your `litestream-endpoint`: In the Hetzner Console → Object Storage → your bucket → the endpoint URL is shown at the top. It looks like `https://fsn1.your-objectstorage.com`.

#### 6b: Secret (`deploy/k8s/secret.yaml`)

```yaml
stringData:
  oauth-github-client-id: "Iv1.abc123..."       # From GitHub OAuth App
  oauth-github-client-secret: "abc123..."        # From GitHub OAuth App
  litestream-access-key-id: "HCxxxx..."          # From Hetzner Object Storage
  litestream-secret-access-key: "abc123..."      # From Hetzner Object Storage
  master-passphrase: "a-strong-random-passphrase"
```

Generate a strong passphrase:
```bash
openssl rand -base64 32
```

#### 6c: Ingress (`deploy/k8s/ingress.yaml`)

Find and replace every occurrence of `hkask.example.com` with your actual domain.

#### 6d: Deployment (`deploy/k8s/deployment.yaml`)

Find `image: ghcr.io/mdz-axo/hkask:kask-main` and change it to your image (we'll create this in Step 7). For local testing:
```yaml
image: hkask:local
imagePullPolicy: IfNotPresent
```

**Success:** Grep for any remaining placeholders. These commands should return nothing:
```bash
grep -r "example.com" deploy/k8s/
grep -r "your-" deploy/k8s/
grep -r "change-me" deploy/k8s/
```

---

### Step 7: Build and Push the Container Image

**What:** Compile the Rust project into a Docker image. Optionally push it to a container registry so K8s can pull it.

**Why:** K8s runs containers from images. The Deployment references an image name like `ghcr.io/your-username/hkask:kask-main`. You need to build that image and make it available to your cluster.

**How (local testing — no registry needed):**
```bash
# Build the image from the repo root
docker build -f deploy/Dockerfile -t hkask:local .

# Load it into minikube (if testing locally)
minikube image load hkask:local

# Or: build directly on the K3s node (SSH in and build there)
ssh root@YOUR_SERVER_IP
cd /path/to/hKask
docker build -f deploy/Dockerfile -t hkask:local .
```

**How (production — push to registry):**
```bash
# Login to GitHub Container Registry
echo "YOUR_GITHUB_TOKEN" | docker login ghcr.io -u YOUR_GITHUB_USERNAME --password-stdin
# Create a classic token at https://github.com/settings/tokens with write:packages scope

# Build and tag
docker build -f deploy/Dockerfile -t ghcr.io/YOUR_USERNAME/hkask:kask-main .

# Push
docker push ghcr.io/YOUR_USERNAME/hkask:kask-main
```

**Success:** `docker images | grep hkask` shows your image. If you pushed to a registry, `docker pull ghcr.io/YOUR_USERNAME/hkask:kask-main` works from any machine.

---

### Step 8: Deploy to Kubernetes

**What:** Apply all the manifest files to your cluster. This is the moment everything goes live.

**Why:** `kubectl apply` sends the YAML files to the K8s API server. The API server validates them, stores the desired state in etcd (K8s's database), and controllers start working to make reality match the desired state.

**Order matters:** Deploy Conduit first because kask's Deployment references the Conduit Service URL (`conduit.hkask-conduit.svc.cluster.local:8008`). If you deploy kask first, it will try to connect to a Conduit that doesn't exist yet and may fail its readiness check.

**How:**
```bash
# Deploy Conduit first
kubectl apply -f deploy/k8s/conduit/namespace.yaml
kubectl apply -f deploy/k8s/conduit/pvc.yaml
kubectl apply -f deploy/k8s/conduit/secret.yaml
kubectl apply -f deploy/k8s/conduit/service.yaml
kubectl apply -f deploy/k8s/conduit/deployment.yaml

# Wait for Conduit to be ready
kubectl -n hkask-conduit wait \
  --for=condition=ready pod \
  --selector=app=conduit \
  --timeout=120s

# Deploy kask
kubectl apply -f deploy/k8s/namespace.yaml
kubectl apply -f deploy/k8s/secret.yaml
kubectl apply -f deploy/k8s/configmap.yaml
kubectl apply -f deploy/k8s/pvc.yaml
kubectl apply -f deploy/k8s/deployment.yaml
kubectl apply -f deploy/k8s/service.yaml
kubectl apply -f deploy/k8s/ingress.yaml

# Watch the pod come to life
kubectl -n hkask get pods -w
# Press Ctrl+C when you see 2/2 Running
```

**What you'll see during `get pods -w`:**
```
NAME                    READY   STATUS            RESTARTS   AGE
hkask-5d7f8b9c4-xyz12   0/2     Pending           0          0s
hkask-5d7f8b9c4-xyz12   0/2     Init:0/1          0          2s
hkask-5d7f8b9c4-xyz12   0/2     PodInitializing   0          5s
hkask-5d7f8b9c4-xyz12   1/2     Running           0          15s
hkask-5d7f8b9c4-xyz12   2/2     Running           0          25s
```

Each state means:
- **Pending**: K8s is finding a node to run the pod on
- **Init:0/1**: The init container (litestream restore) is running
- **PodInitializing**: Init containers completed, main containers starting
- **1/2 Running**: One container (probably litestream) is ready, kask is still starting
- **2/2 Running**: Both containers are ready

**Success:**
```bash
$ kubectl -n hkask get pods
NAME                    READY   STATUS    RESTARTS   AGE
hkask-5d7f8b9c4-xyz12   2/2     Running   0          30s

$ kubectl -n hkask-conduit get pods
NAME                      READY   STATUS    RESTARTS   AGE
conduit-7a8b9c0d-abc12   1/1     Running   0          60s
```

Check the logs:
```bash
# kask logs (should show "=== hKask pod starting ===")
kubectl -n hkask logs deploy/hkask -c kask

# Litestream logs (should show "replicating" or similar)
kubectl -n hkask logs deploy/hkask -c litestream
```

---

### Step 9: Enable External Access

**What:** Get an external IP for the nginx-ingress Service so the internet can reach your cluster. Then verify TLS works.

**Why:** The Ingress resource routes traffic, but someone needs to get traffic TO the Ingress. nginx-ingress creates a Service of type `LoadBalancer` which, on Hetzner, provisions a Hetzner Cloud Load Balancer with a public IP.

**How:**
```bash
# Get the external IP (may take a minute)
kubectl -n ingress-nginx get svc ingress-nginx-controller

# Should show:
# NAME                       TYPE           EXTERNAL-IP     PORT(S)
# ingress-nginx-controller   LoadBalancer   XXX.XXX.XXX.XXX 80:...,443:...
```

If you see `<pending>` in the EXTERNAL-IP column, wait 30 seconds and try again. Hetzner provisions a Cloud Load Balancer automatically — this takes about a minute.

**Success:** You can access your instance:
```bash
# Should return an HTTP redirect to HTTPS
curl -I http://hkask.yourdomain.com

# Should return the hKask sign-in page
curl -k https://hkask.yourdomain.com

# Matrix endpoint should return JSON
curl -k https://hkask.yourdomain.com/_matrix/client/versions
```

If using the staging issuer, `curl -k` is needed because the staging certificate isn't trusted by browsers (that's expected).

---

### Step 10: Switch to Production TLS

**What:** Switch from the Let's Encrypt staging issuer to production. This gives you real TLS certificates that browsers trust.

**Why:** The staging issuer exists so you can test your setup without hitting Let's Encrypt's rate limits (5 certificates per domain per week). Once everything works, switch to production.

**How:**
```bash
# Create the production ClusterIssuer
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: your-email@example.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
EOF

# Update the Ingress to use prod
# Change cert-manager.io/cluster-issuer annotation to letsencrypt-prod

# Force renewal
kubectl -n hkask delete secret tls-cert --ignore-not-found
kubectl -n hkask annotate ingress hkask cert-manager.io/cluster-issuer=letsencrypt-prod --overwrite

# Watch the certificate being issued
kubectl get certificate -n hkask -w
```

**Success:** `kubectl get certificate -n hkask` shows `READY: True`. `curl https://hkask.yourdomain.com` works without `-k`.

---

## 5. Debugging: What To Do When Things Go Wrong

K8s errors are cryptic until you know where to look. Here's the debugging workflow.

### The Golden Debugging Command

```bash
kubectl -n hkask describe pod <pod-name>
```

This shows everything: which node the pod is on, what images it's pulling, every event that's happened to it, and why it's stuck if it's stuck. The `Events` section at the bottom is the most useful — it tells a chronological story of what K8s tried to do and what failed.

### Common Problems

| Symptom | Likely Cause | Debug Command | Fix |
|---------|-------------|---------------|-----|
| Pod stuck in `Pending` | PVC not bound (no storage class), or node has no resources | `kubectl -n hkask describe pod <name> \| grep -A10 Events` | Check PVC status: `kubectl -n hkask get pvc`. If pending, the storage class might not exist. |
| `ImagePullBackOff` | Wrong image name, image doesn't exist, or registry requires auth | `kubectl -n hkask describe pod <name> \| grep -i pull` | Verify the image exists: `docker pull <image>`. Check the image name in deployment.yaml matches what you pushed. |
| `CrashLoopBackOff` | Container starts then immediately exits | `kubectl -n hkask logs deploy/hkask -c kask --previous` | The `--previous` flag shows logs from the crashed container, not the current one. |
| `CreateContainerConfigError` | ConfigMap or Secret referenced in the deployment doesn't exist | `kubectl -n hkask describe pod <name> \| grep -i error` | Check that the ConfigMap/Secret was created: `kubectl -n hkask get configmap,secret`. |
| Pod shows `0/2 Running` for a long time | One container's readiness probe is failing | `kubectl -n hkask logs deploy/hkask -c kask` | Look for error messages in the kask logs. The readiness probe hits `/` on port 3000 — if kask isn't listening, the probe fails. |
| Ingress has no ADDRESS | nginx-ingress not installed, or LoadBalancer provisioning is slow | `kubectl -n ingress-nginx get svc` | On Hetzner, the LoadBalancer takes ~60 seconds to provision. If it's been longer, check `kubectl -n ingress-nginx describe svc ingress-nginx-controller`. |
| Cert stuck in `False` or `Unknown` | DNS not propagated, or HTTP-01 challenge can't reach your server | `kubectl describe certificaterequest -n hkask` | The certificate request's events will show the exact ACME error. Common: "connection refused" (ingress not reachable) or "no such host" (DNS not propagated). |

### Restart Everything

Sometimes the nuclear option is the right one:
```bash
# Delete and recreate the kask deployment
kubectl -n hkask delete deploy hkask
kubectl apply -f deploy/k8s/deployment.yaml

# Delete and recreate an individual pod
kubectl -n hkask delete pod <pod-name>
# The Deployment controller will immediately create a replacement
```

Note: `kubectl delete pod` does NOT cause data loss. The PVC persists independently of the Pod. Your database survives pod deletion.

---

## 6. Day-to-Day Operations

### Viewing Logs

```bash
# kask logs (follow mode)
kubectl -n hkask logs deploy/hkask -c kask -f

# Litestream logs
kubectl -n hkask logs deploy/hkask -c litestream -f

# Conduit logs
kubectl -n hkask-conduit logs deploy/conduit -f
```

### Shell into a Container

```bash
# Get a shell inside the running kask container
kubectl -n hkask exec -it deploy/hkask -c kask -- /bin/bash

# Check the database
ls -la /data/
```

### Resource Usage

```bash
# CPU and memory usage per pod
kubectl -n hkask top pods
kubectl -n hkask-conduit top pods

# Node-level usage
kubectl top nodes
```

### Disk Space

```bash
# Check PVC usage (how much of the 20Gi is used)
kubectl -n hkask exec deploy/hkask -c kask -- df -h /data

# Conduit PVC
kubectl -n hkask-conduit exec deploy/conduit -- df -h /data

# Watch for growth over time (Litestream WAL accumulation if S3 is down)
watch -n 60 'kubectl -n hkask exec deploy/hkask -c kask -- du -sh /data'
```

**Watch thresholds:** If `/data` exceeds 80% (16Gi of 20Gi), investigate.
Causes: Litestream WAL accumulation (S3 unreachable), agent pod databases growing,
or sovereignty export archives accumulating. The most common cause is S3
connectivity loss — Litestream buffers WAL segments locally until S3 recovers.

### Updating Configuration

```bash
# Edit a ConfigMap
kubectl -n hkask edit configmap app-config

# Restart the deployment to pick up changes
kubectl -n hkask rollout restart deploy/hkask
```

Note: Changing a ConfigMap does NOT automatically restart pods. You need to restart the deployment for changes to take effect.

### Updating the Image

```bash
# Build and push the new image
docker build -f deploy/Dockerfile -t ghcr.io/YOUR_USERNAME/hkask:kask-main .
docker push ghcr.io/YOUR_USERNAME/hkask:kask-main

# Update the deployment to use the new image
kubectl -n hkask set image deploy/hkask kask=ghcr.io/YOUR_USERNAME/hkask:kask-main

# Watch the rollout
kubectl -n hkask rollout status deploy/hkask
```

K8s performs a rolling update: it starts a new pod with the new image, waits for it to become Ready, then terminates the old pod. Zero downtime.

### Backup Verification

```bash
# Check litestream is replicating
kubectl -n hkask logs deploy/hkask -c litestream | tail -20
# Should show lines like "sync: new generation" or "snapshot written"

# Test restore: delete the pod and watch the init container restore
kubectl -n hkask delete pod -l app=hkask
kubectl -n hkask logs deploy/hkask -c litestream-restore  # init container logs
```

---

## 7. Understanding the Pod Startup Sequence

When a new Pod is created, this exact sequence happens. Understanding it helps you debug startup failures.

```
1. Scheduler assigns Pod to a Node
   └── Pod shows as "Pending"

2. Kubelet pulls container images
   └── Pod shows as "Pending" or "ContainerCreating"

3. Init container 1: wait-for-conduit
   └── Pod shows as "Init:0/2"
   └── Polls http://conduit.hkask-conduit.svc.cluster.local:8008/_matrix/client/versions
   └── Loops until Conduit responds with 200 OK

4. Init container 2: litestream-restore
   └── Pod shows as "Init:1/2"
   └── Tries to restore /data/kask.db from S3
   └── If no replica exists, exits successfully (empty DB is fine)
   └── If restore succeeds, exits successfully

5. Both init containers complete
   └── Pod shows as "PodInitializing"

6. Main containers start simultaneously:
   └── kask container: calls `kask serve`
   └── litestream container: calls `litestream replicate`

7. Readiness probes start checking
   └── Pod shows as "1/2" then "2/2" as each container passes
   └── Readiness probe hits `/health` — only passes when DB + Conduit are reachable

8. Pod is "Running" and "Ready"
   └── Service starts routing traffic to it
   └── Ingress starts routing external traffic to it
```

**Key insight:** Two init containers run sequentially: `wait-for-conduit` polls the Conduit homeserver until it's ready, then `litestream-restore` pulls the latest database snapshot from S3. The sidecar container (`litestream` running `replicate`) handles continuous WAL replication after startup.

---

## 8. The Deployment Files Explained

Every file in `deploy/k8s/` and what it does:

### `namespace.yaml`
Creates the `hkask` namespace. All hKask resources go here. Without this, everything would go into the `default` namespace — which is an anti-pattern for production.

### `secret.yaml`
Stores sensitive values: OAuth client credentials, S3 access keys, the master key (`HKASK_MASTER_KEY`, used to derive all internal secrets). K8s stores these base64-encoded in etcd. They're referenced by the Deployment via `secretKeyRef` — the values are injected as environment variables at container startup. The container never sees the Secret YAML.

### `configmap.yaml`
Stores non-sensitive configuration: domain name, S3 endpoint, Litestream YAML config. Also injected as environment variables. The `litestream.yml` key contains the full Litestream configuration as a multi-line YAML string — this is mounted as a file in the litestream sidecar container.

### `pvc.yaml`
Requests 20Gi of persistent storage. On Hetzner, this provisions a Hetzner Cloud Volume that survives pod restarts and node failures. The PVC is referenced by the Deployment's `volumes` section.

### `deployment.yaml`
The main deployment definition. Contains:
- `terminationGracePeriodSeconds: 60` (gives Litestream time to flush final WAL before pod shutdown)
- Init container: litestream restore
- Sidecar container: litestream replicate (with liveness probe)
- Main container: kask serve
- Volume mounts for `/data` and litestream config
- Resource requests and limits
- Liveness and readiness probes
- Environment variables from ConfigMap and Secret (including `HKASK_MASTER_KEY`)

### `service.yaml`
Creates a stable network endpoint on port 3000 for the kask Pod. The Service's `selector` matches the Pod's `app: hkask` label. Traffic sent to the Service is load-balanced across all matching Pods (with 1 replica, there's only one).

### `ingress.yaml`
Routes external traffic: `/` goes to the kask Service on port 3000, `/_matrix` goes to the Conduit Service on port 8008. Configures TLS via cert-manager. The nginx annotations set 1-hour timeouts for WebSocket connections.

### `entrypoint.sh`
Removed. The Dockerfile now handles data directory creation and process startup via its `CMD` directive (`mkdir -p ${HKASK_DATA_DIR} && exec kask serve`). This eliminates one file and keeps startup logic colocated with the image definition.

### `conduit/` Directory
Contains the same resource types for the Conduit Matrix homeserver. Conduit runs in its own namespace (`hkask-conduit`) with its own PVC, Secret, Service, and NetworkPolicy.

### `conduit-external-service.yaml`
An ExternalName Service in the `hkask` namespace that bridges to `conduit.hkask-conduit.svc.cluster.local`. Required because the Ingress (in `hkask` namespace) routes `/_matrix` to a Service named `conduit` — Kubernetes Ingress can only route to Services in the same namespace.

### `networkpolicy.yaml` (both namespaces)
Restricts ingress traffic:
- `hkask` namespace: only accepts traffic from the ingress controller
- `hkask-conduit` namespace: only accepts traffic from the ingress controller and the `hkask` namespace
These enforce the design goal that a compromised Conduit pod cannot make network requests to kask.

### `pdb.yaml`
PodDisruptionBudget with `maxUnavailable: 0`. Prevents the cluster from voluntarily evicting the sole kask pod during node maintenance or cluster autoscaling events.

---

## 9. Security Model

**Secrets are NOT encrypted at rest in etcd by default.** They are base64-encoded, which is encoding, not encryption. Anyone with `kubectl` access to the namespace can read them:

```bash
kubectl -n hkask get secret app-secrets -o yaml
```

To protect Secrets:
- Use K8s RBAC to restrict who can read Secrets
- Enable encryption at rest for etcd (K3s supports this with `--secrets-encryption` flag)
- Never commit `secret.yaml` with real values to Git

**The master passphrase** encrypts the SQLCipher database at the application level. Even if someone gets the database file, they can't read it without this passphrase. This is defense in depth — K8s Secrets protect the passphrase, and the passphrase protects the data.

**Network isolation:** The Ingress only exposes ports 3000 (kask HTTP) and 8008 (Conduit Matrix) to the internet. Litestream only needs outbound access to the S3 endpoint. Conduit only needs inbound on 8008 from within the cluster.

---

## 10. What You Learned

If you followed this guide from start to finish, you now understand:

| Concept | How You Used It |
|---------|----------------|
| **Container** | Built one with `docker build` |
| **Pod** | Watched one go through init → running lifecycle |
| **Deployment** | Created one that manages the Pod count |
| **Service** | Created stable endpoints for kask and Conduit |
| **ConfigMap** | Stored domain, S3 config, litestream.yml |
| **Secret** | Stored OAuth credentials, S3 keys, passphrase |
| **PVC** | Provisioned persistent storage for the database |
| **Ingress** | Routed external traffic with TLS |
| **Init container** | Used for database restore before kask starts |
| **Sidecar** | Used for continuous WAL replication to S3 |
| **Namespace** | Isolated kask from Conduit |
| **kubectl apply** | Deployed 13 YAML files to a running cluster |
| **kubectl logs/describe** | Debugged a failing pod |

This is the foundation of Kubernetes operations. Every production K8s deployment — whether it's Netflix running thousands of microservices or a single-person project — uses these same concepts.

---

## 11. Next Steps After Deployment

Once your hKask instance is running:

1. **Create your first agent** — `kask agent create` in the web terminal
2. **Set up Regulation monitoring** — `kask cns health` to check system health
3. **Configure automatic backups** — verify Litestream replication: `kubectl -n hkask logs deploy/hkask -c litestream`
4. **Set up alerting** — Regulation algedonic alerts will fire if variety deficit exceeds thresholds
5. **Read the architecture docs** — `docs/architecture/core/hKask-architecture-master.md` to understand the four-loop architecture

## 12. Reference

- **K3s docs:** https://docs.k3s.io
- **Kubernetes docs (Pods):** https://kubernetes.io/docs/concepts/workloads/pods/
- **cert-manager docs:** https://cert-manager.io/docs/
- **Litestream docs:** https://litestream.io
- **Hightower/Burns/Beda:** *Kubernetes: Up and Running*, 3rd Edition (O'Reilly)
- **Ibryam/Huß:** *Kubernetes Patterns* (O'Reilly)
