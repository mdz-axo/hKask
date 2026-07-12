#!/bin/bash
# ============================================================================
# RunPod Unsloth Training — Qwen3.6-27B Distillation
# ============================================================================
# PREREQUISITE: Add your SSH public key at https://www.runpod.io/console/user/settings
#   If you haven't: ssh-keygen -t ed25519 && cat ~/.ssh/id_ed25519.pub
#
# This script:
#   1. Tries a secure H100 NVL, then falls back to a community A100 PCIe 80GB
#   2. Prints the RunPod dashboard URL + SSH proxy command
#   3. You paste ONE command to start training
#
# Usage: bash scripts/runpod_unsloth.sh [MODE]
#   (no arg)  Launch for reasoning distillation training
#   --eval    Launch for adapter evaluation
#   --rust-coding   Train Rust coding adapter (Strandset-Rust-v1)
#   --rust-analysis Train Rust analysis adapter (introspector/rust-analyser)
#   --rust-both     Train combined Rust adapter (coding + analysis)
# ============================================================================
set -euo pipefail

MODE="train"
for arg in "$@"; do
    case "$arg" in
        --eval) MODE="eval" ;;
        --rust-coding) MODE="rust-coding" ;;
        --rust-analysis) MODE="rust-analysis" ;;
        --rust-both) MODE="rust-both" ;;
        --rust-eval) MODE="rust-eval" ;;
        *) echo "Unknown option: $arg"; exit 1 ;;
    esac
done

export MODE

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RUNPOD_API_KEY=$(grep '^RUNPOD_API_KEY=' "$REPO_ROOT/.env" | cut -d= -f2-)
export RUNPOD_API_KEY
HF_TOKEN=$(grep '^HF_TOKEN=' "$REPO_ROOT/.env" | cut -d= -f2-)
export HF_TOKEN
RUNPOD_PUBLIC_KEY=$(cat "$HOME/.ssh/id_ed25519.pub")
export RUNPOD_PUBLIC_KEY

if [ -z "$RUNPOD_API_KEY" ]; then
    echo "ERROR: RUNPOD_API_KEY not found in .env"
    exit 1
fi
if [ -z "$HF_TOKEN" ]; then
    echo "ERROR: HF_TOKEN not found in .env"
    exit 1
fi
if [ -z "$RUNPOD_PUBLIC_KEY" ]; then
    echo "ERROR: SSH public key not found at ~/.ssh/id_ed25519.pub"
    exit 1
fi

API="https://api.runpod.io/graphql?api_key=${RUNPOD_API_KEY}"

gql() { curl -s --max-time 30 -X POST "$API" -H "Content-Type: application/json" -d "$1"; }

# ── Build GraphQL payload with Python (avoids shell-interpolation JSON bugs) ─
build_payload() {
    CLOUD_TYPE="$1" GPU_TYPE="$2" python3 -c "
import json, os
env = [
    {'key': 'HF_TOKEN', 'value': os.environ['HF_TOKEN']},
    {'key': 'RUNPOD_API_KEY', 'value': os.environ['RUNPOD_API_KEY']},
    {'key': 'PUBLIC_KEY', 'value': os.environ['RUNPOD_PUBLIC_KEY']},
    {'key': 'HF_HOME', 'value': '/workspace/.cache/huggingface'},
    {'key': 'PIP_CACHE_DIR', 'value': '/workspace/.cache/pip'},
    {'key': 'POD_INACTIVITY_TIMEOUT', 'value': '14400'},
    {'key': 'WANDB_DISABLED', 'value': 'true'},
    {'key': 'PYTORCH_CUDA_ALLOC_CONF', 'value': 'expandable_segments:True'},
    {'key': 'HF_DEACTIVATE_ASYNC_LOAD', 'value': '1'},
]
mode = os.environ.get('MODE', 'train')
if mode == 'eval':
    pod_name = 'qwen36-eval'
elif mode == 'rust-coding':
    pod_name = 'rust-coding'
elif mode == 'rust-analysis':
    pod_name = 'rust-analysis'
elif mode == 'rust-both':
    pod_name = 'rust-combined'
elif mode == 'rust-eval':
    pod_name = 'rust-eval'
else:
    pod_name = 'qwen36-unsloth'
payload = {
    'query': 'mutation DeployPod(\$input: PodFindAndDeployOnDemandInput!) { podFindAndDeployOnDemand(input: \$input) { id, name, costPerHr } }',
    'variables': {
        'input': {
            'cloudType': os.environ['CLOUD_TYPE'],
            'gpuCount': 1,
            'volumeInGb': 200,
            'volumeMountPath': '/workspace',
            'containerDiskInGb': 60,
            'minVcpuCount': 4,
            'minMemoryInGb': 30,
            'gpuTypeId': os.environ['GPU_TYPE'],
            'name': pod_name,
            'imageName': 'runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04',
            'startSsh': True,
            'ports': '22/tcp',
            'supportPublicIp': True,
            'env': env,
        }
    }
}
print(json.dumps(payload))
"
}

deploy() {
    CLOUD_TYPE="$1"
    GPU_TYPE="$2"
    echo "=== Launching ${GPU_TYPE} on ${CLOUD_TYPE} cloud ==="
    GQL_PAYLOAD=$(build_payload "$CLOUD_TYPE" "$GPU_TYPE")
    RESP=$(gql "$GQL_PAYLOAD")
    if ! echo "$RESP" | python3 -m json.tool > /dev/null 2>&1; then
        echo "RunPod API returned a non-JSON response."
        return 1
    fi

    POD_ID=$(echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin).get('data') or {}; p=d.get('podFindAndDeployOnDemand') or {}; print(p.get('id',''))" 2>/dev/null || echo "")
    if [ -z "$POD_ID" ]; then
        echo "$RESP" | python3 -m json.tool 2>/dev/null || echo "$RESP"
        return 1
    fi

    COST=$(echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin)['data']['podFindAndDeployOnDemand']; print(d.get('costPerHr','?'))" 2>/dev/null || echo "?")
    echo "Pod: ${POD_ID}  |  ${GPU_TYPE} ${CLOUD_TYPE}  |  \$${COST}/hr"
}

# ── Launch pod ──────────────────────────────────────────────────────────────
if ! deploy "SECURE" "NVIDIA H100 NVL"; then
    echo "H100 NVL unavailable; falling back to community A100 PCIe 80GB."
    if ! deploy "COMMUNITY" "NVIDIA A100 80GB PCIe"; then
        echo "ERROR: No configured GPU is currently available."
        exit 1
    fi
fi

# ── Wait for pod ───────────────────────────────────────────────────────────
echo ""
echo "=== Waiting for boot (up to 5 min) ==="
SSH_HOST=""
SSH_PORT=""
for i in $(seq 1 60); do
    sleep 10
    INFO=$(gql "{\"query\":\"{ pod(input: {podId: \\\"${POD_ID}\\\" }) { desiredStatus runtime { ports { ip privatePort publicPort } } } }\"}")
    STATUS=$(echo "$INFO" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['pod'].get('desiredStatus','UNKNOWN'))" 2>/dev/null || echo "UNKNOWN")
    SSH_ENDPOINT=$(echo "$INFO" | python3 -c "
import sys, json
runtime = (json.load(sys.stdin)['data']['pod'].get('runtime') or {})
for p in runtime.get('ports', []):
    if p.get('privatePort') == 22 and p.get('ip') and p.get('publicPort'):
        print(f\"{p['ip']}:{p['publicPort']}\")
        break
" 2>/dev/null || echo "")
    SSH_HOST=${SSH_ENDPOINT%:*}
    SSH_PORT=${SSH_ENDPOINT##*:}
    if [ $((i % 3)) -eq 0 ]; then echo "  [${i}0s] ${STATUS}"; fi
    if [ "$STATUS" = "RUNNING" ] && [ -n "$SSH_HOST" ] && [ -n "$SSH_PORT" ]; then break; fi
done

if [ -z "$SSH_HOST" ] || [ -z "$SSH_PORT" ]; then
    echo "Pod not ready yet. Dashboard: https://www.runpod.io/console/pods/${POD_ID}"
    exit 1
fi

# ── Instructions ─────────────────────────────────────────────────────────
TRAIN_URL="https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/bfedff55f47bcf0286ff49584635e25912147c97/train_unsloth.sh"
EVAL_URL="https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/a64ac5b58963d828a4d6591d816ee54fc9a221a7/eval_unsloth.sh"
RUST_TRAIN_URL="https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/9c25b9ca4360146959be4f2b82af639c5afb2624/train_rust_adapter.sh"
RUST_EVAL_URL="https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/eae9bcdd2605a0b80e81af728d89278b0c368ce9/eval_rust_adapter.sh"

if [ "$MODE" = "eval" ]; then
    echo ""
    echo "==============================================="
    echo "POD READY: ${POD_ID}  |  \$${COST}/hr  |  EVAL MODE"
    echo ""
    echo "→ Dashboard:"
    echo "  https://www.runpod.io/console/pods/${POD_ID}"
    echo ""
    echo "→ Run evaluation (paste ONE command in Web Terminal or SSH):"
    echo ""
    echo "  curl -sL ${EVAL_URL} | bash"
    echo ""
    echo "→ Or via SSH:"
    echo "  ssh root@${SSH_HOST} -p ${SSH_PORT} 'curl -sL ${EVAL_URL} | bash'"
    echo ""
    echo "→ Monitor:"
    echo "  ssh root@${SSH_HOST} -p ${SSH_PORT} 'tail -f /workspace/eval.log'"
    echo ""
    echo "Results saved to /workspace/eval_results/ on the pod."
    echo "==============================================="
elif [ "$MODE" = "rust-coding" ] || [ "$MODE" = "rust-analysis" ] || [ "$MODE" = "rust-both" ]; then
    if [ "$MODE" = "rust-coding" ]; then
        RUST_MODE="coding"
        REPO="Axolotl-Partners/qwen36-rust-coding-lora"
    elif [ "$MODE" = "rust-analysis" ]; then
        RUST_MODE="analysis"
        REPO="Axolotl-Partners/qwen36-rust-analysis-lora"
    else
        RUST_MODE="both"
        REPO="Axolotl-Partners/qwen36-rust-combined-lora"
    fi
    echo ""
    echo "==============================================="
    echo "POD READY: ${POD_ID}  |  \$${COST}/hr  |  RUST ${RUST_MODE^^} MODE"
    echo ""
    echo "→ Dashboard:"
    echo "  https://www.runpod.io/console/pods/${POD_ID}"
    echo ""
    echo "→ Run training (paste ONE command):"
    echo ""
    echo "  MODE=${RUST_MODE} curl -sL ${RUST_TRAIN_URL} | bash"
    echo ""
    echo "→ Or via SSH:"
    echo "  ssh root@${SSH_HOST} -p ${SSH_PORT} 'MODE=${RUST_MODE} curl -sL ${RUST_TRAIN_URL} | bash'"
    echo ""
    echo "→ Monitor:"
    echo "  ssh root@${SSH_HOST} -p ${SSH_PORT} 'tail -f /workspace/training.log'"
    echo ""
    echo "Dashboard: https://www.runpod.io/console/pods/${POD_ID}"
    echo "Results:   HF → ${REPO}"
    echo "==============================================="
elif [ "$MODE" = "rust-eval" ]; then
    echo ""
    echo "==============================================="
    echo "POD READY: ${POD_ID}  |  \$${COST}/hr  |  RUST EVAL MODE"
    echo ""
    echo "→ Dashboard:"
    echo "  https://www.runpod.io/console/pods/${POD_ID}"
    echo ""
    echo "→ Run eval (paste ONE command):"
    echo ""
    echo "  curl -sL ${RUST_EVAL_URL} | bash"
    echo ""
    echo "→ Or via SSH:"
    echo "  ssh root@${SSH_HOST} -p ${SSH_PORT} 'curl -sL ${RUST_EVAL_URL} | bash'"
    echo ""
    echo "→ Monitor:"
    echo "  ssh root@${SSH_HOST} -p ${SSH_PORT} 'tail -f /workspace/rust_eval.log'"
    echo ""
    echo "Results saved to /workspace/eval_results/ on the pod."
    echo "==============================================="
else
    echo ""
    echo "==============================================="
    echo "POD READY: ${POD_ID}  |  \$${COST}/hr"
    echo ""
    echo "→ Open the pod dashboard and choose Connect → Web Terminal:"
    echo "  https://www.runpod.io/console/pods/${POD_ID}"
    echo ""
    echo "→ Paste this ONE command:"
    echo ""
    echo "  curl -sL ${TRAIN_URL} | bash"
    echo ""
    echo "→ Or via SSH (requires SSH key on your RunPod account):"
    echo "  ssh root@${SSH_HOST} -p ${SSH_PORT} 'curl -sL ${TRAIN_URL} | bash'"
    echo ""
    echo "→ Monitor:"
    echo "  ssh root@${SSH_HOST} -p ${SSH_PORT} 'tail -f /workspace/training.log'"
    echo ""
    echo "Dashboard: https://www.runpod.io/console/pods/${POD_ID}"
    echo "Results:   HF → Axolotl-Partners/qwen36-distill-opus-dsv4-lora"
    echo "==============================================="
fi
