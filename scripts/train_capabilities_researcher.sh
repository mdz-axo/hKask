#!/bin/bash
# ============================================================================
# Capabilities Researcher — Axolotl + PiSSA LoRA Training — runs on RunPod pod
# ============================================================================
# Image: runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04
# Usage: bash /workspace/train_capabilities_researcher.sh
#
# Uses Axolotl CLI with PiSSA LoRA initialization.
# Liger Kernel + Cut Cross Entropy + SDPA attention.
# Data: local ChatML JSONL files uploaded via SCP.
# ============================================================================
set -euo pipefail
exec > /workspace/training.log 2>&1

trap 'cp /workspace/training.log /workspace/crash_$(date +%s).log 2>/dev/null || true' ERR TERM

# Read env vars from pod environment (set during pod creation)
if [ -r /proc/1/environ ]; then
    while IFS= read -r -d '' entry; do
        case "$entry" in
            HF_TOKEN=*|RUNPOD_API_KEY=*) export "${entry?}" ;;
        esac
    done < /proc/1/environ
fi

echo "============================================"
echo "Capabilities Researcher | Axolotl + PiSSA | $(date -u)"
echo "GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null || echo 'unknown')"
echo "============================================"

export DEBIAN_FRONTEND=noninteractive
export PATH="$HOME/.local/bin:$PATH"
export HF_HOME=/workspace/.cache/huggingface
export PIP_CACHE_DIR=/workspace/.cache/pip
export TMPDIR=/workspace/tmp
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True
export HF_DEACTIVATE_ASYNC_LOAD=1
export HF_XET_HIGH_PERFORMANCE=1
# Axolotl installs PyTorch cu130 but the RunPod image has CUDA 12.4.
# libnvJitLink.so.13 lives in the nvidia/cu13 pip package — must add to LD_LIBRARY_PATH.
export LD_LIBRARY_PATH=/usr/local/lib/python3.11/dist-packages/nvidia/cu13/lib:${LD_LIBRARY_PATH:-}
mkdir -p "$HF_HOME" "$PIP_CACHE_DIR" "$TMPDIR"

echo "" && echo "=== SYSTEM ==="
apt-get update -qq && apt-get install -y -qq git build-essential >/dev/null 2>&1
pip install --cache-dir "$PIP_CACHE_DIR" -q --upgrade pip setuptools wheel

echo "=== INSTALLING AXOLOTL ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q axolotl

# Fix torchvision version mismatch — axolotl install may upgrade PyTorch to cu130
# but leave torchvision at cu124, causing 'operator torchvision::nms does not exist'
pip install --cache-dir "$PIP_CACHE_DIR" -q "torchvision>=0.24.0" --index-url https://download.pytorch.org/whl/cu130 2>/dev/null || true

echo "=== VERIFY KERNELS ==="
python3 -c "
import torch
print(f'Torch: {torch.__version__} | CUDA: {torch.version.cuda}')
print(f'BF16: {torch.cuda.is_bf16_supported()}')
try:
    import liger_kernel
    print(f'Liger Kernel: {liger_kernel.__version__}')
except: print('Liger Kernel: NOT FOUND')
try:
    import fla, tilelang
    print('FLA kernels: OK')
except: print('FLA kernels: not installed')
"

echo "Install complete."
cd /workspace

# ── Write Axolotl config ───────────────────────────────────────────────────
cat > /workspace/axolotl_config.yml << 'YAMLEOF'
# Axolotl Config — Capabilities Researcher LoRA with PiSSA
# Base: unsloth/Qwen3.6-27B (dense, 27B)
# Training data: 64,757 ChatML QAs (Capabilities Researcher corpus)
#
# Lessons from docs/how-to/axolotl-pissa-runpod-guide.md:
#   - flash_attention: false → SDPA (no flash-attn compile needed)
#   - sample_packing disabled (requires flash-attn for cross-sample masking)
#   - gradient_checkpointing essential for 27B on single H100
#   - eval_batch_size: 1 prevents OOM during eval
#   - early_stopping_patience: 25 (not 7 — cosine LR needs runway)
#   - lora_dropout: 0 required for PiSSA (principal components must not be dropped)
#   - save_total_limit: 5 preserves best checkpoint

base_model: unsloth/Qwen3.6-27B
adapter: lora
load_in_4bit: false
load_in_8bit: false
trust_remote_code: true

sequence_len: 4096
bf16: true

# PiSSA LoRA — SVD-based initialization from principal singular values
# 30-50% faster convergence vs random init. Free lunch (~2 min init).
lora_r: 16
lora_alpha: 32
lora_dropout: 0
peft_init_lora_weights: pissa_niter_4
lora_target_modules:
  - q_proj
  - k_proj
  - v_proj
  - o_proj
  - gate_proj
  - up_proj
  - down_proj

datasets:
  - path: /workspace/data/train_chat.jsonl
    type: chat_template
  - path: /workspace/data/val_chat.jsonl
    type: chat_template

# Training
num_epochs: 3
learning_rate: 1e-4
warmup_steps: 100
micro_batch_size: 1
eval_batch_size: 1
gradient_accumulation_steps: 16
gradient_checkpointing: true
lr_scheduler: cosine
weight_decay: 0.01
max_grad_norm: 0.3
optim: adamw_8bit

# Eval
val_set_size: 0.0012
eval_steps: 200
save_steps: 200
save_total_limit: 5
early_stopping_patience: 25

# Optimizations (from axolotl-pissa-runpod-guide.md)
liger_kernel: true
flash_attention: false
cut_cross_entropy: true

# Output — no hub_model_id (avoids 403 if HF namespace doesn't exist yet).
# Script uploads to HF manually after training completes.
output_dir: /workspace/outputs

# Misc
strict: false
YAMLEOF

echo "=== STARTING AXOLOTL TRAINING ==="
echo "Config: /workspace/axolotl_config.yml"
echo "Data: /workspace/data/train_chat.jsonl (+ val_chat.jsonl)"

set +e
axolotl train /workspace/axolotl_config.yml
EXIT_CODE=$?
set -e

echo "=== FINISHED (exit=${EXIT_CODE}) ==="

if [ "${EXIT_CODE}" -eq 0 ]; then
    # Upload to HuggingFace
    if [ -n "${HF_TOKEN:-}" ]; then
        echo "Uploading to HuggingFace..."
        python3 << 'PYEOF'
import os
from huggingface_hub import HfApi
api = HfApi(token=os.environ["HF_TOKEN"])
repo = "mdz-axolotl/capabilities-researcher-pissa-lora"
api.create_repo(repo_id=repo, repo_type="model", exist_ok=True)
api.upload_folder(folder_path="/workspace/outputs", path_in_repo=".", repo_id=repo, repo_type="model")
print(f"Uploaded to {repo}")
PYEOF
    fi

    # Auto-terminate pod
    POD_ID=$(hostname 2>/dev/null || echo "")
    if [ -n "${POD_ID}" ] && [ -n "${RUNPOD_API_KEY:-}" ]; then
        echo "Training OK. Terminating pod ${POD_ID} in 60s..."
        sleep 60
        curl -s --max-time 30 -X POST \
            "https://api.runpod.io/graphql?api_key=${RUNPOD_API_KEY}" \
            -H "Content-Type: application/json" \
            -d "{\"query\":\"mutation { podTerminate(input: {podId: \\\"${POD_ID}\\\"}) }\"}"
        echo "Pod terminated."
    fi
else
    echo "Training FAILED (exit=${EXIT_CODE}). Pod kept alive for debugging."
    echo "Check /workspace/training.log and /workspace/crash_*.log"
fi