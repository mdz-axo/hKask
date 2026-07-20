#!/usr/bin/env bash
# entrypoint.sh — Minimal Axolotl LoRA Trainer entrypoint
#
# This is the ONLY script in the image. It is pure bash (no Python config
# generator — the axolotl YAML is rendered by Rust `AxolotlHarness::render_config`
# and passed in the HKASK_AXOLOTL_CONFIG env var by `RunpodHost::submit`).
#
# Responsibilities (in order):
#   1. pip install axolotl + huggingface_hub at pod startup (NOT baked in image)
#   2. Write HKASK_AXOLOTL_CONFIG env var to /workspace/config.yml
#   3. Optionally pre-fetch the base model + dataset from HuggingFace
#   4. Run `axolotl train /workspace/config.yml`
#   5. Upload the adapter to HuggingFace via `huggingface-cli upload`
#   6. Write the completion manifest (JSON, written with bash — no Python)
#   7. exec sleep infinity so the pod stays alive for SSH debugging
#
# Env vars (set by RunpodHost::submit in mcp-servers/hkask-mcp-training):
#   HKASK_AXOLOTL_CONFIG          — full axolotl YAML (rendered by Rust)
#   HKASK_JOB_ID                  — hKask job id
#   HKASK_BASE_MODEL              — HuggingFace base model repo id
#   HKASK_HF_DATASET_REPOSITORY   — dataset repo id
#   HKASK_HF_DATASET_REVISION    — dataset revision (optional)
#   HKASK_HF_DATASET_PATH         — path within dataset repo (optional)
#   HKASK_HF_MODEL_REPOSITORY     — destination adapter repo on HuggingFace
#   HKASK_COMPLETION_MANIFEST_PATH — where to write the completion manifest
#   HF_TOKEN                       — HuggingFace token (private repos / upload)
#   AXOLOTL_OUTPUT_DIR            — override axolotl output dir (optional)
#
# The lora-training skill's gate catalog (G-M1..G-M5, G-Q1..G-Q6, G-D1..G-D3,
# G-F1..G-F2) is enforced upstream by `AxolotlHarness::render_config` and the
# `audit-config` phase — this entrypoint does not re-derive them. The
# `peft_init_lora_weights` value baked into the YAML (eva, pissa, lora_ga, etc.)
# selects the PEFT init method per the skill's method catalog.

set -euo pipefail

WORKSPACE=/workspace
CONFIG_PATH="${WORKSPACE}/config.yml"
# OUTPUT_DIR is resolved after the config is written (step 2) by parsing
# `output_dir:` from the YAML. AXOLOTL_OUTPUT_DIR is an override escape hatch.
DEFAULT_OUTPUT_DIR="${WORKSPACE}/outputs"
OUTPUT_DIR="${AXOLOTL_OUTPUT_DIR:-${DEFAULT_OUTPUT_DIR}}"
MANIFEST_PATH="${HKASK_COMPLETION_MANIFEST_PATH:-${WORKSPACE}/completion.json}"
LOG_DIR="${WORKSPACE}/logs"
mkdir -p "${WORKSPACE}" "${OUTPUT_DIR}" "${LOG_DIR}" \
    "${WORKSPACE}/.cache/huggingface" "${WORKSPACE}/.cache/pip" "${WORKSPACE}/tmp"

# ── Critical environment variables (per docs/how-to/axolotl-pissa-runpod-guide.md) ─
# The RunPod container disk is only ~60GB. All caches MUST go to the 200GB+
# workspace volume, or the disk fills up during pip install / dataset
# tokenization and causes `No space left on device` → SIGSEGV crash → pod
# restart loop. This was the root cause of the restart loops in earlier runs.
export HF_HOME="${WORKSPACE}/.cache/huggingface"
export PIP_CACHE_DIR="${WORKSPACE}/.cache/pip"
export TMPDIR="${WORKSPACE}/tmp"
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True
export HF_HUB_ENABLE_HF_TRANSFER=1  # faster HF downloads (hf_transfer is a dep of axolotl)

# Redirect all stdout+stderr to a log file AND the console (RunPod captures
# container stderr). This ensures we can inspect failures via the pod's
# log output even if the container exits.
exec > >(tee -a "${LOG_DIR}/entrypoint.log") 2>&1
set -x  # trace every command for debugging

log() { printf '[entrypoint] %s\n' "$*" >&2; }

# ── 1. Install axolotl + huggingface_hub at pod startup ─────────────────────
# Not baked into the image — keeps the image small and lets us pick up the
# latest axolotl/peft/transformers versions without rebuilding.
# If a wheel needs compilation (rare for the axolotl dependency tree on H100
# pods), we transiently install build-essential and remove it after.
if ! command -v axolotl >/dev/null 2>&1; then
    log "pip installing axolotl + huggingface_hub (this takes a few minutes)"
    python3 -m pip install --no-cache-dir --upgrade pip wheel
    if ! python3 -m pip install --no-cache-dir axolotl huggingface_hub 2>"${LOG_DIR}/pip.log"; then
        log "pip install failed — retrying with build-essential (transient)"
        apt-get update && apt-get install -y --no-install-recommends build-essential
        python3 -m pip install --no-cache-dir axolotl huggingface_hub \
            || { cat "${LOG_DIR}/pip.log" >&2; log "pip install failed — sleeping for debugging"; exec sleep infinity; }
        apt-get purge -y build-essential && apt-get autoremove -y
        rm -rf /var/lib/apt/lists/*
    fi
else
    log "axolotl already installed — skipping pip install"
fi

# ── 2. Write the axolotl config from HKASK_AXOLOTL_CONFIG ────────────────────
if [ -z "${HKASK_AXOLOTL_CONFIG:-}" ]; then
    log "FATAL: HKASK_AXOLOTL_CONFIG is not set"
    log "RunpodHost::submit must render the config via AxolotlHarness::render_config"
    log "and pass it as HKASK_AXOLOTL_CONFIG. Refusing to start training."
    log "sleeping for debugging — cancel via RunpodHost::cancel"
    exec sleep infinity
fi
log "writing axolotl config to ${CONFIG_PATH}"
printf '%s\n' "${HKASK_AXOLOTL_CONFIG}" > "${CONFIG_PATH}"

# Resolve OUTPUT_DIR from the YAML (rendered by AxolotlHarness::output_dir).
# AXOLOTL_OUTPUT_DIR env var remains the override escape hatch.
if [ -z "${AXOLOTL_OUTPUT_DIR:-}" ]; then
    YAML_OUTPUT_DIR=$(grep -E '^output_dir:' "${CONFIG_PATH}" 2>/dev/null | head -1 | sed 's/^output_dir:[[:space:]]*//' || true)
    if [ -n "${YAML_OUTPUT_DIR}" ]; then
        OUTPUT_DIR="${YAML_OUTPUT_DIR}"
        mkdir -p "${OUTPUT_DIR}"
        log "resolved output_dir from config: ${OUTPUT_DIR}"
    fi
fi

# ── 3. HuggingFace login (if token provided) ────────────────────────────────
if [ -n "${HF_TOKEN:-}" ]; then
    log "logging in to HuggingFace Hub"
    huggingface-cli login --token "${HF_TOKEN}" --add-to-git-credential 2>/dev/null || true
fi

# ── 4. Run training ─────────────────────────────────────────────────────────
log "starting axolotl train (config: ${CONFIG_PATH})"
# axolotl reads base_model + datasets from the YAML and pulls them from
# HuggingFace at runtime (public or private via HF_TOKEN).
axolotl train "${CONFIG_PATH}" 2>&1 | tee "${LOG_DIR}/train.log"
TRAIN_RC=${PIPESTATUS[0]}
log "axolotl train exited with rc=${TRAIN_RC}"

if [ "${TRAIN_RC}" -ne 0 ]; then
    log "training failed — writing failure manifest and sleeping for debugging"
    cat > "${MANIFEST_PATH}" <<EOF
{
  "job_id": "${HKASK_JOB_ID:-}",
  "base_model": "${HKASK_BASE_MODEL:-}",
  "status": "failed",
  "return_code": ${TRAIN_RC},
  "output_path": "${HKASK_HF_MODEL_REPOSITORY:-}"
}
EOF
    exec sleep infinity
fi

# ── 5. Upload adapter to HuggingFace ────────────────────────────────────────
ADAPTER_REPO="${HKASK_HF_MODEL_REPOSITORY:-}"
if [ -n "${ADAPTER_REPO}" ] && [ -n "${HF_TOKEN:-}" ]; then
    log "uploading adapter from ${OUTPUT_DIR} to ${ADAPTER_REPO}"
    huggingface-cli repo create "${ADAPTER_REPO}" --type model --exist-ok 2>/dev/null || true
    huggingface-cli upload "${ADAPTER_REPO}" "${OUTPUT_DIR}" --commit-message "hKask adapter upload (job ${HKASK_JOB_ID:-})"
    log "adapter uploaded to ${ADAPTER_REPO}"
elif [ -z "${ADAPTER_REPO}" ]; then
    log "HKASK_HF_MODEL_REPOSITORY not set — skipping adapter upload"
elif [ -z "${HF_TOKEN:-}" ]; then
    log "HF_TOKEN not set — skipping adapter upload (adapter remains in ${OUTPUT_DIR})"
fi

# ── 6. Write completion manifest (pure bash, no Python) ──────────────────────
log "writing completion manifest to ${MANIFEST_PATH}"
cat > "${MANIFEST_PATH}" <<EOF
{
  "job_id": "${HKASK_JOB_ID:-}",
  "base_model": "${HKASK_BASE_MODEL:-}",
  "status": "completed",
  "return_code": 0,
  "output_path": "${ADAPTER_REPO}",
  "adapter_local_path": "${OUTPUT_DIR}",
  "config_path": "${CONFIG_PATH}"
}
EOF

# ── 7. Keep the pod alive for SSH debugging ─────────────────────────────────
# RunPod pods terminate when the entrypoint exits. We `exec sleep infinity` so
# the operator can SSH in and inspect /workspace/outputs and /workspace/logs
# after training completes. RunpodHost::cancel() tears the pod down via the
# GraphQL API when the operator is done.
log "training complete — pod will sleep for SSH debugging (cancel via RunpodHost::cancel)"
exec sleep infinity
