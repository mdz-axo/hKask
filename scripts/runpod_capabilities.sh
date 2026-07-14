#!/bin/bash
# ============================================================================
# RunPod Training — Capabilities Researcher PiSSA LoRA
# ============================================================================
# PREREQUISITE: SSH key at ~/.ssh/id_ed25519 (add pubkey to RunPod settings)
#
# This script:
#   1. Launches an H100 NVL pod on RunPod (falls back to A100 80GB)
#   2. Waits for SSH access
#   3. SCPs training data + script to the pod
#   4. Prints the SSH command to start training
#
# Usage: bash scripts/runpod_capabilities.sh
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RUNPOD_API_KEY=$(grep '^RUNPOD_API_KEY=' "$REPO_ROOT/.env" | cut -d= -f2-)
HF_TOKEN=$(grep '^HF_TOKEN=' "$REPO_ROOT/.env" | cut -d= -f2-)
RUNPOD_PUBLIC_KEY=$(cat "$HOME/.ssh/id_ed25519.pub" 2>/dev/null || echo "")

if [ -z "$RUNPOD_API_KEY" ]; then echo "ERROR: RUNPOD_API_KEY not in .env"; exit 1; fi
if [ -z "$HF_TOKEN" ]; then echo "ERROR: HF_TOKEN not in .env"; exit 1; fi
if [ -z "$RUNPOD_PUBLIC_KEY" ]; then echo "ERROR: SSH key not at ~/.ssh/id_ed25519.pub"; exit 1; fi

export RUNPOD_API_KEY HF_TOKEN RUNPOD_PUBLIC_KEY

API="https://api.runpod.io/graphql?api_key=${RUNPOD_API_KEY}"
gql() { curl -s --max-time 30 -X POST "$API" -H "Content-Type: application/json" -d "$1"; }

# ── Build pod payload ──────────────────────────────────────────────────────
build_payload() {
    CLOUD_TYPE="$1" GPU_TYPE="$2" python3 << 'PYEOF'
import json, os
env = [
    {'key': 'HF_TOKEN', 'value': os.environ['HF_TOKEN']},
    {'key': 'RUNPOD_API_KEY', 'value': os.environ['RUNPOD_API_KEY']},
    {'key': 'PUBLIC_KEY', 'value': os.environ['RUNPOD_PUBLIC_KEY']},
    {'key': 'HF_HOME', 'value': '/workspace/.cache/huggingface'},
    {'key': 'PIP_CACHE_DIR', 'value': '/workspace/.cache/pip'},
    {'key': 'TMPDIR', 'value': '/workspace/tmp'},
    {'key': 'POD_INACTIVITY_TIMEOUT', 'value': '14400'},
    {'key': 'WANDB_DISABLED', 'value': 'true'},
    {'key': 'PYTORCH_CUDA_ALLOC_CONF', 'value': 'expandable_segments:True'},
    {'key': 'HF_DEACTIVATE_ASYNC_LOAD', 'value': '1'},
    {'key': 'HF_XET_HIGH_PERFORMANCE', 'value': '1'},
]
payload = {
    'query': 'mutation DeployPod($input: PodFindAndDeployOnDemandInput!) { podFindAndDeployOnDemand(input: $input) { id, name, costPerHr } }',
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
            'name': 'capabilities-researcher',
            'imageName': 'runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04',
            'startSsh': True,
            'ports': '22/tcp',
            'supportPublicIp': True,
            'env': env,
        }
    }
}
print(json.dumps(payload))
PYEOF
}

deploy() {
    CLOUD_TYPE="$1" GPU_TYPE="$2"
    echo "=== Launching ${GPU_TYPE} on ${CLOUD_TYPE} cloud ==="
    GQL_PAYLOAD=$(build_payload "$CLOUD_TYPE" "$GPU_TYPE")
    RESP=$(gql "$GQL_PAYLOAD")
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
    echo "H100 NVL unavailable; falling back to community A100 80GB PCIe."
    if ! deploy "COMMUNITY" "NVIDIA A100 80GB PCIe"; then
        echo "ERROR: No configured GPU is currently available."
        exit 1
    fi
fi

# ── Wait for pod ───────────────────────────────────────────────────────────
echo ""
echo "=== Waiting for boot (up to 5 min) ==="
SSH_HOST="" SSH_PORT=""
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
    echo "Pod not ready. Dashboard: https://www.runpod.io/console/pods/${POD_ID}"
    exit 1
fi

echo "Pod running at ${SSH_HOST}:${SSH_PORT}"

# ── Upload training data + script via SCP ──────────────────────────────────
echo ""
echo "=== Uploading training data ==="
ssh-keyscan -p "$SSH_PORT" "$SSH_HOST" >> ~/.ssh/known_hosts 2>/dev/null
scp -P "$SSH_PORT" \
    "$REPO_ROOT/corpus/qa_pairs/train_chat.jsonl" \
    "$REPO_ROOT/corpus/qa_pairs/val_chat.jsonl" \
    "$REPO_ROOT/scripts/train_capabilities_researcher.sh" \
    root@"${SSH_HOST}":/workspace/ 2>/dev/null

ssh -p "$SSH_PORT" root@"${SSH_HOST}" "mkdir -p /workspace/data && mv /workspace/train_chat.jsonl /workspace/val_chat.jsonl /workspace/data/"

echo "Data uploaded."

# ── Instructions ───────────────────────────────────────────────────────────
echo ""
echo "==============================================="
echo "POD READY: ${POD_ID}  |  \$${COST}/hr"
echo ""
echo "→ Dashboard:"
echo "  https://www.runpod.io/console/pods/${POD_ID}"
echo ""
echo "→ Start training (paste ONE command):"
echo ""
echo "  ssh -p ${SSH_PORT} root@${SSH_HOST} 'bash /workspace/train_capabilities_researcher.sh'"
echo ""
echo "→ Monitor:"
echo "  ssh -p ${SSH_PORT} root@${SSH_HOST} 'tail -f /workspace/training.log'"
echo ""
echo "→ Check eval loss:"
echo "  ssh -p ${SSH_PORT} root@${SSH_HOST} 'grep eval_loss /workspace/training.log'"
echo ""
echo "Results: /workspace/outputs/ on pod"
echo "==============================================="