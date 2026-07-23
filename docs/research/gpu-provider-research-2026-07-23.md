# GPU Provider Research for H100+ Training — 2026-07-23

> **Requirement**: H100 or better GPU, SSH access, API for programmatic
> instance management, per-hour billing, available to individual developers.
>
> **Update 2026-07-23**: DeepInfra API verified. DeepInfra offers **B200 only**
> (not H100 as originally researched). Pricing and configs corrected below.
>
> **Update 2026-07-23**: Nebius CLI verified end-to-end — disk create, VM create,
> SSH access, H100 GPU confirmed, cleanup tested. All CLI flags and JSON paths
> validated against live API.
>
> **Update 2026-07-23**: B200 + Axolotl compatibility researched. Axolotl
> officially supports Blackwell with PyTorch 2.9.1 + CUDA 13.0. The install
> script now checks for pre-installed harnesses to avoid overwriting GPU-specific
> PyTorch builds.

## Comprehensive Provider Comparison

### Tier 1: H100+ with SSH + API + Per-Hour Billing

| Provider | H100 $/hr | B200 $/hr | SSH | API | Billing | Public IP | Owns HW | Notes |
|---|---|---|---|---|---|---|---|---|
| **DeepInfra** | N/A | **$3.69** | ✅ | REST | Per-minute | ✅ | ✅ | B200 only (no H100). Dedicated containers with SSH. cloud-init. SOC2. Verified via API 2026-07-23. |
| **Nebius** | $2.95 | $5.50 | ✅ | CLI + REST | Per-second | ✅ (auto-assign) | ✅ | NVIDIA Exemplar validated. Owns servers. 99% reliability. Pre-installed CUDA images. |
| **RunPod Secure** | $2.89 | $5.89 | ✅ (Secure only) | GraphQL | Per-second | ✅ (Secure only) | ❌ (resells) | Community Cloud = no SSH. Secure Cloud works but expensive. Opaque templates. |
| **Lambda Labs** | $2.99 | $4.99 | ✅ | REST | Per-hour | ✅ | ✅ | Often sold out. Good reputation. Limited availability. |
| **CoreWeave** | $2.23 | $4.69 | ✅ | Kubernetes API | Per-second | ✅ | ✅ | Enterprise-focused. Minimum commitments. |
| **Spheron** | $2.01 | $6.02 | ✅ | REST | Per-second | ✅ | Aggregator | Aggregates multiple providers. Spot instances available. |
| **GMI Cloud** | $2.00 | N/A | ✅ | REST | Per-minute | ✅ | ✅ | Competitive pricing. Limited GPU types. |
| **Fluence** | $1.24 | N/A | ✅ | REST | Per-hour | ✅ | Decentralized | Decentralized marketplace. Cheapest H100 listed but verify availability. |

### Tier 2: Expensive but Available

| Provider | H100 $/hr | SSH | Notes |
|---|---|---|---|
| **AWS** (p5) | $6.88 | ✅ | Most expensive. Egress fees. |
| **GCP** (a3) | ~$3.00 spot | ✅ | Spot only for H100. Egress fees. |
| **Azure** | ~$6.98 | ✅ | Enterprise pricing. |
| **Modal** | $3.95 | ❌ (serverless) | No SSH — serverless only. Per-second billing. |
| **Baseten** | $6.50 | ❌ (managed) | No SSH — managed deployments only. |

### Tier 3: No H100 (eliminated)

| Provider | Best GPU | H100? |
|---|---|---|
| **Linode/Akamai** | RTX 4000 Ada (20GB) | ❌ |
| **Hetzner** | RTX 4000 SFF Ada (20GB) | ❌ |
| **Cerebrium** | H100 (Enterprise only, $10K/mo min) | ❌ (not for individuals) |

## DeepInfra — B200 Dedicated Containers

> **Correction**: Original research listed DeepInfra H100 at $1.79/hr.
> API verification on 2026-07-23 found DeepInfra offers **B200 only** at $3.69/hr.
> The H100/A100/H200/B300 configs in the original research were unverified and
> do not exist on the DeepInfra containers API.

**DeepInfra** GPU Instances provide dedicated B200 containers with SSH access:

1. **B200 at $3.69/hr**: NVIDIA Blackwell, 180GB HBM3e (verified via API)
2. **SSH access**: Dedicated containers with full SSH (`ssh ubuntu@IP`)
3. **REST API**: `POST /v1/containers` to create, `GET /v1/containers/{id}` to
   check status, `DELETE /v1/containers/{id}` to terminate
4. **cloud-init**: Inject SSH keys and startup scripts via `cloud_init_user_data`
5. **Per-minute billing**: Fair granularity
6. **Public IP**: Containers get public IPs once running (null while starting/failed)
7. **Pre-built images**: `di-cont-ubuntu-torch:latest` has PyTorch + CUDA pre-installed
8. **API key already in .env**: `DI_API_KEY` is configured
9. **Failure reasons**: API returns `fail_reason` field (e.g. "out of capacity")
10. **Capacity risk**: B200 capacity may be limited — test containers have failed
    with "Start failed: out of capacity" during smoke testing

### DeepInfra API Flow (Verified 2026-07-23)

```bash
# Create a GPU container (B200 only)
curl -X POST https://api.deepinfra.com/v1/containers \
  -H "Authorization: Bearer $DI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hkask-training-abc12345",
    "gpu_config": "1xB200-180GB",
    "container_image": "di-cont-ubuntu-torch:latest",
    "cloud_init_user_data": "#cloud-config\nusers:\n- name: ubuntu\n  sudo: ALL=(ALL) NOPASSWD:ALL\n  ssh_authorized_keys:\n  - ssh-rsa AAAA..."
  }'
# Returns: {"container_id":"container-xxxxxxxx"}

# Check status (use container_id from create response)
curl -s https://api.deepinfra.com/v1/containers/container-xxxxxxxx \
  -H "Authorization: Bearer $DI_API_KEY"
# Returns: {"id":"container-xxx","name":"...","state":"running","ip":"1.2.3.4",
#           "gpu_config":"1xB200-180GB","price_per_hour":3.69,"fail_reason":null}

# List all containers
curl -s https://api.deepinfra.com/v1/containers \
  -H "Authorization: Bearer $DI_API_KEY"

# Terminate
curl -X DELETE https://api.deepinfra.com/v1/containers/container-xxxxxxxx \
  -H "Authorization: Bearer $DI_API_KEY"
```

### DeepInfra GPU Configs (Verified)

| Config | GPU | VRAM | Price | Status |
|---|---|---|---|---|
| `1xB200-180GB` | B200 (Blackwell) | 180GB HBM3e | $3.69/hr | ✅ Verified (capacity may be limited) |
| `8xB200-180GB` | 8x B200 | 8x180GB HBM3e | ~$29.52/hr | From docs (unverified) |

> Note: B300 configs returned "Invalid gpu_config" during testing.
> H100/H200/A100 configs do not exist on the containers API.

## Nebius — H100 VMs with CLI

Nebius provides H100 GPU VMs via the `nebius` CLI:

1. **H100 at $2.95/hr** ($2.15/hr preemptible): Cheapest H100 option
2. **NVIDIA Exemplar validated**: Proven training performance
3. **Owns all hardware**: Better reliability than resellers
4. **CLI tool**: `nebius compute instance create` with `--parent-id` and cloud-init
5. **Pre-installed CUDA**: `ubuntu24.04-cuda13.0` image family
6. **Public IP auto-assigned**: SSH works out of the box
7. **99% 30-day reliability**: Best reliability score in the market
8. **Project configured**: `NEBIUS_PROJECT_ID` and `NEBIUS_SUBNET_ID` in `.env`
9. **CLI installed**: `~/.nebius/bin/nebius` (federation auth)

### Nebius CLI Flow

```bash
# Create boot disk from CUDA image
nebius compute disk create \
  --parent-id $NEBIUS_PROJECT_ID \
  --name hkask-training-disk \
  --size-gibibytes 200 \
  --type network_ssd \
  --source-image-family-image-family ubuntu24.04-cuda13.0 \
  --format json

# Create VM with GPU, public IP, and cloud-init
nebius compute instance create \
  --parent-id $NEBIUS_PROJECT_ID \
  --name hkask-training-vm \
  --resources-platform gpu-h100-sxm \
  --resources-preset 1gpu-16vcpu-200gb \
  --boot-disk-existing-disk-id <disk-id> \
  --boot-disk-attach-mode READ_WRITE \
  --cloud-init-user-data "#cloud-config\n..." \
  --network-interfaces '[{"name":"net1","subnet_id":"...","public_ip_address":{}}]' \
  --format json

# Check status
nebius compute instance get --id <vm-id> --format json

# Stop VM (stops billing, keeps disk)
nebius compute instance stop --id <vm-id>
```

## Three-Host Architecture

hKask implements three training hosts, all implementing the `TrainingHost` trait:

| Host | GPU | $/hr | API | Image | Status |
|---|---|---|---|---|---|
| **DeepInfraHost** | B200 (180GB) | $3.69 | REST | Pre-built PyTorch | ✅ Implemented, capacity-limited |
| **NebiusHost** | H100 (80GB) | $2.95 | CLI | Ubuntu+CUDA | ✅ Implemented |
| **RunpodHost** | H100 (80GB) | $2.89 | GraphQL | Custom template | ✅ Implemented, primary |

### Host Selection Logic

Auto-detection (in `lib.rs::run()` and `TrainingHostConfig::default()`):
1. `HKASK_TRAINING_HOST` env var overrides (values: `runpod`, `deepinfra`, `nebius`)
2. If `DI_API_KEY` is set → DeepInfra
3. If `NEBIUS_PROJECT_ID` is set → Nebius
4. Otherwise → Runpod

### Shared Infrastructure

All three hosts share:
- `generate_install_script()` — provider-agnostic bash script for axolotl/TRL/Ludwig
- Cloud-init user-data template — creates user, writes script, executes it
- Completion manifest — written to `/workspace/completion.json`, uploaded to HuggingFace
- SSH access — every pod/VM/container gets a public IP and SSH

## OxiCUDA — DEPRECATED

OxiCUDA is not a real option. The repo is unverified, the claims are
unconfirmed, and Rust-native GPU training is not a viable path. Python
harness rendering (Axolotl/TRL/Ludwig) on GPU containers is the production
training path.