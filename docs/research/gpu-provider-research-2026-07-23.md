# GPU Provider Research for H100+ Training — 2026-07-23

> **Requirement**: H100 or better GPU, SSH access, API for programmatic
> instance management, per-hour billing, available to individual developers.

## Comprehensive Provider Comparison

### Tier 1: H100 with SSH + API + Per-Hour Billing

| Provider | H100 $/hr | H200 $/hr | B200 $/hr | SSH | API | Billing | Public IP | Owns HW | Notes |
|---|---|---|---|---|---|---|---|---|---|
| **DeepInfra** | **$1.79** | **$2.19** | **$2.79** | ✅ | REST | Per-minute | ✅ | ✅ | Cheapest H100. Dedicated containers with SSH. cloud-init for script injection. SOC2. |
| **Nebius** | $2.95 | $3.50 | $5.50 | ✅ | CLI + REST | Per-second | ✅ (auto-assign) | ✅ | NVIDIA Exemplar validated. Owns servers. 99% reliability. Pre-installed CUDA images. |
| **RunPod Secure** | $2.89 | $4.39 | $5.89 | ✅ (Secure only) | GraphQL | Per-second | ✅ (Secure only) | ❌ (resells) | Community Cloud = no SSH. Secure Cloud works but expensive. Opaque templates. |
| **Lambda Labs** | $2.99 | $3.79 | $4.99 | ✅ | REST | Per-hour | ✅ | ✅ | Often sold out. Good reputation. Limited availability. |
| **CoreWeave** | $2.23 | $3.67 | $4.69 | ✅ | Kubernetes API | Per-second | ✅ | ✅ | Enterprise-focused. Minimum commitments. |
| **Spheron** | $2.01 | $4.54 | $6.02 | ✅ | REST | Per-second | ✅ | Aggregator | Aggregates multiple providers. Spot instances available. |
| **GMI Cloud** | $2.00 | $2.50 | N/A | ✅ | REST | Per-minute | ✅ | ✅ | Competitive pricing. Limited GPU types. |
| **Fluence** | $1.24 | $2.96 | N/A | ✅ | REST | Per-hour | ✅ | Decentralized | Decentralized marketplace. Cheapest H100 listed but verify availability. |

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

## DeepInfra — The Clear Winner

**DeepInfra** is the best option for hKask training:

1. **Cheapest H100**: $1.79/hr (vs RunPod $2.89, Nebius $2.95)
2. **SSH access**: Dedicated containers with full SSH (`ssh ubuntu@IP`)
3. **REST API**: `POST /v1/containers` to create, `GET /v1/containers` to list,
   `DELETE /v1/containers/{name}` to terminate
4. **cloud-init**: Inject SSH keys and startup scripts via `cloud_init_user_data`
5. **Per-minute billing**: No per-second overhead, fair granularity
6. **Public IP**: Containers get public IPs by default
7. **Owns hardware**: SOC2 certified, US-based infrastructure
8. **Pre-built images**: `di-cont-ubuntu-torch:latest` has PyTorch + CUDA pre-installed
9. **API key already in .env**: `DI_API_KEY` is configured

### DeepInfra API Flow

```bash
# Create a GPU container
curl -X POST https://api.deepinfra.com/v1/containers \
  -H "Authorization: Bearer $DEEPINFRA_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hkask-training-abc12345",
    "gpu_config": "1xH100-80GB",
    "container_image": "di-cont-ubuntu-torch:latest",
    "cloud_init_user_data": "#cloud-config\nusers:\n- name: ubuntu\n  sudo: ALL=(ALL) NOPASSWD:ALL\n  ssh_authorized_keys:\n  - ssh-rsa AAAA..."
  }'

# Check status
curl -s https://api.deepinfra.com/v1/containers \
  -H "Authorization: Bearer $DEEPINFRA_TOKEN"

# Terminate
curl -X DELETE https://api.deepinfra.com/v1/containers/hkask-training-abc12345 \
  -H "Authorization: Bearer $DEEPINFRA_TOKEN"
```

### DeepInfra GPU Configs Available

| Config | GPU | VRAM | Price |
|---|---|---|---|
| `1xA100-80GB` | A100 | 80GB | $0.89/hr |
| `1xH100-80GB` | H100 | 80GB | $1.79/hr |
| `1xH200-141GB` | H200 | 141GB | $2.19/hr |
| `1xB200-180GB` | B200 | 180GB | $2.79/hr |
| `1xB300-270GB` | B300 | 270GB | $4.89/hr |

## Nebius — Strong Runner-Up

Nebius is the second-best option:

1. **H100 at $2.95/hr**: Slightly more expensive than DeepInfra but still reasonable
2. **NVIDIA Exemplar validated**: Proven training performance
3. **Owns all hardware**: Better reliability than resellers
4. **CLI tool**: `nebius compute instance create` with cloud-init
5. **Pre-installed CUDA**: Ubuntu 22.04 LTS for NVIDIA GPUs image
6. **Public IP auto-assigned**: SSH works out of the box
7. **99% 30-day reliability**: Best reliability score in the market

## Recommendation

**Implement `DeepInfraHost` as the primary training host.** Reasons:
- 38% cheaper than RunPod ($1.79 vs $2.89/hr)
- REST API (standard, well-documented, not GraphQL)
- SSH access by default (no Community/Secure Cloud distinction)
- `DI_API_KEY` already in `.env`
- Pre-built PyTorch image (no 20-minute pip install)
- cloud-init for script injection (same pattern as our install script)

**Keep `RunpodHost` as a secondary option** for when DeepInfra is unavailable.

**The `TrainingHost` trait generalizes perfectly.** The three methods
(`submit`, `status`, `cancel`) map directly to DeepInfra's API:
- `submit` → `POST /v1/containers` with cloud-init containing the install script
- `status` → `GET /v1/containers/{name}` returns container status + SSH info
- `cancel` → `DELETE /v1/containers/{name}` terminates the container

The install script, completion manifest, HuggingFace manifest detection, and
all training logic are already provider-agnostic.

## OxiCUDA — DEPRECATED

OxiCUDA is not a real option. The repo is unverified, the claims are
unconfirmed, and Rust-native GPU training is not a viable path. Python
harness rendering (Axolotl/TRL/Ludwig) on DeepInfra containers is the
production training path.