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
#   7. exec sleep infinity to keep pod alive for SSH debugging
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

# ── Setup (no set -e — we want to log errors and sleep, not exit) ───────────
# The entrypoint must NEVER exit on error. If it exits, RunPod restarts the
# container in a tight loop, and we lose the log. Instead, we log the error
# and fall through to `exec sleep infinity` so the pod stays alive for SSH
# debugging. See docs/how-to/runpod-lora-training-guide.md Lesson 1.
WORKSPACE=/workspace
CONFIG_PATH="${WORKSPACE}/config.yml"
DEFAULT_OUTPUT_DIR="${WORKSPACE}/outputs"
OUTPUT_DIR="${AXOLOTL_OUTPUT_DIR:-${DEFAULT_OUTPUT_DIR}}"
MANIFEST_PATH="${HKASK_COMPLETION_MANIFEST_PATH:-${WORKSPACE}/completion.json}"
LOG_DIR="${WORKSPACE}/logs"

# Create dirs — don't fail if this fails (workspace might not be mounted yet).
mkdir -p "${WORKSPACE}" "${OUTPUT_DIR}" "${LOG_DIR}" \
    "${WORKSPACE}/.cache/huggingface" "${WORKSPACE}/.cache/pip" "${WORKSPACE}/tmp" 2>/dev/null || true

# ── Critical environment variables (per docs/how-to/runpod-lora-training-guide.md Lesson 1) ─
# The RunPod container disk is only ~60GB. All caches MUST go to the 200GB+
# workspace volume, or the disk fills up during pip install / dataset
# tokenization and causes `No space left on device` → SIGSEGV crash → pod
# restart loop.
export HF_HOME="${WORKSPACE}/.cache/huggingface"
export PIP_CACHE_DIR="${WORKSPACE}/.cache/pip"
export TMPDIR="${WORKSPACE}/tmp"
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True
export HF_HUB_ENABLE_HF_TRANSFER=1

# Redirect all stdout+stderr to a log file AND the console.
# Also write a copy to /entrypoint.log on the container disk (always writable)
# in case /workspace isn't mounted yet.
exec > >(tee -a "${LOG_DIR}/entrypoint.log" /entrypoint.log) 2>&1
set -x  # trace every command for debugging

log() { printf '[entrypoint] %s\n' "$*" >&2; }

log "entrypoint started at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
log "WORKSPACE=${WORKSPACE}"
log "CONFIG_PATH=${CONFIG_PATH}"
log "OUTPUT_DIR=${OUTPUT_DIR}"
log "MANIFEST_PATH=${MANIFEST_PATH}"
log "LOG_DIR=${LOG_DIR}"
log "HF_HOME=${HF_HOME}"
log "PIP_CACHE_DIR=${PIP_CACHE_DIR}"
log "TMPDIR=${TMPDIR}"
log "disk space:"
df -h 2>/dev/null || true
log "memory:"
free -h 2>/dev/null || true
log "env vars:"
env | grep -E '^(HKASK_|HF_|AXOLOTL_|RUNPOD_)' | sort || true

# ── 1. Install axolotl + huggingface_hub at pod startup ─────────────────────
# Not baked into the image — keeps the image small and lets us pick up the
# latest axolotl/peft/transformers versions without rebuilding.
if ! command -v axolotl >/dev/null 2>&1; then
    log "pip installing axolotl + huggingface_hub (this takes a few minutes)"
    python3 -m pip install --no-cache-dir --upgrade pip wheel || log "WARN: pip upgrade failed, continuing"
    if ! python3 -m pip install --no-cache-dir axolotl huggingface_hub 2>"${LOG_DIR}/pip.log"; then
        log "pip install failed — retrying with build-essential (transient)"
        apt-get update && apt-get install -y --no-install-recommends build-essential || log "WARN: apt-get failed"
        python3 -m pip install --no-cache-dir axolotl huggingface_hub \
            || { log "ERROR: pip install failed — see ${LOG_DIR}/pip.log"; cat "${LOG_DIR}/pip.log" 2>/dev/null; exec sleep infinity; }
        apt-get purge -y build-essential 2>/dev/null || true
        apt-get autoremove -y 2>/dev/null || true
        rm -rf /var/lib/apt/lists/* 2>/dev/null || true
    fi
else
    log "axolotl already installed — skipping pip install"
fi

log "axolotl version: $(axolotl --version 2>/dev/null || echo 'unknown')"

# ── 2. Write the axolotl config from HKASK_AXOLOTL_CONFIG ────────────────────
if [ -z "${HKASK_AXOLOTL_CONFIG:-}" ]; then
    log "FATAL: HKASK_AXOLOTL_CONFIG is not set"
    log "RunpodHost::submit must render the config via AxolotlHarness::render_config"
    log "and pass it as HKASK_AXOLOTL_CONFIG. Sleeping for debugging."
    exec sleep infinity
fi
log "writing axolotl config to ${CONFIG_PATH}"
printf '%s\n' "${HKASK_AXOLOTL_CONFIG}" > "${CONFIG_PATH}"
log "config written, size=$(wc -c < "${CONFIG_PATH}") bytes, lines=$(wc -l < "${CONFIG_PATH}")"

# Resolve OUTPUT_DIR from the YAML (rendered by AxolotlHarness::output_dir).
# AXOLOTL_OUTPUT_DIR env var remains the override escape hatch.
if [ -z "${AXOLOTL_OUTPUT_DIR:-}" ]; then
    YAML_OUTPUT_DIR=$(grep -E '^output_dir:' "${CONFIG_PATH}" 2>/dev/null | head -1 | sed 's/^output_dir:[[:space:]]*//' || true)
    if [ -n "${YAML_OUTPUT_DIR}" ]; then
        OUTPUT_DIR="${YAML_OUTPUT_DIR}"
        mkdir -p "${OUTPUT_DIR}" 2>/dev/null || true
        log "resolved output_dir from config: ${OUTPUT_DIR}"
    fi
fi

log "=== axolotl config ==="
cat "${CONFIG_PATH}" || true
log "=== end config ==="

# ── 3. HuggingFace login (if token provided) ────────────────────────────────
if [ -n "${HF_TOKEN:-}" ]; then
    log "logging in to HuggingFace Hub"
    huggingface-cli login --token "${HF_TOKEN}" --add-to-git-credential 2>/dev/null || log "WARN: HF login failed"
else
    log "HF_TOKEN not set — skipping HuggingFace login"
fi

# ── 4. Run training ─────────────────────────────────────────────────────────
log "starting axolotl train (config: ${CONFIG_PATH})"
# axolotl reads base_model + datasets from the YAML and pulls them from
# HuggingFace at runtime (public or private via HF_TOKEN).
axolotl train "${CONFIG_PATH}" 2>&1 | tee "${LOG_DIR}/train.log" || true
TRAIN_RC=${PIPESTATUS[0]:-1}
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
    log "=== last 50 lines of train.log ==="
    tail -50 "${LOG_DIR}/train.log" 2>/dev/null || true
    log "=== sleeping for SSH debugging — cancel via RunpodHost::cancel ==="
    exec sleep infinity
fi

# ── 5. Upload adapter to HuggingFace ────────────────────────────────────────
ADAPTER_REPO="${HKASK_HF_MODEL_REPOSITORY:-}"
if [ -n "${ADAPTER_REPO}" ] && [ -n "${HF_TOKEN:-}" ]; then
    log "uploading adapter from ${OUTPUT_DIR} to ${ADAPTER_REPO}"
    huggingface-cli repo create "${ADAPTER_REPO}" --type model --exist-ok 2>/dev/null || true
    huggingface-cli upload "${ADAPTER_REPO}" "${OUTPUT_DIR}" --commit-message "hKask adapter upload (job ${HKASK_JOB_ID:-})" || log "WARN: adapter upload failed"
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
log "training complete — pod will sleep for SSH debugging (cancel via RunpodHost::cancel)"
exec sleep infinity
