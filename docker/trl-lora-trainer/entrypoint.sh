#!/usr/bin/env bash
# trl-lora-trainer entrypoint — parallel to axolotl-lora-trainer/entrypoint.sh
#
# Handles the full TRL training lifecycle:
# 1. pip install TRL + deps (pinned versions — see Lesson 12)
# 2. Write $HKASK_TRL_SCRIPT → /workspace/train.py
# 3. huggingface-cli login (if HF_TOKEN set)
# 4. python /workspace/train.py
# 5. huggingface-cli upload adapter → $HKASK_HF_MODEL_REPOSITORY
# 6. Write completion manifest (bash heredoc, no Python)
# 7. exec sleep infinity (SSH debugging)
#
# If training fails, writes a failure manifest and still exec sleep infinitys
# so the operator can SSH in to inspect logs.
set -euo pipefail

# ── Logging ─────────────────────────────────────────────────────────────────
exec > >(tee -a /workspace/logs/entrypoint.log) 2>&1
mkdir -p /workspace/logs /workspace/outputs

echo "=== hKask TRL Trainer Entrypoint ==="
echo "Job ID: ${HKASK_JOB_ID:-unknown}"
echo "Base model: ${HKASK_BASE_MODEL:-unknown}"
echo "Harness: ${HKASK_HARNESS:-trl}"
echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# ── Environment ────────────────────────────────────────────────────────────
# Critical: cache to workspace volume, not container disk (Lesson 1).
export HF_HOME=${HF_HOME:-/workspace/.cache/huggingface}
export PIP_CACHE_DIR=${PIP_CACHE_DIR:-/workspace/.cache/pip}
export TMPDIR=${TMPDIR:-/workspace/tmp}
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True
mkdir -p "$HF_HOME" "$PIP_CACHE_DIR" "$TMPDIR"

# ── Step 1: pip install TRL + deps (pinned versions) ──────────────────────
# Version pinning is critical (Lesson 12): TRL is under active development
# and API changes between versions can produce adapters that don't behave
# the same at inference. The rendered script includes a version assertion
# that fails fast if the installed version doesn't match.
#
# Pinning strategy:
# - trl: pinned to the latest stable release (v1.8.0 as of 2026-07-21)
# - peft: pinned to v0.19.0 (the version the lora-training skill is anchored to)
# - transformers: pinned to a compatible version (v5.9.0)
# - bitsandbytes: for QLoRA 4-bit quantization
# - accelerate: for distributed training support
# - liger-kernel: for memory-efficient kernels (optional but recommended)
echo "=== Installing TRL + dependencies (pinned versions) ==="
pip install --no-cache-dir \
    "trl==1.8.0" \
    "peft==0.19.0" \
    "transformers==5.9.0" \
    "bitsandbytes>=0.43.0" \
    "accelerate>=0.30.0" \
    "huggingface_hub>=0.24.0" \
    "liger-kernel>=0.3.0"

# ── Step 2: Write the TRL script ──────────────────────────────────────────
# The script is rendered by Rust (TrlHarness::render_config) and passed as
# the HKASK_TRL_SCRIPT env var. Write it to /workspace/train.py.
if [ -z "${HKASK_TRL_SCRIPT:-}" ]; then
    echo "FATAL: HKASK_TRL_SCRIPT not set — RunpodHost::submit() must render the script" >&2
    exit 1
fi
printf '%s' "$HKASK_TRL_SCRIPT" > /workspace/train.py
echo "=== TRL script written to /workspace/train.py ==="

# ── Step 3: HuggingFace login ─────────────────────────────────────────────
if [ -n "${HF_TOKEN:-}" ]; then
    echo "=== Logging in to HuggingFace ==="
    huggingface-cli login --token "$HF_TOKEN" --add-to-git-credential
else
    echo "WARNING: HF_TOKEN not set — cannot access private datasets or upload to private repos" >&2
fi

# ── Step 4: Run training ──────────────────────────────────────────────────
echo "=== Starting TRL training ==="
TRAINING_START=$(date +%s)
if python /workspace/train.py; then
    TRAINING_END=$(date +%s)
    TRAINING_DURATION=$((TRAINING_END - TRAINING_START))
    echo "=== TRL training completed in ${TRAINING_DURATION}s ==="
    TRAINING_STATUS="success"
else
    TRAINING_END=$(date +%s)
    TRAINING_DURATION=$((TRAINING_END - TRAINING_START))
    echo "=== TRL training FAILED after ${TRAINING_DURATION}s ===" >&2
    TRAINING_STATUS="failed"
fi

# ── Step 5: Upload adapter ─────────────────────────────────────────────────
OUTPUT_DIR="${HKASK_OUTPUT_DIR:-/workspace/outputs}"
if [ "$TRAINING_STATUS" = "success" ] && [ -n "${HKASK_HF_MODEL_REPOSITORY:-}" ]; then
    echo "=== Uploading adapter to ${HKASK_HF_MODEL_REPOSITORY} ==="
    if huggingface-cli upload "$HKASK_HF_MODEL_REPOSITORY" "$OUTPUT_DIR" --commit-message "hKask TRL training: ${HKASK_JOB_ID:-unknown}"; then
        echo "=== Adapter uploaded successfully ==="
    else
        echo "WARNING: Adapter upload failed — adapter remains at $OUTPUT_DIR" >&2
    fi
fi

# ── Step 6: Write completion manifest ──────────────────────────────────────
# Pure bash heredoc — no Python (Lesson 11: Rust-only tooling policy).
MANIFEST_PATH="${HKASK_COMPLETION_MANIFEST_PATH:-/workspace/completion.json}"
cat > "$MANIFEST_PATH" <<EOF
{
    "job_id": "${HKASK_JOB_ID:-unknown}",
    "base_model": "${HKASK_BASE_MODEL:-unknown}",
    "harness": "${HKASK_HARNESS:-trl}",
    "status": "${TRAINING_STATUS}",
    "training_duration_secs": ${TRAINING_DURATION},
    "output_dir": "${OUTPUT_DIR}",
    "trl_version": "1.8.0",
    "peft_version": "0.19.0",
    "transformers_version": "5.9.0",
    "completed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
echo "=== Completion manifest written to $MANIFEST_PATH ==="

# ── Step 7: Keep pod alive for SSH debugging ──────────────────────────────
echo "=== Entrypoint complete. Pod staying alive for SSH debugging. ==="
echo "To inspect: ssh into the pod, check /workspace/logs/entrypoint.log"
exec sleep infinity
