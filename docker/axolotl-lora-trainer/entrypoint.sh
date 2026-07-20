#!/bin/bash
# =============================================================================
# hKask Axolotl LoRA Trainer — Training Script
# =============================================================================
# This script is called by the base image's entrypoint when MODE=axolotl-train.
# It runs inside the container after the base image has set up CUDA, Python, etc.
#
# Environment variables (set by RunpodHost::submit in Rust):
#   HKASK_BASE_MODEL              — HuggingFace base model ID
#   HKASK_HF_DATASET_REPOSITORY   — HuggingFace dataset repository
#   HKASK_HF_DATASET_PATH         — Dataset file within the repo
#   HKASK_HF_DATASET_REVISION     — Dataset revision/branch (default: main)
#   HKASK_HF_MODEL_REPOSITORY     — HuggingFace model repo for adapter upload
#   HKASK_HF_TOKEN                — HuggingFace API token (set via pod env)
#   HKASK_JOB_ID                  — Training job ID
#   HKASK_COMPLETION_MANIFEST_PATH — Path for completion manifest
#   HKASK_LORA_R                  — LoRA rank
#   HKASK_LORA_ALPHA              — LoRA alpha
#   HKASK_LORA_DROPOUT            — LoRA dropout
#   HKASK_LORA_TARGET_MODULES     — Comma-separated target modules
#   HKASK_LORA_INIT_WEIGHTS       — Init method (true, eva, pissa)
#   HKASK_NUM_EPOCHS              — Number of training epochs
#   HKASK_LEARNING_RATE           — Learning rate
#   HKASK_BATCH_SIZE              — Micro batch size
#   HKASK_GRAD_ACCUM              — Gradient accumulation steps
#   HKASK_WEIGHT_DECAY            — Weight decay
#   HKASK_MAX_GRAD_NORM           — Max gradient norm
#   HKASK_WARMUP_STEPS            — Warmup steps
#   HKASK_LR_SCHEDULER            — LR scheduler
#   HKASK_SEQ_LEN                 — Max sequence length
# =============================================================================

set -euo pipefail

# ── Map Rust env var names to local vars ──────────────────────────────────
BASE_MODEL="${HKASK_BASE_MODEL:-}"
DATASET_REPO="${HKASK_HF_DATASET_REPOSITORY:-}"
DATASET_FILE="${HKASK_HF_DATASET_PATH:-train_chat_full.jsonl}"
DATASET_REVISION="${HKASK_HF_DATASET_REVISION:-main}"
MODEL_REPO="${HKASK_HF_MODEL_REPOSITORY:-}"
HF_TOKEN="${HKASK_HF_TOKEN:-}"
JOB_ID="${HKASK_JOB_ID:-unknown}"
MANIFEST_PATH="${HKASK_COMPLETION_MANIFEST_PATH:-/workspace/completion.json}"

LORA_R="${HKASK_LORA_R:-16}"
LORA_ALPHA="${HKASK_LORA_ALPHA:-32}"
LORA_DROPOUT="${HKASK_LORA_DROPOUT:-0}"
LORA_TARGET_MODULES="${HKASK_LORA_TARGET_MODULES:-q_proj,k_proj,v_proj,o_proj,gate_proj,up_proj,down_proj}"
PEFT_INIT="${HKASK_LORA_INIT_WEIGHTS:-true}"

NUM_EPOCHS="${HKASK_NUM_EPOCHS:-3}"
LEARNING_RATE="${HKASK_LEARNING_RATE:-0.0001}"
WARMUP_STEPS="${HKASK_WARMUP_STEPS:-100}"
MICRO_BATCH_SIZE="${HKASK_BATCH_SIZE:-1}"
GRAD_ACCUM="${HKASK_GRAD_ACCUM:-16}"
WEIGHT_DECAY="${HKASK_WEIGHT_DECAY:-0.01}"
MAX_GRAD_NORM="${HKASK_MAX_GRAD_NORM:-0.3}"
LR_SCHEDULER="${HKASK_LR_SCHEDULER:-cosine}"
SEQ_LEN="${HKASK_SEQ_LEN:-4096}"

echo "============================================"
echo "hKask Axolotl LoRA Trainer"
echo "============================================"
echo "Job ID: $JOB_ID"
echo "Base model: $BASE_MODEL"
echo "Dataset: $DATASET_REPO/$DATASET_FILE"
echo "Model repo: $MODEL_REPO"
echo "LoRA: r=$LORA_R alpha=$LORA_ALPHA init=$PEFT_INIT"
echo "Training: epochs=$NUM_EPOCHS lr=$LEARNING_RATE"
echo "============================================"
echo

# ── Step 0: Validate required env vars ────────────────────────────────────
if [ -z "$BASE_MODEL" ]; then
    echo "ERROR: HKASK_BASE_MODEL is required"
    exit 1
fi
if [ -z "$DATASET_REPO" ]; then
    echo "ERROR: HKASK_HF_DATASET_REPOSITORY is required"
    exit 1
fi
if [ -z "$MODEL_REPO" ]; then
    echo "ERROR: HKASK_HF_MODEL_REPOSITORY is required"
    exit 1
fi

# ── Step 1: Set up HuggingFace authentication ─────────────────────────────
if [ -n "$HF_TOKEN" ]; then
    echo "Logging in to HuggingFace..."
    python3 -c "
from huggingface_hub import login
import os
login(token=os.environ['HF_TOKEN'])
print('HuggingFace login successful')
"
else
    echo "Warning: HKASK_HF_TOKEN not set, using anonymous access"
fi

# ── Step 2: Download dataset from HuggingFace ─────────────────────────────
mkdir -p /workspace/data
DATASET_PATH="/workspace/data/$DATASET_FILE"

echo "Downloading dataset from HuggingFace..."
python3 -c "
from huggingface_hub import hf_hub_download
import shutil, os

path = hf_hub_download(
    repo_id=os.environ['DATASET_REPO'],
    filename=os.environ['DATASET_FILE'],
    repo_type='dataset',
    revision=os.environ.get('DATASET_REVISION', 'main'),
    token=os.environ.get('HF_TOKEN') or None,
)
shutil.copy(path, os.environ['DATASET_PATH'])
size = os.path.getsize(os.environ['DATASET_PATH'])
print(f'Dataset downloaded: {size} bytes ({size / 1024 / 1024:.1f} MB)')
"

# ── Step 3: Generate axolotl config ───────────────────────────────────────
echo "Generating axolotl config..."
python3 -c "
import yaml, os

config = {
    'base_model': os.environ['BASE_MODEL'],
    'adapter': 'lora',
    'load_in_4bit': False,
    'load_in_8bit': False,
    'trust_remote_code': True,
    'sequence_len': int(os.environ.get('SEQ_LEN', 4096)),
    'bf16': True,
    'lora_r': int(os.environ.get('LORA_R', 16)),
    'lora_alpha': int(os.environ.get('LORA_ALPHA', 32)),
    'lora_dropout': float(os.environ.get('LORA_DROPOUT', 0.0)),
    'lora_target_modules': [m.strip() for m in os.environ.get('LORA_TARGET_MODULES', 'q_proj,k_proj,v_proj,o_proj,gate_proj,up_proj,down_proj').split(',')],
    'datasets': [{
        'path': os.environ['DATASET_REPO'],
        'data_files': os.environ['DATASET_FILE'],
        'type': 'chat_template',
    }],
    'num_epochs': int(os.environ.get('NUM_EPOCHS', 3)),
    'learning_rate': float(os.environ.get('LEARNING_RATE', 0.0001)),
    'warmup_steps': int(os.environ.get('WARMUP_STEPS', 100)),
    'micro_batch_size': int(os.environ.get('MICRO_BATCH_SIZE', 1)),
    'eval_batch_size': 1,
    'gradient_accumulation_steps': int(os.environ.get('GRAD_ACCUM', 16)),
    'gradient_checkpointing': True,
    'lr_scheduler': os.environ.get('LR_SCHEDULER', 'cosine'),
    'weight_decay': float(os.environ.get('WEIGHT_DECAY', 0.01)),
    'max_grad_norm': float(os.environ.get('MAX_GRAD_NORM', 0.3)),
    'optim': 'adamw_8bit',
    'val_set_size': 0.05,
    'eval_steps': 200,
    'save_steps': 200,
    'save_total_limit': 5,
    'early_stopping_patience': 25,
    'liger_kernel': True,
    'flash_attention': False,
    'cut_cross_entropy': True,
    'output_dir': '/workspace/outputs',
    'strict': False,
    'hub_model_id': os.environ.get('MODEL_REPO', ''),
}

# EVA initialization
peft_init = os.environ.get('PEFT_INIT', 'true')
if peft_init == 'eva':
    config['peft_init_lora_weights'] = 'eva'
    config['eva_config'] = {
        'dataloader': os.environ['DATASET_REPO'],
        'rho': 2.5,
    }
elif peft_init == 'pissa':
    config['peft_init_lora_weights'] = 'pissa_niter_4'

with open('/workspace/config.yaml', 'w') as f:
    yaml.dump(config, f, default_flow_style=False, sort_keys=False)
print('Config written')
"

echo "Config written to /workspace/config.yaml"
echo "--- Config preview ---"
head -20 /workspace/config.yaml
echo "..."
echo

# ── Step 4: Run training ──────────────────────────────────────────────────
echo "============================================"
echo "Starting axolotl training"
echo "============================================"
cd /workspace

# Run training, capture exit code
set +e
axolotl train /workspace/config.yaml 2>&1 | tee /workspace/training.log
TRAIN_EXIT=$?
set -e

if [ $TRAIN_EXIT -eq 0 ]; then
    echo "Training completed successfully"
else
    echo "Training failed with exit code $TRAIN_EXIT"
    echo "Check /workspace/training.log for details"
fi

# ── Step 5: Upload adapter to HuggingFace ───────────────────────────────
if [ -d /workspace/outputs ] && [ "$(ls -A /workspace/outputs 2>/dev/null)" ]; then
    echo "Uploading adapter to HuggingFace..."
    python3 -c "
from huggingface_hub import HfApi, create_repo
import os

api = HfApi(token=os.environ.get('HF_TOKEN') or None)
repo = os.environ['MODEL_REPO']

try:
    create_repo(repo_id=repo, repo_type='model', exist_ok=True, token=os.environ.get('HF_TOKEN') or None)
    print(f'Repository ready: {repo}')
except Exception as e:
    print(f'Repository creation: {e}')

api.upload_folder(
    folder_path='/workspace/outputs',
    repo_id=repo,
    repo_type='model',
)
print(f'Adapter uploaded to {repo}')
" || echo "Upload failed (adapter may still be on the pod)"
else
    echo "No output directory found, skipping upload"
fi

# ── Step 6: Write completion manifest ─────────────────────────────────────
echo "Writing completion manifest..."
python3 -c "
import json, os, datetime

manifest = {
    'job_id': os.environ.get('JOB_ID', 'unknown'),
    'status': 'completed' if $TRAIN_EXIT == 0 else 'failed',
    'base_model': os.environ.get('BASE_MODEL', ''),
    'dataset_repo': os.environ.get('DATASET_REPO', ''),
    'dataset_file': os.environ.get('DATASET_FILE', ''),
    'model_repo': os.environ.get('MODEL_REPO', ''),
    'lora_r': int(os.environ.get('LORA_R', 16)),
    'lora_alpha': int(os.environ.get('LORA_ALPHA', 32)),
    'num_epochs': int(os.environ.get('NUM_EPOCHS', 3)),
    'finished_at': datetime.datetime.utcnow().isoformat() + 'Z',
}

manifest_path = os.environ.get('MANIFEST_PATH', '/workspace/completion.json')
with open(manifest_path, 'w') as f:
    json.dump(manifest, f, indent=2)
print(f'Manifest written to {manifest_path}')
" || echo "Manifest write failed"

# ── Step 7: Keep container alive for SSH debugging ────────────────────────
echo "============================================"
echo "Training complete (exit code: $TRAIN_EXIT)"
echo "Container staying alive for SSH debugging."
echo "To terminate: use RunPod console or drain_all_pods"
echo "============================================"

# Keep the container alive
exec sleep infinity
