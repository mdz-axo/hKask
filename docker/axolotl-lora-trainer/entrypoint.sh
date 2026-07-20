#!/bin/bash
# =============================================================================
# hKask Axolotl LoRA Trainer — Training Script
# =============================================================================
# This script is called by the base image's entrypoint when MODE=axolotl-train.
# It runs inside the container after the base image has set up CUDA, Python, etc.
# =============================================================================

set -euo pipefail

echo "============================================"
echo "hKask Axolotl LoRA Trainer"
echo "============================================"
echo "Base model: $HKASK_BASE_MODEL"
echo "Dataset: $HKASK_HF_DATASET_REPO/$HKASK_HF_DATASET_FILE"
echo "Model repo: $HKASK_HF_MODEL_REPO"
echo "LoRA: r=$HKASK_LORA_R alpha=$HKASK_LORA_ALPHA init=$HKASK_PEFT_INIT"
echo "Training: epochs=$HKASK_NUM_EPOCHS lr=$HKASK_LEARNING_RATE"
echo "============================================"
echo

# ── Step 0: Validate required env vars ────────────────────────────────────
if [ -z "$HKASK_BASE_MODEL" ]; then
    echo "ERROR: HKASK_BASE_MODEL is required"
    exit 1
fi
if [ -z "$HKASK_HF_DATASET_REPO" ]; then
    echo "ERROR: HKASK_HF_DATASET_REPO is required"
    exit 1
fi
if [ -z "$HKASK_HF_MODEL_REPO" ]; then
    echo "ERROR: HKASK_HF_MODEL_REPO is required"
    exit 1
fi

# ── Step 1: Set up HuggingFace authentication ─────────────────────────────
if [ -n "$HKASK_HF_TOKEN" ]; then
    echo "Logging in to HuggingFace..."
    python3 -c "
from huggingface_hub import login
import os
login(token=os.environ['HKASK_HF_TOKEN'])
print('HuggingFace login successful')
"
else
    echo "Warning: HKASK_HF_TOKEN not set, using anonymous access"
fi

# ── Step 2: Download dataset from HuggingFace ─────────────────────────────
mkdir -p "$HKASK_WORKSPACE/data"
DATASET_PATH="$HKASK_WORKSPACE/data/$HKASK_HF_DATASET_FILE"

echo "Downloading dataset from HuggingFace..."
python3 -c "
from huggingface_hub import hf_hub_download
import shutil, os

path = hf_hub_download(
    repo_id=os.environ['HKASK_HF_DATASET_REPO'],
    filename=os.environ['HKASK_HF_DATASET_FILE'],
    repo_type='dataset',
    revision=os.environ.get('HKASK_HF_DATASET_REVISION', 'main'),
    token=os.environ.get('HKASK_HF_TOKEN') or None,
)
shutil.copy(path, os.environ['DATASET_PATH'])
size = os.path.getsize(os.environ['DATASET_PATH'])
print(f'Dataset downloaded: {size} bytes ({size / 1024 / 1024:.1f} MB)')
"

# ── Step 3: Generate axolotl config ───────────────────────────────────────
echo "Generating axolotl config..."
python3 /workspace/generate_config.py > "$HKASK_WORKSPACE/config.yaml"
echo "Config written to $HKASK_WORKSPACE/config.yaml"
echo "--- Config preview ---"
head -20 "$HKASK_WORKSPACE/config.yaml"
echo "..."
echo

# ── Step 4: Run training ──────────────────────────────────────────────────
echo "============================================"
echo "Starting axolotl training"
echo "============================================"
cd "$HKASK_WORKSPACE"

# Run training, capture exit code
set +e
axolotl train "$HKASK_WORKSPACE/config.yaml" 2>&1 | tee "$HKASK_WORKSPACE/training.log"
TRAIN_EXIT=$?
set -e

if [ $TRAIN_EXIT -eq 0 ]; then
    echo "Training completed successfully"
else
    echo "Training failed with exit code $TRAIN_EXIT"
    echo "Check $HKASK_WORKSPACE/training.log for details"
fi

# ── Step 5: Upload adapter to HuggingFace ───────────────────────────────
if [ -d "$HKASK_OUTPUT_DIR" ] && [ "$(ls -A $HKASK_OUTPUT_DIR 2>/dev/null)" ]; then
    echo "Uploading adapter to HuggingFace..."
    python3 -c "
from huggingface_hub import HfApi, create_repo
import os

api = HfApi(token=os.environ.get('HKASK_HF_TOKEN') or None)
repo = os.environ['HKASK_HF_MODEL_REPO']

try:
    create_repo(repo_id=repo, repo_type='model', exist_ok=True, token=os.environ.get('HKASK_HF_TOKEN') or None)
    print(f'Repository ready: {repo}')
except Exception as e:
    print(f'Repository creation: {e}')

api.upload_folder(
    folder_path=os.environ['HKASK_OUTPUT_DIR'],
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
    'job_id': os.environ.get('HKASK_JOB_ID', 'unknown'),
    'status': 'completed' if $TRAIN_EXIT == 0 else 'failed',
    'base_model': os.environ.get('HKASK_BASE_MODEL', ''),
    'dataset_repo': os.environ.get('HKASK_HF_DATASET_REPO', ''),
    'dataset_file': os.environ.get('HKASK_HF_DATASET_FILE', ''),
    'model_repo': os.environ.get('HKASK_HF_MODEL_REPO', ''),
    'lora_r': int(os.environ.get('HKASK_LORA_R', 16)),
    'lora_alpha': int(os.environ.get('HKASK_LORA_ALPHA', 32)),
    'num_epochs': int(os.environ.get('HKASK_NUM_EPOCHS', 3)),
    'finished_at': datetime.datetime.utcnow().isoformat() + 'Z',
}

manifest_path = os.path.join(os.environ.get('HKASK_WORKSPACE', '/workspace'), 'completion.json')
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
